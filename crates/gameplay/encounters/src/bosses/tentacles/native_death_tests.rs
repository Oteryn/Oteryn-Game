use super::*;
use oteryn_gameplay_entities::carrier::{ActorState, LocalPosition, Slot};
#[cfg(any(test, feature = "test-support"))]
#[allow(
    clippy::expect_used,
    reason = "test-only native fixture assertions retain descriptive failures"
)]
#[test]
fn crystal_tentacle_native_proof() {
    fn id(tag: u8) -> [u8; 16] {
        let mut b = [0; 16];
        b[6] = 0x70;
        b[8] = 0x80;
        b[15] = tag;
        b
    }
    fn runtime() -> ChannelRuntimeV1 {
        let world = WorldId::decode(&id(1)).expect("valid native fixture operation must succeed");
        ChannelRuntimeV1::from_committed_assignment(
            world,
            ChannelId::decode(&id(2)).expect("valid native fixture operation must succeed"),
            NodeId::decode(&id(3)).expect("valid native fixture operation must succeed"),
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            16,
            ChannelContentPin::from_activation(
                world,
                1,
                [1; 32],
                [2; 32],
                [3; 32],
                [4; 32],
                (100, 100, 7),
            ),
        )
        .expect("valid native fixture operation must succeed")
    }
    fn dead(r: &mut ChannelRuntimeV1, key: &str, at: MovementLocalPosition) -> ExactActorRef {
        let a = r
            .admit_encounter_fixture_actor(
                ActorState(0),
                key,
                2500,
                Some(LocalPosition {
                    x: at.x,
                    y: at.y,
                    floor: at.floor,
                }),
            )
            .expect("valid native fixture operation must succeed");
        let a = ExactActorRef(a);
        r.borrow_exact_actor_commit()
            .commit_damage(
                a,
                OwnerDamageCommand {
                    target: key.as_bytes(),
                    occurrence: b"tentacle-death",
                    binding: b"tentacle-death\0source2500",
                    damage: 2500,
                },
            )
            .expect("valid native fixture operation must succeed");
        let mut o = r.borrow_combat_death();
        let receipt = o
            .committed_lethal_receipt(a)
            .expect("valid native fixture operation must succeed");
        o.project_committed_lethal(receipt)
            .expect("valid native fixture operation must succeed");
        a
    }
    for n in 1..=10 {
        let key = if n == 1 {
            "oteryn:creature.tentacle".to_owned()
        } else {
            format!("oteryn:creature.tentacle{n}")
        };
        let line = crystal_tentacle_line(key.as_bytes())
            .expect("valid native fixture operation must succeed");
        assert!(line.len() >= 2);
        let mut r = runtime();
        let a = dead(&mut r, &key, line[0]);
        let mut ledger = TentacleDeathLedger::default();
        let result = r
            .commit_tentacle_next(&mut ledger, a)
            .expect("valid native fixture operation must succeed");
        assert!(result.newly_committed);
        assert_eq!(result.position, Some(line[1]));
        let created = result
            .spawned
            .expect("valid native fixture operation must succeed");
        assert_eq!(
            r.read_actor_position(created)
                .expect("valid native fixture operation must succeed")
                .position(),
            line[1]
        );
        let index = r
            .encounter_actor_index(created)
            .expect("valid native fixture operation must succeed");
        assert!(
            matches!(r.encounter_slot(index),Slot::CreatureOccupied{health: 2500,target_identity,..} if target_identity.as_ref()==key.as_bytes())
        );
        assert!(
            !r.commit_tentacle_next(&mut ledger, a)
                .expect("valid native fixture operation must succeed")
                .newly_committed
        );
        let final_actor = dead(
            &mut r,
            &key,
            *line
                .last()
                .expect("valid native fixture operation must succeed"),
        );
        let result = r
            .commit_tentacle_next(&mut ledger, final_actor)
            .expect("valid native fixture operation must succeed");
        assert!(result.newly_committed);
        assert!(result.spawned.is_none());
        assert!(result.position.is_none());
        let fallback = dead(
            &mut r,
            &key,
            MovementLocalPosition {
                x: 1,
                y: 1,
                floor: 1,
            },
        );
        let result = r
            .commit_tentacle_next(&mut ledger, fallback)
            .expect("valid native fixture operation must succeed");
        assert_eq!(result.position, Some(line[0]));
        assert_eq!(
            r.read_actor_position(
                result
                    .spawned
                    .expect("valid native fixture operation must succeed")
            )
            .expect("valid native fixture operation must succeed")
            .position(),
            line[0]
        );
        r.remove_encounter_actor(a)
            .expect("valid native fixture operation must succeed");
        assert!(r.commit_tentacle_next(&mut ledger, a).is_err());
    }
    assert!(crystal_tentacle_line(b"oteryn:creature.rat").is_err());
    println!(
        "PASS all10 Tentacle actual native sealed-death spawn-next fresh generation+HP2500, exact ordered/reversed lines, final no-spawn, outside-line first fallback, repeated death and retired source guard"
    );
}
