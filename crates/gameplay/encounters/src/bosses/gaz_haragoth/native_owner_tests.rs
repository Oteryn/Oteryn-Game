#![allow(clippy::expect_used)]
fn uuid_v7(raw: u64) -> [u8; 16] {
    let mut value = [0; 16];
    value[8..].copy_from_slice(&raw.to_be_bytes());
    value[6] = 0x70;
    value[8] = (value[8] & 0x3f) | 0x80;
    value
}

fn node(raw: u64) -> NodeId {
    NodeId::decode(&uuid_v7(raw)).expect("valid NodeId fixture")
}

fn runtime(capacity: usize) -> ChannelRuntimeV1 {
    ChannelRuntimeV1::from_committed_assignment(
        WorldId::decode(&uuid_v7(20)).expect("world"),
        ChannelId::decode(&uuid_v7(21)).expect("channel"),
        node(22),
        7,
        3,
        11,
        "runtime-scope-assignment:11",
        capacity,
        ChannelContentPin::test(WorldId::decode(&uuid_v7(20)).expect("world")),
    )
    .expect("runtime")
}

use oteryn_gameplay_entities::carrier::{ActorState, CREATUREAI0_SUMMON_ENTRIES_PER_CREATURE_MAX};
use oteryn_gameplay_entities::*;
#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "test-only native fixture assertions retain descriptive failures"
)]
mod native_callback_origin_tests {
    use super::*;
    #[test]
    fn source_callback_origin_is_not_behavior_index_and_current_fence_controls_admission() {
        // Foundation is path-loaded by PostgreSQL integration targets; use its own
        // actual carrier fixture, without depending on the unrelated transport test tree.
        let mut r = runtime(8);
        let parent = r
            .admit_monster_lab_creature(
                MovementLocalPosition {
                    x: 100,
                    y: 100,
                    floor: 7,
                },
                "oteryn:creature.gaz_haragoth",
                10000,
            )
            .expect("valid native fixture operation must succeed");
        let b = r.binding();
        let (f, stamp) = oteryn_gameplay_foundation::foundation::crystal_timer_fixture(
            oteryn_gameplay_foundation::foundation::RuntimeScopeRefV1::channel(
                b.world_id(),
                b.channel_id(),
            ),
            b.scope_generation(),
        )
        .expect("valid native fixture operation must succeed");
        let spec = NativeSummonAdmissionSpec::qualified_callback(
            "oteryn:creature.gaz_haragoth",
            "oteryn:creature.minion_of_gaz_haragoth",
            1000,
            "oteryn:ability.spell.gaz_haragoth_summon",
            "definition-r1",
            "ai-profile:source-fixture:0:attack:11",
            7,
            7,
            r.content_pin().server_artifact_digest(),
        )
        .expect("valid native fixture operation must succeed");
        let child = r
            .admit_native_summon(
                &f,
                stamp,
                parent,
                &spec,
                MovementLocalPosition {
                    x: 101,
                    y: 100,
                    floor: 7,
                },
            )
            .expect("valid native fixture operation must succeed");
        assert!(
            matches!(r.native_summon_origin(child).expect("valid native fixture operation must succeed"),Some(NativeSummonOrigin::SourceCallback{ability,occurrence,..})if ability.as_ref()=="oteryn:ability.spell.gaz_haragoth_summon"&&occurrence.as_ref()=="ai-profile:source-fixture:0:attack:11")
        );
        let (wrong, wrong_stamp) = oteryn_gameplay_foundation::foundation::crystal_timer_fixture(
            oteryn_gameplay_foundation::foundation::RuntimeScopeRefV1::channel(
                b.world_id(),
                b.channel_id(),
            ),
            oteryn_gameplay_foundation::foundation::ScopeOwnershipGeneration::new(2)
                .expect("valid native fixture operation must succeed"),
        )
        .expect("valid native fixture operation must succeed");
        assert_eq!(
            r.admit_native_summon(
                &wrong,
                wrong_stamp,
                parent,
                &spec,
                MovementLocalPosition {
                    x: 102,
                    y: 100,
                    floor: 7
                }
            ),
            Err(CarrierError::WrongScope)
        );
        r.remove_test_actor(parent)
            .expect("valid native fixture operation must succeed");
        assert!(r.native_summon_origin(child).is_err());
    }

    /// `SPAWNCEIL-RL-02` (SPAWN-CEILINGS-1, Codex 4243039217): one retirement call examines at
    /// most 256 links, from where the previous one stopped, so it never costs every link of 640;
    /// the dead children it did not reach retire on later calls, and an admission first retires
    /// its own parent's dead children, so the deferred ones never refuse a summon.
    #[test]
    fn summon_retirement_is_bounded_per_call_and_an_admission_retires_its_parent_first() {
        const SUMMONERS: usize = 40;
        const CHILDREN: usize = 16;
        const LINKS: usize = SUMMONERS * CHILDREN;
        const BUDGET: usize = NATIVE_SUMMON_RETIREMENT_LINKS_PER_CALL_MAX;
        let mut r = runtime(SUMMONERS * (CHILDREN + 1) + 8);
        let b = r.binding();
        let (f, stamp) = oteryn_gameplay_foundation::foundation::crystal_timer_fixture(
            oteryn_gameplay_foundation::foundation::RuntimeScopeRefV1::channel(
                b.world_id(),
                b.channel_id(),
            ),
            b.scope_generation(),
        )
        .expect("the fixture fence is current");
        let spec = NativeSummonAdmissionSpec::qualified(
            "oteryn:creature.gaz_haragoth",
            "oteryn:creature.minion_of_gaz_haragoth",
            1000,
            0,
            16,
            16,
            r.content_pin().server_artifact_digest(),
        )
        .expect("the summon spec qualifies");
        let at = |i: usize, dx: usize, dy: usize| MovementLocalPosition {
            x: 100 + i32::try_from(6 * (i % 20) + dx).expect("a small column"),
            y: 100 + i32::try_from(6 * (i / 20) + dy).expect("a small row"),
            floor: 7,
        };
        let mut parents = Vec::new();
        let mut children = Vec::new();
        for i in 0..SUMMONERS {
            let parent = r
                .admit_monster_lab_creature(at(i, 0, 0), "oteryn:creature.gaz_haragoth", 10000)
                .expect("a summoner is admitted");
            for k in 0..CHILDREN {
                children.push((
                    i,
                    r.admit_native_summon(&f, stamp, parent, &spec, at(i, 1 + k % 4, 1 + k / 4))
                        .expect("every summoner summons 16 children"),
                ));
            }
            parents.push(parent);
        }
        assert_eq!(r.encounter_fixture_summon_progress().0, LINKS);
        // A pass over live links examines 256 of them and retires none.
        r.set_encounter_fixture_retire_cursor(0);
        assert_eq!(r.retire_native_summons(&f, stamp), Ok(0));
        assert_eq!(r.encounter_fixture_summon_progress().1, 256);
        for &(_, child) in &children {
            r.commit_monster_lab_damage(child, b"native-child-lethal", 1000)
                .expect("a child dies");
        }
        // 640 dead children: one call retires 256 of them (ordinals 256..512, summoners 16..31).
        assert_eq!(r.retire_native_summons(&f, stamp), Ok(BUDGET));
        assert_eq!(r.encounter_fixture_summon_progress().0, LINKS - BUDGET);
        assert_eq!(r.encounter_fixture_summon_progress().1, 512);
        // Summoner 15's dead children (ordinals 240..256) lie past the next pass (ordinals
        // 512..640, then wrapping, 0..128): its admission retires that pass and then them.
        const LAST: usize = 15;
        let last = parents[LAST];
        let summoned = r
            .admit_native_summon(&f, stamp, last, &spec, at(LAST, 1, 1))
            .expect("deferred dead children never refuse a summon");
        assert_eq!(r.native_summon_role(summoned), Ok(Some(last)));
        assert_eq!(r.native_summon_count(last, None), 1);
        assert_eq!(
            r.encounter_fixture_summon_progress().0,
            LINKS - 2 * BUDGET - CHILDREN + 1
        );
        assert_eq!(r.encounter_fixture_summon_progress().1, 128);
        for &(i, child) in &children {
            if i == LAST {
                assert!(r.native_summon_role(child).is_err());
            }
        }
        // The remaining dead children (summoners 8..14) retire on the next call; then nothing.
        let remaining = children
            .iter()
            .filter(|(_, child)| r.native_summon_role(*child).is_ok())
            .count();
        assert_eq!(remaining, 7 * CHILDREN);
        assert_eq!(r.retire_native_summons(&f, stamp), Ok(remaining));
        assert_eq!(r.retire_native_summons(&f, stamp), Ok(0));
        assert_eq!(r.encounter_fixture_summon_progress().0, 1);
        for &parent in &parents {
            assert_eq!(
                r.native_summon_count(parent, None),
                u32::from(parent == last)
            );
        }
    }

    /// SPAWN-CEILINGS-1 (Codex 4243039220): the memo table answers by exact creature and entry
    /// past the D57 envelope; a creature's first touch drops only what its slot's former
    /// occupant left, and the full-table prune drops only dead creatures' memos.
    #[test]
    fn actor_memo_table_is_keyed_by_exact_creature_and_drops_stale_memos_by_slot() {
        const CREATURES: usize = 300;
        const ENTRIES: usize = CREATUREAI0_SUMMON_ENTRIES_PER_CREATURE_MAX;
        let mut r = runtime(CREATURES + 8);
        let mut table = ActorMemoTable::<usize, u64>::default();
        assert!(table.is_empty());
        let mut actors = Vec::new();
        for i in 0..CREATURES {
            let actor = ExactActorRef(
                r.admit_encounter_fixture_actor(
                    ActorState(0),
                    "oteryn:creature.boreth",
                    1000,
                    None,
                )
                .expect("a creature is admitted"),
            );
            for entry in 0..ENTRIES {
                assert_eq!(
                    table.insert(actor, entry, (i * ENTRIES + entry) as u64),
                    None
                );
            }
            actors.push(actor);
        }
        assert_eq!(table.len(), CREATURES * ENTRIES);
        for (i, &actor) in actors.iter().enumerate() {
            assert_eq!(table.actor_len(actor), ENTRIES);
            assert_eq!(table.get(actor, &3), Some(&((i * ENTRIES + 3) as u64)));
        }
        assert_eq!(
            table.insert(actors[7], 3, 1),
            Some((7 * ENTRIES + 3) as u64)
        );
        assert_eq!(table.len(), CREATURES * ENTRIES);
        // The first creature's slot is taken again: its memos never answer the new occupant,
        // and the new occupant's first touch drops exactly them.
        r.remove_test_actor(actors[0]).expect("removed");
        let next = ExactActorRef(
            r.admit_encounter_fixture_actor(ActorState(0), "oteryn:creature.boreth", 1000, None)
                .expect("a creature is admitted"),
        );
        assert_eq!(next.actor_local_id(), actors[0].actor_local_id());
        assert_eq!(table.get(next, &3), None);
        assert_eq!(table.actor_len(next), 0);
        assert_eq!(table.evict_stale_at(actors[1]), 0);
        assert_eq!(table.evict_stale_at(next), ENTRIES);
        assert_eq!(table.len(), (CREATURES - 1) * ENTRIES);
        assert_eq!(table.get(actors[0], &3), None);
        // A dead creature whose slot nobody reuses keeps its memos until the full-table prune.
        r.remove_test_actor(actors[1]).expect("removed");
        assert_eq!(table.insert(next, 0, 9), None);
        assert_eq!(table.len(), (CREATURES - 1) * ENTRIES + 1);
        table.prune(|actor| r.contains_live_creature(actor));
        assert_eq!(table.len(), (CREATURES - 2) * ENTRIES + 1);
        assert_eq!(table.actor_len(actors[1]), 0);
        assert_eq!(table.actor_len(actors[2]), ENTRIES);
        assert_eq!(table.get(next, &0), Some(&9));
        assert!(!table.is_empty());
        table.prune(|_| false);
        assert!(table.is_empty());
        assert_eq!(table.get(next, &0), None);
    }
}
