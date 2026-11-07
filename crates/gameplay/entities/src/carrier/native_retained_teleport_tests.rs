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
fn srx3_native_retained_teleport_moves_registered_hp0_but_not_completed_source() {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            let world = oteryn_gameplay_content::content::native_gameplay_test_world_id()
                .expect("actual native World");
            let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../../content/spells.manifest.json");
            let native = oteryn_gameplay_content::content::native_gameplay::retained_callback_native_fixture_at(world, &manifest);
            let key = "oteryn:creature.professor_maxxen";
            let record = native.creature_profiles().records.iter()
                .find(|record| record.profile.target.key == key)
                .expect("actual Maxxen source body");
            let source = qualify_native_callback(&native, &record.profile.target, 3, NativeCallbackList::Defence)
                .expect("source closure")
                .expect("actual Maxxen teleport slot");
            assert_eq!(source.entry().ability.key, "oteryn:ability.spell.maxxenteleport");
            let pin = ChannelContentPin::from_activation(world, 1, native.source_digest(), [2; 32], [3; 32], [4; 32], (100, 100, 7));
            let mut runtime = ChannelRuntimeV1::from_committed_assignment(world, ChannelId::decode(&id(81)).unwrap(), NodeId::decode(&id(82)).unwrap(), 1, 1, 1, "runtime-scope-assignment:1", 8, pin).unwrap();
            native.install_companion_policies(&mut runtime).unwrap();
            let maximum = runtime.companion_policy(key).unwrap().maximum_health;
            let start = MovementLocalPosition { x: 100, y: 100, floor: 7 };
            let actor = runtime.admit_source_pinned_lab_creature(start, key, maximum).unwrap();
            runtime.install_creature_policy(actor, key).unwrap();
            let stamp = runtime.issue_owner_work().unwrap();
            let retained = runtime.capture_native_retained_callback_teleport(actor, &source, stamp).unwrap();
            assert!(retained.matches_current(&runtime));
            let live = runtime.read_actor_position(actor).unwrap();
            let middle = MovementLocalPosition { x: 101, ..start };
            let live_moved = runtime.commit_native_retained_callback_teleport(&retained, live, middle).unwrap();
            assert_eq!(live_moved.position(), middle);
            let hit = runtime.borrow_exact_actor_commit_at_us(1_000_000).commit_damage(actor, OwnerDamageCommand {
                target: key.as_bytes(), occurrence: b"retained-teleport:lethal", binding: b"retained-teleport:lethal\0source", damage: maximum,
            }).unwrap();
            assert_eq!(hit.health_after, 0);
            let receipt = runtime.borrow_combat_death().committed_lethal_receipt(actor).unwrap();
            let death = receipt.projection.occurrence.death_key().unwrap();
            runtime.borrow_combat_death().project_committed_lethal(receipt).unwrap();
            assert!(runtime.source_creature_is_registered(actor).unwrap());
            assert!(runtime.retains_corpse_projection(actor));
            assert!(retained.matches_current(&runtime));
            assert!(runtime.borrow_movement_position().commit_creature_relocation(actor, live_moved, start).is_err(), "ordinary relocation remains live-only");
            let moved = runtime.commit_native_retained_callback_teleport(&retained, live_moved, start)
                .expect("older source teleport must move its exact registered HP0 actor");
            assert_eq!(moved.position(), start);
            assert!(runtime.commit_native_retained_callback_teleport(&retained, live_moved, middle).is_err(), "stale original snapshot is rejected");
            runtime.complete_source_creature_death(actor, death).unwrap();
            assert!(!retained.matches_current(&runtime));
            assert!(runtime.commit_native_retained_callback_teleport(&retained, moved, middle).is_err(), "completed corpse slot is not a source teleport actor");
        })
        .unwrap()
        .join()
        .unwrap();
}
