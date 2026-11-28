//! Correlate catalogue placements with the same actor incarnations seen by visibility.
use crate::content::channel_content_pin::ChannelContentPin;
use crate::foundation::{
    ChannelId, ChannelRuntimeV1, MovementFacing, MovementLocalPosition, NodeId, NpcPlacement,
    NpcPlacementTable, NpcPlacements, WorldId,
};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn runtime() -> TestResult<ChannelRuntimeV1> {
    let id = |tag| [1, 0, 0, 0, 0, tag, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, tag];
    let world = WorldId::decode(&id(1)).map_err(|error| format!("{error:?}"))?;
    let channel = ChannelId::decode(&id(2)).map_err(|error| format!("{error:?}"))?;
    let node = NodeId::decode(&id(3)).map_err(|error| format!("{error:?}"))?;
    let pin = ChannelContentPin::from_activation(
        world,
        1,
        [1; 32],
        [2; 32],
        [3; 32],
        [4; 32],
        (2, 11, 7),
    );
    ChannelRuntimeV1::from_committed_assignment(
        world,
        channel,
        node,
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        8,
        pin,
    )
    .map_err(|error| format!("{error:?}").into())
}

#[test]
fn map_spawn_identity_distinguishes_nearby_rats_and_tracks_respawn() -> TestResult {
    let mut runtime = runtime()?;
    let home = runtime.respawn_position();
    let other_home = MovementLocalPosition {
        x: home.x + 2,
        ..home
    };
    const RAT: &str = "oteryn:creature.rat";
    const RAT_HEALTH: u64 = 20;
    let point = |at: MovementLocalPosition| {
        (
            (at.x, at.y, at.floor),
            30_000,
            RAT.to_owned(),
            RAT_HEALTH,
            false,
        )
    };
    let count = runtime
        .begin_activation_spawn_sources(vec![(vec![point(home), point(other_home)], 5_000)])
        .map_err(|error| format!("{error:?}"))?;
    let progress = runtime
        .realize_activation_spawn_window(0)
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(progress.realized, count);
    let visible = runtime.visible_entities();
    let rat_at = |at| {
        visible
            .creatures
            .iter()
            .find(|entry| entry.position == at)
            .map(|entry| entry.actor)
    };
    let rat = rat_at(home).ok_or("rat at sampled home")?;
    let other = rat_at(other_home).ok_or("rat at neighbouring home")?;
    assert_eq!(runtime.creature_spawn_actor(RAT, home), Some(rat));
    assert_eq!(runtime.creature_spawn_actor(RAT, other_home), Some(other));
    assert_eq!(
        runtime.creature_spawn_actor("oteryn:creature.cave_rat", home),
        None
    );

    let mut movement = runtime.borrow_movement_position();
    let here = movement.read(rat).map_err(|error| format!("{error:?}"))?;
    let next = MovementLocalPosition {
        x: home.x + 1,
        ..home
    };
    movement
        .commit_cardinal(here, next)
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(runtime.creature_spawn_actor(RAT, home), Some(rat));
    assert_eq!(runtime.creature_spawn_actor(RAT, next), None);

    runtime
        .commit_monster_lab_damage(rat, b"map-spawn-identity", i64::try_from(RAT_HEALTH)?)
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(runtime.creature_spawn_actor(RAT, home), None);
    assert_eq!(runtime.creature_spawn_actor(RAT, other_home), Some(other));
    let (point, _, _) = runtime.spawn_point_creatures()[0];
    crate::gameplay_transport::drive_owner_spawns(&mut runtime, 0);
    let due = runtime
        .pending_respawn(point)
        .ok_or("pending rat respawn")?
        .due_micros;
    crate::gameplay_transport::drive_owner_spawns(&mut runtime, due);
    let admit = runtime
        .pending_respawn(point)
        .ok_or("pending rat admission")?
        .due_micros;
    assert!(admit > due);
    crate::gameplay_transport::drive_owner_spawns(&mut runtime, admit);
    let replacement = runtime
        .creature_spawn_actor(RAT, home)
        .ok_or("replacement rat")?;
    assert_ne!(replacement, rat);
    assert!(
        runtime
            .visible_entities()
            .creatures
            .iter()
            .any(|entry| entry.actor == replacement)
    );
    Ok(())
}

#[test]
fn map_spawn_identity_tracks_each_npc_placement_after_walking() -> TestResult {
    let mut runtime = runtime()?;
    let home = runtime.respawn_position();
    let other_home = MovementLocalPosition {
        x: home.x + 2,
        ..home
    };
    let placement = |at: MovementLocalPosition| -> TestResult<NpcPlacement> {
        Ok(NpcPlacement {
            floor: i8::try_from(-at.floor)?,
            x: u16::try_from(at.x)?,
            y: u16::try_from(at.y)?,
            direction: MovementFacing::South,
        })
    };
    let key = crate::foundation::rashid_schedule::RASHID_NPC_KEY;
    let placed = runtime
        .place_npcs(
            &NpcPlacementTable {
                npcs: vec![NpcPlacements {
                    key: key.to_owned(),
                    placements: vec![placement(home)?, placement(other_home)?],
                }],
            },
            |admitted| admitted == key,
            true,
        )
        .map_err(|error| format!("{error:?}"))?;
    let npc = placed.actors[0];
    let other = placed.actors[1];

    let mut movement = runtime.borrow_movement_position();
    let here = movement.read(npc).map_err(|error| format!("{error:?}"))?;
    let next = MovementLocalPosition {
        x: home.x + 1,
        ..home
    };
    movement
        .commit_cardinal(here, next)
        .map_err(|error| format!("{error:?}"))?;
    let visible = runtime.visible_entities();
    assert!(
        visible
            .npcs
            .iter()
            .any(|entry| entry.actor == npc && entry.position == next)
    );
    assert_eq!(runtime.npc_placement_actor(key, home), Some(npc));
    assert_eq!(runtime.npc_placement_actor(key, other_home), Some(other));
    assert_eq!(runtime.npc_placement_actor(key, next), None);
    assert_eq!(
        runtime.npc_placement_actor("oteryn:npc.seymour", home),
        None
    );
    let overlapping = runtime
        .place_dynamic_npc(key, placement(home)?)
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(runtime.npc_placement_actor(key, home), None);
    runtime
        .remove_dynamic_npc_actor(npc)
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(runtime.npc_placement_actor(key, home), Some(overlapping));
    runtime
        .remove_dynamic_npc_actor(overlapping)
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(runtime.npc_placement_actor(key, home), None);
    assert_eq!(runtime.npc_placement_actor(key, other_home), Some(other));
    Ok(())
}
