//! NX-8/P08: production owner publishes every catalogue weapon and live, uncheckpointed XP.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    FirstEntryOutcome, FreshAdmissionAuthority, UseCommand,
};
use crate::gameplay_transport::weapon_proficiency::ProductionProficiencyCatalogue;
use oteryn_protocol_oteryn::weapon_proficiency::ProficiencyCataloguePage;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;

#[test]
fn nx8_proficiency_catalogue_projects_all_tracks_and_live_experience() -> TestResult {
    std::thread::Builder::new()
        .name("nx8_proficiency_catalogue".into())
        .stack_size(64 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "catalogue scenario panicked")?
        .map_err(Into::into)
}
fn scenario() -> TestResult {
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "nx8_proficiency_catalogue", false, false)
                .await?;
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, pin) =
            super::death_ml_loss_composed::production_generation_with_pin(world, "content-1")?;
        let generation = controller.active().ok_or("active generation")?;
        let room = pin.qualified_room().clone();
        let deps = Dependencies::with_content(
            pin.into_channel_pin(),
            room,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        deps.runtime
            .lock()
            .await
            .initialize_pinned_test_position(
                deps.actor,
                crate::foundation::MovementLocalPosition {
                    x: 7,
                    y: 0,
                    floor: 7,
                },
            )
            .map_err(debug)?;
        let catalogue = ProductionProficiencyCatalogue::embedded().ok_or("catalogue")?;
        let mut owner = deps.owner(&harness, &authority);
        owner.active_generation = Some(generation);
        owner.proficiency_catalogue = Some(Arc::clone(&catalogue));
        deps.initialize(&owner).await;
        let session = deps.admitted.game_session_id;
        let key = "oteryn:item.tibia.i3264";
        {
            let mut states = owner.states.proficiencies.lock().map_err(|_| "tracks")?;
            let mut track = catalogue.empty_track(key).ok_or("sword")?;
            track.progress = 12345;
            states.entry(session).or_default().install_track(key, track);
        }
        let fence = owner
            .current_quest_fence(session)
            .await
            .map_err(|_| "current fence")?
            .ok_or("current fence")?
            .item_fence();
        deps.runtime
            .lock()
            .await
            .bind_attacker_lease(
                deps.actor,
                session,
                crate::foundation::CharacterLease::new(
                    crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                        .map_err(debug)?,
                    fence.character_lease_generation,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        let command = UseCommand {
            game_session_id: session,
            command_id: 1,
            item_fence: Some(fence),
        };
        assert!(
            FreshAdmissionAuthority::weapon_proficiency_catalogue(
                &owner,
                deps.actor,
                UseCommand {
                    item_fence: None,
                    ..command
                },
                0
            )
            .await
            .is_none()
        );
        for stale in [
            crate::durability::item_transfer::CurrentCharacterItemFence {
                character_lease_generation: fence.character_lease_generation + 1,
                ..fence
            },
            crate::durability::item_transfer::CurrentCharacterItemFence {
                scope_ownership_generation: crate::foundation::ScopeOwnershipGeneration::new(
                    fence.scope_ownership_generation.get() + 1,
                )
                .map_err(debug)?,
                ..fence
            },
            crate::durability::item_transfer::CurrentCharacterItemFence {
                connection_generation: crate::foundation::ConnectionGeneration::new(
                    fence.connection_generation.get() + 1,
                )
                .map_err(debug)?,
                ..fence
            },
        ] {
            assert!(
                FreshAdmissionAuthority::weapon_proficiency_catalogue(
                    &owner,
                    deps.actor,
                    UseCommand {
                        item_fence: Some(stale),
                        ..command
                    },
                    0
                )
                .await
                .is_none()
            );
        }
        assert!(
            FreshAdmissionAuthority::weapon_proficiency_catalogue(&owner, deps.actor, command, 0)
                .await
                .is_some()
        );
        let mut admitted = deps.admitted;
        admitted.item_fence = Some(fence);
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,
            &[4, 6, 195],
        )
        .ok_or("capabilities")?;
        let served = super::proficiency_select_perk::connection(&owner, admitted, async |client| {
        let started = std::time::Instant::now();
        let mut offset = 0_u32;
        let mut command_id = 1_u64;
        let mut keys = std::collections::BTreeSet::new();
        let mut saw_sword = false;
        loop {
            let protocol = oteryn_protocol_oteryn::encode_client_command(1, &oteryn_protocol_oteryn::ClientCommandValue {
                command_id,
                command_type: 660,
                payload: &offset.to_be_bytes(),
            }).map_err(debug)?;
            client.write_u32(u32::try_from(protocol.len())?).await?;
            client.write_all(&protocol).await?;
            let page = loop {
                let bytes = super::item_look::read_frame(client).await?;
                let envelope = oteryn_protocol_oteryn::decode_wire_envelope(&bytes).map_err(debug)?;
                if envelope.message_type() == oteryn_protocol_oteryn::MessageType::CommandResult {
                    let result = oteryn_protocol_oteryn::decode_command_result(envelope.payload()).map_err(debug)?;
                    assert_eq!(result.status, oteryn_protocol_oteryn::CommandStatus::Accepted);
                    break ProficiencyCataloguePage::decode(result.payload).ok_or("wire catalogue page")?;
                }
            };
            assert_eq!(ProficiencyCataloguePage::decode(&page.encode().ok_or("encode")?), Some(page.clone()));
            assert_eq!(page.offset, offset);
            for entry in &page.entries {
                assert!(keys.insert(entry.weapon.clone()), "catalogue repeats a weapon");
                assert!(!entry.levels.is_empty());
                assert!(entry.levels.iter().all(|level| !level.perks.is_empty()));
                if entry.weapon == key {
                    saw_sword = true;
                    assert_eq!(entry.experience, 12345);
                    assert_ne!(entry.definition, 0);
                } else {
                    assert_eq!(entry.experience, 0);
                }
            }
            command_id += 1;
            offset += u32::try_from(page.entries.len())?;
            if offset == page.total { break; }
            assert!(!page.entries.is_empty(), "catalogue stopped before its total");
        }
        assert!(saw_sword);
        assert_eq!(keys.len(), crate::durability::character_proficiency::ProficiencyDefinitions::active_weapon_count(catalogue.as_ref()));
        eprintln!("nx8_proficiency_catalogue: executed all {offset} weapon tracks through production wire in {:?}", started.elapsed());
        Ok(())
        }).await?;
        assert!(served);
        Ok(())
    })
}
