//! SEAM-2: required join observations retry contention without inventing optional state.
use super::*;
use crate::gameplay_transport::connection::FreshAdmissionAuthority;

#[test]
fn seam_join_tracker_snapshot_retries_busy_holder_without_progression() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            run(async |admin| {
                let h =
                    Harness::create_with_admission_seed(admin, "seam_join_snapshot", false, false)
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
                let mut admitted = deps.admitted;
                admitted.item_fence = crate::gameplay_transport::item_fence_of(
                    FreshAdmissionStore::from_root(h.root.clone())
                        .current_session(admitted.game_session_id)
                        .await?,
                );
                let expected = owner
                    .cyclopedia_tracker_snapshot(admitted)
                    .await
                    .ok_or("initial empty tracker snapshot")?;
                let observed = {
                    let held = h.root.try_issue_semantic_pass()?;
                    let mut snapshot = std::pin::pin!(owner.cyclopedia_tracker_snapshot(admitted));
                    std::future::poll_fn(|cx| {
                        use std::future::Future;
                        assert!(
                            snapshot.as_mut().poll(cx).is_pending(),
                            "busy holder must not become a missing required snapshot"
                        );
                        std::task::Poll::Ready(())
                    })
                    .await;
                    held.run(|_, _| Box::pin(async { Ok(()) })).await?;
                    snapshot.await
                };
                assert_eq!(observed, Some(expected));
                drop(owner);
                drop(seal);
                h.cleanup().await
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "join snapshot test thread panicked")??;
    Ok(())
}

#[test]
fn seam_join_spell_catalog_retries_busy_holder_without_losing_knowledge() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("seam-spell-catalog", || {
        run(async |admin| {
            let h = Harness::create_with_admission_seed(admin, "seam_spell_catalog", false, false)
                .await?;
            let request = prepare_current_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let deps = Dependencies::new()?;
            let owner = deps.owner(&h, &authority);
            super::tutorial_vitals_fixture::admit(&owner, &deps, &request).await?;
            let session = deps.admitted.game_session_id;
            let key = deps.spells.spells.first().ok_or("spell")?.key.clone();
            let fence = owner
                .current_quest_fence(session)
                .await
                .map_err(|()| "fence")?
                .ok_or("session")?;
            h.root
                .acquire_automatic_spells(
                    &authority,
                    &h.node,
                    fence,
                    &std::collections::BTreeSet::from([key]),
                )
                .await
                .map_err(debug)?;
            let expected = owner
                .observe_spell_catalog(deps.actor, session)
                .await
                .ok_or("initial catalog")?;
            assert!(expected.iter().any(|entry| entry.learned));
            let observed = {
                let held = h.root.try_issue_semantic_pass()?;
                let mut snapshot = std::pin::pin!(owner.observe_spell_catalog(deps.actor, session));
                std::future::poll_fn(|cx| {
                    use std::future::Future;
                    assert!(
                        snapshot.as_mut().poll(cx).is_pending(),
                        "busy holder must not become a missing required spell catalog"
                    );
                    std::task::Poll::Ready(())
                })
                .await;
                held.run(|_, _| Box::pin(async { Ok(()) })).await?;
                snapshot.await
            };
            assert_eq!(observed, Some(expected));
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    })
}

#[test]
fn seam_join_named_entities_retry_busy_holder_before_chat_open() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("seam-named-entities", || {
        run(async |admin| {
            let h = Harness::create_with_admission_seed(admin, "seam_named_entities", true, false)
                .await?;
            let request = prepare_current_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let deps = Dependencies::new()?;
            let owner = deps.owner(&h, &authority);
            super::tutorial_vitals_fixture::admit(&owner, &deps, &request).await?;
            let session = deps.admitted.game_session_id;
            assert!(owner.states.chat.speaker_name(session).is_none());
            let expected = owner
                .observe_named_visible_entities(deps.actor)
                .await
                .ok_or("initial named entities")?;
            let player = expected
                .entities
                .iter()
                .find(|entity| entity.identity == deps.actor.placement_identity())
                .ok_or("own player")?;
            assert!(player.display_name.is_some());
            let observed = {
                let held = h.root.try_issue_semantic_pass()?;
                let mut snapshot = std::pin::pin!(owner.observe_named_visible_entities(deps.actor));
                std::future::poll_fn(|cx| {
                    use std::future::Future;
                    assert!(
                        snapshot.as_mut().poll(cx).is_pending(),
                        "busy holder must not become a missing required named-entities snapshot"
                    );
                    std::task::Poll::Ready(())
                })
                .await;
                held.run(|_, _| Box::pin(async { Ok(()) })).await?;
                snapshot.await.ok_or("named entities after contention")?
            };
            assert_eq!(observed.entities, expected.entities);
            assert!(owner.states.chat.speaker_name(session).is_none());
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    })
}

#[test]
fn seam_input_house_transfer_guard_retries_busy_holder_before_dispatch() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("seam-house-input", || {
        run(async |admin| {
            let h = Harness::create_with_admission_seed(admin, "seam_house_input", false, false)
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
            let mut admitted = deps.admitted;
            admitted.item_fence = crate::gameplay_transport::item_fence_of(
                FreshAdmissionStore::from_root(h.root.clone())
                    .current_session(admitted.game_session_id)
                    .await?,
            );
            admitted.continuity.selected_capabilities =
                crate::gameplay_transport::capabilities::SelectedCapabilities::select(
                    owner.offered_capabilities(),
                    &[86],
                )
                .ok_or("house transfer offer")?;
            assert!(owner.house_transfer_input(admitted).await);
            {
                let held = h.root.try_issue_background_semantic_pass()?;
                let mut missing = admitted;
                missing.item_fence = None;
                assert!(!owner.house_transfer_input(missing).await);
                let input = crate::durability::DurabilityRoot::with_holder_owner(
                    owner.house_transfer_input(admitted),
                );
                let mut input = std::pin::pin!(input);
                std::future::poll_fn(|cx| {
                    use std::future::Future;
                    assert!(
                        input.as_mut().poll(cx).is_pending(),
                        "busy holder must not disconnect a current session before dispatch"
                    );
                    std::task::Poll::Ready(())
                })
                .await;
                held.run(|_, _| Box::pin(async { Ok(()) })).await?;
                for _ in 0..16 {
                    assert!(matches!(
                        h.root.try_issue_background_semantic_pass(),
                        Err(crate::durability::DurabilityError::RootUnavailable)
                    ));
                }
                assert!(input.await);
            }
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
