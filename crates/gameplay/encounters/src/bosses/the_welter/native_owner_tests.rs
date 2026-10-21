use super::*;
use oteryn_gameplay_entities::carrier::{ActorState, LocalPosition};
// Qualification entry point only: not included in product proposal.
#[cfg(any(test, feature = "test-support"))]
#[allow(
    clippy::expect_used,
    reason = "test-only native fixture assertions retain descriptive failures"
)]
#[test]
fn welter_native_source_owner_proof() {
    fn id(tag: u8) -> [u8; 16] {
        let mut b = [0; 16];
        b[6] = 0x70;
        b[8] = 0x80;
        b[15] = tag;
        b
    }
    fn runtime() -> ChannelRuntimeV1 {
        let w = WorldId::decode(&id(1)).expect("valid native fixture operation must succeed");
        ChannelRuntimeV1::from_committed_assignment(
            w,
            ChannelId::decode(&id(2)).expect("valid native fixture operation must succeed"),
            NodeId::decode(&id(3)).expect("valid native fixture operation must succeed"),
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            8,
            ChannelContentPin::from_activation(
                w,
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
    fn actor(
        r: &mut ChannelRuntimeV1,
        target: &str,
        hp: i64,
        x: i32,
        y: i32,
        floor: i16,
    ) -> ExactActorRef {
        let a = r
            .admit_encounter_fixture_actor(
                ActorState(1),
                target,
                hp,
                Some(LocalPosition { x, y, floor }),
            )
            .expect("valid native fixture operation must succeed");
        ExactActorRef(a)
    }
    let mut r = runtime();
    let b = actor(&mut r, "oteryn:creature.the_welter", 25_000, 100, 100, 7);
    let damage = r
        .borrow_exact_actor_commit()
        .commit_damage(
            b,
            OwnerDamageCommand {
                target: b"oteryn:creature.the_welter",
                occurrence: b"source-test-damage",
                binding: b"source-test-damage\0hp-18000",
                damage: 18000,
            },
        )
        .expect("valid native fixture operation must succeed");
    assert_eq!(damage.health_after, 7000);
    let e = actor(&mut r, "oteryn:creature.egg", 100, 110, 110, 7);
    let s = actor(
        &mut r,
        "oteryn:creature.spawn_of_the_welter",
        100,
        100,
        100,
        7,
    );
    let values:serde_json::Value=serde_json::from_str(include_str!("../../../../../../docs/agents/evidence/monster-full-mechanics-20261004/lanes/encounters/welter-native-profiles.json")).expect("valid native fixture operation must succeed");
    let reg = r
        .bind_welter_pinned_definition(
            values[0]["target"]["key"]
                .as_str()
                .expect("valid native fixture operation must succeed"),
            values[0]["data"]["profile"]["details"]["display_name"]
                .as_str()
                .expect("valid native fixture operation must succeed"),
            values[0]["data"]["profile"]["health"]
                .as_u64()
                .expect("valid native fixture operation must succeed"),
            values[1]["target"]["key"]
                .as_str()
                .expect("valid native fixture operation must succeed"),
            values[1]["data"]["profile"]["details"]["display_name"]
                .as_str()
                .expect("valid native fixture operation must succeed"),
            values[2]["target"]["key"]
                .as_str()
                .expect("valid native fixture operation must succeed"),
            values[2]["data"]["profile"]["details"]["display_name"]
                .as_str()
                .expect("valid native fixture operation must succeed"),
        )
        .expect("valid native fixture operation must succeed");
    assert!(
        r.bind_welter_pinned_definition(
            "oteryn:creature.the_welter",
            "The Welter",
            25001,
            "oteryn:creature.egg",
            "Egg",
            "oteryn:creature.spawn_of_the_welter",
            "Spawn of the Welter"
        )
        .is_err()
    );
    let mut wrong_reg = reg;
    wrong_reg.server_digest[0] ^= 1;
    let snapshot = r.encounter_fixture_snapshot();
    assert_eq!(
        r.commit_welter_consume(&mut WelterConsumeLedger::default(), &wrong_reg, b, 1),
        Err(WelterConsumeError::RegistrationContentPinMismatch)
    );
    assert_eq!(snapshot, r.encounter_fixture_snapshot());
    let mut ledger = WelterConsumeLedger::default();
    let before_failure = r.encounter_fixture_snapshot();
    assert_eq!(
        commit_welter_consume_inner(&mut r, &mut ledger, &reg, b, 1, true),
        Err(WelterConsumeError::Carrier(
            CarrierError::InjectedCommitFailure
        ))
    );
    assert_eq!(before_failure, r.encounter_fixture_snapshot());
    assert!(ledger.entries.is_empty());
    let a = r
        .commit_welter_consume(&mut ledger, &reg, b, 1)
        .expect("valid native fixture operation must succeed");
    assert_eq!(a.consumed, Some((e, WelterPrey::Egg)));
    assert_eq!((a.health_before, a.health_after), (7000, 25000));
    assert!(!r.contains_live_creature(e));
    assert_eq!(
        a.prey_position,
        Some(MovementLocalPosition {
            x: 110,
            y: 110,
            floor: 7
        })
    );
    assert!(r.encounter_fixture_death_counts().0 == 0);
    assert!(r.encounter_fixture_death_counts().1 == 0);
    assert!(r.contains_live_creature(s));
    let replay = r
        .commit_welter_consume(&mut ledger, &reg, b, 1)
        .expect("valid native fixture operation must succeed");
    assert!(!replay.newly_committed);
    assert!(r.contains_live_creature(s));
    assert_eq!(
        r.commit_welter_consume(&mut ledger, &reg, b, 0),
        Err(WelterConsumeError::InvalidSequence)
    );
    let second = r
        .commit_welter_consume(&mut ledger, &reg, b, 2)
        .expect("valid native fixture operation must succeed");
    assert_eq!(second.consumed, Some((s, WelterPrey::Spawn)));
    assert_eq!(second.health_after, 25000);
    assert_eq!(
        r.commit_welter_consume(&mut ledger, &reg, b, 1),
        Err(WelterConsumeError::SupersededOccurrence)
    );
    let empty = r
        .commit_welter_consume(&mut ledger, &reg, b, 3)
        .expect("valid native fixture operation must succeed");
    assert!(empty.consumed.is_none());
    assert_eq!(empty.health_before, empty.health_after);
    let far = actor(&mut r, "oteryn:creature.egg", 100, 111, 100, 7);
    let upstairs = actor(&mut r, "oteryn:creature.egg", 100, 100, 100, 8);
    let no = r
        .commit_welter_consume(&mut ledger, &reg, b, 4)
        .expect("valid native fixture operation must succeed");
    assert!(no.consumed.is_none());
    assert!(r.contains_live_creature(far));
    assert!(r.contains_live_creature(upstairs));
    let wrong = actor(&mut r, "oteryn:creature.rat", 10, 100, 100, 7);
    let before = r.encounter_fixture_snapshot();
    assert_eq!(
        r.commit_welter_consume(&mut ledger, &reg, wrong, 1),
        Err(WelterConsumeError::WrongBossDefinition)
    );
    assert_eq!(before, r.encounter_fixture_snapshot());
    let mut other = runtime();
    let foreign = actor(&mut other, "oteryn:creature.the_welter", 1, 100, 100, 7); // distinct assignment below
    other
        .encounter_fixture_advance(
            ScopeOwnershipGeneration::new(2).expect("valid native fixture operation must succeed"),
        )
        .expect("valid native fixture operation must succeed");
    let state = other.encounter_fixture_snapshot();
    assert!(
        other
            .commit_welter_consume(&mut WelterConsumeLedger::default(), &reg, foreign, 1)
            .is_err()
    );
    assert_eq!(state, other.encounter_fixture_snapshot());
    r.remove_encounter_actor(b)
        .expect("valid native fixture operation must succeed");
    let replacement = actor(&mut r, "oteryn:creature.the_welter", 500, 100, 100, 7);
    assert_ne!(replacement, b);
    let state = r.encounter_fixture_snapshot();
    assert!(r.commit_welter_consume(&mut ledger, &reg, b, 99).is_err());
    assert_eq!(state, r.encounter_fixture_snapshot());
    println!(
        "PASS actual ChannelRuntimeV1: first eligible/cap/one removal/replay/older/empty/range/floor/wrongidentity/retiredowner/stalegeneration"
    );
}
