use super::*;
use oteryn_gameplay_entities::carrier::{ActorState, LocalPosition};
#[cfg(any(test, feature = "test-support"))]
#[allow(
    clippy::expect_used,
    reason = "test-only native fixture assertions retain descriptive failures"
)]
#[test]
fn crystal_boreth_native_proof() {
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
    fn actor(r: &mut ChannelRuntimeV1, key: &str, x: i32, y: i32, floor: u8) -> ExactActorRef {
        let a = r
            .admit_encounter_fixture_actor(
                ActorState(0),
                key,
                10,
                Some(LocalPosition {
                    x,
                    y,
                    floor: i16::from(floor),
                }),
            )
            .expect("valid native fixture operation must succeed");
        ExactActorRef(a)
    }
    let boss = actor(&mut r, "oteryn:creature.boreth", 1, 1, 7);
    let a = actor(&mut r, "oteryn:creature.plaguethrower", 32936, 31474, 1);
    let b = actor(&mut r, "oteryn:creature.plaguethrower", 32944, 31482, 1);
    let outside = actor(&mut r, "oteryn:creature.plaguethrower", 32945, 31482, 1);
    let upstairs = actor(&mut r, "oteryn:creature.plaguethrower", 32940, 31477, 2);
    let rat = actor(&mut r, "oteryn:creature.rat", 32940, 31477, 1);
    let mut ledger = BorethDeathLedger::default();
    assert!(r.commit_boreth_death(&mut ledger, boss).is_err());
    assert!(r.contains_live_creature(a));
    r.borrow_exact_actor_commit()
        .commit_damage(
            boss,
            OwnerDamageCommand {
                target: b"oteryn:creature.boreth",
                occurrence: b"boreth-death",
                binding: b"boreth-death\0hp10",
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
    let corpses = r.encounter_fixture_death_counts().0;
    let rewards = r.encounter_fixture_death_counts().1;
    let result = r
        .commit_boreth_death(&mut ledger, boss)
        .expect("valid native fixture operation must succeed");
    assert_eq!(result.removed, vec![a, b]);
    assert!(result.newly_committed);
    assert!(!r.contains_live_creature(a));
    assert!(r.contains_live_creature(outside));
    assert!(r.contains_live_creature(upstairs));
    assert!(r.contains_live_creature(rat));
    assert_eq!(r.encounter_fixture_death_counts().0, corpses);
    assert_eq!(r.encounter_fixture_death_counts().1, rewards);
    let replacement = actor(&mut r, "oteryn:creature.plaguethrower", 32940, 31477, 1);
    assert!(
        !r.commit_boreth_death(&mut ledger, boss)
            .expect("valid native fixture operation must succeed")
            .newly_committed
    );
    assert!(r.contains_live_creature(replacement));
    assert!(r.commit_boreth_death(&mut ledger, rat).is_err());
    r.remove_encounter_actor(boss)
        .expect("valid native fixture operation must succeed");
    assert!(r.commit_boreth_death(&mut ledger, boss).is_err());
    assert!(r.contains_live_creature(replacement));
    println!(
        "PASS Boreth actual native committed death + administrative exact-rectangle removal; source guard, before-death guard, boundaries, floor, other species, no new corpses/rewards, retry replacement and stale generation"
    );
}
