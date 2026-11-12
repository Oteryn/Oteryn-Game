//! A required appearance observation retries only a transiently busy durable holder.
use super::appearance_change_postgres::observer_visibility_tests;
use super::*;
use crate::content::native_gameplay::test_support::cosmetics_fixture::{
    activated_with_optional_cosmetics, cosmetics_test_packet,
};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_protocol_oteryn::appearance;
use std::{future::Future, task::Poll};

#[test]
fn appearance_observation_retries_busy_holder_with_bounded_current_runtime_checks() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("appearance-observation-busy", || {
        run(async |admin| {
            let harness = Harness::create_appearance(admin, "appearance_observation_busy").await?;
            prepare_current_session(&harness).await?;
            harness.seed_appearance_observer().await?;
            observer_visibility_tests::admit(&harness).await?;
            let seal = harness.recovery.seal_current().map_err(debug)?;
            let authority = harness
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
            let mut packet = cosmetics_test_packet();
            packet["authoring"]["document"]["content_revision"] = "content-1".into();
            let (controller, pin) =
                activated_with_optional_cosmetics(world, None, Some(&serde_json::to_vec(&packet)?));
            let room = pin.qualified_room().clone();
            let (channel_pin, _, _, _) = pin.into_channel_parts();
            let dependencies = Dependencies::with_content(
                channel_pin,
                room,
                crate::spell::cast::v1_spell_book().map_err(debug)?,
            )?;
            let mut owner = dependencies.owner(&harness, &authority);
            owner.active_generation = controller.active();
            {
                let mut runtime = dependencies.runtime.lock().await;
                runtime
                    .initialize_first_entry_position(dependencies.actor)
                    .map_err(debug)?;
                runtime
                    .bind_attacker_lease(
                        dependencies.actor,
                        dependencies.admitted.game_session_id,
                        crate::foundation::CharacterLease::new(
                            crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?,
                            2,
                        )
                        .map_err(debug)?,
                    )
                    .map_err(debug)?;
            }
            dependencies.initialize(&owner).await;
            let observer = observer_visibility_tests::initialize(&dependencies, &owner).await?;
            let session = GameSessionId::decode(&id(110)).map_err(debug)?;
            let visible = owner
                .observe_visible_entities(observer)
                .await
                .ok_or("observer visibility")?;
            let entities = visible
                .entities
                .iter()
                .filter(|entity| {
                    entity.kind == crate::gameplay_transport::world_spatial::VisibleKind::Player
                })
                .map(|entity| appearance::EntityRef {
                    identity: entity.identity,
                    generation: entity.generation,
                })
                .collect::<Vec<_>>();
            assert_eq!(entities.len(), 2);
            let initial = owner
                .appearance_observation(observer, session, entities.clone())
                .await
                .ok_or("initial two-player appearance")?;
            assert_eq!(initial.bindings.len(), 2);

            // Acquisition fails synchronously while this real pass owns the only holder.
            // The first Pending therefore observes the retry, before the holder is returned.
            let recovered = {
                let held = harness.root.try_issue_semantic_pass()?;
                let mut observation = std::pin::pin!(owner.appearance_observation(
                    observer,
                    session,
                    entities.clone(),
                ));
                let first =
                    std::future::poll_fn(|cx| Poll::Ready(observation.as_mut().poll(cx))).await;
                let runtime_free = owner.runtime.try_lock().is_ok();
                let spells_free = owner.states.spell_states.try_lock().is_ok();
                held.run(|_, _| Box::pin(async { Ok(()) })).await?;
                assert!(
                    first.is_pending(),
                    "a busy holder must enter the retry path"
                );
                assert!(
                    runtime_free && spells_free,
                    "retry must release owner guards"
                );
                match first {
                    Poll::Pending => observation.await,
                    Poll::Ready(result) => result,
                }
                .ok_or("appearance after holder return")?
            };
            assert_eq!(recovered.catalogue, initial.catalogue);
            assert_eq!(recovered.bindings.len(), initial.bindings.len());
            for (current, previous) in recovered.bindings.iter().zip(&initial.bindings) {
                assert!(current.owner_time_us >= previous.owner_time_us);
                let mut expected = previous.clone();
                expected.owner_time_us = current.owner_time_us;
                assert_eq!(*current, expected);
            }

            // Drive the existing retry window without sleeping or extending any budget.
            // The semantic read cannot acquire a holder under paused time. Return the held
            // pass only after resuming the clock used by this test's runtime.
            {
                let held = harness.root.try_issue_semantic_pass()?;
                tokio::time::pause();
                let mut observation = std::pin::pin!(owner.appearance_observation(
                    observer,
                    session,
                    entities.clone(),
                ));
                let first =
                    std::future::poll_fn(|cx| Poll::Ready(observation.as_mut().poll(cx))).await;
                let pending = first.is_pending();
                let expired = if pending {
                    tokio::time::advance(crate::gameplay_transport::COMMAND_HOLDER_WINDOW).await;
                    std::future::poll_fn(|cx| Poll::Ready(observation.as_mut().poll(cx))).await
                } else {
                    first
                };
                tokio::time::resume();
                held.run(|_, _| Box::pin(async { Ok(()) })).await?;
                assert!(pending, "persistent contention must first enter retry");
                assert!(
                    matches!(expired, Poll::Ready(None)),
                    "persistent contention must stop when the existing retry window expires"
                );
            }
            // Service the ready demand left by the refused read before the next case.
            assert!(harness.root.maintain_ready_once().await?);

            // A retry must re-observe current authority rather than return the earlier view.
            {
                let held = harness.root.try_issue_semantic_pass()?;
                let mut observation =
                    std::pin::pin!(owner.appearance_observation(observer, session, entities,));
                let first =
                    std::future::poll_fn(|cx| Poll::Ready(observation.as_mut().poll(cx))).await;
                let retired = if let Ok(mut runtime) = owner.runtime.try_lock() {
                    runtime.retire_owner_cycle();
                    true
                } else {
                    false
                };
                let spells_free = owner.states.spell_states.try_lock().is_ok();
                held.run(|_, _| Box::pin(async { Ok(()) })).await?;
                assert!(first.is_pending(), "runtime changes after the busy read");
                assert!(
                    retired && spells_free,
                    "owner guards must be free during retry"
                );
                let stale = match first {
                    Poll::Pending => observation.await,
                    Poll::Ready(result) => result,
                };
                assert!(
                    stale.is_none(),
                    "a retired runtime cannot supply an appearance"
                );
            }
            let receipts: i64 =
                sqlx::query_scalar("SELECT count(*) FROM game_character_appearance_receipts")
                    .fetch_one(&harness.pool)
                    .await?;
            assert_eq!(
                receipts, 0,
                "observing appearance never commits a selection"
            );
            let revisions: Vec<String> = sqlx::query_scalar(
                "SELECT character_revision::text FROM game_character_roots ORDER BY character_id",
            )
            .fetch_all(&harness.pool)
            .await?;
            assert_eq!(revisions, vec!["1", "1"]);
            drop(owner);
            drop(authority);
            drop(seal);
            harness.cleanup().await
        })
    })
}
