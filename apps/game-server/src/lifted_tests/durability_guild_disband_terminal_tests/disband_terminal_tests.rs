use super::*;
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
        assert_eq!(h.count("game_guild_event_outbox").await?, 0);
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
            (2, 2, "23514"),
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
                oteryn_gameplay_social::guild::run_guild_disband_pass(
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
        let pass = oteryn_gameplay_social::guild::run_guild_disband_pass(
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
