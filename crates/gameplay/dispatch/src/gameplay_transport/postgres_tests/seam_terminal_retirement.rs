//! A terminal actor may be gone while its scoped offline acknowledgement still retries.
use super::*;
use crate::durability::fresh_admission::ExpiredLossReleaseV1;
use crate::foundation::GameSessionState;
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, GraceExpiryResult};

#[test]
fn seam_terminal_retirement_retries_offline_ack_after_actor_removal_postgres() -> TestResult {
    kill_reward_e2e::scenario_thread::run_async(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "terminal_retire_retry", true, false)
            .await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let deps = Dependencies::new()?;
        let owner = deps.owner(&h, &authority);
        deps.initialize(&owner).await;
        owner
            .runtime
            .lock()
            .await
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        track(&owner, &deps)?;
        let session = deps.admitted.game_session_id;
        let store = FreshAdmissionStore::from_root(h.root.clone());
        terminal_release(&store, deps.admitted).await?;
        let terminal = store.current_session(session).await?;
        assert_eq!(terminal.session_state(), GameSessionState::Terminal);
        let receipts = lifecycle_receipts(&h).await?;

        // Reuse the isolated CLOSING boundary from the account-deletion fixture.
        // The session, claim release and runtime absence remain real owner evidence.
        seed_closing(&h).await?;
        sqlx::raw_sql(
            "CREATE SEQUENCE seam_retire_ack_attempt;
             CREATE FUNCTION seam_retire_ack_fail_once() RETURNS trigger
             LANGUAGE plpgsql SECURITY DEFINER AS $$
             BEGIN
                 IF nextval('seam_retire_ack_attempt') = 1 THEN
                     RAISE EXCEPTION 'terminal offline acknowledgement unavailable'
                         USING ERRCODE = '40001';
                 END IF;
                 RETURN NEW;
             END $$;
             CREATE TRIGGER seam_retire_ack_failure
             BEFORE INSERT ON game_account_closure_offline
             FOR EACH ROW EXECUTE FUNCTION seam_retire_ack_fail_once();",
        )
        .execute(&h.pool)
        .await?;
        assert_eq!(
            owner.retire(session, deps.actor).await,
            GraceExpiryResult::Unknown,
            "the real acknowledgement fails after actor removal",
        );
        assert!(
            !owner
                .runtime
                .lock()
                .await
                .contains_player_session(session)
                .map_err(debug)?,
        );
        assert_eq!(offline_acknowledgements(&h, session).await?, 0);
        assert!(
            sqlx::query_scalar::<_, bool>("SELECT is_called FROM seam_retire_ack_attempt")
                .fetch_one(&h.pool)
                .await?
        );
        assert_eq!(ack_attempts(&h).await?, 1);
        maintain_ready(&h.root).await?;

        // A missing actor does not turn an unreadable durable row into success.
        let held = h.root.try_issue_semantic_pass()?;
        assert_eq!(
            owner.expire_control_loss(deps.admitted).await,
            GraceExpiryResult::Unknown,
        );
        assert!(owner.runtime.try_lock().is_ok());
        assert!(owner.states.spell_states.try_lock().is_ok());
        held.run(|_, _| Box::pin(async { Ok(()) })).await?;

        // The original grace callback now reaches the existing terminal reconciler.
        // It acknowledges current scoped absence without replaying terminal release.
        assert_eq!(
            owner.expire_control_loss(deps.admitted).await,
            GraceExpiryResult::Released,
        );
        assert_eq!(offline_acknowledgements(&h, session).await?, 1);
        assert_eq!(ack_attempts(&h).await?, 2);
        assert_eq!(store.current_session(session).await?, terminal);
        assert_eq!(lifecycle_receipts(&h).await?, receipts);
        assert!(owner.states.fence_holders.resume(session).is_none());
        assert_eq!(
            owner.expire_control_loss(deps.admitted).await,
            GraceExpiryResult::Released,
        );
        assert_eq!(offline_acknowledgements(&h, session).await?, 1);
        assert_eq!(ack_attempts(&h).await?, 2, "acknowledgement is idempotent");
        assert_eq!(lifecycle_receipts(&h).await?, receipts);
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

#[test]
fn seam_terminal_retirement_keeps_unknown_without_terminal_or_scoped_absence_postgres() -> TestResult
{
    kill_reward_e2e::scenario_thread::run_async(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "terminal_retire_fences", true, false)
            .await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let deps = Dependencies::new()?;
        let owner = deps.owner(&h, &authority);
        deps.initialize(&owner).await;
        let session = deps.admitted.game_session_id;
        let store = FreshAdmissionStore::from_root(h.root.clone());
        let active = store.current_session(session).await?;
        // Retiring the real runtime invalidates exact actor facts. Neither that
        // failure nor a stale actor reference authorizes release of ACTIVE state.
        owner.runtime.lock().await.retire_owner_cycle();
        assert_eq!(
            owner.expire_control_loss(deps.admitted).await,
            GraceExpiryResult::Unknown,
        );
        assert_eq!(store.current_session(session).await?, active);
        assert_eq!(offline_acknowledgements(&h, session).await?, 0);

        terminal_release(&store, deps.admitted).await?;
        seed_closing(&h).await?;
        let terminal = store.current_session(session).await?;
        let receipts = lifecycle_receipts(&h).await?;
        assert_eq!(terminal.session_state(), GameSessionState::Terminal);
        assert!(
            owner
                .runtime
                .lock()
                .await
                .prove_player_session_absent_in(session, terminal.current_runtime_scope())
                .is_err(),
            "a retired owner cannot supply current scoped absence",
        );
        assert_eq!(
            owner.expire_control_loss(deps.admitted).await,
            GraceExpiryResult::Unknown,
        );
        assert_eq!(offline_acknowledgements(&h, session).await?, 0);
        assert_eq!(store.current_session(session).await?, terminal);
        assert_eq!(lifecycle_receipts(&h).await?, receipts);
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

async fn terminal_release(store: &FreshAdmissionStore, admitted: AdmittedSession) -> TestResult {
    let controller = admitted.controller.ok_or("controller")?;
    assert!(matches!(
        store
            .release_logout_session(
                admitted.game_session_id,
                &crate::gameplay_transport::canonical_uuid(&controller.account_id),
                controller.transport,
            )
            .await?,
        ExpiredLossReleaseV1::Released { .. },
    ));
    Ok(())
}

async fn seed_closing(h: &Harness) -> TestResult {
    sqlx::query(
        "INSERT INTO game_account_closures(
             account_id,termination_id,close_operation_id,lifecycle_generation,
             close_generation,state,recovery_generation,closed_at)
         VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,
             encode($3,'hex')::uuid,1,1,'CLOSING',1,game_account_closure_now())",
    )
    .bind(id(40).as_slice())
    .bind(id(191).as_slice())
    .bind(id(192).as_slice())
    .execute(&h.pool)
    .await?;
    Ok(())
}

async fn offline_acknowledgements(h: &Harness, session: GameSessionId) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT count(*) FROM game_account_closure_offline
         WHERE game_session_id=encode($1,'hex')::uuid",
    )
    .bind(session.as_bytes().as_slice())
    .fetch_one(&h.pool)
    .await?)
}

async fn ack_attempts(h: &Harness) -> TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT last_value::bigint FROM seam_retire_ack_attempt")
            .fetch_one(&h.pool)
            .await?,
    )
}

async fn lifecycle_receipts(h: &Harness) -> TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT count(*) FROM game_durability_admission_lifecycle_receipts")
            .fetch_one(&h.pool)
            .await?,
    )
}
