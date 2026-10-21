use oteryn_gameplay_entities::carrier::{ActorState, Slot};
use oteryn_gameplay_entities::*;
// Actual HP component fixture only. Loaded source-graph proof is separate.
#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "test-only native fixture assertions retain descriptive failures"
)]
mod creature_self_heal_owner_tests {
    use super::*;
    #[test]
    fn independent_defenses_think0_a_b_retry_a_do_not_redamage_or_reheal() {
        fn id(tag: u8) -> [u8; 16] {
            let mut b = [0; 16];
            b[6] = 0x70;
            b[8] = 0x80;
            b[15] = tag;
            b
        }
        let world_id =
            WorldId::decode(&id(1)).expect("valid native fixture operation must succeed");
        let mut r = ChannelRuntimeV1::from_committed_assignment(
            world_id,
            ChannelId::decode(&id(2)).expect("valid native fixture operation must succeed"),
            NodeId::decode(&id(3)).expect("valid native fixture operation must succeed"),
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            8,
            ChannelContentPin::from_activation(
                world_id,
                1,
                [1; 32],
                [2; 32],
                [3; 32],
                [4; 32],
                (100, 100, 7),
            ),
        )
        .expect("valid native fixture operation must succeed");
        let registration = r
            .bind_creature_self_heal(
                "oteryn:creature.boreth",
                "oteryn:ability.creature.boreth.defense-1",
                1400,
                100,
                200,
            )
            .expect("valid native fixture operation must succeed");
        assert_eq!(registration.encounter_fixture_bounds(), (1400, 100, 200));
        let a = r
            .admit_encounter_fixture_actor(ActorState(0), "oteryn:creature.boreth", 1000, None)
            .expect("valid native fixture operation must succeed");
        let actor = ExactActorRef(a);
        let mut ledger = CreatureSelfHealLedger::default();
        assert!(
            r.commit_creature_self_heal(&mut ledger, &registration, actor, 1, 99)
                .is_err()
        );
        let first = r
            .commit_creature_self_heal(&mut ledger, &registration, actor, 0, 200)
            .expect("valid native fixture operation must succeed");
        assert_eq!((first.health_before, first.health_after), (1000, 1200)); // Independent defense B is an explicit owner-component fixture, not a donor catalog claim.
        let b = r
            .bind_creature_self_heal(
                "oteryn:creature.boreth",
                "oteryn:ability.creature.boreth.fixture_second_heal",
                1400,
                100,
                100,
            )
            .expect("valid native fixture operation must succeed");
        let interleaved = r
            .commit_creature_self_heal(&mut ledger, &b, actor, 0, 100)
            .expect("valid native fixture operation must succeed");
        assert_eq!(interleaved.health_after, 1300);
        assert!(
            !r.commit_creature_self_heal(&mut ledger, &registration, actor, 0, 200)
                .expect("valid native fixture operation must succeed")
                .applied
        );
        let index = r
            .encounter_actor_index(actor)
            .expect("valid native fixture operation must succeed");
        assert!(matches!(
            r.encounter_slot(index),
            Slot::CreatureOccupied { health: 1300, .. }
        ));
        let second = r
            .commit_creature_self_heal(&mut ledger, &registration, actor, 1, 200)
            .expect("valid native fixture operation must succeed");
        assert_eq!(second.health_after, 1400);
        let capped = r
            .commit_creature_self_heal(&mut ledger, &registration, actor, 2, 200)
            .expect("valid native fixture operation must succeed");
        assert_eq!(capped.health_after, 1400);
        assert!(
            !r.commit_creature_self_heal(&mut ledger, &registration, actor, 2, 200)
                .expect("valid native fixture operation must succeed")
                .applied
        );
        assert_eq!(
            r.commit_creature_self_heal(&mut ledger, &registration, actor, 0, 200)
                .expect_err("fixture must reject this invalid operation"),
            CarrierError::StaleAttackerSequence
        );
        assert_eq!(
            r.commit_creature_self_heal(&mut ledger, &registration, actor, 2, 100)
                .expect_err("fixture must reject this invalid operation"),
            CarrierError::PlanConflict
        );
        let mut bad = registration.clone();
        bad.encounter_fixture_change_server();
        assert!(
            r.commit_creature_self_heal(&mut ledger, &bad, actor, 4, 100)
                .is_err()
        );
        r.borrow_exact_actor_commit()
            .commit_damage(
                actor,
                OwnerDamageCommand {
                    target: b"oteryn:creature.boreth",
                    occurrence: b"dead",
                    binding: b"dead\0health1400",
                    damage: 1400,
                },
            )
            .expect("valid native fixture operation must succeed");
        assert!(
            r.commit_creature_self_heal(&mut ledger, &registration, actor, 4, 100)
                .is_err()
        );
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "test-only native fixture assertions retain descriptive failures"
)]
mod selfheal_generation_guard_tests {
    use super::*;
    #[test]
    fn stale_generation_cannot_heal_replacement_or_prune_replay() {
        let id = |t: u8| [1, 0, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, t];
        let w = WorldId::decode(&id(1)).expect("valid native fixture operation must succeed");
        let mut r = ChannelRuntimeV1::from_committed_assignment(
            w,
            ChannelId::decode(&id(2)).expect("valid native fixture operation must succeed"),
            NodeId::decode(&id(3)).expect("valid native fixture operation must succeed"),
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            8,
            ChannelContentPin::test(w),
        )
        .expect("valid native fixture operation must succeed");
        let actor = ExactActorRef(
            r.admit_encounter_fixture_actor(ActorState(0), "oteryn:creature.boreth", 100, None)
                .expect("valid native fixture operation must succeed"),
        );
        let reg = r
            .bind_creature_self_heal(
                "oteryn:creature.boreth",
                "oteryn:ability.creature.boreth.fixture_heal",
                1400,
                100,
                100,
            )
            .expect("valid native fixture operation must succeed");
        let mut ledger = CreatureSelfHealLedger::default();
        r.commit_creature_self_heal(&mut ledger, &reg, actor, 0, 100)
            .expect("valid native fixture operation must succeed");
        let index = r
            .encounter_actor_index(actor)
            .expect("valid native fixture operation must succeed");
        // Same occupied slot and same identity, different generation: stale token must refuse.
        r.advance_encounter_fixture_actor_generation(actor)
            .expect("fixture must contain a native creature");
        assert_eq!(
            r.commit_creature_self_heal(&mut ledger, &reg, actor, 1, 100)
                .expect_err("fixture must reject this invalid operation"),
            CarrierError::StaleActorGeneration
        );
        assert!(matches!(
            r.encounter_slot(index),
            Slot::CreatureOccupied { health: 200, .. }
        ));
        assert_eq!(ledger.encounter_fixture_entries_len(), 1);
        assert_eq!(
            ledger.encounter_fixture_sequence(actor, "oteryn:ability.creature.boreth.fixture_heal"),
            Some(0)
        );
    }
}
