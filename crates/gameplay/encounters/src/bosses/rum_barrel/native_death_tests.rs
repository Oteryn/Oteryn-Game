use super::*;
use oteryn_gameplay_entities::carrier::{ActorState, LocalPosition};
#[cfg(any(test, feature = "test-support"))]
#[allow(
    clippy::expect_used,
    reason = "test-only native fixture assertions retain descriptive failures"
)]
#[test]
fn crystal_rum_native_proof() {
    fn id(tag: u8) -> [u8; 16] {
        let mut b = [0; 16];
        b[6] = 0x70;
        b[8] = 0x80;
        b[15] = tag;
        b
    }
    let world = WorldId::decode(&id(1)).expect("valid native fixture operation must succeed");
    let mut r = ChannelRuntimeV1::from_committed_assignment(
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
    .expect("valid native fixture operation must succeed");
    fn actor(
        r: &mut ChannelRuntimeV1,
        key: &str,
        health: i64,
        x: i32,
        y: i32,
        floor: i16,
    ) -> ExactActorRef {
        let a = r
            .admit_encounter_fixture_actor(
                ActorState(0),
                key,
                health,
                Some(LocalPosition { x, y, floor }),
            )
            .expect("valid native fixture operation must succeed");
        ExactActorRef(a)
    }
    let boss = actor(&mut r, "oteryn:creature.rum_barrel", 10, 100, 100, 7);
    let a = actor(&mut r, "oteryn:creature.weak_spot", 200000, 99, 99, 7);
    let b = actor(&mut r, "oteryn:creature.weak_spot", 200000, 101, 101, 7);
    let upstairs = actor(&mut r, "oteryn:creature.weak_spot", 200000, 101, 101, 8);
    let outside = actor(&mut r, "oteryn:creature.weak_spot", 200000, 102, 100, 7);
    let overlapped = actor(&mut r, "oteryn:creature.weak_spot", 200000, 99, 100, 7);
    let rat = actor(&mut r, "oteryn:creature.rat", 10, 99, 100, 7);
    let mut ledger = RumBarrelDeathLedger::default();
    assert!(r.commit_rum_barrel_death(&mut ledger, boss, 70000).is_err());
    r.borrow_exact_actor_commit()
        .commit_damage(
            boss,
            OwnerDamageCommand {
                target: b"oteryn:creature.rum_barrel",
                occurrence: b"rum-death",
                binding: b"rum-death\0hp10",
                damage: 10,
            },
        )
        .expect("valid native fixture operation must succeed");
    {
        let mut o = r.borrow_combat_death();
        let receipt = o
            .committed_lethal_receipt(boss)
            .expect("valid native fixture operation must succeed");
        o.project_committed_lethal(receipt)
            .expect("valid native fixture operation must succeed");
    }
    assert!(r.commit_rum_barrel_death(&mut ledger, boss, 66665).is_err());
    assert!(ledger.entries.is_empty());
    let result = r
        .commit_rum_barrel_death(&mut ledger, boss, 70000)
        .expect("valid native fixture operation must succeed");
    assert!(result.newly_committed);
    assert_eq!(result.targets.len(), 2);
    for (_, damage) in result.targets {
        assert_eq!(
            (damage.health_before, damage.health_after),
            (200000, 130000)
        );
    }
    assert!(r.contains_live_creature(upstairs));
    assert!(r.contains_live_creature(outside));
    assert!(r.contains_live_creature(overlapped));
    assert!(r.contains_live_creature(rat));
    let repeat = r
        .commit_rum_barrel_death(&mut ledger, boss, 70000)
        .expect("valid native fixture operation must succeed");
    assert!(!repeat.newly_committed);
    assert_eq!(repeat.targets[0].1.health_after, 130000);
    assert_eq!(
        r.commit_rum_barrel_death(&mut ledger, boss, 70001)
            .expect_err("fixture must reject this invalid operation"),
        CarrierError::PlanConflict
    );
    r.remove_encounter_actor(a)
        .expect("valid native fixture operation must succeed");
    let replacement = actor(&mut r, "oteryn:creature.weak_spot", 200000, 99, 99, 7);
    assert!(
        !r.commit_rum_barrel_death(&mut ledger, boss, 70000)
            .expect("valid native fixture operation must succeed")
            .newly_committed
    );
    let idx = r
        .encounter_actor_index(replacement)
        .expect("valid native fixture operation must succeed");
    assert!(matches!(
        r.encounter_slot(idx),
        Slot::CreatureOccupied { health: 200000, .. }
    ));
    assert!(r.commit_rum_barrel_death(&mut ledger, b, 70000).is_err());
    println!(
        "PASS Rum real owner HP commit shared one source draw, death/definition/range/floor/ambiguous-stack guards, replay and changed draw conflict, replacement generation untouched"
    );
}
