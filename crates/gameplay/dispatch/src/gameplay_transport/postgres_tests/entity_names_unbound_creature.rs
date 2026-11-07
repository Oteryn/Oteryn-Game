//! Entity-name negotiation follows the booted generation’s native gameplay binding.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use oteryn_protocol_oteryn::world_spatial_entities::{
    EntityFields, EntityKind, decode_world_spatial_entities_snapshot_with_fields,
};
use oteryn_protocol_oteryn::{MessageType, decode_snapshot_chunk, decode_wire_envelope};

#[test]
fn entity_names_baseline_selects_v1_and_sends_unbound_creature() -> TestResult {
    run_scenario(false)
}

#[test]
fn entity_names_native_gameplay_selects_v2_and_sends_bound_names() -> TestResult {
    run_scenario(true)
}

fn run_scenario(native: bool) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || scenario(native).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "entity names negotiation test panicked")??;
    eprintln!(
        "ENTITY-NAMES-PG: native={native} executed in {:?}",
        started.elapsed()
    );
    Ok(())
}

fn scenario(native: bool) -> TestResult {
    run(async |admin| {
        let h =
            Harness::create_with_admission_seed(admin, "entity_names_unbound", true, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, deps, creature) = if native {
            let (controller, pin) =
                crate::content::native_gameplay::test_support::activated_loop_monster(world, false);
            let active = controller.active().ok_or("active generation")?;
            let (deps, creature) = super::loop_monster::fixture::dependencies(active, &pin)?;
            (controller, deps, creature)
        } else {
            use crate::content::{
                ContentActivationController, NativeEntryActivationIssuance, NodeBootQuiescence,
                activate_native_entry_room, qualify_native_entry_room,
            };
            let room = qualify_native_entry_room(world).map_err(debug)?;
            let issuance = NativeEntryActivationIssuance {
                world_id: world,
                activation_sequence: 1,
                server_artifact_digest: room.compiled().server_digest(),
                client_artifact_digest: room.compiled().client_digest(),
                frame_binding_digest: room.frame_binding().digest(),
            };
            let mut controller = ContentActivationController::new();
            let pin = activate_native_entry_room(
                &mut controller,
                &NodeBootQuiescence::before_channel_runtime(),
                world,
                &issuance,
            )
            .map_err(debug)?;
            let deps = Dependencies::with_content_and_capacity(
                pin.into_channel_pin(),
                room,
                crate::spell::cast::v1_spell_book().map_err(debug)?,
                8,
            )?;
            let creature = {
                let mut runtime = deps.runtime.lock().await;
                runtime
                    .initialize_first_entry_position(deps.actor)
                    .map_err(debug)?;
                // Use the same authored source and realization path as fixture node boot.
                let source = deps.room.spawn_source().ok_or("entry room spawn source")?;
                runtime
                    .realize_activation_spawns(vec![source.activation_facts()])
                    .map_err(debug)?;
                let actor = runtime
                    .visible_entities()
                    .creatures
                    .first()
                    .ok_or("entry room creature spawned")?
                    .actor;
                assert!(runtime.companion_snapshot(actor).is_err());
                actor
            };
            (controller, deps, creature)
        };
        let active = controller.active().ok_or("active generation")?;
        assert_eq!(active.native_gameplay().is_some(), native);
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
        deps.initialize(&owner).await;
        assert_eq!(
            owner.offered_capabilities().iter().any(|cap| cap.id == 286),
            native
        );
        let mut admitted = deps.admitted;
        admitted.runtime_actor = Some(deps.actor);
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.continuity.selected_capabilities =
            crate::gameplay_transport::connection::fresh_capability_selection(&owner, &[6, 286])
                .ok_or("named selection")?;
        let selected = admitted.continuity.selected_capabilities;
        assert_eq!(
            selected.as_slice(),
            if native { &[6, 286][..] } else { &[6][..] }
        );
        let (_, output) =
            super::weather_wire::fixture_access::serve_input(&owner, admitted, &[]).await?;
        let mut cursor = 0;
        let mut entities = Vec::new();
        while cursor < output.len() {
            let length = u32::from_be_bytes(output[cursor..cursor + 4].try_into()?) as usize;
            let envelope = decode_wire_envelope(&output[cursor + 4..cursor + 4 + length])?;
            cursor += 4 + length;
            if envelope.message_type() == MessageType::SnapshotChunk {
                let (_, domains) = decode_snapshot_chunk(envelope.payload())?;
                for domain in domains {
                    if domain.domain_id == 1 {
                        let (snapshot, _) = decode_world_spatial_entities_snapshot_with_fields(
                            domain.payload,
                            EntityFields::selected(selected.as_slice()),
                        )
                        .map_err(debug)?;
                        entities.extend(snapshot.entities);
                    }
                }
            }
        }
        let player = entities
            .iter()
            .find(|entity| entity.entity.identity == deps.actor.placement_identity())
            .ok_or("player sent")?;
        assert_eq!(player.kind, EntityKind::Player);
        let creature = entities
            .iter()
            .find(|entity| entity.entity.identity == creature.placement_identity())
            .ok_or("creature sent")?;
        assert_eq!(creature.kind, EntityKind::Creature);
        if native {
            assert_eq!(
                player
                    .display_name
                    .ok_or("player name sent")?
                    .as_str()
                    .map_err(debug)?,
                "Fixture Hero"
            );
            assert_eq!(
                creature
                    .display_name
                    .ok_or("creature name sent")?
                    .as_str()
                    .map_err(debug)?,
                "Rat"
            );
        } else {
            assert!(player.display_name.is_none());
            assert!(creature.display_name.is_none());
        }
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
