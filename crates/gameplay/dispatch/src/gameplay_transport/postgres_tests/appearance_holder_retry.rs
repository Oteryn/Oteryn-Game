//! Holder contention must not turn the read-only appearance projection into a disconnect.
use super::*;
use crate::content::native_gameplay::test_support::cosmetics_fixture::{
    activated_with_optional_cosmetics, cosmetics_test_packet,
};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_protocol_oteryn::appearance;
use std::future::Future;
use std::task::Poll;

#[test]
fn appearance_holder_busy_postgres_waits_without_runtime_locks_and_keeps_stale_refusal()
-> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::item_look::large_stack(scenario)
}

fn scenario() -> TestResult {
    run(async |admin| {
        let harness = Harness::create_appearance(admin, "appearance_holder_retry").await?;
        prepare_current_session(&harness).await?;
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
        let actor = dependencies.actor;
        let session = dependencies.admitted.game_session_id;
        {
            let mut runtime = owner.runtime.lock().await;
            runtime
                .initialize_first_entry_position(actor)
                .map_err(debug)?;
            runtime
                .bind_attacker_lease(
                    actor,
                    session,
                    crate::foundation::CharacterLease::new(
                        crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?,
                        2,
                    )
                    .map_err(debug)?,
                )
                .map_err(debug)?;
        }
        dependencies.initialize(&owner).await;
        let entity = appearance::EntityRef {
            identity: actor.placement_identity(),
            generation: actor.actor_local_generation(),
        };
        let initial = owner
            .appearance_observation(actor, session, vec![entity])
            .await
            .ok_or("initial appearance projection")?;

        // Reproduce the real monster-owner order: runtime, spell state, then holder.
        // Its existing pass can outlast COMMAND_HOLDER_WINDOW. Demand must already
        // be retained while waiting for those guards; the one-second DB retry
        // budget starts only after that owner releases them.
        let runtime = owner.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        let holder = owner
            .root
            .try_issue_background_semantic_pass()
            .map_err(debug)?;
        let started = tokio::time::Instant::now();
        let (projected, released) = fixture::join_two(
            owner.appearance_observation(actor, session, vec![entity]),
            async {
                tokio::time::sleep(
                    crate::gameplay_transport::COMMAND_HOLDER_WINDOW + Duration::from_millis(200),
                )
                .await;
                drop(holder);
                assert!(matches!(
                    owner.root.try_issue_background_semantic_pass(),
                    Err(crate::durability::DurabilityError::RootUnavailable)
                ));
                drop(states);
                drop(runtime);
                Ok::<(), Box<dyn std::error::Error>>(())
            },
        )
        .await;
        released?;
        assert!(started.elapsed() >= crate::gameplay_transport::COMMAND_HOLDER_WINDOW);
        assert_projection(
            projected.ok_or("appearance after existing monster owner")?,
            &initial,
        );

        // A busy holder without runtime guards reaches the typed retry path. That
        // wait must release both guards, and the returning holder cannot be stolen
        // by another background pass before the projection resumes.
        let holder = owner.root.try_issue_semantic_pass().map_err(debug)?;
        {
            let projected = owner.appearance_observation(actor, session, vec![entity]);
            let mut projected = std::pin::pin!(projected);
            std::future::poll_fn(|context| {
                assert!(projected.as_mut().poll(context).is_pending());
                Poll::Ready(())
            })
            .await;
            assert!(owner.runtime.try_lock().is_ok());
            assert!(owner.states.spell_states.try_lock().is_ok());
            tokio::time::sleep(Duration::from_millis(150)).await;
            drop(holder);
            assert!(matches!(
                owner.root.try_issue_background_semantic_pass(),
                Err(crate::durability::DurabilityError::RootUnavailable)
            ));
            assert_projection(
                projected
                    .await
                    .ok_or("appearance after busy holder return")?,
                &initial,
            );
        }
        drop(
            owner
                .root
                .try_issue_background_semantic_pass()
                .map_err(debug)?,
        );

        // A runtime fence refusal remains final; holder availability is not
        // authority to publish a projection for a retired owner.
        owner.runtime.lock().await.retire_owner_cycle();
        assert!(
            owner
                .appearance_observation(actor, session, vec![entity])
                .await
                .is_none()
        );

        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

fn assert_projection(
    actual: crate::gameplay_transport::appearance_change::observation::Observation,
    expected: &crate::gameplay_transport::appearance_change::observation::Observation,
) {
    assert_eq!(actual.catalogue, expected.catalogue);
    assert_eq!(actual.bindings.len(), 1);
    assert_eq!(actual.bindings[0].entity, expected.bindings[0].entity);
    assert_eq!(actual.bindings[0].look, expected.bindings[0].look);
    assert_eq!(
        actual.bindings[0].root_revision,
        expected.bindings[0].root_revision
    );
    assert_eq!(
        actual.bindings[0].selection_revision,
        expected.bindings[0].selection_revision
    );
}
