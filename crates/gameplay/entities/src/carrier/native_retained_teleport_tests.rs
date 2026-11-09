use super::*;
use crate::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_content::content::native_monster_callbacks::{
    NativeCallbackList, qualify_native_callback,
};

fn id(tag: u8) -> [u8; 16] {
    let mut bytes = [tag; 16];
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    bytes
}

#[test]
fn srx3_native_retained_teleport_moves_registered_hp0_but_not_completed_source()
-> Result<(), String> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| -> Result<(), String> {
            let world = oteryn_gameplay_content::content::native_gameplay_test_world_id()
                .map_err(|error| format!("actual native World: {error:?}"))?;
            let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../../content/spells.manifest.json");
            let native = oteryn_gameplay_content::content::native_gameplay::retained_callback_native_fixture_at(world, &manifest);
            let key = "oteryn:creature.professor_maxxen";
            let record = native.creature_profiles().records.iter()
                .find(|record| record.profile.target.key == key)
                .ok_or("actual Maxxen source body")?;
            let source = qualify_native_callback(&native, &record.profile.target, 3, NativeCallbackList::Defence)
                .map_err(|error| format!("source closure: {error:?}"))?
                .ok_or("actual Maxxen teleport slot")?;
            assert_eq!(source.entry().ability.key, "oteryn:ability.spell.maxxenteleport");
            let pin = ChannelContentPin::from_activation(world, 1, native.source_digest(), [2; 32], [3; 32], [4; 32], (100, 100, 7));
            let mut runtime = ChannelRuntimeV1::from_committed_assignment(world, ChannelId::decode(&id(81)).map_err(|error| format!("retained teleport fixture: {error:?}"))?, NodeId::decode(&id(82)).map_err(|error| format!("retained teleport fixture: {error:?}"))?, 1, 1, 1, "runtime-scope-assignment:1", 8, pin).map_err(|error| format!("retained teleport fixture: {error:?}"))?;
            native.install_companion_policies(&mut runtime).map_err(|error| format!("retained teleport fixture: {error:?}"))?;
            let maximum = runtime.companion_policy(key).map_err(|error| format!("retained teleport fixture: {error:?}"))?.maximum_health;
            let start = MovementLocalPosition { x: 100, y: 100, floor: 7 };
            let actor = runtime.admit_source_pinned_lab_creature(start, key, maximum).map_err(|error| format!("retained teleport fixture: {error:?}"))?;
            runtime.install_creature_policy(actor, key).map_err(|error| format!("retained teleport fixture: {error:?}"))?;
            let stamp = runtime.issue_owner_work().map_err(|error| format!("retained teleport fixture: {error:?}"))?;
            let retained = runtime.capture_native_retained_callback_teleport(actor, &source, stamp).map_err(|error| format!("retained teleport fixture: {error:?}"))?;
            assert!(retained.matches_current(&runtime));
            let live = runtime.read_actor_position(actor).map_err(|error| format!("retained teleport fixture: {error:?}"))?;
            let middle = MovementLocalPosition { x: 101, ..start };
            let live_moved = runtime.commit_native_retained_callback_teleport(&retained, live, middle).map_err(|error| format!("retained teleport fixture: {error:?}"))?;
            assert_eq!(live_moved.position(), middle);
            let hit = runtime.borrow_exact_actor_commit_at_us(1_000_000).commit_damage(actor, OwnerDamageCommand {
                target: key.as_bytes(), occurrence: b"retained-teleport:lethal", binding: b"retained-teleport:lethal\0source", damage: maximum,
            }).map_err(|error| format!("retained teleport fixture: {error:?}"))?;
            assert_eq!(hit.health_after, 0);
            let receipt = runtime.borrow_combat_death().committed_lethal_receipt(actor).map_err(|error| format!("retained teleport fixture: {error:?}"))?;
            let death = receipt.projection.occurrence.death_key().map_err(|error| format!("retained teleport fixture: {error:?}"))?;
            runtime.borrow_combat_death().project_committed_lethal(receipt).map_err(|error| format!("retained teleport fixture: {error:?}"))?;
            assert!(runtime.source_creature_is_registered(actor).map_err(|error| format!("retained teleport fixture: {error:?}"))?);
            assert!(runtime.retains_corpse_projection(actor));
            assert!(retained.matches_current(&runtime));
            assert!(runtime.borrow_movement_position().commit_creature_relocation(actor, live_moved, start).is_err(), "ordinary relocation remains live-only");
            let moved = runtime.commit_native_retained_callback_teleport(&retained, live_moved, start)
                .map_err(|error| format!("older source teleport must move its exact registered HP0 actor: {error:?}"))?;
            assert_eq!(moved.position(), start);
            assert!(runtime.commit_native_retained_callback_teleport(&retained, live_moved, middle).is_err(), "stale original snapshot is rejected");
            runtime.complete_source_creature_death(actor, death).map_err(|error| format!("retained teleport fixture: {error:?}"))?;
            assert!(!retained.matches_current(&runtime));
            assert!(runtime.commit_native_retained_callback_teleport(&retained, moved, middle).is_err(), "completed corpse slot is not a source teleport actor");
            Ok(())
        })
        .map_err(|error| format!("retained teleport test thread: {error}"))?
        .join()
        .map_err(|_| "retained teleport test thread panicked".to_owned())?
}
