//! PLAT-WIRE-1: real migrated PostgreSQL samples, online predicate and death writer.
use crate::bestiary_postgres_harness::{
    Harness, TestResult, WORLD, configured_admin, debug, fence, id, runtime,
};
use crate::durability::character_death::CharacterDeathOutcome;
use sqlx::Row;

fn run(
    name: &'static str,
    case: impl AsyncFnOnce(&Harness) -> TestResult + Send + 'static,
) -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            runtime()
                .map_err(debug)?
                .block_on(async {
                    let h = Harness::create(admin, name, false).await?;
                    let result = case(&h).await;
                    h.cleanup().await?;
                    result
                })
                .map_err(debug)
        })?
        .join()
        .map_err(|_| "platform totals test thread failed")?
        .map_err(Into::into)
}

#[test]
fn platform_world_totals_are_durable_samples_of_online_players_and_committed_deaths() -> TestResult
{
    run("platform_totals", async |h| {
        sqlx::query("INSERT INTO game_character_progression_state VALUES (encode($1,'hex')::uuid,1,9,6500,'profile-1','ruleset-1','content-1','simulation-1','evidence-1','declaration-1','policy-1','reward-1')")
            .bind(id(41).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_public_world_totals_sample(encode($1,'hex')::uuid)")
            .bind(id(WORLD).as_slice())
            .execute(&h.pool)
            .await?;
        let first = sqlx::query("SELECT * FROM game_public_world_totals_samples")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(first.try_get::<i64, _>("online_players")?, 0); // control loss
        assert_eq!(first.try_get::<i64, _>("player_deaths")?, 0);
        // Occurrence replay counts the committed death once.
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let death = crate::character_progression_postgres_cases::pvp_death_request(99)?;
        let committed = h
            .root
            .commit_character_death(&authority, &h.node, fence(1)?, death.clone())
            .await
            .map_err(debug)?;
        let replay = h
            .root
            .commit_character_death(&authority, &h.node, fence(1)?, death)
            .await
            .map_err(debug)?;
        assert!(
            matches!((committed, replay), (CharacterDeathOutcome::Committed(a), CharacterDeathOutcome::AlreadyCommitted(b)) if a == b)
        );
        let deaths: i64 = sqlx::query_scalar("SELECT player_deaths FROM game_public_world_deaths")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(deaths, 1);
        // Owner moves only the scheduler clock to avoid a wall-clock sleep in the test.
        sqlx::query("UPDATE game_public_world_totals_heads SET sampled_at=0")
            .execute(&h.pool)
            .await?;
        h.root
            .sample_public_world_totals(&authority, "")
            .await
            .map_err(debug)?;
        let samples = h
            .root
            .next_public_world_totals(&authority)
            .await
            .map_err(debug)?;
        assert_eq!(samples.len(), 2);
        assert_eq!(samples[0].player_deaths, 0);
        assert_eq!(samples[1].player_deaths, 1);
        assert_eq!(samples[1].projection_revision, 2);
        // A later unchanged count is still a new series sample, not a coalesced snapshot.
        sqlx::query("UPDATE game_public_world_totals_heads SET sampled_at=0")
            .execute(&h.pool)
            .await?;
        h.root
            .sample_public_world_totals(&authority, "")
            .await
            .map_err(debug)?;
        let samples = h
            .root
            .next_public_world_totals(&authority)
            .await
            .map_err(debug)?;
        assert_eq!(samples.len(), 3);
        assert_eq!(samples[2].projection_revision, 3);
        assert_eq!(samples[2].player_deaths, samples[1].player_deaths);
        assert_eq!(samples[2].online_players, samples[1].online_players);
        // Without ACK (including after a reconnect/restart), persisted payloads are identical.
        assert_eq!(
            samples,
            h.root
                .next_public_world_totals(&authority)
                .await
                .map_err(debug)?
        );
        h.root
            .ack_public_world_totals(&authority, &samples[0])
            .await
            .map_err(debug)?;
        assert_eq!(
            h.root
                .next_public_world_totals(&authority)
                .await
                .map_err(debug)?,
            samples[1..]
        );
        Ok(())
    })
}

#[test]
fn platform_world_totals_online_watermark_resync_and_concurrent_sampling() -> TestResult {
    run("platform_totals_replay", async |h| {
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        assert!(
            h.root
                .public_world_totals_facts(&authority)
                .await
                .map_err(debug)?
                .is_none()
        );
        // The real online predicate: fresh admission, live transport, no control loss.
        sqlx::query("INSERT INTO game_durability_fresh_admission_receipts SELECT \
            '\\x01'::bytea || decode(repeat('00',32),'hex'),game_session_id,account_id,character_id,world_id,runtime_scope_channel_id,character_lease_generation,scope_ownership_generation,1,current_transport_ref,1,'{}',1 \
            FROM game_durability_reconnect_sessions ON CONFLICT DO NOTHING")
            .execute(&h.pool).await?;
        sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state=2,control_loss_epoch=NULL,original_grace_deadline=NULL,predecessor_generation=NULL,fresh_replay_key='\\x01'::bytea || decode(repeat('00',32),'hex')")
            .execute(&h.pool).await?;
        // Two independent connections exercise the database's shared sampler lock.
        let sample = "SELECT game_public_world_totals_sample(encode($1,'hex')::uuid)";
        let world = id(WORLD);
        let (a, b) = crate::character_progression_postgres_cases::join_pvp_writers(
            sqlx::query(sample).bind(world.as_slice()).execute(&h.pool),
            sqlx::query(sample).bind(world.as_slice()).execute(&h.pool),
        )
        .await;
        a?;
        b?;
        let samples = h
            .root
            .next_public_world_totals(&authority)
            .await
            .map_err(debug)?;
        assert_eq!(samples.len(), 1);
        assert_eq!(samples[0].online_players, 1);
        let pending = h
            .root
            .public_world_totals_facts(&authority)
            .await
            .map_err(debug)?
            .ok_or("facts")?;
        assert_eq!(pending.complete_through, samples[0].source_observed_at - 1);
        h.root
            .ack_public_world_totals(&authority, &samples[0])
            .await
            .map_err(debug)?;
        assert!(
            h.root
                .next_public_world_totals(&authority)
                .await
                .map_err(debug)?
                .is_empty()
        );
        let acked = h
            .root
            .public_world_totals_facts(&authority)
            .await
            .map_err(debug)?
            .ok_or("facts")?;
        assert_eq!(acked.complete_through, samples[0].source_observed_at);
        // An empty queue does not move coverage beyond the last sample.
        assert_eq!(
            h.root
                .public_world_totals_facts(&authority)
                .await
                .map_err(debug)?
                .ok_or("facts")?
                .complete_through,
            acked.complete_through
        );
        sqlx::query("SELECT game_public_projection_resync('world_totals',false)")
            .execute(&h.pool)
            .await?;
        assert_eq!(
            samples,
            h.root
                .next_public_world_totals(&authority)
                .await
                .map_err(debug)?
        );
        sqlx::query("SELECT game_public_projection_resync('all',true)")
            .execute(&h.pool)
            .await?;
        assert!(
            h.root
                .next_public_world_totals(&authority)
                .await
                .map_err(debug)?
                .is_empty()
        );
        assert!(
            h.root
                .public_world_totals_facts(&authority)
                .await
                .map_err(debug)?
                .is_none()
        );
        h.root
            .sample_public_world_totals(&authority, "")
            .await
            .map_err(debug)?;
        let raised = h
            .root
            .next_public_world_totals(&authority)
            .await
            .map_err(debug)?;
        assert_eq!(raised.len(), 1);
        assert!(raised[0].projection_epoch > samples[0].projection_epoch);
        assert_eq!(raised[0].projection_revision, 2);
        // Least privilege: no public execute, no runtime content mutation or deletion.
        let privileges: (bool,bool,bool,bool) = sqlx::query_as("SELECT \
            has_function_privilege('oteryn_game_runtime','game_public_world_totals_sample(uuid)','EXECUTE'), \
            has_table_privilege('oteryn_game_runtime','game_public_world_totals_samples','DELETE'), \
            has_column_privilege('oteryn_game_runtime','game_public_world_totals_samples','online_players','UPDATE'), \
            has_column_privilege('oteryn_game_runtime','game_public_world_totals_samples','acked','UPDATE')")
            .fetch_one(&h.pool).await?;
        assert_eq!(privileges, (true, false, false, true));
        Ok(())
    })
}

#[test]
fn platform_world_totals_page_worlds_and_pending_history_without_truncation() -> TestResult {
    run("platform_totals_pages", async |h| {
        sqlx::query("INSERT INTO game_public_world_deaths SELECT \
            (lpad(to_hex(n),2,'0') || '020304-0506-7008-800a-0b0c0d0e0f' || lpad(to_hex(n),2,'0'))::uuid,0 \
            FROM generate_series(1,33) n").execute(&h.pool).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let first = h
            .root
            .sample_public_world_totals(&authority, "")
            .await
            .map_err(debug)?
            .ok_or("first page")?;
        assert_eq!(h.count("game_public_world_totals_heads").await?, 32);
        assert!(
            h.root
                .public_world_totals_facts(&authority)
                .await
                .map_err(debug)?
                .is_none()
        );
        let last = h
            .root
            .sample_public_world_totals(&authority, &first)
            .await
            .map_err(debug)?
            .ok_or("second page")?;
        assert!(last > first);
        assert!(
            h.root
                .sample_public_world_totals(&authority, &last)
                .await
                .map_err(debug)?
                .is_none()
        );
        assert_eq!(h.count("game_public_world_totals_heads").await?, 34);
        let batch = h
            .root
            .next_public_world_totals(&authority)
            .await
            .map_err(debug)?;
        assert_eq!(batch.len(), 32);
        for sample in &batch {
            assert_eq!((sample.online_players, sample.player_deaths), (0, 0));
            h.root
                .ack_public_world_totals(&authority, sample)
                .await
                .map_err(debug)?;
        }
        assert_eq!(
            h.root
                .next_public_world_totals(&authority)
                .await
                .map_err(debug)?
                .len(),
            2
        );
        Ok(())
    })
}
