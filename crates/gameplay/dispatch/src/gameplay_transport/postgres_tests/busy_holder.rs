//! NODEBOOT-1: admission entry steps and command gating reads wait for a busy durability
//! holder instead of refusing at once.
use super::fixture::{Harness, debug, run};
use super::{Dependencies, TestResult, prepare_current_session, track};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::AuthenticatedTransportRefV1;
use crate::gameplay_transport::AdmissionRetryBudget;
use crate::gameplay_transport::connection::FirstEntryOutcome;
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use std::time::Duration;

#[test]
fn exiva_settings_query_returns_a_refusal_with_the_holder_busy() -> TestResult {
    use crate::durability::DurabilityRoot;
    use crate::gameplay_transport::capabilities::SelectedCapabilities;
    use oteryn_protocol_oteryn::exiva::Request;

    super::kill_reward_e2e_fixtures::scenario_thread::run("exiva-busy-holder", || {
        run(async |admin| {
            let h = Harness::create_with_admission_seed(admin, "exiva_query_busy", true, false).await?;
            prepare_current_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
            let deps = Dependencies::new()?;
            let mut owner = deps.owner(&h, &authority);
            let (controller, _) = crate::content::native_gameplay::test_support::activated_with_loot_tables(
                deps.admitted.world_id, None,
            );
            owner.active_generation = Some(controller.active().ok_or("active Exiva generation")?);
            let mut admitted = deps.admitted;
            admitted.item_fence = crate::gameplay_transport::item_fence_of(
                FreshAdmissionStore::from_root(h.root.clone())
                    .current_session_at(admitted.game_session_id).await?.0,
            );
            admitted.continuity.selected_capabilities = SelectedCapabilities::select(
                owner.offered_capabilities(), &[6, 7, 147],
            ).ok_or("Exiva capability selection")?;
            let request = Request::decode(&[5]).ok_or("Exiva list request")?;
            let held = h.root.try_issue_background_semantic_pass()?;
            let busy = DurabilityRoot::with_holder_owner(owner.exiva_settings(
                admitted, admitted.continuity.next_command_id, request,
            )).await;
            held.run(|_, _| Box::pin(async { Ok(()) })).await?;
            assert_eq!(busy.as_deref(), Some(&[0][..]), "busy Exiva read must reply");
            drop(owner);
            drop(seal);
            h.cleanup().await
        })
    })
}

#[test]
fn party_skull_query_refuses_without_waiting_on_a_busy_holder() -> TestResult {
    use crate::durability::{DurabilityError, DurabilityRoot};
    use crate::gameplay_transport::capabilities::SelectedCapabilities;
    use std::{future::Future, task::Poll};

    super::kill_reward_e2e_fixtures::scenario_thread::run("party-skull-busy-holder", || {
        run(async |admin| {
            let h = Harness::create_with_admission_seed(admin, "party_skull_busy", true, false).await?;
            prepare_current_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
            let deps = Dependencies::new()?;
            let owner = deps.owner(&h, &authority);
            deps.initialize(&owner).await;
            let mut admitted = deps.admitted;
            admitted.runtime_actor = Some(deps.actor);
            admitted.item_fence = crate::gameplay_transport::item_fence_of(
                FreshAdmissionStore::from_root(h.root.clone())
                    .current_session_at(admitted.game_session_id).await?.0,
            );
            admitted.continuity.selected_capabilities = SelectedCapabilities::select(
                owner.offered_capabilities(), &[6, 42, 134],
            ).ok_or("party icon selection")?;

            let held = h.root.try_issue_background_semantic_pass()?;
            let lane = owner.spell_lane.acquire().await.map_err(|_| "spell lane unresolved")?;
            let observation = {
                let mut observation = std::pin::pin!(DurabilityRoot::with_holder_owner(
                    owner.observe_party_icons(admitted),
                ));
                std::future::poll_fn(|cx| Poll::Ready(observation.as_mut().poll(cx))).await
            };
            let status = {
                let mut status = std::pin::pin!(DurabilityRoot::with_holder_owner(
                    owner.observe_pvp_status(admitted),
                ));
                std::future::poll_fn(|cx| Poll::Ready(status.as_mut().poll(cx))).await
            };
            drop(lane);
            held.run(|_, _| Box::pin(async { Ok(()) })).await?;
            assert!(matches!(observation, Poll::Ready(Err(DurabilityError::RootUnavailable))),
                "a party query must return Busy while the writer holds the lane and holder: {observation:?}");
            assert!(matches!(status, Poll::Ready(Err(DurabilityError::RootUnavailable))),
                "a PvP state query must return Busy while the writer holds the lane and holder: {status:?}");
            drop(owner);
            drop(seal);
            h.cleanup().await
        })
    })
}

#[test]
fn item_move_receipt_waits_for_the_busy_holder_postgres() -> TestResult {
    use crate::durability::DurabilityRoot;
    use crate::gameplay_transport::connection::UseCommand;
    use crate::gameplay_transport::item_move::{self, ItemMoveStep};
    use crate::gameplay_transport::item_view::SessionItemView;
    use oteryn_protocol_oteryn::item_view::{
        ItemHandle, ItemMoveDestination, ItemMoveIntent, ItemMoveOutcome, ItemMoveSelection,
        encode_item_move_intent,
    };
    use std::{future::Future, task::Poll};

    super::kill_reward_e2e_fixtures::scenario_thread::run("move-receipt-busy-holder", || {
        run(async |admin| {
            let h = Harness::create_with_admission_seed(admin, "move_receipt_busy", true, false).await?;
            prepare_current_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
            let deps = Dependencies::new()?;
            let owner = deps.owner(&h, &authority);
            let session = deps.admitted.game_session_id;
            let command = UseCommand {
                game_session_id: session,
                command_id: deps.admitted.continuity.next_command_id,
                item_fence: crate::gameplay_transport::item_fence_of(
                    FreshAdmissionStore::from_root(h.root.clone()).current_session_at(session).await?.0,
                ),
            };
            let mut view = SessionItemView::resume(deps.admitted.continuity.item_view);
            let selection = ItemMoveSelection {
                equip_drop: true, container_tree: false, world_interaction: false, manual_sort: false,
            };
            let payload = encode_item_move_intent(&ItemMoveIntent {
                source: ItemHandle::MIN,
                destination: ItemMoveDestination::MainBackpack,
            }).map_err(debug)?;
            let held = h.root.try_issue_background_semantic_pass()?;
            {
                let mut attempt = std::pin::pin!(DurabilityRoot::with_holder_owner(item_move::item_move(
                    &owner, deps.actor, command, &payload, selection, false, &mut view,
                )));
                let first = std::future::poll_fn(|cx| Poll::Ready(attempt.as_mut().poll(cx))).await;
                held.run(|_, _| Box::pin(async { Ok(()) })).await?;
                assert!(first.is_pending(), "MOVE must wait to read its receipt: {first:?}");
                assert_eq!(attempt.await, ItemMoveStep::Result(ItemMoveOutcome::Stale, Vec::new()));
            }
            drop(owner);
            drop(seal);
            h.cleanup().await
        })
    })
}

#[test]
fn party_queries_get_a_foreground_turn_when_only_the_holder_is_busy() -> TestResult {
    use crate::durability::DurabilityRoot;
    use crate::gameplay_transport::capabilities::SelectedCapabilities;
    use std::{future::Future, task::Poll};

    super::kill_reward_e2e_fixtures::scenario_thread::run("party-query-holder-turn", || {
        run(async |admin| {
            let h = Harness::create_with_admission_seed(admin, "party_query_turn", true, false).await?;
            prepare_current_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
            let deps = Dependencies::new()?;
            let owner = deps.owner(&h, &authority);
            let mut admitted = deps.admitted;
            let session = admitted.game_session_id;
            let (current, _) = FreshAdmissionStore::from_root(h.root.clone()).current_session_at(session).await?;
            {
                let mut runtime = owner.runtime.lock().await;
                runtime.bind_attacker_lease(deps.actor, session, current.current_character_lease()).map_err(debug)?;
                runtime.initialize_first_entry_position(deps.actor).map_err(debug)?;
            }
            deps.initialize(&owner).await;
            admitted.runtime_actor = Some(deps.actor);
            admitted.item_fence = crate::gameplay_transport::item_fence_of(current);
            admitted.continuity.selected_capabilities = SelectedCapabilities::select(
                owner.offered_capabilities(), &[6, 42, 134],
            ).ok_or("party icon selection")?;

            let held = h.root.try_issue_background_semantic_pass()?;
            {
                let mut query = std::pin::pin!(DurabilityRoot::with_holder_owner(owner.observe_pvp_status(admitted)));
                let first = std::future::poll_fn(|cx| Poll::Ready(query.as_mut().poll(cx))).await;
                held.run(|_, _| Box::pin(async { Ok(()) })).await?;
                assert!(first.is_pending(), "PvP query must queue for the holder: {first:?}");
                assert!(h.root.try_issue_background_semantic_pass().is_err(), "the waiting observation owns the next holder turn");
                assert_eq!(query.await?.points_milli, [0; 3]);
            }
            {
                let held = h.root.try_issue_background_semantic_pass()?;
                let mut refresh = std::pin::pin!(DurabilityRoot::with_background_holder(
                    owner.observe_party_icons(admitted),
                ));
                let first = std::future::poll_fn(|cx| Poll::Ready(refresh.as_mut().poll(cx))).await;
                held.run(|_, _| Box::pin(async { Ok(()) })).await?;
                assert!(matches!(first, Poll::Ready(Err(crate::durability::DurabilityError::RootUnavailable))),
                    "a periodic party refresh must leave the holder to player commands: {first:?}");
            }
            drop(owner);
            drop(seal);
            h.cleanup().await
        })
    })
}

#[test]
fn a_committed_actor_enters_after_a_busy_holder_is_returned_postgres() -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run("first-entry-busy-holder", || {
        run(async |admin| {
            let h =
                Harness::create_with_admission_seed(admin, "first_entry_busy_holder", false, false)
                    .await?;
            let request = prepare_current_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let dependencies = Dependencies::new()?;
            let owner = dependencies.owner(&h, &authority);
            let session = dependencies.admitted.game_session_id;
            // One background pass holds the holder past the whole blind retry span the first
            // entry used to have (RECONCILE_ATTEMPTS rounds of RECONCILE_BACKOFF).
            let busy = hold_holder(&h.root, Duration::from_millis(1_500))?;
            let budget = AdmissionRetryBudget::for_root(&session, &h.root);
            let (outcome, item_fence, _) = Box::pin(owner.enter_committed_actor(
                &budget,
                &request,
                session,
                AuthenticatedTransportRefV1::decode(&[9; 16]).map_err(debug)?,
                dependencies.actor,
            ))
            .await;
            busy.await??;
            assert!(
                matches!(
                    outcome,
                    FirstEntryOutcome::Positioned | FirstEntryOutcome::Reconciled
                ),
                "{outcome:?}"
            );
            assert!(item_fence.is_some());
            drop(owner);
            drop(seal);
            h.cleanup().await
        })
    })
}

#[test]
fn trade_open_retains_foreground_demand_until_its_name_reads_complete_postgres() -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run("trade-open-busy-holder", || {
        run(async |admin| {
            let h =
                Harness::create_with_admission_seed(admin, "trade_open_busy_holder", true, false)
                    .await?;
            prepare_current_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let dependencies = Dependencies::new()?;
            let owner = dependencies.owner(&h, &authority);
            dependencies.initialize(&owner).await;
            owner
                .runtime
                .lock()
                .await
                .initialize_first_entry_position(dependencies.actor)
                .map_err(debug)?;
            let held = h.root.try_issue_background_semantic_pass().map_err(debug)?;
            {
                let opening = crate::durability::DurabilityRoot::with_holder_owner(
                    owner.trade_open(dependencies.admitted),
                );
                let mut opening = std::pin::pin!(opening);
                let first = std::future::poll_fn(|cx| {
                    std::task::Poll::Ready(std::future::Future::poll(opening.as_mut(), cx))
                })
                .await;
                assert!(
                    first.is_pending(),
                    "trade open must wait for the busy holder"
                );
                held.run(|_, _| Box::pin(async { Ok(()) })).await?;
                // Reproduce the scheduler retaking the holder before the waiting name
                // reader is polled again. Its FIFO ticket alone does not stop background passes.
                for _ in 0..16 {
                    assert!(matches!(
                        h.root.try_issue_background_semantic_pass(),
                        Err(crate::durability::DurabilityError::RootUnavailable)
                    ));
                }
                assert_eq!(opening.await, Some(()));
            }
            // A completed admission must release its preference for ordinary scheduled work.
            h.root
                .try_issue_background_semantic_pass()?
                .run(|_, _| Box::pin(async { Ok(()) }))
                .await?;
            drop(owner);
            drop(seal);
            h.cleanup().await
        })
    })
}

/// Every step refreshes the session's PvP locks from durable state first; a step that met a
/// background pass on the holder was rejected (the node-boot use-wire `step_blocked` stage).
#[test]
fn a_step_lock_refresh_waits_for_a_busy_holder_postgres() -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run("pvp-refresh-busy-holder", || {
        run(async |admin| {
            let h =
                Harness::create_with_admission_seed(admin, "pvp_refresh_busy_holder", true, false)
                    .await?;
            prepare_current_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let dependencies = Dependencies::new()?;
            let owner = dependencies.owner(&h, &authority);
            dependencies.initialize(&owner).await;
            owner
                .runtime
                .lock()
                .await
                .initialize_first_entry_position(dependencies.actor)
                .map_err(debug)?;
            track(&owner, &dependencies)?;
            let session = dependencies.admitted.game_session_id;
            assert!(owner.refresh_pvp_locks(dependencies.actor, session).await);
            let busy = hold_holder(&h.root, Duration::from_millis(300))?;
            let refreshed = owner.refresh_pvp_locks(dependencies.actor, session).await;
            busy.await??;
            assert!(refreshed, "the timed refresh refused on a busy holder");
            let refreshed = priority_handoff(
                &h.root,
                owner.refresh_pvp_locks(dependencies.actor, session),
            )
            .await?;
            assert!(refreshed, "the refresh refused on a busy holder");
            drop(owner);
            drop(seal);
            h.cleanup().await
        })
    })
}

#[test]
fn activity_input_waits_for_a_busy_holder_postgres() -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run("activity-input-busy-holder", || {
        run(async |admin| {
            let h = Harness::create_with_admission_seed(
                admin,
                "activity_input_busy_holder",
                true,
                false,
            )
            .await?;
            prepare_current_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let dependencies = Dependencies::new()?;
            let owner = dependencies.owner(&h, &authority);
            let mut admitted = dependencies.admitted;
            admitted.item_fence = crate::gameplay_transport::item_fence_of(
                FreshAdmissionStore::from_root(h.root.clone())
                    .current_session_at(admitted.game_session_id)
                    .await?
                    .0,
            );
            admitted.continuity.selected_capabilities =
                crate::gameplay_transport::capabilities::SelectedCapabilities::select(
                    &[crate::gameplay_transport::capabilities::OfferedCapability {
                        id: oteryn_protocol_oteryn::boss_encounter::CAPABILITY_BOSS_ENCOUNTER_V1,
                        requires: &[],
                    }],
                    &[oteryn_protocol_oteryn::boss_encounter::CAPABILITY_BOSS_ENCOUNTER_V1],
                )
                .ok_or("activity capability")?;
            assert!(owner.activity_transfer_input(admitted).await.is_ok());
            let busy = hold_holder(&h.root, Duration::from_millis(300))?;
            let accepted = owner.activity_transfer_input(admitted).await;
            busy.await??;
            assert!(
                accepted.is_ok(),
                "the timed activity input refused on a busy holder"
            );
            let accepted = priority_handoff(&h.root, async {
                owner.activity_transfer_input(admitted).await.is_ok()
            })
            .await?;
            assert!(
                accepted,
                "a background pass must not close ordinary activity-capable input"
            );
            // A persistently unavailable root still authorizes no input.
            let held = h.root.try_issue_semantic_pass()?;
            assert!(matches!(
                owner.activity_transfer_input(admitted).await,
                Err(crate::durability::DurabilityError::RootUnavailable)
            ));
            // Fresh admission recorded this exact session as having no activity
            // origin. The ordinary input path needs no transfer receipt.
            admitted.continuity.ordinary_activity_session = Some(admitted.game_session_id);
            owner.activity_transfer_input(admitted).await?;
            held.run(|_, _| Box::pin(async { Ok(()) })).await?;
            drop(owner);
            drop(seal);
            h.cleanup().await
        })
    })
}

// The caller is the real fenced read/input guard. Keep its holder identity independent
// from the fixture returning the pass, then reproduce immediate scheduler reacquisition.
async fn priority_handoff(
    root: &crate::durability::DurabilityRoot,
    operation: impl std::future::Future<Output = bool>,
) -> TestResult<bool> {
    use std::{future::Future, task::Poll};
    let held = root.try_issue_background_semantic_pass().map_err(debug)?;
    let operation = crate::durability::DurabilityRoot::with_holder_owner(operation);
    let mut operation = std::pin::pin!(operation);
    let first = std::future::poll_fn(|cx| Poll::Ready(operation.as_mut().poll(cx))).await;
    assert!(first.is_pending(), "a busy guard must await the holder");
    held.run(|_, _| Box::pin(async { Ok(()) })).await?;
    for _ in 0..16 {
        assert!(matches!(
            root.try_issue_background_semantic_pass(),
            Err(crate::durability::DurabilityError::RootUnavailable)
        ));
    }
    let result = operation.await;
    root.try_issue_background_semantic_pass()?
        .run(|_, _| Box::pin(async { Ok(()) }))
        .await?;
    Ok(result)
}

fn hold_holder(
    root: &crate::durability::DurabilityRoot,
    hold: Duration,
) -> TestResult<tokio::task::JoinHandle<Result<(), crate::durability::DurabilityError>>> {
    let pass = root.try_issue_semantic_pass().map_err(debug)?;
    Ok(tokio::spawn(pass.run(move |_holder, _deadline| {
        Box::pin(async move {
            tokio::time::sleep(hold).await;
            Ok(())
        })
    })))
}
