//! LOGOUT-WIRE-1 (ARCH-KILL-REWARD-LOGOUT-1 §1.6, §2.2) on PostgreSQL: the real composed
//! owner's logout of the fresh-admission fixture session. The terminal release commits through
//! the exact fenced store path, writes the actor's final position (CHAR-POSITION-1) and retires
//! the actor; a refusal changes nothing; an unknown outcome is reconciled from the row.
//!
//! Nested under the monk-save PostgreSQL tests to reuse their fresh-admission fixture.

use super::fixture::{Harness, debug, id};
use super::kill_reward_e2e::scenario_thread::run_async as run;
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
fn nbeof_logout_busy_holder_postgres() -> TestResult {
    run(async |admin| {
        use crate::gameplay_transport::connection::FreshAdmissionAuthority;
        use std::time::{Duration, Instant};

        reset();
        let harness =
            Harness::create_with_admission_seed(admin, "logout_busy_holder", true, false).await?;
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

        let held = owner.root.try_issue_semantic_pass()?;
        let started = Instant::now();
        assert_eq!(
            owner
                .reconcile_terminal_session(dependencies.admitted)
                .await,
            GraceExpiryResult::Unknown
        );
        assert!(
            started.elapsed() >= Duration::from_millis(500),
            "reconciliation must retry the busy holder"
        );
        assert!(
            started.elapsed() < Duration::from_secs(2),
            "reconciliation retry must be bounded"
        );
        assert!(owner.runtime.try_lock().is_ok());
        assert!(owner.states.spell_states.try_lock().is_ok());
        held.run(|_, _| Box::pin(async { Ok(()) })).await?;

        {
            use crate::gameplay_transport::{FenceHold, FenceStep, TerminalRelease, UnendedSettle};
            let controller = dependencies.admitted.controller.ok_or("controller")?;
            let token = TerminalRelease::Logout(controller.transport)
                .fence(&mut *owner.runtime.lock().await)
                .map_err(debug)?;
            let mut hold = FenceHold::new(dependencies.admitted.game_session_id, token);
            assert!(matches!(
                owner
                    .fence_transition(dependencies.actor, &mut hold, true)
                    .await,
                FenceStep::Fenced
            ));
            let held = owner.root.try_issue_semantic_pass()?;
            let store = FreshAdmissionStore::from_root(harness.root.clone());
            let settling = owner.settle_unended(&store, dependencies.actor, &mut hold);
            tokio::pin!(settling);
            let polled = std::future::poll_fn(|cx| {
                std::task::Poll::Ready(std::future::Future::poll(settling.as_mut(), cx))
            })
            .await;
            assert!(
                polled.is_pending(),
                "release reconciliation must wait for its holder"
            );
            assert!(owner.runtime.try_lock().is_ok());
            assert!(owner.states.spell_states.try_lock().is_ok());
            held.run(|_, _| Box::pin(async { Ok(()) })).await?;
            assert!(matches!(settling.await, UnendedSettle::Lifted));
        }

        {
            let held = owner.root.try_issue_semantic_pass()?;
            let logout = owner.logout(dependencies.admitted);
            tokio::pin!(logout);
            let polled = std::future::poll_fn(|cx| {
                std::task::Poll::Ready(std::future::Future::poll(logout.as_mut(), cx))
            })
            .await;
            assert!(polled.is_pending(), "logout must wait for a busy holder");
            assert!(owner.runtime.try_lock().is_ok());
            assert!(owner.states.spell_states.try_lock().is_ok());
            held.run(|_, _| Box::pin(async { Ok(()) })).await?;
            // Do not yield after the return: the scheduled owner must refuse the
            // idle holder until the real logout, including its saves, progresses.
            assert!(matches!(
                owner.root.try_issue_background_semantic_pass(),
                Err(crate::durability::DurabilityError::RootUnavailable)
            ));
            assert_eq!(logout.await, LogoutReply::Result(LogoutResult::Accepted));
            owner
                .root
                .try_issue_background_semantic_pass()?
                .run(|_, _| Box::pin(async { Ok(()) }))
                .await?;
        }
        assert_eq!(
            session_state(&harness, &dependencies).await?,
            GameSessionState::Terminal
        );
        assert!(!actor_present(&owner, &dependencies).await);
        assert_eq!(last_positions(&harness).await?, 1);
        eprintln!(
            "NBEOF-2 executed busy-holder reconciliation and real logout; elapsed_ms={}",
            started.elapsed().as_millis()
        );
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

#[test]
fn nbeof_lifecycle_reads_keep_holder_preference_postgres() -> TestResult {
    run(async |admin| {
        use crate::durability::DurabilityError;
        use crate::gameplay_transport::AdmissionRetryBudget;
        use crate::gameplay_transport::actor_spell::TrainingSave;

        reset();
        let harness =
            Harness::create_with_admission_seed(admin, "lifecycle_holder_reads", true, false)
                .await?;
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let dependencies = Dependencies::new()?;
        let (generation, _) = super::death_ml_loss_composed::production_generation_with_pin(
            dependencies.admitted.world_id,
            "content-1",
        )?;
        let active = generation.active().ok_or("activated generation")?;
        assert!(
            active
                .native_gameplay()
                .and_then(|native| native.training_formula())
                .is_some()
        );
        let mut owner = dependencies.owner(&harness, &authority);
        owner.active_generation = Some(active);
        enter(&owner, &dependencies).await?;

        // A fresh actor has no training checkpoint to replay. Its native save
        // still reads the current session and Character before discovering that.
        let before = revision(&harness).await?;
        let before_build_receipts: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_character_build_receipts")
                .fetch_one(&harness.pool)
                .await?;
        let held = owner.root.try_issue_semantic_pass()?;
        let mut training = Box::pin(owner.save_spell_training(
            dependencies.actor,
            dependencies.admitted.game_session_id,
            true,
        ));
        let polled = std::future::poll_fn(|cx| {
            std::task::Poll::Ready(std::future::Future::poll(training.as_mut(), cx))
        })
        .await;
        assert!(
            polled.is_pending(),
            "native training reads must await the holder"
        );
        assert!(owner.runtime.try_lock().is_ok());
        assert!(owner.states.spell_states.try_lock().is_ok());
        held.run(|_, _| Box::pin(async { Ok(()) })).await?;
        assert!(matches!(
            owner.root.try_issue_background_semantic_pass(),
            Err(DurabilityError::RootUnavailable)
        ));
        assert_eq!(training.await, TrainingSave::NotApplicable);
        assert_eq!(revision(&harness).await?, before);

        // Cancelling an owner that is still waiting releases only its demand.
        let held = owner.root.try_issue_semantic_pass()?;
        let mut cancelled = Box::pin(async {
            let budget = AdmissionRetryBudget::for_foreground_command(owner.root)?;
            budget
                .retry_durability("cancelled_read", async || {
                    FreshAdmissionStore::from_root(owner.root.clone())
                        .current_session(dependencies.admitted.game_session_id)
                        .await
                })
                .await
        });
        let polled = std::future::poll_fn(|cx| {
            std::task::Poll::Ready(std::future::Future::poll(cancelled.as_mut(), cx))
        })
        .await;
        assert!(polled.is_pending());
        drop(cancelled);
        held.run(|_, _| Box::pin(async { Ok(()) })).await?;
        owner
            .root
            .try_issue_background_semantic_pass()?
            .run(|_, _| Box::pin(async { Ok(()) }))
            .await?;

        // Loss keeps the same demand from its first read through its single
        // durable decision. No gap is inserted before the background retake.
        let held = owner.root.try_issue_semantic_pass()?;
        let mut loss = Box::pin(owner.commit_control_loss(
            dependencies.admitted.game_session_id,
            dependencies.actor,
            dependencies.admitted.controller.ok_or("controller")?,
        ));
        let polled = std::future::poll_fn(|cx| {
            std::task::Poll::Ready(std::future::Future::poll(loss.as_mut(), cx))
        })
        .await;
        assert!(
            polled.is_pending(),
            "loss must await its first current read"
        );
        held.run(|_, _| Box::pin(async { Ok(()) })).await?;
        assert!(matches!(
            owner.root.try_issue_background_semantic_pass(),
            Err(DurabilityError::RootUnavailable)
        ));
        assert_eq!(loss.await, ControlLossResult::Recorded);
        let current = FreshAdmissionStore::from_root(owner.root.clone())
            .current_session(dependencies.admitted.game_session_id)
            .await?;
        assert_eq!(current.session_state(), GameSessionState::Reconnectable);
        assert_eq!(
            current
                .current_control_loss_epoch()
                .map(|epoch| epoch.get()),
            Some(1)
        );
        assert_eq!(revision(&harness).await?, before);
        let build_receipts: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_character_build_receipts")
                .fetch_one(&harness.pool)
                .await?;
        assert_eq!(
            build_receipts, before_build_receipts,
            "waiting reads must not manufacture a checkpoint"
        );

        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

#[test]
fn nbeof_training_readback_waits_without_replaying_write_postgres() -> TestResult {
    run(async |admin| {
        use crate::durability::DurabilityError;
        use crate::gameplay_transport::AdmissionRetryBudget;
        use crate::gameplay_transport::actor_spell::{TRAINING_READBACK_BARRIER, TrainingSave};
        use oteryn_gameplay_progression::hit_training::SkillFamily;
        use std::sync::Arc;
        use tokio::sync::Notify;

        reset();
        let harness =
            Harness::create_with_admission_seed(admin, "training_readback_busy", true, false)
                .await?;
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let dependencies = Dependencies::new()?;
        let (generation, _) = super::death_ml_loss_composed::production_generation_with_pin(
            dependencies.admitted.world_id,
            "content-1",
        )?;
        let active = generation.active().ok_or("activated generation")?;
        let formula = active
            .native_gameplay()
            .and_then(|native| native.training_formula())
            .ok_or("training formula")?;
        let mut owner = dependencies.owner(&harness, &authority);
        owner.active_generation = Some(active);
        let actor = dependencies.actor;
        let session = dependencies.admitted.game_session_id;
        let character = owner
            .current_quest_fence(session)
            .await
            .map_err(|()| "current character fence")?
            .ok_or("current character fence")?
            .character_id;
        let build = owner
            .root
            .read_character_build_state(&authority, character)
            .await
            .map_err(debug)?;
        {
            let mut runtime = owner.runtime.lock().await;
            runtime
                .initialize_first_entry_position(actor)
                .map_err(debug)?;
            let mut states = owner.states.spell_states.lock().await;
            assert!(
                states
                    .initialize(
                        &runtime,
                        actor,
                        session,
                        crate::spell::cast::CharacterCastFacts {
                            vocation: crate::spell::Vocation::from_key(build.vocation())
                                .ok_or("build vocation")?,
                            magic_level: u32::from(build.magic().0),
                            ..crate::gameplay_transport::actor_spell::tests::FACTS
                        },
                        (0, 0),
                        owner.owner_now(),
                    )
                    .is_some()
            );
            assert!(states.load_owned_build(&runtime, actor, session, &build));
            states
                .get_mut(&runtime, actor, session)
                .ok_or("actor")?
                .enable_owned_training(&build, formula)
                .map_err(debug)?;
        }

        for stale in [false, true] {
            {
                let runtime = owner.runtime.lock().await;
                let mut states = owner.states.spell_states.lock().await;
                states
                    .get_mut(&runtime, actor, session)
                    .ok_or("actor")?
                    .train_skill(SkillFamily::Sword, owner.owner_now().get(), formula, |_| 1)
                    .map_err(debug)?;
            }
            let before_receipts: i64 =
                sqlx::query_scalar("SELECT count(*) FROM game_character_build_receipts")
                    .fetch_one(&harness.pool)
                    .await?;
            let barrier = Arc::new((Notify::new(), Notify::new()));
            // Inspection of a real committed receipt shares one existing root
            // budget; it never renews the production command retry deadline.
            let budget = AdmissionRetryBudget::for_foreground_root(&session, owner.root)?;
            let mut saving = Box::pin(TRAINING_READBACK_BARRIER.scope(
                Arc::clone(&barrier),
                owner.save_spell_training_with_budget(actor, session, true, &budget),
            ));
            tokio::select! {
                result = saving.as_mut() => return Err(format!("save ended before readback: {result:?}").into()),
                () = barrier.0.notified() => {}
            }
            assert!(owner.runtime.try_lock().is_ok());
            assert!(owner.states.spell_states.try_lock().is_ok());
            let retained = {
                let runtime = owner.runtime.lock().await;
                let states = owner.states.spell_states.lock().await;
                states
                    .get(&runtime, actor, session)
                    .ok_or("actor")?
                    .pending_training_checkpoint()
                    .ok_or("retained checkpoint")?
                    .clone()
            };
            let committed_receipts: i64 =
                sqlx::query_scalar("SELECT count(*) FROM game_character_build_receipts")
                    .fetch_one(&harness.pool)
                    .await?;
            assert_eq!(committed_receipts, before_receipts + 1);

            if stale {
                // A genuine independently current terminal release invalidates
                // the old live-install authority after the checkpoint committed.
                let controller = dependencies.admitted.controller.ok_or("controller")?;
                let account = crate::gameplay_transport::canonical_uuid(&controller.account_id);
                let released = FreshAdmissionStore::from_root(owner.root.clone())
                    .release_logout_session(session, &account, controller.transport)
                    .await?;
                assert!(matches!(released, ExpiredLossReleaseV1::Released { .. }));
            }
            let held = owner.root.try_issue_semantic_pass()?;
            barrier.1.notify_one();
            let polled = std::future::poll_fn(|cx| {
                std::task::Poll::Ready(std::future::Future::poll(saving.as_mut(), cx))
            })
            .await;
            assert!(
                polled.is_pending(),
                "busy readback must wait without repeating the write"
            );
            assert!(owner.runtime.try_lock().is_ok());
            assert!(owner.states.spell_states.try_lock().is_ok());
            held.run(|_, _| Box::pin(async { Ok(()) })).await?;
            assert!(matches!(
                owner.root.try_issue_background_semantic_pass(),
                Err(DurabilityError::RootUnavailable)
            ));
            assert_eq!(
                saving.await,
                if stale {
                    TrainingSave::FencedOut
                } else {
                    TrainingSave::Saved
                }
            );
            let after_receipts: i64 =
                sqlx::query_scalar("SELECT count(*) FROM game_character_build_receipts")
                    .fetch_one(&harness.pool)
                    .await?;
            assert_eq!(
                after_receipts, committed_receipts,
                "readback must never resend the write"
            );
            let runtime = owner.runtime.lock().await;
            let states = owner.states.spell_states.lock().await;
            let pending = states
                .get(&runtime, actor, session)
                .ok_or("actor")?
                .pending_training_checkpoint();
            if stale {
                assert_eq!(
                    pending,
                    Some(&retained),
                    "stale authority must not acknowledge the checkpoint"
                );
            } else {
                assert!(pending.is_none());
            }
        }

        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

#[test]
fn nbeof_logout_wire_postgres() -> TestResult {
    run(async |admin| {
        use crate::foundation::{MessageType, decode_wire_envelope};
        use crate::gameplay_transport::capabilities::{SelectedCapabilities, test_command};
        use crate::gameplay_transport::connection::{
            ConnectionEnd, FirstEntryOutcome, FreshAdmissionAuthority, IDLE_LIVENESS,
            serve_admitted,
        };
        use oteryn_protocol_oteryn::logout::{
            CAPABILITY_LOGOUT_V1, COMMAND_TYPE_LOGOUT_INTENT, decode_logout_result,
            encode_logout_intent,
        };
        use tokio::io::AsyncWriteExt;

        reset();
        let harness =
            Harness::create_with_admission_seed(admin, "logout_wire", true, false).await?;
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
        let mut admitted = dependencies.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(owner.offered_capabilities(), &[CAPABILITY_LOGOUT_V1])
                .ok_or("logout capability")?;
        {
            let (mut server, mut client) = tokio::io::duplex(1 << 20);
            let serving = async {
                let end = serve_admitted(&mut server, admitted, &owner, IDLE_LIVENESS).await;
                assert!(
                    matches!(end, ConnectionEnd::AdmittedThenLoggedOut(_)),
                    "{end:?}"
                );
                drop(server);
            };
            let requesting = async {
                let command = test_command(1, COMMAND_TYPE_LOGOUT_INTENT, &encode_logout_intent());
                client.write_u32(u32::try_from(command.len())?).await?;
                client.write_all(&command).await?;
                loop {
                    let frame = super::item_look::read_frame(&mut client).await?;
                    let envelope = decode_wire_envelope(&frame).map_err(debug)?;
                    if envelope.message_type() == MessageType::CommandResult {
                        let result =
                            oteryn_protocol_oteryn::decode_command_result(envelope.payload())
                                .map_err(debug)?;
                        assert_eq!(result.command_id, 1);
                        assert_eq!(
                            result.status,
                            oteryn_protocol_oteryn::CommandStatus::Accepted
                        );
                        assert_eq!(
                            decode_logout_result(result.payload).map_err(debug)?,
                            LogoutResult::Accepted
                        );
                        break;
                    }
                }
                Ok::<_, Box<dyn std::error::Error>>(())
            };
            let ((), result) = tokio::join!(serving, requesting);
            result?;
        }
        assert_eq!(
            session_state(&harness, &dependencies).await?,
            GameSessionState::Terminal
        );
        assert!(!actor_present(&owner, &dependencies).await);
        eprintln!(
            "NBEOF-2 executed LOGOUT_INTENT through real serve dispatch and PostgreSQL terminal release"
        );
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
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
        let mut demand = Some(owner.root.foreground_demand()?);
        assert!(
            !owner
                .hold_while_in_fight_releasing_demand(
                    dependencies.actor,
                    dependencies.admitted.game_session_id,
                    &mut demand,
                )
                .await
        );
        assert!(
            demand.is_some(),
            "a quick no-fight check must keep the same window"
        );
        drop(demand);
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
        let mut demand = Some(owner.root.foreground_demand()?);
        let mut hold = Box::pin(owner.hold_while_in_fight_releasing_demand(
            dependencies.actor,
            dependencies.admitted.game_session_id,
            &mut demand,
        ));
        let polled = std::future::poll_fn(|cx| {
            std::task::Poll::Ready(std::future::Future::poll(hold.as_mut(), cx))
        })
        .await;
        assert!(polled.is_pending());
        // A real combat wait must permit background work even when a previous
        // release attempt retained a demand. Cancel before the melee step.
        owner
            .root
            .try_issue_background_semantic_pass()?
            .run(|_, _| Box::pin(async { Ok(()) }))
            .await?;
        drop(hold);
        assert!(demand.is_none());

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
