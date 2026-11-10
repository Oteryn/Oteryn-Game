//! The original familiar clean end must settle before logout or loss can pass its guard.
use super::*;
use crate::durability::character_familiar::{FamiliarCleanEndResolution, FamiliarStateOccurrence};
use crate::durability::fresh_admission::ExpiredLossReleaseV1;
use crate::foundation::{CharacterId, CharacterLease, GameSessionState};
use crate::gameplay_transport::actor_spell::{FamiliarActorEnd, FamiliarLogoutSave};
use crate::gameplay_transport::connection::{
    ControlLossResult, FreshAdmissionAuthority, LogoutReply,
};
use oteryn_protocol_oteryn::logout::LogoutResult;

#[test]
fn familiar_logout_committed_unknown_retries_logout_without_replaying_write_postgres() -> TestResult
{
    scenario(true, Finish::Logout)
}

#[test]
fn familiar_logout_uncommitted_busy_retries_logout_after_proven_absence_postgres() -> TestResult {
    scenario(false, Finish::Logout)
}

#[test]
fn familiar_logout_committed_unknown_allows_loss_and_grace_without_replay_postgres() -> TestResult {
    scenario(true, Finish::Loss)
}

#[test]
fn familiar_logout_uncommitted_busy_allows_loss_and_exact_grace_reconciliation_postgres()
-> TestResult {
    scenario(false, Finish::Loss)
}

#[test]
fn familiar_logout_reconciliation_keeps_stale_and_terminal_original_postgres() -> TestResult {
    scenario(true, Finish::Terminal)
}

#[test]
fn familiar_logout_reconciliation_keeps_retired_scope_original_postgres() -> TestResult {
    scenario(true, Finish::RetiredOwner)
}

#[derive(Clone, Copy)]
enum Finish {
    Logout,
    Loss,
    Terminal,
    RetiredOwner,
}

fn scenario(committed: bool, finish: Finish) -> TestResult {
    // This helper owns a 64 MiB scenario thread and skips only route-less lanes.
    kill_reward_e2e::scenario_thread::run_async(async move |admin| {
        let name = match (committed, finish) {
            (true, Finish::Logout) => "familiar_logout_known_retry",
            (false, Finish::Logout) => "familiar_logout_absent_retry",
            (true, Finish::Loss) => "familiar_logout_known_loss",
            (false, Finish::Loss) => "familiar_logout_absent_loss",
            (_, Finish::Terminal) => "familiar_logout_terminal",
            (_, Finish::RetiredOwner) => "familiar_logout_retired",
        };
        let h = Harness::create_with_admission_seed(admin, name, true, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = fence(1)?.runtime_scope.world_id();
        let (generation, pin) =
            death_ml_loss_composed::production_generation_with_pin(world, "content-1")?;
        let active = generation.active().ok_or("active generation")?;
        let spells = active
            .native_gameplay()
            .ok_or("native gameplay")?
            .spell_book()
            .clone();
        let room = pin.qualified_room().clone();
        let (channel_pin, _, _, _) = pin.into_channel_parts();
        let deps = Dependencies::with_content(channel_pin, room, spells)?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
        deps.initialize(&owner).await;
        let actor = deps.actor;
        let session = deps.admitted.game_session_id;
        let controller = deps.admitted.controller.ok_or("controller")?;
        {
            let mut runtime = owner.runtime.lock().await;
            runtime
                .initialize_first_entry_position(actor)
                .map_err(debug)?;
            runtime
                .bind_attacker_lease(
                    actor,
                    session,
                    CharacterLease::new(CharacterId::decode(&id(41)).map_err(debug)?, 2)
                        .map_err(debug)?,
                )
                .map_err(debug)?;
            assert!(
                runtime
                    .owned_companions(actor, session)
                    .map_err(debug)?
                    .is_empty()
            );
        }
        track(&owner, &deps)?;
        let store = FreshAdmissionStore::from_root(h.root.clone());

        // This is the real logout producer and its minted transition, with no
        // physical familiar. Hide only a real commit acknowledgement, or hold
        // the real root precisely after retention and before the write attempt.
        let (first, writes) = owner
            .test_familiar_clean_end_probe(owner.logout_session(deps.admitted), Some(committed))
            .await;
        assert_eq!(first, LogoutReply::Result(LogoutResult::Busy));
        assert_eq!(
            writes, 1,
            "the first logout reaches the real familiar writer"
        );
        let original = {
            let states = owner.states.spell_states.lock().await;
            assert_eq!(states.pending_familiar_logouts.len(), 1);
            states.pending_familiar_logouts[0].clone()
        };
        let (end, original_fence, request) = original.test_original();
        let FamiliarActorEnd::Logout(transport, attempt) = end else {
            return Err("the original must belong to logout".into());
        };
        assert_eq!(transport, controller.transport);
        assert!(attempt > 0);
        assert_eq!(
            receipt_count_for(&h, None).await?,
            if committed { 1 } else { 0 }
        );
        assert_eq!(
            store.current_session(session).await?.session_state(),
            GameSessionState::Active
        );
        let live = {
            let runtime = owner.runtime.lock().await;
            owner
                .states
                .spell_states
                .lock()
                .await
                .get(&runtime, actor, session)
                .ok_or("live player")?
                .clone()
        };

        // Neither a missing holder nor another actor end is proof that this
        // original disappeared. Both must leave its exact snapshot retained.
        let held = h.root.try_issue_semantic_pass()?;
        assert!(
            !owner
                .reconcile_pending_familiar_logout(actor, session)
                .await
        );
        assert_eq!(
            owner
                .states
                .spell_states
                .lock()
                .await
                .pending_familiar_logouts,
            vec![original.clone()]
        );
        held.run(|_, _| Box::pin(async { Ok(()) })).await?;
        let (other, writes) = owner
            .test_familiar_clean_end_probe(
                owner.save_familiar_logout(
                    actor,
                    session,
                    FamiliarActorEnd::Logout(
                        transport,
                        attempt.checked_add(1).ok_or("attempt overflow")?,
                    ),
                ),
                None,
            )
            .await;
        assert_eq!(other, FamiliarLogoutSave::Unknown);
        assert_eq!(writes, 0);
        assert_eq!(
            owner
                .states
                .spell_states
                .lock()
                .await
                .pending_familiar_logouts,
            vec![original.clone()]
        );

        if matches!(finish, Finish::Terminal | Finish::RetiredOwner) {
            assert!(matches!(
                h.root
                    .reconcile_familiar_clean_end(&authority, original_fence, request.clone(),)
                    .await
                    .map_err(debug)?,
                FamiliarCleanEndResolution::Committed(_)
            ));
            // Historical success must not erase an original whose lease is no
            // longer the independently current one.
            let mut stale = original.clone();
            let mut stale_fence = original_fence;
            stale_fence.character_lease_generation = stale_fence
                .character_lease_generation
                .checked_add(1)
                .ok_or("lease overflow")?;
            stale.test_set_fence(stale_fence);
            owner
                .states
                .spell_states
                .lock()
                .await
                .pending_familiar_logouts[0] = stale.clone();
            assert!(
                !owner
                    .reconcile_pending_familiar_logout(actor, session)
                    .await
            );
            assert_eq!(
                owner
                    .states
                    .spell_states
                    .lock()
                    .await
                    .pending_familiar_logouts,
                vec![stale]
            );
            owner
                .states
                .spell_states
                .lock()
                .await
                .pending_familiar_logouts[0] = original.clone();
            match finish {
                Finish::Terminal => {
                    assert!(matches!(
                        store
                            .release_logout_session(
                                session,
                                &crate::gameplay_transport::canonical_uuid(&controller.account_id),
                                transport,
                            )
                            .await?,
                        ExpiredLossReleaseV1::Released { .. }
                    ));
                    assert_eq!(
                        store.current_session(session).await?.session_state(),
                        GameSessionState::Terminal
                    );
                }
                Finish::RetiredOwner => {
                    owner.runtime.lock().await.retire_owner_cycle();
                    assert!(owner.runtime.lock().await.owner_fence().is_err());
                    assert_eq!(
                        store.current_session(session).await?.session_state(),
                        GameSessionState::Active
                    );
                }
                Finish::Logout | Finish::Loss => return Err("not a refusal case".into()),
            }
            assert!(
                !owner
                    .reconcile_pending_familiar_logout(actor, session)
                    .await
            );
            assert_eq!(
                owner
                    .states
                    .spell_states
                    .lock()
                    .await
                    .pending_familiar_logouts,
                vec![original]
            );
            assert_eq!(receipt_count_for(&h, Some(request.occurrence)).await?, 1);
            drop(owner);
            drop(authority);
            drop(seal);
            return h.cleanup().await;
        }

        match finish {
            Finish::Logout => {
                let (reply, writes) = owner
                    .test_familiar_clean_end_probe(owner.logout_session(deps.admitted), None)
                    .await;
                assert_eq!(reply, LogoutReply::Result(LogoutResult::Accepted));
                assert_eq!(writes, if committed { 0 } else { 1 });
            }
            Finish::Loss => {
                let (loss, writes) = owner
                    .test_familiar_clean_end_probe(
                        owner.lose_control(deps.admitted, Duration::ZERO, None),
                        None,
                    )
                    .await;
                assert_eq!(loss, ControlLossResult::Recorded);
                assert_eq!(
                    writes, 0,
                    "loss reconciles; it cannot replay the logout write"
                );
                assert!(
                    owner
                        .states
                        .spell_states
                        .lock()
                        .await
                        .pending_familiar_logouts
                        .is_empty()
                );
                let current = store.current_session(session).await?;
                assert_eq!(current.session_state(), GameSessionState::Reconnectable);
                assert_eq!(
                    current
                        .current_control_loss_epoch()
                        .ok_or("loss epoch")?
                        .get(),
                    1
                );
                {
                    let runtime = owner.runtime.lock().await;
                    assert_eq!(
                        owner
                            .states
                            .spell_states
                            .lock()
                            .await
                            .get(&runtime, actor, session),
                        Some(&live)
                    );
                }
                let wait = Duration::from_secs(
                    u64::try_from(crate::gameplay_transport::SAME_SESSION_GRACE_SECONDS)? + 30,
                );
                // If logout never wrote, this is the first genuine clean end.
                // Its own lost acknowledgement must also reconcile under the
                // original grace epoch, without sending the write a second time.
                let (released, writes) = tokio::time::timeout(
                    wait,
                    owner.test_familiar_clean_end_probe(
                        owner.release_after_grace(deps.admitted),
                        Some(true),
                    ),
                )
                .await?;
                assert_eq!(released, GraceExpiryResult::Released);
                assert_eq!(writes, if committed { 0 } else { 1 });
            }
            Finish::Terminal | Finish::RetiredOwner => return Err("not a release case".into()),
        }
        assert_eq!(
            store.current_session(session).await?.session_state(),
            GameSessionState::Terminal
        );
        assert!(
            owner
                .states
                .spell_states
                .lock()
                .await
                .pending_familiar_logouts
                .is_empty()
        );
        assert_eq!(
            receipt_count_for(&h, None).await?,
            1,
            "exactly one durable clean end"
        );
        assert_eq!(
            receipt_count_for(&h, Some(request.occurrence)).await?,
            if committed { 1 } else { 0 }
        );
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

async fn receipt_count_for(
    h: &Harness,
    occurrence: Option<FamiliarStateOccurrence>,
) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT count(*) FROM game_character_familiar_receipts
         WHERE character_id=encode($1,'hex')::uuid
           AND ($2::bytea IS NULL OR familiar_occurrence_id=encode($2,'hex')::uuid)",
    )
    .bind(id(41).as_slice())
    .bind(occurrence.as_ref().map(|v| v.as_bytes().as_slice()))
    .fetch_one(&h.pool)
    .await?)
}
