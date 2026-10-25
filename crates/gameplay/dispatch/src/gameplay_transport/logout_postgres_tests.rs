//! LOGOUT-WIRE-1 (ARCH-KILL-REWARD-LOGOUT-1 §1.6, §2.2) on PostgreSQL: the real composed
//! owner's logout of the fresh-admission fixture session. The terminal release commits through
//! the exact fenced store path, writes the actor's final position (CHAR-POSITION-1) and retires
//! the actor; a refusal changes nothing; an unknown outcome is reconciled from the row.
//!
//! Nested under the monk-save PostgreSQL tests to reuse their fresh-admission fixture.

use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset, revision, track};
use crate::durability::fresh_admission::{ExpiredLossReleaseV1, FreshAdmissionStore};
use crate::foundation::{AuthenticatedTransportRefV1, GameSessionState};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::connection::{ControlLossResult, GraceExpiryResult, LogoutReply};
use oteryn_protocol_oteryn::logout::LogoutResult;

async fn session_state(
    harness: &Harness,
    dependencies: &Dependencies,
) -> TestResult<GameSessionState> {
    let (current, _) = FreshAdmissionStore::from_root(harness.root.clone())
        .current_session_at(dependencies.admitted.game_session_id)
        .await
        .map_err(debug)?;
    Ok(current.session_state())
}

async fn last_positions(harness: &Harness) -> TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT count(*) FROM game_character_last_positions")
            .fetch_one(&harness.pool)
            .await?,
    )
}

async fn actor_present(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    dependencies: &Dependencies,
) -> bool {
    owner
        .runtime
        .lock()
        .await
        .player_control_facts(dependencies.actor, dependencies.admitted.game_session_id)
        .is_ok()
}

/// The admitted actor stands on its first-entry cell and its session is tracked, so the
/// release has a final position to write.
async fn enter(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    dependencies: &Dependencies,
) -> TestResult {
    dependencies.initialize(owner).await;
    owner
        .runtime
        .lock()
        .await
        .initialize_first_entry_position(dependencies.actor)
        .map_err(debug)?;
    track(owner, dependencies)
}

#[test]
fn logout_commits_terminal_saves_the_position_and_retires_the_actor() -> TestResult {
    run(async |admin| {
        reset();
        let harness =
            Harness::create_with_admission_seed(admin, "logout_accepted", true, false).await?;
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let dependencies = Dependencies::new()?;
        let owner = dependencies.owner(&harness, &authority);
        enter(&owner, &dependencies).await?;
        assert_eq!(last_positions(&harness).await?, 0);

        assert_eq!(
            owner.logout_session(dependencies.admitted).await,
            LogoutReply::Result(LogoutResult::Accepted)
        );
        // §1.6: TERMINAL with no grace and no loss epoch; the actor has left the Channel.
        assert_eq!(
            session_state(&harness, &dependencies).await?,
            GameSessionState::Terminal
        );
        let terminal: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_durability_reconnect_sessions \
             WHERE session_state=3 AND control_loss_epoch IS NULL",
        )
        .fetch_one(&harness.pool)
        .await?;
        assert_eq!(terminal, 1, "TERMINAL with no loss epoch");
        assert!(!actor_present(&owner, &dependencies).await);
        // CHAR-POSITION-1: the release wrote the fenced actor's cell.
        assert_eq!(last_positions(&harness).await?, 1);
        // The monk save point ran before the lease ended.
        assert_eq!(revision(&harness).await?, 2);

        // A replay of the release is TERMINAL, never a second commit.
        let replay = FreshAdmissionStore::from_root(harness.root.clone())
            .release_logout_session(
                dependencies.admitted.game_session_id,
                &crate::gameplay_transport::canonical_uuid(&id(40)),
                dependencies
                    .admitted
                    .controller
                    .ok_or("controller")?
                    .transport,
            )
            .await
            .map_err(debug)?;
        assert!(matches!(replay, ExpiredLossReleaseV1::Terminal));
        // A second logout of the ended session refuses at once: there is no actor left.
        assert_eq!(
            owner.logout_session(dependencies.admitted).await,
            LogoutReply::Result(LogoutResult::Busy)
        );
        assert_eq!(revision(&harness).await?, 2);
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

#[test]
fn logout_in_fight_is_refused_and_changes_nothing() -> TestResult {
    run(async |admin| {
        reset();
        let harness =
            Harness::create_with_admission_seed(admin, "logout_in_fight", true, false).await?;
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let dependencies = Dependencies::new()?;
        let owner = dependencies.owner(&harness, &authority);
        enter(&owner, &dependencies).await?;
        let now = owner.owner_now();
        {
            let runtime = owner.runtime.lock().await;
            owner.states.attack.lock().await.record_hit_taken(
                &runtime,
                dependencies.actor,
                dependencies.admitted.game_session_id,
                now,
            );
        }
        let until = owner
            .states
            .attack
            .lock()
            .await
            .in_fight_until(
                dependencies.actor,
                dependencies.admitted.game_session_id,
                now,
            )
            .ok_or("the hit starts the in-fight deadline")?;
        let LogoutReply::Result(LogoutResult::InFight { retry_after_ms }) =
            owner.logout_session(dependencies.admitted).await
        else {
            return Err("an in-fight logout is IN_FIGHT".into());
        };
        // The retry is the deadline left, in whole milliseconds rounded up, read after `now`.
        let left_ms = until.get().saturating_sub(now.get()).div_ceil(1_000);
        assert!(retry_after_ms >= 1 && u64::from(retry_after_ms) <= left_ms);
        assert!(left_ms - u64::from(retry_after_ms) <= 1_000);
        // Nothing changed: the session is ACTIVE, the actor present, nothing written.
        assert_eq!(
            session_state(&harness, &dependencies).await?,
            GameSessionState::Active
        );
        assert!(actor_present(&owner, &dependencies).await);
        assert_eq!(last_positions(&harness).await?, 0);
        assert_eq!(revision(&harness).await?, 1);
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

#[test]
fn logout_of_a_superseded_controller_is_busy_and_lifts_its_fence() -> TestResult {
    run(async |admin| {
        reset();
        let harness =
            Harness::create_with_admission_seed(admin, "logout_stale", true, false).await?;
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let dependencies = Dependencies::new()?;
        let owner = dependencies.owner(&harness, &authority);
        enter(&owner, &dependencies).await?;
        // A newer controller generation holds the session on another transport: the logout's
        // stale generation releases nothing.
        sqlx::query(
            "UPDATE game_durability_reconnect_sessions SET current_transport_ref=$1, \
             current_generation=current_generation+1 WHERE game_session_id=encode($2,'hex')::uuid",
        )
        .bind([0x0b_u8; 16].as_slice())
        .bind(id(50).as_slice())
        .execute(&harness.pool)
        .await?;
        assert_eq!(
            owner.logout_session(dependencies.admitted).await,
            LogoutReply::Result(LogoutResult::Busy)
        );
        assert_eq!(
            session_state(&harness, &dependencies).await?,
            GameSessionState::Active
        );
        assert!(actor_present(&owner, &dependencies).await);
        // The store itself refuses the stale transport.
        assert!(matches!(
            FreshAdmissionStore::from_root(harness.root.clone())
                .release_logout_session(
                    dependencies.admitted.game_session_id,
                    &crate::gameplay_transport::canonical_uuid(&id(40)),
                    dependencies
                        .admitted
                        .controller
                        .ok_or("controller")?
                        .transport,
                )
                .await
                .map_err(debug)?,
            ExpiredLossReleaseV1::NotApplicable
        ));

        // The BUSY lifted the fence: once the session is this controller's again, a retry
        // fences and commits.
        sqlx::query(
            "UPDATE game_durability_reconnect_sessions SET current_transport_ref=$1, \
             current_generation=current_generation+1 WHERE game_session_id=encode($2,'hex')::uuid",
        )
        .bind([9_u8; 16].as_slice())
        .bind(id(50).as_slice())
        .execute(&harness.pool)
        .await?;
        assert_eq!(
            owner.logout_session(dependencies.admitted).await,
            LogoutReply::Result(LogoutResult::Accepted)
        );
        assert_eq!(
            session_state(&harness, &dependencies).await?,
            GameSessionState::Terminal
        );
        assert!(!actor_present(&owner, &dependencies).await);
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

#[test]
fn an_uncommitted_unknown_logout_becomes_an_ordinary_loss() -> TestResult {
    run(async |admin| {
        reset();
        let harness =
            Harness::create_with_admission_seed(admin, "logout_unknown", true, false).await?;
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let dependencies = Dependencies::new()?;
        let owner = dependencies.owner(&harness, &authority);
        enter(&owner, &dependencies).await?;
        // The release transaction cannot read the claim rows: its outcome is unknown.
        sqlx::query("ALTER TABLE game_durability_admission_guard_history RENAME TO logout_fixture_unavailable_guards").execute(&harness.pool).await?;
        assert_eq!(
            owner.logout_session(dependencies.admitted).await,
            LogoutReply::Unknown
        );
        sqlx::query("ALTER TABLE logout_fixture_unavailable_guards RENAME TO game_durability_admission_guard_history").execute(&harness.pool).await?;
        assert_eq!(
            session_state(&harness, &dependencies).await?,
            GameSessionState::Active
        );
        assert!(actor_present(&owner, &dependencies).await);

        // The connection closed with no result: the lifecycle lifts the kept fence and records
        // the transport loss as for any closed client.
        let controller = dependencies.admitted.controller.ok_or("controller")?;
        assert_eq!(
            owner
                .commit_control_loss(
                    dependencies.admitted.game_session_id,
                    dependencies.actor,
                    controller,
                )
                .await,
            ControlLossResult::Recorded
        );
        assert_eq!(
            session_state(&harness, &dependencies).await?,
            GameSessionState::Reconnectable
        );
        assert!(actor_present(&owner, &dependencies).await);
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

#[test]
fn a_committed_logout_whose_acknowledgement_was_lost_is_reconciled_from_the_row() -> TestResult {
    run(async |admin| {
        reset();
        let harness =
            Harness::create_with_admission_seed(admin, "logout_reconcile", true, false).await?;
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let dependencies = Dependencies::new()?;
        let owner = dependencies.owner(&harness, &authority);
        enter(&owner, &dependencies).await?;
        let controller = dependencies.admitted.controller.ok_or("controller")?;
        // The release committed, but the owner never learned it: the actor is still present.
        assert!(matches!(
            FreshAdmissionStore::from_root(harness.root.clone())
                .release_logout_session(
                    dependencies.admitted.game_session_id,
                    &crate::gameplay_transport::canonical_uuid(&id(40)),
                    controller.transport,
                )
                .await
                .map_err(debug)?,
            ExpiredLossReleaseV1::Released { .. }
        ));
        assert!(actor_present(&owner, &dependencies).await);

        // The lifecycle finds the row TERMINAL and retires the actor; nothing is resumed.
        assert_eq!(
            owner
                .commit_control_loss(
                    dependencies.admitted.game_session_id,
                    dependencies.actor,
                    controller,
                )
                .await,
            ControlLossResult::Terminal
        );
        assert_eq!(
            owner
                .reconcile_terminal_session(dependencies.admitted)
                .await,
            GraceExpiryResult::Released
        );
        assert!(!actor_present(&owner, &dependencies).await);
        // A repeat (another reconcile pass) finds the slot gone and still settles.
        assert_eq!(
            owner
                .reconcile_terminal_session(dependencies.admitted)
                .await,
            GraceExpiryResult::Released
        );
        // The ended session's stale generation cannot write its character any more.
        assert_eq!(
            owner
                .save_monk_state(&dependencies.admitted, dependencies.actor)
                .await,
            super::MonkSave::FencedOut
        );
        assert_eq!(revision(&harness).await?, 1);
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

#[test]
fn a_transport_other_than_the_current_one_never_releases() -> TestResult {
    run(async |admin| {
        reset();
        let harness =
            Harness::create_with_admission_seed(admin, "logout_transport", true, false).await?;
        prepare_current_session(&harness).await?;
        let store = FreshAdmissionStore::from_root(harness.root.clone());
        let session = crate::foundation::GameSessionId::decode(&id(50)).map_err(debug)?;
        let account = crate::gameplay_transport::canonical_uuid(&id(40));
        let other = AuthenticatedTransportRefV1::decode(&[0x0b; 16]).map_err(debug)?;
        assert!(matches!(
            store
                .release_logout_session(session, &account, other)
                .await
                .map_err(debug)?,
            ExpiredLossReleaseV1::NotApplicable
        ));
        // A RECONNECTABLE session is not a logout's either.
        sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state=1, current_transport_ref=NULL, control_loss_epoch=1, predecessor_generation=current_generation, original_grace_deadline=extract(epoch FROM clock_timestamp())::bigint + 60 WHERE game_session_id=encode($1,'hex')::uuid")
            .bind(id(50).as_slice()).execute(&harness.pool).await?;
        let own = AuthenticatedTransportRefV1::decode(&[9; 16]).map_err(debug)?;
        assert!(matches!(
            store
                .release_logout_session(session, &account, own)
                .await
                .map_err(debug)?,
            ExpiredLossReleaseV1::NotApplicable
        ));
        let (current, _) = store.current_session_at(session).await.map_err(debug)?;
        assert_eq!(current.session_state(), GameSessionState::Reconnectable);
        harness.cleanup().await
    })
}
