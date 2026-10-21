use crate::domain::guild::GuildId;
use crate::durability::DurabilityRoot;
use crate::durability::guild::*;
use crate::item_transfer_postgres_cases::{
    self as fixture, CHARACTER, Harness, TestResult, WORLD, configured_admin, debug, fence, id,
    runtime,
};
use sqlx::Executor;

fn job() -> TestResult<GuildDisbandJobRequest> {
    let salted = |salt| {
        let mut value = id(90);
        value[1] = salt;
        value
    };
    Ok(GuildDisbandJobRequest {
        world_id: fence()?.runtime_scope.world_id(),
        guild_id: GuildId::from_bytes(id(90)).map_err(debug)?,
        hall_identity: crate::durability::guildhall::GuildhallEscrowIdentity {
            occurrence_id: salted(1),
            transaction_id: salted(2),
            guild_entry_id: salted(3),
            account_entry_id: salted(4),
        },
        bank_identity: crate::durability::guild_bank::GuildBankIdentity {
            occurrence: salted(5),
            transaction_id: salted(6),
            guild_event_id: salted(7),
            bank_event_id: salted(8),
            guild_entry_id: salted(9),
            bank_entry_id: salted(10),
            occurred_at_unix_ms: 1_790_000_000_000,
        },
    })
}

async fn install(h: &Harness) -> TestResult {
    assert_eq!(
        sqlx::query_scalar::<_, String>("SHOW server_version_num")
            .fetch_one(&h.pool)
            .await?,
        "170006"
    );
    eprintln!("GUILD-TERMINAL-NATIVE-PG server_version_num=170006");
    // Exact genuine owner proposals. No auction/property fixture, fake closure function,
    // privileged release witness, or disabled native guard is installed by this lane.
    for schema in [
        GUILD_TABLES_PROPOSAL,
        crate::durability::guildhall::BID_SOURCE_SCHEMA,
        crate::durability::guildhall::OPERATION_SCHEMA,
        crate::durability::guild_bank::GUILD_BANK_SCHEMA_PROPOSAL,
        crate::durability::guild_bank::GUILDHALL_LEDGER_SCHEMA_PROPOSAL,
        crate::durability::guildhall::HALL_GUARDS_SCHEMA,
    ] {
        h.pool.execute(sqlx::raw_sql(schema)).await?;
    }
    sqlx::query("INSERT INTO game_guilds(guild_id,world_id,name,state,formation_deadline,vice_deficit_since,revision,created_at) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Terminal Guild',2,9999999999999,1,1,1)")
        .bind(id(90).as_slice()).bind(id(WORLD).as_slice()).execute(&h.pool).await?;
    for (level, name) in [(1_i16, "Leader"), (2, "Vice Leader"), (3, "Member")] {
        sqlx::query("INSERT INTO game_guild_ranks(guild_id,level,name) VALUES(encode($1,'hex')::uuid,$2,$3)")
            .bind(id(90).as_slice()).bind(level).bind(name).execute(&h.pool).await?;
    }
    sqlx::query("INSERT INTO game_guild_members(character_id,guild_id,rank_level,title,joined_at) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,'',1)")
        .bind(id(CHARACTER).as_slice()).bind(id(90).as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_account_guild_leadership(account_id,guild_id,character_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid)")
        .bind(id(40).as_slice()).bind(id(90).as_slice()).bind(id(CHARACTER).as_slice()).execute(&h.pool).await?;
    h.pool.execute(sqlx::raw_sql(GUILD_GUARDS_PROPOSAL)).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let request = job()?;
    assert_eq!(
        h.root
            .start_guild_deadline_disband(&authority, &h.node, request.world_id, request.guild_id)
            .await
            .map_err(debug)?,
        GuildDeadlineOutcome::Started
    );
    Ok(())
}

fn run<F>(tag: &'static str, body: F) -> TestResult
where
    F: AsyncFnOnce(&Harness) -> TestResult,
{
    let admin = configured_admin().ok_or("native Guild terminal PG route required")?;
    runtime()?.block_on(async move {
        let h = Harness::create(admin, tag).await?;
        let result = async {
            install(&h).await?;
            body(&h).await
        }
        .await;
        h.cleanup().await?;
        result
    })
}

async fn snapshot(h: &Harness) -> TestResult<Vec<String>> {
    let mut values = Vec::new();
    for table in [
        "game_guilds",
        "game_guild_members",
        "game_guild_invitations",
        "game_guild_ranks",
        "game_account_guild_leadership",
        "game_guild_disband_steps",
        "game_guild_disband_step_events",
        "game_guild_bank_operations",
        "game_guild_disband_claims",
        "game_guildhall_operations",
    ] {
        values.push(sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT coalesce(jsonb_agg(x ORDER BY x::text),'[]'::jsonb)::text FROM (SELECT to_jsonb(t) AS x FROM {table} t) snapshots"))).fetch_one(&h.pool).await?);
    }
    Ok(values)
}

#[test]
fn guild_terminal_native_real_owner_wait_retains_all_state_on_retry_and_root_restart() -> TestResult
{
    run("guild_terminal_wait", async |h| {
        let before = snapshot(h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        for _ in 0..2 {
            assert_eq!(
                h.root
                    .advance_guild_disband(&authority, &h.node, job()?)
                    .await
                    .map_err(debug)?,
                GuildDisbandOutcome::WaitingForHouseDisposition
            );
            assert_eq!(snapshot(h).await?, before);
        }
        let restarted = DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(restarted.maintain_ready_once().await?);
        let current = restarted
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        assert_eq!(
            restarted
                .advance_guild_disband(&current, &h.node, job()?)
                .await
                .map_err(debug)?,
            GuildDisbandOutcome::WaitingForHouseDisposition
        );
        assert_eq!(snapshot(h).await?, before);
        assert_eq!(h.count("game_guild_bank_operations").await?, 0);
        assert_eq!(h.count("game_guild_members").await?, 1);
        Ok(())
    })
}

#[test]
fn guild_terminal_native_context_rejects_foreign_physical_transaction_and_unassigned_node()
-> TestResult {
    run("guild_terminal_context", async |h| {
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let before = snapshot(h).await?;
        let mut tx = h.pool.begin().await?;
        let context =
            lock_guild_disband_transaction(&mut tx, &h.root, &authority, &h.node, &job()?)
                .await
                .map_err(debug)?
                .ok_or("pending context absent")?;
        context.assert_current(&mut tx).await.map_err(debug)?;
        let mut foreign = h.pool.begin().await?;
        assert!(matches!(
            context.assert_current(&mut foreign).await,
            Err(GuildError::NotCurrent)
        ));
        foreign.rollback().await?;
        tx.rollback().await?;
        let mut replacement = h.pool.begin().await?;
        assert!(matches!(
            context.assert_current(&mut replacement).await,
            Err(GuildError::NotCurrent)
        ));
        replacement.rollback().await?;
        let unassigned = fixture::register(&h.root, 3).await?;
        assert!(matches!(
            h.root
                .advance_guild_disband(&authority, &unassigned, job()?)
                .await,
            Err(GuildError::NotCurrent)
        ));
        let other = DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(other.maintain_ready_once().await?);
        assert!(matches!(
            other
                .advance_guild_disband(&authority, &h.node, job()?)
                .await,
            Err(GuildError::NotCurrent)
        ));
        let mut wrong_world = job()?;
        wrong_world.world_id = crate::foundation::WorldId::decode(&id(91)).map_err(debug)?;
        assert!(matches!(
            h.root
                .advance_guild_disband(&authority, &h.node, wrong_world)
                .await,
            Err(GuildError::NotCurrent)
        ));
        assert_eq!(snapshot(h).await?, before);
        Ok(())
    })
}

#[test]
fn guild_terminal_native_current_recovery_rechecked_after_retained_start() -> TestResult {
    run("guild_terminal_recovery", async |h| {
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let before = snapshot(h).await?;
        h.pool.execute("INSERT INTO game_character_recovery_admissions(authority_scope_id,recovery_generation,recovery_event_id,predecessor_generation,predecessor_digest,issued_at,issuer_identity,reconciled_at) SELECT authority_scope_id,recovery_generation+1,'10000000-0000-7000-8000-000000000001',recovery_generation,decode(repeat('00',32),'hex'),issued_at+1,issuer_identity,reconciled_at FROM game_character_recovery_admissions ORDER BY recovery_generation DESC LIMIT 1").await?;
        assert!(
            h.root
                .advance_guild_disband(&authority, &h.node, job()?)
                .await
                .is_err()
        );
        assert_eq!(snapshot(h).await?, before);
        Ok(())
    })
}

#[test]
fn guild_terminal_native_sql_refuses_skipped_phases_unbound_deletes_and_false_closure() -> TestResult
{
    run("guild_terminal_guards", async |h| {
        let before = snapshot(h).await?;
        for sql in [
            "DELETE FROM game_guild_members",
            "DELETE FROM game_guild_ranks",
            "DELETE FROM game_account_guild_leadership",
            "UPDATE game_guilds SET state=4,revision=revision+1",
        ] {
            let error = h
                .pool
                .execute(sql)
                .await
                .err()
                .ok_or("unbound mutation admitted")?;
            assert_eq!(
                error.as_database_error().and_then(|e| e.code()).as_deref(),
                Some("23514")
            );
            assert_eq!(snapshot(h).await?, before);
        }
        for (step, phase, code) in [
            (2_i32, 3_i16, "23514"),
            (3, 2, "23514"),
            (2, 6, "23514"),
            (2, 2, "0A000"),
        ] {
            let error=sqlx::query("INSERT INTO game_guild_disband_steps(guild_id,step,phase,world_id,previous_revision,revision,member_ids,invitation_ids,leadership_accounts,occurred_at) SELECT guild_id,$1,$2,world_id,revision,revision+1,'{}','{}','{}',floor(extract(epoch FROM transaction_timestamp())*1000)::bigint FROM game_guilds")
                .bind(step).bind(phase).execute(&h.pool).await.err().ok_or("false closure/phase admitted")?;
            assert_eq!(
                error.as_database_error().and_then(|e| e.code()).as_deref(),
                Some(code)
            );
            assert_eq!(snapshot(h).await?, before);
        }
        Ok(())
    })
}

#[test]
fn guild_terminal_native_pending_cursor_and_bounded_pass_refuse_duplicate_or_foreign_data()
-> TestResult {
    run("guild_terminal_batch", async |h| {
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let request = job()?;
        let before = snapshot(h).await?;
        assert_eq!(
            h.root
                .pending_guild_disbands_after(&authority, &h.node, request.world_id, None)
                .await
                .map_err(debug)?,
            vec![request.guild_id]
        );
        assert!(
            h.root
                .pending_guild_disbands_after(
                    &authority,
                    &h.node,
                    request.world_id,
                    Some(request.guild_id)
                )
                .await
                .map_err(debug)?
                .is_empty()
        );
        for requests in [vec![request.clone(); 101], vec![request.clone(); 2]] {
            assert!(matches!(
                crate::node::guild_jobs::run_guild_disband_pass(
                    &h.root,
                    &authority,
                    &h.node,
                    request.world_id,
                    requests
                )
                .await,
                Err(GuildError::InvalidInput)
            ));
        }
        let pass = crate::node::guild_jobs::run_guild_disband_pass(
            &h.root,
            &authority,
            &h.node,
            request.world_id,
            vec![request.clone()],
        )
        .await
        .map_err(debug)?;
        assert_eq!(
            pass.outcomes,
            vec![(
                request.guild_id,
                GuildDisbandOutcome::WaitingForHouseDisposition
            )]
        );
        assert_eq!(snapshot(h).await?, before);
        Ok(())
    })
}

#[test]
fn guild_terminal_native_two_current_roots_serialize_wait_without_payout_or_cleanup() -> TestResult
{
    run("guild_terminal_race", async |h| {
        let other = DurabilityRoot::connect_test_runtime(&h.database.url)?;
        assert!(other.maintain_ready_once().await?);
        let seal = h.recovery.seal_current().map_err(debug)?;
        let first = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let second = other.open_character_authority(&seal).await.map_err(debug)?;
        let before = snapshot(h).await?;
        let (left, right) = fixture::join_two(
            h.root.advance_guild_disband(&first, &h.node, job()?),
            other.advance_guild_disband(&second, &h.node, job()?),
        )
        .await;
        assert_eq!(
            left.map_err(debug)?,
            GuildDisbandOutcome::WaitingForHouseDisposition
        );
        assert_eq!(
            right.map_err(debug)?,
            GuildDisbandOutcome::WaitingForHouseDisposition
        );
        assert_eq!(snapshot(h).await?, before);
        Ok(())
    })
}
