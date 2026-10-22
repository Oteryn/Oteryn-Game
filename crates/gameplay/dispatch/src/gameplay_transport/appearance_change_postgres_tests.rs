//! Native dispatch, active artifact, Character sex and the fenced PostgreSQL writer.
#![allow(clippy::expect_used)]
#[path = "appearance_change_postgres_tests/command_replay_tests.rs"]
mod command_replay_tests;
#[path = "appearance_change_postgres_tests/native_admission_tests.rs"]
mod native_admission_tests;
#[path = "appearance_change_postgres_tests/observer_visibility_tests.rs"]
mod observer_visibility_tests;
#[path = "appearance_change_postgres_tests/receipt_substitution_tests.rs"]
mod receipt_substitution_tests;
#[path = "appearance_change_postgres_tests/temporary_appearance_tests.rs"]
mod temporary_appearance_tests;
#[path = "appearance_change_postgres_tests/temporary_item_tests.rs"]
mod temporary_item_tests;
use super::*;
use crate::content::native_gameplay::test_support::cosmetics_fixture::{
    activated_with_optional_cosmetics, cosmetics_test_packet,
};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use oteryn_protocol_oteryn::appearance;

#[test]
fn appearance_change_real_dispatch_commits_reloads_and_rejects_stale_root_on_postgres() -> TestResult
{
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    run(async |admin| {
        let mut harness = Harness::create_appearance(admin, "appearance_dispatch").await?;
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
        let channel = owner
            .observe_visible_entities(dependencies.actor)
            .await
            .ok_or("visibility")?;
        let entity = appearance::EntityRef {
            identity: dependencies.actor.placement_identity(),
            generation: dependencies.actor.actor_local_generation(),
        };
        let initial = owner
            .appearance_observation(
                dependencies.actor,
                dependencies.admitted.game_session_id,
                vec![entity],
            )
            .await
            .ok_or("initial appearance")?;
        assert_eq!(
            initial.catalogue.context.content_generation,
            channel.content_generation
        );
        let option = initial.catalogue.outfits.first().ok_or("starter")?;
        let change = appearance::Change {
            context: initial.catalogue.context,
            expected_root_revision: 1,
            prior: None,
            selection: appearance::Selection {
                outfit_key: option.key.clone(),
                colours: [1, 2, 3, 4],
                addons: 0,
                mount_key: None,
            },
            content_revision: "content-1".into(),
        };
        let current = crate::gameplay_transport::character_gameplay_fence::current(
            &owner,
            dependencies.admitted.game_session_id,
            false,
        )
        .await
        .ok_or("current fence")?;
        let original = current.item_fence();
        let command = UseCommand {
            game_session_id: dependencies.admitted.game_session_id,
            command_id: 1,
            item_fence: Some(original),
        };
        native_admission_tests::qualify(&owner, dependencies.actor, command, &change).await?;
        assert_eq!(revision(&harness).await?, 1);
        let receipts: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_character_appearance_receipts")
                .fetch_one(&harness.pool)
                .await?;
        assert_eq!(receipts, 0);
        let mut wrong_connection = original;
        wrong_connection.connection_generation =
            crate::foundation::ConnectionGeneration::new(99).map_err(debug)?;
        let mut wrong_lease = original;
        wrong_lease.character_lease_generation += 1;
        for item_fence in [None, Some(wrong_connection), Some(wrong_lease)] {
            let rejected = crate::gameplay_transport::connection::appearance_change::dispatch(
                &owner,
                dependencies.actor,
                UseCommand {
                    item_fence,
                    ..command
                },
                &change.encode().map_err(debug)?,
            )
            .await;
            assert_eq!(rejected, Some(appearance::ChangeResult::Rejected));
            assert_eq!(revision(&harness).await?, 1);
        }
        temporary_appearance_tests::install(&owner, dependencies.actor, command.game_session_id)
            .await?;
        let result = crate::gameplay_transport::connection::appearance_change::dispatch(
            &owner,
            dependencies.actor,
            command,
            &change.encode().map_err(debug)?,
        )
        .await;
        assert_eq!(result, Some(appearance::ChangeResult::Committed(2)));
        temporary_appearance_tests::qualify(
            &mut owner,
            dependencies.actor,
            command.game_session_id,
            entity,
        )
        .await?;
        let updated = owner
            .appearance_observation(dependencies.actor, command.game_session_id, vec![entity])
            .await
            .ok_or("committed appearance")?;
        assert_eq!(updated.bindings[0].selection_revision, Some(2));
        assert!(matches!(
            updated.bindings[0].look,
            appearance::Look::Outfit {
                colours: [1, 2, 3, 4],
                ..
            }
        ));
        assert_eq!(revision(&harness).await?, 2);
        observer_visibility_tests::qualify(&owner, observer, entity).await?;
        let successor =
            temporary_item_tests::qualify(&mut owner, dependencies.actor, command, entity, &change)
                .await?;
        receipt_substitution_tests::qualify(&harness, &owner, dependencies.actor, command, &change)
            .await?;
        receipt_substitution_tests::qualify(
            &harness,
            &owner,
            dependencies.actor,
            UseCommand {
                command_id: 3,
                ..command
            },
            &successor,
        )
        .await?;
        let stale = crate::gameplay_transport::connection::appearance_change::dispatch(
            &owner,
            dependencies.actor,
            UseCommand {
                command_id: 2,
                ..command
            },
            &change.encode().map_err(debug)?,
        )
        .await;
        assert_eq!(stale, Some(appearance::ChangeResult::Rejected));
        assert_eq!(revision(&harness).await?, 3);
        let reloaded = dependencies.owner(&harness, &authority);
        let _ = reloaded
            .root
            .read_current_character_appearance(
                reloaded.character,
                reloaded.holder,
                crate::gameplay_transport::character_gameplay_fence::current(
                    &owner,
                    command.game_session_id,
                    false,
                )
                .await
                .ok_or("current fence")?,
            )
            .await
            .map_err(debug)?;
        drop(reloaded);
        owner.runtime.lock().await.retire_owner_cycle();
        assert!(
            owner
                .appearance_observation(dependencies.actor, command.game_session_id, vec![entity])
                .await
                .is_none()
        );
        drop(owner);
        drop(authority);
        drop(seal);
        harness.restart_appearance_root().await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let owner = dependencies.owner(&harness, &authority);
        command_replay_tests::qualify(&owner, dependencies.actor, command, &change).await?;
        assert_eq!(revision(&harness).await?, 3);
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await?;
        Ok(())
    })
}
