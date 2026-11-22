//! W05c exercises the world-clock tick and reads the real bundle presentation queue.
#![allow(clippy::expect_used, reason = "test fixtures and tick assertions")]

use crate::foundation::{ChannelRuntimeV1, MovementFacing, NpcPlacement, NpcPlacementError};
use crate::map::boot::{BundleWorld, tests::door_world};
use crate::map::overlay::TilePos;
use crate::node::serve::bundle_npcs::{
    TimedNpcSpawns,
    tests::{at_minute, channel_runtime, timed_boot},
};

const TALILA: &str = "oteryn:npc.talila";
const VALINDARA: &str = "oteryn:npc.valindara";
const SHARED_TILE: TilePos = TilePos {
    x: 33_504,
    y: 32_222,
    floor: -7,
};

fn timed_npcs(
    runtime: &ChannelRuntimeV1,
    world: &BundleWorld,
    admitted: &[&str],
) -> TimedNpcSpawns {
    let room = crate::content::qualify_native_entry_room(runtime.binding().world_id())
        .expect("entry room");
    let cells = world
        .movement_cells(room.movement_cells())
        .expect("bundle cells");
    TimedNpcSpawns::from_boot(Some(&timed_boot(admitted))).with_bundle_effects(&cells)
}

#[test]
fn timed_npc_period_swap_emits_departure_and_arrival_at_committed_positions() {
    let world = door_world().expect("bundle");
    let mut runtime = channel_runtime();
    let mut timed = timed_npcs(&runtime, &world, &[TALILA, VALINDARA]);
    timed
        .tick(&mut runtime, at_minute(359))
        .expect("clock")
        .expect("Talila arrives");
    let cursor = world
        .facts()
        .movement_effects_since(0)
        .last()
        .map_or(0, |effect| effect.0);

    let talila = runtime.visible_entities().npcs[0].actor;
    assert_eq!(runtime.npc_key(talila), Some(TALILA));
    let before = runtime
        .borrow_movement_position()
        .read(talila)
        .expect("Talila's position");
    let mut moved = before.position();
    moved.x += 1;
    runtime
        .borrow_movement_position()
        .commit_cardinal(before, moved)
        .expect("Talila wanders");

    timed
        .tick(&mut runtime, at_minute(360))
        .expect("clock")
        .expect("sunrise swap");
    assert_eq!(runtime.npc_key(talila), None);
    let visible = runtime.visible_entities();
    assert_eq!(visible.npcs.len(), 1);
    assert_eq!(runtime.npc_key(visible.npcs[0].actor), Some(VALINDARA));
    assert_eq!(
        (
            visible.npcs[0].position.x,
            visible.npcs[0].position.y,
            visible.npcs[0].position.floor
        ),
        (33_504, 32_222, 7)
    );
    timed
        .tick(&mut runtime, at_minute(361))
        .expect("clock")
        .expect("steady day");
    assert_eq!(
        world.facts().movement_effects_since(cursor),
        [
            (
                cursor + 1,
                TilePos {
                    x: SHARED_TILE.x + 1,
                    ..SHARED_TILE
                },
                11
            ),
            (cursor + 2, SHARED_TILE, 11),
        ]
    );
}

#[test]
fn timed_npc_blocked_arrival_emits_only_on_successful_retry() {
    let world = door_world().expect("bundle");
    let mut runtime = channel_runtime();
    let mut timed = timed_npcs(&runtime, &world, &[VALINDARA]);
    let blocker = runtime
        .place_dynamic_npc(
            "oteryn:npc.rascalio",
            NpcPlacement {
                x: SHARED_TILE.x,
                y: SHARED_TILE.y,
                floor: SHARED_TILE.floor,
                direction: MovementFacing::South,
            },
        )
        .expect("arrival cell occupied");

    assert_eq!(
        timed.tick(&mut runtime, at_minute(360)),
        Some(Err(NpcPlacementError::CellOccupied))
    );
    assert_eq!(runtime.visible_entities().npcs.len(), 1);
    assert!(world.facts().movement_effects_since(0).is_empty());

    runtime
        .remove_dynamic_npc_actor(blocker)
        .expect("free arrival cell");
    timed
        .tick(&mut runtime, at_minute(361))
        .expect("clock")
        .expect("retry places Valindara");
    let visible = runtime.visible_entities();
    assert_eq!(visible.npcs.len(), 1);
    assert_eq!(runtime.npc_key(visible.npcs[0].actor), Some(VALINDARA));
    timed
        .tick(&mut runtime, at_minute(362))
        .expect("clock")
        .expect("steady day");
    assert_eq!(
        world.facts().movement_effects_since(0),
        [(1, SHARED_TILE, 11)]
    );
}
