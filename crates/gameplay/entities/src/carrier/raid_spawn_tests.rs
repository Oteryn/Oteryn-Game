use super::*;
use crate::{CompiledCreaturePolicies, CompiledCreaturePolicy, CreatureFlags};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn checked<T, E: std::fmt::Debug>(value: Result<T, E>) -> TestResult<T> {
    value.map_err(|error| format!("{error:?}").into())
}

fn id(n: u8) -> [u8; 16] {
    [n, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, n]
}

fn runtime(generation: u64) -> TestResult<ChannelRuntimeV1> {
    let world = checked(WorldId::decode(&id(1)))?;
    let mut runtime = checked(ChannelRuntimeV1::from_committed_assignment(
        world,
        checked(ChannelId::decode(&id(2)))?,
        checked(NodeId::decode(&id(3)))?,
        1,
        generation,
        generation,
        &format!("runtime-scope-assignment:{generation}"),
        8,
        ChannelContentPin::test(world),
    ))?;
    let policy = CompiledCreaturePolicy {
        definition_key: "creature:rat".into(),
        definition_revision: "test-1".into(),
        display_name: "rat".into(),
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
    };
    checked(runtime.install_companion_policies(checked(
        CompiledCreaturePolicies::from_active_artifact(
            runtime.content_pin().server_artifact_digest(),
            vec![policy],
        ),
    )?))?;
    Ok(runtime)
}

fn identity(index: u16) -> RaidSpawnIdentity {
    RaidSpawnIdentity {
        firing_id: id(4),
        wave: 0,
        entry: 0,
        index,
    }
}

fn spec(runtime: &ChannelRuntimeV1, boss: bool) -> TestResult<NativeRaidSpawnSpec> {
    checked(NativeRaidSpawnSpec::qualified(
        "creature:rat",
        "test-1",
        runtime.content_pin().server_artifact_digest(),
        boss,
    ))
}

fn position(x: i32) -> MovementLocalPosition {
    MovementLocalPosition {
        x,
        y: 100,
        floor: 7,
    }
}

#[test]
fn raid_owner_places_real_actor_once_and_replay_survives_death() -> TestResult {
    let mut runtime = runtime(1)?;
    let spec = spec(&runtime, true)?;
    let stamp = checked(runtime.issue_owner_work())?;
    let first = checked(runtime.realize_raid_spawn(
        stamp,
        identity(0),
        &spec,
        Some(position(100)),
        |_, _| Some(true),
    ))?;
    let RaidSpawnOutcome::Placed(actor) = first.outcome else {
        return Err("expected live raid actor".into());
    };
    assert!(runtime.contains_live_creature(actor));
    assert_eq!(runtime.visible_entities().creatures.len(), 1);
    assert_eq!(
        checked(runtime.companion_snapshot(actor))?.maximum_health,
        20
    );
    let mut read_map = false;
    let replay = checked(runtime.realize_raid_spawn(
        stamp,
        identity(0),
        &spec,
        Some(position(101)),
        |_, _| {
            read_map = true;
            Some(true)
        },
    ))?;
    assert!(replay.replayed);
    assert_eq!(replay.outcome, first.outcome);
    assert!(!read_map);
    checked(runtime.commit_monster_lab_damage(actor, b"raid-boss-death", 20))?;
    assert!(!runtime.contains_live_creature(actor));
    let dead_replay =
        checked(runtime.realize_raid_spawn(stamp, identity(0), &spec, None, |_, _| Some(true)))?;
    assert_eq!(dead_replay.outcome, first.outcome);
    let statuses = checked(runtime.raid_creatures(id(4)))?;
    assert_eq!(statuses.len(), 1);
    assert!(statuses[0].boss);
    assert!(!statuses[0].alive);
    Ok(())
}

#[test]
fn raid_owner_records_failed_placement_and_never_retries_it() -> TestResult {
    let mut runtime = runtime(1)?;
    let spec = spec(&runtime, false)?;
    let stamp = checked(runtime.issue_owner_work())?;
    let failure =
        checked(runtime.realize_raid_spawn(stamp, identity(0), &spec, None, |_, _| Some(true)))?;
    assert_eq!(failure.outcome, RaidSpawnOutcome::Failed);
    let replay = checked(runtime.realize_raid_spawn(
        stamp,
        identity(0),
        &spec,
        Some(position(100)),
        |_, _| Some(true),
    ))?;
    assert_eq!(replay.outcome, RaidSpawnOutcome::Failed);
    assert!(replay.replayed);
    assert!(runtime.visible_entities().creatures.is_empty());
    let blocked = checked(runtime.realize_raid_spawn(
        stamp,
        identity(1),
        &spec,
        Some(position(100)),
        |_, _| Some(false),
    ))?;
    assert_eq!(blocked.outcome, RaidSpawnOutcome::Failed);
    assert_eq!(checked(runtime.raid_creatures(id(4)))?.len(), 2);
    Ok(())
}

#[test]
fn raid_owner_checks_exact_policy_and_current_generation_before_any_replay() -> TestResult {
    let mut runtime = runtime(1)?;
    let spec = spec(&runtime, false)?;
    let stamp = checked(runtime.issue_owner_work())?;
    checked(
        runtime.realize_raid_spawn(stamp, identity(0), &spec, Some(position(100)), |_, _| {
            Some(true)
        }),
    )?;
    let changed = checked(NativeRaidSpawnSpec::qualified(
        "creature:rat",
        "test-2",
        spec.content,
        false,
    ))?;
    assert!(
        runtime
            .realize_raid_spawn(stamp, identity(0), &changed, Some(position(100)), |_, _| {
                Some(true)
            })
            .is_err()
    );
    assert!(
        runtime
            .realize_raid_spawn(stamp, identity(1), &changed, Some(position(101)), |_, _| {
                Some(true)
            })
            .is_err()
    );
    assert_eq!(runtime.visible_entities().creatures.len(), 1);
    runtime.retire_owner_cycle();
    assert!(
        runtime
            .realize_raid_spawn(stamp, identity(0), &spec, None, |_, _| Some(true))
            .is_err()
    );
    let mut successor = self::runtime(2)?;
    assert!(
        successor
            .realize_raid_spawn(stamp, identity(0), &spec, Some(position(100)), |_, _| Some(
                true
            ))
            .is_err()
    );
    assert!(successor.visible_entities().creatures.is_empty());
    assert!(checked(successor.raid_creatures(id(4)))?.is_empty());
    Ok(())
}

#[test]
fn raid_owner_persistent_close_keeps_creatures_and_explicit_cleanup_removes_without_death()
-> TestResult {
    let mut runtime = runtime(1)?;
    let spec = spec(&runtime, false)?;
    let stamp = checked(runtime.issue_owner_work())?;
    let receipt = checked(runtime.realize_raid_spawn(
        stamp,
        identity(0),
        &spec,
        Some(position(100)),
        |_, _| Some(true),
    ))?;
    let RaidSpawnOutcome::Placed(actor) = receipt.outcome else {
        return Err("expected actor".into());
    };
    assert_eq!(checked(runtime.finish_raid_run(stamp, id(4), false))?, 0);
    assert!(runtime.contains_live_creature(actor));
    assert!(
        runtime
            .realize_raid_spawn(stamp, identity(1), &spec, Some(position(101)), |_, _| Some(
                true
            ))
            .is_err()
    );
    assert_eq!(checked(runtime.finish_raid_run(stamp, id(4), true))?, 1);
    assert!(!runtime.contains_live_creature(actor));
    assert!(runtime.visible_entities().creatures.is_empty());
    assert!(runtime.carrier.corpse_projections.is_empty());
    assert_eq!(checked(runtime.finish_raid_run(stamp, id(4), true))?, 0);
    let replay =
        checked(runtime.realize_raid_spawn(stamp, identity(0), &spec, None, |_, _| Some(true)))?;
    assert!(replay.replayed);
    assert_eq!(replay.outcome, receipt.outcome);
    Ok(())
}

#[test]
fn raid_shared_admission_restores_only_its_selected_slot_on_failure() -> TestResult {
    let mut runtime = runtime(1)?;
    let spec = spec(&runtime, false)?;
    let stamp = checked(runtime.issue_owner_work())?;
    let selected = runtime.carrier.free_head.ok_or("free slot")?;
    let index = usize::try_from(selected)?;
    let Slot::VacantReusable { generation, .. } = &mut runtime.carrier.slots[index] else {
        return Err("free head must be vacant".into());
    };
    *generation = u64::MAX;
    let before = runtime.carrier.slots.clone();
    let occupied = runtime.carrier.occupied.clone();
    assert!(
        runtime
            .realize_raid_spawn(stamp, identity(0), &spec, Some(position(100)), |_, _| Some(
                true
            ))
            .is_err()
    );
    assert_eq!(runtime.carrier.slots, before);
    assert_eq!(runtime.carrier.free_head, Some(selected));
    assert_eq!(runtime.carrier.occupied, occupied);
    assert!(checked(runtime.raid_creatures(id(4)))?.is_empty());
    Ok(())
}

#[test]
fn raid_capacity_reserves_close_and_unknown_lost_cannot_block_existing_cleanup() -> TestResult {
    let mut runtime = runtime(1)?;
    runtime.raid_spawns.test_limit = Some(3);
    let spec = spec(&runtime, false)?;
    let stamp = checked(runtime.issue_owner_work())?;
    for index in 0..2 {
        checked(runtime.realize_raid_spawn(
            stamp,
            identity(index),
            &spec,
            Some(position(100 + i32::from(index))),
            |_, _| Some(true),
        ))?;
    }
    assert_eq!(runtime.raid_spawns.size(), 3);
    assert!(
        runtime
            .realize_raid_spawn(stamp, identity(2), &spec, None, |_, _| Some(true))
            .is_err()
    );
    assert_eq!(checked(runtime.finish_raid_run(stamp, id(5), true))?, 0);
    assert!(runtime.raid_spawns.sealed);
    assert_eq!(checked(runtime.finish_raid_run(stamp, id(4), true))?, 2);
    assert!(runtime.visible_entities().creatures.is_empty());
    assert_eq!(checked(runtime.acknowledge_raid_run_end(stamp, id(4)))?, 2);
    assert_eq!(checked(runtime.acknowledge_raid_run_end(stamp, id(5)))?, 0);
    let unknown = RaidSpawnIdentity {
        firing_id: id(5),
        ..identity(0)
    };
    assert!(
        runtime
            .realize_raid_spawn(stamp, unknown, &spec, None, |_, _| Some(true))
            .is_err()
    );
    Ok(())
}

#[test]
fn raid_durable_end_compacts_terminal_receipts_but_keeps_persistent_actors() -> TestResult {
    let mut runtime = runtime(1)?;
    runtime.raid_spawns.test_limit = Some(3);
    let spec = spec(&runtime, false)?;
    let stamp = checked(runtime.issue_owner_work())?;
    checked(runtime.realize_raid_spawn(stamp, identity(0), &spec, None, |_, _| Some(true)))?;
    let live = checked(runtime.realize_raid_spawn(
        stamp,
        identity(1),
        &spec,
        Some(position(100)),
        |_, _| Some(true),
    ))?;
    assert!(runtime.acknowledge_raid_run_end(stamp, id(4)).is_err());
    checked(runtime.finish_raid_run(stamp, id(4), false))?;
    assert_eq!(checked(runtime.acknowledge_raid_run_end(stamp, id(4)))?, 1);
    assert_eq!(checked(runtime.raid_creatures(id(4)))?.len(), 1);
    let RaidSpawnOutcome::Placed(actor) = live.outcome else {
        return Err("live raid actor".into());
    };
    assert!(runtime.contains_live_creature(actor));
    assert!(
        runtime
            .realize_raid_spawn(stamp, identity(0), &spec, None, |_, _| Some(true))
            .is_err()
    );
    assert_eq!(checked(runtime.finish_raid_run(stamp, id(4), true))?, 1);
    assert_eq!(checked(runtime.acknowledge_raid_run_end(stamp, id(4)))?, 1);
    let next = RaidSpawnIdentity {
        firing_id: id(5),
        ..identity(0)
    };
    assert_eq!(
        checked(runtime.realize_raid_spawn(stamp, next, &spec, None, |_, _| Some(true)))?.outcome,
        RaidSpawnOutcome::Failed
    );
    assert_eq!(runtime.raid_spawns.size(), 3);
    Ok(())
}
