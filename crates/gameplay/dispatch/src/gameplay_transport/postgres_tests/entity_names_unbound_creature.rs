//! S-ENTNAME: a source-spawned creature that no active Creature policy names (the native entry
//! room's fixture spawn of the node boot) stands unbound by design; a named-entities session
//! still admits and sends it without a display name.
use super::fixture::{Harness, debug, id, run};
use super::{TestResult, prepare_current_session};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use oteryn_protocol_oteryn::world_spatial_entities::{
    EntityFields, EntityKind, decode_world_spatial_entities_snapshot_with_fields,
};
use oteryn_protocol_oteryn::{MessageType, decode_snapshot_chunk, decode_wire_envelope};

#[test]
fn entity_names_unbound_spawned_creature_is_sent_without_a_name() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "unbound creature names test panicked")??;
    Ok(())
}

fn scenario() -> TestResult {
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
        let (controller, pin) =
            crate::content::native_gameplay::test_support::activated_loop_monster(world, false);
        let active = controller.active().ok_or("active generation")?;
        let (deps, _) = super::loop_monster::fixture::dependencies(active, &pin)?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
        deps.initialize(&owner).await;
        // A creature with the node boot's native entry room key, which the active policy table
        // does not name: admitted unbound, as `bind_spawned_creature_policy` leaves it.
        let unbound = {
            let mut runtime = owner.runtime.lock().await;
            let actor = runtime
                .admit_pinned_monster_lab_creature(
                    crate::foundation::MovementLocalPosition {
                        x: 3,
                        y: 12,
                        floor: -7,
                    },
                    "oteryn:creature/rat",
                    20,
                )
                .map_err(debug)?;
            assert!(runtime.companion_snapshot(actor).is_err());
            actor
        };
        let mut admitted = deps.admitted;
        admitted.runtime_actor = Some(deps.actor);
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(owner.offered_capabilities(), &[6, 286])
                .ok_or("named selection")?;
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
                            EntityFields::selected(&[6, 286]),
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
        assert_eq!(
            player
                .display_name
                .ok_or("player name sent")?
                .as_str()
                .map_err(debug)?,
            "Fixture Hero"
        );
        let creature = entities
            .iter()
            .find(|entity| entity.entity.identity == unbound.placement_identity())
            .ok_or("unbound creature sent")?;
        assert_eq!(creature.kind, EntityKind::Creature);
        assert!(creature.display_name.is_none());
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
