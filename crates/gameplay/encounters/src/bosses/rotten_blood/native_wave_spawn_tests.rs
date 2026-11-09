#![allow(clippy::expect_used, reason = "descriptive native fixture assertions")]
use super::super::RoomPosition;
use super::*;
use oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin;
use oteryn_gameplay_entities::{CompiledCreaturePolicies, CompiledCreaturePolicy, CreatureFlags};
use oteryn_gameplay_foundation::foundation::{ChannelId, NodeId, WorldId, crystal_timer_fixture};

fn uuid(n: u8) -> [u8; 16] {
    [n, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, n]
}

fn setup(
    key: &str,
) -> (
    ChannelRuntimeV1,
    ExactActorRef,
    ScopeRuntimeFence,
    RuntimeWorkStamp,
) {
    let world = WorldId::decode(&uuid(1)).expect("world");
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        world,
        ChannelId::decode(&uuid(2)).expect("channel"),
        NodeId::decode(&uuid(3)).expect("node"),
        1,
        3,
        11,
        "runtime-scope-assignment:11",
        32,
        ChannelContentPin::test(world),
    )
    .expect("runtime");
    let policies = [
        key,
        "oteryn:creature.mushroom",
        "oteryn:creature.elder_bloodjaw",
    ]
    .into_iter()
    .map(|key| CompiledCreaturePolicy {
        definition_key: key.into(),
        definition_revision: "definition-r1".into(),
        display_name: key.into(),
        maximum_health: 20,
        forge_difficulty: None,
        base_speed: 220,
        outfit_look_type: 21,
        object_look_type: None,
        summonable: false,
        convinceable: false,
        mana_cost: None,
        is_familiar: false,
        condition_immunities: Vec::new(),
        preferred_distance: Some(1),
        reward_boss: Some(false),
        blocked_by_nearby_players: false,
        defense: Some(1),
        armor: Some(0),
        mitigation: None,
        resistances: Vec::new(),
        damage_immunities: Vec::new(),
        healing_from_damage: Vec::new(),
        flags: CreatureFlags {
            attackable: true,
            illusionable: false,
            health_hidden: false,
        },
    })
    .collect();
    runtime
        .install_companion_policies(
            CompiledCreaturePolicies::from_active_artifact(
                runtime.content_pin().server_artifact_digest(),
                policies,
            )
            .expect("fixture policy table"),
        )
        .expect("install policies");
    let boss = runtime
        .admit_pinned_monster_lab_creature(
            MovementLocalPosition {
                x: 100,
                y: 100,
                floor: 7,
            },
            key,
            20,
        )
        .expect("boss");
    runtime
        .install_creature_policy(boss, key)
        .expect("boss policy");
    let binding = runtime.binding();
    let (fence, stamp) = crystal_timer_fixture(binding.runtime_scope(), binding.scope_generation())
        .expect("current fence");
    (runtime, boss, fence, stamp)
}

struct Map {
    answer: Option<(MovementLocalPosition, bool)>,
    calls: usize,
    last_occurrence: Option<(u64, u8)>,
}
impl NativeWaveSpawnMap for Map {
    fn resolve(
        &mut self,
        _: &ChannelRuntimeV1,
        _: ExactActorRef,
        _: &SummonWaveEffect,
        sequence: u64,
        ordinal: u8,
        _: RuntimeWorkStamp,
    ) -> Option<(MovementLocalPosition, bool)> {
        self.calls += 1;
        self.last_occurrence = Some((sequence, ordinal));
        self.answer
    }
}

fn effect(offset: u8) -> SummonWaveEffect {
    SummonWaveEffect::Spawn {
        creature_key: "oteryn:creature.mushroom",
        around: RoomPosition {
            x: 100,
            y: 100,
            floor: 7,
        },
        placement: SummonPlacement::Random {
            offset_tiles: offset,
        },
    }
}
fn map() -> Map {
    Map {
        answer: Some((
            MovementLocalPosition {
                x: 101,
                y: 100,
                floor: 7,
            },
            true,
        )),
        calls: 0,
        last_occurrence: None,
    }
}

#[test]
fn source_wave_uses_real_native_policy_and_replays_without_another_child() {
    for (key, offset) in [
        ("oteryn:creature.ichgahal", 2),
        ("oteryn:creature.murcion", 1),
    ] {
        let (mut runtime, boss, fence, stamp) = setup(key);
        let mut owner = NativeWaveSpawnOwner::for_boss(&runtime, boss).expect("source owner");
        let mut map = map();
        let receipt = owner
            .spawn(&mut runtime, &fence, stamp, 1, 0, &effect(offset), &mut map)
            .expect("native spawn");
        assert!(receipt.newly_created);
        assert_eq!(
            map.last_occurrence,
            Some((1, 0)),
            "stable placement draw key"
        );
        let child = runtime
            .companion_snapshot(receipt.child)
            .expect("actual native child");
        assert_eq!(
            child.state.policy.definition_key,
            "oteryn:creature.mushroom"
        );
        assert_eq!(child.state.policy.maximum_health, 20);
        assert!(
            runtime.native_summon_master(receipt.child).is_none(),
            "source createMonster is masterless"
        );
        map.answer = None;
        let binding = runtime.binding();
        let (wrong, wrong_stamp) = crystal_timer_fixture(
            binding.runtime_scope(),
            oteryn_gameplay_foundation::foundation::ScopeOwnershipGeneration::new(4)
                .expect("different generation"),
        )
        .expect("stale fence fixture");
        assert_eq!(
            owner.spawn(
                &mut runtime,
                &wrong,
                wrong_stamp,
                1,
                0,
                &effect(offset),
                &mut map
            ),
            Err(NativeWaveSpawnError::StaleOwner),
            "current custody is checked before an exact replay"
        );
        let replay = owner
            .spawn(&mut runtime, &fence, stamp, 1, 0, &effect(offset), &mut map)
            .expect("exact replay");
        assert_eq!(replay.child, receipt.child);
        assert!(!replay.newly_created);
        assert_eq!(
            map.calls, 1,
            "replay does not resolve changed current facts"
        );
        assert_eq!(
            owner.spawn(&mut runtime, &fence, stamp, 1, 0, &effect(3), &mut map),
            Err(NativeWaveSpawnError::ReplayConflict)
        );
    }
}

#[test]
fn current_map_refusal_and_outside_source_position_stay_closed() {
    let (mut runtime, boss, fence, stamp) = setup("oteryn:creature.ichgahal");
    let mut owner = NativeWaveSpawnOwner::for_boss(&runtime, boss).expect("source owner");
    let before = runtime.positioned_melee_census().expect("before").len();
    let mut map = map();
    map.answer = Some((
        MovementLocalPosition {
            x: 103,
            y: 100,
            floor: 7,
        },
        true,
    ));
    assert_eq!(
        owner.spawn(&mut runtime, &fence, stamp, 1, 0, &effect(2), &mut map),
        Err(NativeWaveSpawnError::InvalidPosition)
    );
    map.answer = Some((
        MovementLocalPosition {
            x: 101,
            y: 100,
            floor: 7,
        },
        false,
    ));
    assert_eq!(
        owner.spawn(&mut runtime, &fence, stamp, 2, 0, &effect(2), &mut map),
        Err(NativeWaveSpawnError::Native(
            oteryn_gameplay_entities::EncounterSpawnError::RefusedMap
        ))
    );
    assert_eq!(
        runtime.positioned_melee_census().expect("after").len(),
        before
    );
    assert_eq!(
        owner.spawn(&mut runtime, &fence, stamp, 1, 0, &effect(2), &mut map),
        Err(NativeWaveSpawnError::StaleSequence)
    );
    assert_eq!(map.calls, 2, "superseded occurrence does no map work");
}

#[test]
fn unknown_map_and_source_offset_widening_never_admit_a_child() {
    let (mut runtime, boss, fence, stamp) = setup("oteryn:creature.ichgahal");
    let mut owner = NativeWaveSpawnOwner::for_boss(&runtime, boss).expect("source owner");
    let before = runtime.positioned_melee_census().expect("before").len();
    let mut map = Map {
        answer: None,
        calls: 0,
        last_occurrence: None,
    };
    assert_eq!(
        owner.spawn(&mut runtime, &fence, stamp, 1, 0, &effect(2), &mut map),
        Err(NativeWaveSpawnError::MissingMap)
    );
    assert_eq!(
        runtime.positioned_melee_census().expect("after").len(),
        before
    );
    assert_eq!(
        owner.spawn(&mut runtime, &fence, stamp, 2, 0, &effect(3), &mut map),
        Err(NativeWaveSpawnError::InvalidSource)
    );
    assert_eq!(
        map.calls, 1,
        "invalid source is rejected before current map work"
    );
}

#[test]
fn installed_owner_runs_wave_spawn_and_cleanup_after_the_boss_leaves() {
    let (mut runtime, boss, _, _) = setup("oteryn:creature.ichgahal");
    let mut owner = NativeWaveSpawnOwner::for_boss(&runtime, boss).expect("source owner");
    let mut foreign = NativeWaveSpawnOwner::for_boss(&runtime, boss).expect("independent consumer");
    let stamp = runtime.issue_owner_work().expect("installed owner work");
    let receipt = owner
        .spawn_current(&mut runtime, stamp, 1, 0, &effect(2), &mut map())
        .expect("current native spawn");
    assert!(receipt.newly_created);
    assert_eq!(
        foreign.retire_wave_child(&mut runtime, stamp, receipt.child),
        Err(NativeWaveSpawnError::UnownedChild)
    );
    assert!(runtime.read_actor_position(receipt.child).is_ok());
    runtime.remove_test_actor(boss).expect("boss leaves");
    assert_eq!(
        owner.retire_wave_child(&mut runtime, stamp, receipt.child),
        Ok(true)
    );
    assert!(runtime.read_actor_position(receipt.child).is_err());
    assert_eq!(
        owner.retire_wave_child(&mut runtime, stamp, receipt.child),
        Ok(false)
    );
}

#[test]
fn cleanup_excludes_population_repairs_and_requires_a_live_scope_owner() {
    let (mut runtime, boss, _, _) = setup("oteryn:creature.ichgahal");
    let mut owner = NativeWaveSpawnOwner::for_boss(&runtime, boss).expect("source owner");
    let stamp = runtime.issue_owner_work().expect("installed owner work");
    let mut repair = effect(2);
    if let SummonWaveEffect::Spawn { placement, .. } = &mut repair {
        *placement = SummonPlacement::Close { offset_tiles: 2 };
    }
    let receipt = owner
        .spawn_current(&mut runtime, stamp, 1, 0, &repair, &mut map())
        .expect("population repair");
    assert_eq!(
        owner.retire_wave_child(&mut runtime, stamp, receipt.child),
        Err(NativeWaveSpawnError::UnownedChild)
    );
    runtime.retire_owner_cycle();
    assert_eq!(
        owner.retire_wave_child(&mut runtime, stamp, receipt.child),
        Err(NativeWaveSpawnError::StaleOwner)
    );
    assert_eq!(
        owner.spawn_current(&mut runtime, stamp, 1, 0, &repair, &mut map()),
        Err(NativeWaveSpawnError::StaleOwner)
    );
}

#[test]
fn mushroom_current_owner_removes_orphan_without_a_death() {
    let (mut runtime, boss, _, _) = setup("oteryn:creature.ichgahal");
    let mut owner = NativeWaveSpawnOwner::for_boss(&runtime, boss).expect("wave owner");
    let stamp = runtime.issue_owner_work().expect("current owner");
    let child = owner
        .spawn_current(&mut runtime, stamp, 1, 0, &effect(2), &mut map())
        .expect("native Mushroom")
        .child;
    let content = runtime.content_pin().server_artifact_digest();
    assert_eq!(
        super::super::native_mushroom::remove_orphan(&mut runtime, child, content),
        Ok(false)
    );
    assert!(runtime.read_actor_position(child).is_ok());
    runtime.remove_test_actor(boss).expect("boss leaves");
    assert_eq!(
        super::super::native_mushroom::remove_orphan(&mut runtime, child, content),
        Ok(true)
    );
    assert!(runtime.read_actor_position(child).is_err());
}

#[test]
fn mushroom_cleanup_rejects_wrong_pin_identity_and_retired_owner() {
    let (mut runtime, boss, _, _) = setup("oteryn:creature.murcion");
    let mut owner = NativeWaveSpawnOwner::for_boss(&runtime, boss).expect("wave owner");
    let stamp = runtime.issue_owner_work().expect("current owner");
    let child = owner
        .spawn_current(&mut runtime, stamp, 1, 0, &effect(1), &mut map())
        .expect("native Mushroom")
        .child;
    let content = runtime.content_pin().server_artifact_digest();
    assert!(super::super::native_mushroom::remove_orphan(&mut runtime, child, [0; 32]).is_err());
    assert!(super::super::native_mushroom::remove_orphan(&mut runtime, boss, content).is_err());
    assert!(runtime.read_actor_position(child).is_ok());
    runtime.remove_test_actor(boss).expect("boss leaves");
    runtime.retire_owner_cycle();
    assert!(super::super::native_mushroom::remove_orphan(&mut runtime, child, content).is_err());
}
