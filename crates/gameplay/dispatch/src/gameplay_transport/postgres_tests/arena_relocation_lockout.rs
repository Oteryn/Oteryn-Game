use super::arena_match_fixtures::fixture;
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::arena_match::{MatchState, donor};
use crate::foundation::GameSessionId;
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use oteryn_protocol_oteryn::arena::{self as wire, ArenaAction as A, ArenaDisposition as D};

#[test]
fn arena_pending_victory_after_death_and_relog_allows_leave() -> TestResult {
    scenario(
        "arena_pending_victory_after_death_and_relog_allows_leave",
        Case::Victory,
    )
}

#[test]
fn arena_leave_before_poll_supersedes_victory_after_death_and_relog() -> TestResult {
    scenario(
        "arena_leave_before_poll_supersedes_victory_after_death_and_relog",
        Case::VictoryLeaveFirst,
    )
}

#[test]
fn arena_leave_before_poll_preserves_the_other_characters_pit_lease() -> TestResult {
    scenario(
        "arena_leave_before_poll_preserves_the_other_characters_pit_lease",
        Case::PitTaken,
    )
}

#[test]
fn arena_relocation_census_reaches_rows_past_unactionable_ones() -> TestResult {
    scenario(
        "arena_relocation_census_reaches_rows_past_unactionable_ones",
        Case::Census,
    )
}

#[test]
fn arena_dead_pending_entry_after_relog_is_idled_by_command() -> TestResult {
    scenario(
        "arena_dead_pending_entry_after_relog_is_idled_by_command",
        Case::DeadEntry,
    )
}

#[test]
fn arena_dead_pending_entry_after_relog_is_superseded_by_poll() -> TestResult {
    scenario(
        "arena_dead_pending_entry_after_relog_is_superseded_by_poll",
        Case::DeadEntryPoll,
    )
}

#[derive(Clone, Copy)]
enum Case {
    Victory,
    VictoryLeaveFirst,
    PitTaken,
    Census,
    DeadEntry,
    DeadEntryPoll,
}

fn scenario(test: &'static str, case: Case) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            run(async |admin| {
                let harness =
                    Harness::create_with_admission_seed(admin, "arena_lockout", false, false)
                        .await?;
                eprintln!("arenafix-marker: {test} db ready");
                let result = exercise(&harness, case).await;
                harness.cleanup().await?;
                result
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "arena relocation regression panicked")??;
    eprintln!("arenafix-marker: {test} duration={:?}", started.elapsed());
    Ok(())
}

async fn exercise(harness: &Harness, case: Case) -> TestResult {
    prepare_current_session(harness).await?;
    let seal = harness.recovery.seal_current().map_err(debug)?;
    let authority = harness
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let mut deps = Dependencies::new()?;
    let (controller, pin) = super::death_ml_loss_composed_fixtures::production_generation_with_pin(
        deps.admitted.world_id,
        "content-1",
    )?;
    let active = controller.active().ok_or("active generation")?;
    let native = active.native_gameplay().ok_or("native gameplay")?;
    let (bytes, pins) = fixture::bundle(native)?;
    let map = fixture::boot_world(&deps, native, &bytes, &pins)?;
    fixture::runtime(&mut deps, active, &pin, &map)?;
    let cells = map
        .movement_cells(deps.room.movement_cells())
        .map_err(debug)?;
    let mut owner = deps.owner(harness, &authority);
    owner.active_generation = Some(active);
    owner.movement_cells = (&cells).into();
    deps.initialize(&owner).await;

    let mut tx = harness.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    let now_ms: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()) * 1000)::bigint")
            .fetch_one(&mut *tx)
            .await?;
    let old_session = id(90);
    let fighting = MatchState {
        rank: 1,
        pit: 1,
        phase: 1,
        deadline_ms: now_ms + i64::from(donor::PIT_SECONDS) * 1000,
        entry_revision: 1,
        run: id(91),
        session: old_session,
        scope_generation: 1,
        ..MatchState::default()
    };
    if matches!(case, Case::DeadEntry | Case::DeadEntryPoll | Case::PitTaken) {
        seed_pending(
            &mut tx,
            id(41),
            &MatchState::default(),
            &fighting,
            A::Enter,
            "ENTRY",
            None,
        )
        .await?;
    } else {
        let pit = u8::try_from(donor::CENTERS.len())?;
        let ready = MatchState {
            phase: 2,
            pit,
            ..fighting.clone()
        };
        let won = MatchState {
            completed: fighting.rank,
            phase: 3,
            pit: 0,
            deadline_ms: 0,
            ..fighting
        };
        seed_pending(
            &mut tx,
            id(41),
            &ready,
            &won,
            A::Advance,
            "VICTORY",
            Some(i16::from(pit)),
        )
        .await?;
    }
    if matches!(case, Case::Census) {
        let mut sessions = Vec::new();
        for ordinal in 1..=(u8::try_from(donor::CENTERS.len())? + 1) {
            let character = id(ordinal);
            let session = id(ordinal + 100);
            seed_unbound_admission(&mut tx, character, session, ordinal).await?;
            let before = MatchState {
                rank: 1,
                phase: 1,
                pit: 1,
                session,
                scope_generation: 1,
                ..MatchState::default()
            };
            seed_pending(
                &mut tx,
                character,
                &before,
                &before.idle(),
                A::Leave,
                "EXIT",
                None,
            )
            .await?;
            sessions.push(GameSessionId::decode(&session).map_err(debug)?);
        }
        tx.commit().await?;
        {
            let mut runtime = deps.runtime.lock().await;
            for session in sessions {
                let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
                let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
                runtime
                    .initialize_first_entry_position(actor)
                    .map_err(debug)?;
                assert!(runtime.bound_attacker_lease(actor, session).is_err());
            }
        }
        let now = owner.owner_now().get();
        let cadence = crate::ai_think::CREATURE_THINK_INTERVAL_MILLIS * 1_000;
        for tick in 0..=2 {
            owner.drain_arena_matches(now + tick * cadence).await;
        }
        assert_eq!(
            relocation(harness, old_session).await?,
            (true, false),
            "an actionable relocation after more than one full page is acknowledged",
        );
        let still_pending: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_arena_relocations WHERE relocated_at IS NULL AND superseded_at IS NULL",
    ).fetch_one(&harness.pool).await?;
        assert_eq!(still_pending, i64::try_from(donor::CENTERS.len())? + 1);
    } else {
        let other_character = id(1);
        if matches!(case, Case::PitTaken) {
            let other_session = id(101);
            seed_unbound_admission(&mut tx, other_character, other_session, 1).await?;
            sqlx::query(
                "INSERT INTO game_svargrond_pit_leases
                    (world_id,channel_id,pit,character_id,game_session_id,run_id,scope_generation,expires_at_ms)
                 VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,encode($3,'hex')::uuid,
                    encode($4,'hex')::uuid,encode($5,'hex')::uuid,1,$6)",
            )
            .bind(id(42).as_slice())
            .bind(id(43).as_slice())
            .bind(other_character.as_slice())
            .bind(other_session.as_slice())
            .bind(id(93).as_slice())
            .bind(now_ms + i64::from(donor::PIT_SECONDS) * 1000)
            .execute(&mut *tx).await?;
        } else {
            seed_death(&mut tx).await?;
        }
        tx.commit().await?;
        if matches!(case, Case::Victory | Case::DeadEntryPoll) {
            owner.drain_arena_matches(owner.owner_now().get()).await;
        }
        if matches!(case, Case::DeadEntryPoll) {
            assert_eq!(relocation(harness, old_session).await?, (false, true));
        }
        let mut admitted = deps.admitted;
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(debug)?
            .ok_or("readmitted gameplay fence")?;
        assert_ne!(old_session, *admitted.game_session_id.as_bytes());
        admitted.item_fence = Some(fence.item_fence());
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,
            &[wire::CAPABILITY_ARENA_V1],
        )
        .ok_or("arena capability")?;
        let payload = wire::encode_match_request(A::Leave).map_err(debug)?;
        let reply = owner
            .arena_match_wire(admitted, admitted.continuity.next_command_id, &payload)
            .await
            .ok_or("Leave returned no reply")?;
        let result = wire::decode_match_result(&reply).map_err(debug)?;
        let (expected, completed, relocation_result) = match case {
            Case::Victory => (D::Applied, 1, (true, false)),
            Case::VictoryLeaveFirst => (D::Applied, 1, (false, true)),
            Case::PitTaken => (D::Applied, 0, (false, true)),
            _ => (D::WrongState, 0, (false, true)),
        };
        assert_eq!(result.disposition, expected, "{result:?}");
        assert_eq!(
            (
                result.state.phase,
                result.state.active_rank,
                result.state.completed_rank
            ),
            (0, 0, completed)
        );
        assert_eq!(relocation(harness, old_session).await?, relocation_result);
        if matches!(case, Case::PitTaken) {
            let lease: Vec<u8> = sqlx::query_scalar(
                "SELECT uuid_send(character_id) FROM game_svargrond_pit_leases WHERE pit=1",
            ).fetch_one(&harness.pool).await?;
            assert_eq!(lease, other_character);
        }
    }
    Ok(())
}

async fn relocation(harness: &Harness, session: [u8; 16]) -> TestResult<(bool, bool)> {
    Ok(sqlx::query_as(
        "SELECT relocated_at IS NOT NULL,superseded_at IS NOT NULL FROM game_arena_relocations
         WHERE game_session_id=encode($1,'hex')::uuid",
    )
    .bind(session.as_slice())
    .fetch_one(&harness.pool)
    .await?)
}

async fn seed_pending(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: [u8; 16],
    before: &MatchState,
    after: &MatchState,
    action: A,
    kind: &str,
    legacy_pit: Option<i16>,
) -> TestResult {
    let session = if after.rank == 0 {
        before.session
    } else {
        after.session
    };
    let reply = wire::encode_match_result(&wire::ArenaResult {
        disposition: D::Applied,
        state: after.wire(0, [0; 3]),
    })
    .map_err(debug)?;
    sqlx::query(
        "WITH receipt AS (
            INSERT INTO game_arena_match_receipts
                (character_id,game_session_id,command_id,payload,before_state,after_state,reply)
            VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,$3,$4,$5,$6)
            RETURNING character_id,game_session_id,command_id,after_state
         ), saved AS (
            INSERT INTO game_character_arena_matches(character_id,body,game_session_id,command_id)
            SELECT character_id,after_state,game_session_id,command_id FROM receipt
         )
         INSERT INTO game_arena_relocations
            (character_id,game_session_id,command_id,world_id,channel_id,scope_generation,kind,legacy_victory_pit)
         SELECT character_id,game_session_id,command_id,encode($7,'hex')::uuid,encode($8,'hex')::uuid,1,$9,$10
         FROM receipt",
    )
    .bind(character.as_slice())
    .bind(session.as_slice())
    .bind(wire::encode_match_request(action).map_err(debug)?)
    .bind(serde_json::to_value(before)?)
    .bind(serde_json::to_value(after)?)
    .bind(reply)
    .bind(id(42).as_slice())
    .bind(id(43).as_slice())
    .bind(kind)
    .bind(legacy_pit)
    .execute(&mut **tx).await?;
    Ok(())
}

async fn seed_unbound_admission(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: [u8; 16],
    session: [u8; 16],
    ordinal: u8,
) -> TestResult {
    sqlx::query(
        "INSERT INTO game_character_roots(character_id,account_id,world_id,lifecycle,character_revision,
            profile_revision,ruleset_revision,content_revision,starter_template_revision,name,sex)
         SELECT encode($1,'hex')::uuid,encode($2,'hex')::uuid,world_id,lifecycle,character_revision,
            profile_revision,ruleset_revision,content_revision,starter_template_revision,$3,sex
         FROM game_character_roots WHERE character_id=encode($4,'hex')::uuid",
    )
    .bind(character.as_slice()).bind(id(ordinal + 60).as_slice())
    .bind(format!("Arena Census {}", char::from(b'A' + ordinal - 1)))
    .bind(id(41).as_slice()).execute(&mut **tx).await?;
    sqlx::query(
        "INSERT INTO game_durability_reconnect_sessions
            (game_session_id,account_id,character_id,world_id,runtime_scope_kind,runtime_scope_world_id,
             runtime_scope_channel_id,control_loss_epoch,original_grace_deadline,predecessor_generation,
             character_lease_generation,scope_ownership_generation,current_generation,current_transport_ref,session_state)
         SELECT encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,world_id,
            runtime_scope_kind,runtime_scope_world_id,runtime_scope_channel_id,1,
            extract(epoch FROM statement_timestamp())::bigint,current_generation,character_lease_generation,
            scope_ownership_generation,current_generation,$1,2
         FROM game_durability_reconnect_sessions WHERE game_session_id=encode($4,'hex')::uuid",
    )
    .bind(session.as_slice()).bind(id(ordinal + 60).as_slice()).bind(character.as_slice())
    .bind(id(50).as_slice()).execute(&mut **tx).await?;
    Ok(())
}

async fn seed_death(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>) -> TestResult {
    sqlx::query(
        "INSERT INTO game_character_death_receipts(
            death_occurrence_id,command_binding,policy_digest,character_id,
            original_character_revision,committed_character_revision,level_before,level_after,
            experience_before,experience_after,experience_lost,blessings_before,blessings_after,
            lost_item_ids,death_world_id,death_channel_id,death_spatial_position,
            death_map_revision,respawn_position,death_policy_revision,profile_revision,
            ruleset_revision,content_revision,simulation_revision,evidence_revision,
            declaration_revision,policy_revision,reward_revision,committed_at)
         VALUES(encode($1,'hex')::uuid,decode('01','hex'),decode(repeat('01',32),'hex'),
            encode($2,'hex')::uuid,1,2,20,20,0,0,0,'{}','{}','{}',
            encode($3,'hex')::uuid,encode($4,'hex')::uuid,decode('01','hex'),'fixture',
            decode('01','hex'),'fixture','fixture','fixture','fixture','fixture',
            'fixture','fixture','fixture','fixture',1)",
    )
    .bind(id(92).as_slice())
    .bind(id(41).as_slice())
    .bind(id(42).as_slice())
    .bind(id(43).as_slice())
    .execute(&mut **tx)
    .await?;
    Ok(())
}
