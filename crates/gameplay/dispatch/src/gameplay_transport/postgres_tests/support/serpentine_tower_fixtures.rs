//! Use the Serpentine Tower lever through production dispatch.
use super::fixture::debug;
use super::{
    AdmittedSession, ComposedFreshAdmission, TestResult, quest_world_callbacks_fixtures as fixture,
};
use crate::foundation::{MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements};
use crate::map::{
    boot::BundleWorld,
    overlay::TilePos,
    view::{MapFacts, placement_key},
};
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Offering, Position};
use oteryn_protocol_oteryn::world_object::{
    UseDisposition, WorldObjectTarget, decode_use_result, encode_use_intent,
};

pub(super) const LEVER: Position = Position {
    x: 33152,
    y: 32866,
    floor: -8,
};
pub(super) const NAME: &str = "serpentine-tower-lever";

fn walls() -> [Position; 4] {
    [
        Position {
            x: 33148,
            y: 32867,
            floor: -9,
        },
        Position {
            x: 33149,
            y: 32867,
            floor: -9,
        },
        Position {
            x: 33148,
            y: 32868,
            floor: -9,
        },
        Position {
            x: 33149,
            y: 32868,
            floor: -9,
        },
    ]
}

pub(super) fn world(callback: &Callback) -> TestResult<BundleWorld> {
    let objects = walls().map(|cell| (cell, 2129));
    let landings = [33148, 33149].map(|x| Position {
        x,
        y: 32869,
        floor: -9,
    });
    fixture::world_cells(callback, LEVER, 2772, 1, &objects, &[], None, &landings)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    world: &BundleWorld,
) -> TestResult {
    let lever = native(LEVER);
    let key = placement_key(lever, 1).ok_or("lever")?;
    let appearance_id = *world
        .base()
        .tile(lever.x, lever.y, lever.floor)
        .and_then(|tile| tile.ids().get(1))
        .ok_or("lever base item")?;
    let occupied = walls()[0];
    let landing = Position {
        y: 32869,
        ..occupied
    };
    let mut placement = world.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(debug)?
        .ok_or("scene item fence")?;
    let movable = fixture::callback("inquisition-weakened-coffin")?
        .offerings
        .into_iter()
        .next()
        .ok_or("movable item fixture")?;
    for (command, appearance, walls_hidden) in [(1, 2773, true), (2, 2772, false), (3, 2773, true)]
    {
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(2))
            .ok_or("clock")?;
        let target = WorldObjectTarget {
            placement: placement.clone(),
            expected_revision: world.facts().object_revision(key),
        };
        let result = fixture::wire(
            owner,
            admitted,
            command,
            2,
            encode_use_intent(&target).map_err(debug)?,
        )
        .await?;
        assert_eq!(
            decode_use_result(&result).map_err(debug)?,
            UseDisposition::Committed,
            "lever operation {command}",
        );
        assert_eq!(
            world
                .facts()
                .base_entry(lever, 1, appearance_id)
                .ok_or("lever facts")?
                .appearance_id,
            appearance
        );
        for wall in walls() {
            let wall_key = placement_key(native(wall), 1).ok_or("forcefield")?;
            assert_eq!(
                world.facts().base_hidden(wall_key),
                walls_hidden,
                "{wall:?}"
            );
        }
        if command == 1 {
            for (index, position) in walls().into_iter().enumerate() {
                fixture::offering(
                    owner,
                    fence.item_fence(),
                    &Offering {
                        position,
                        ..movable.clone()
                    },
                    100 + u64::try_from(index)?,
                )
                .await?;
            }
            owner.states.loop_ground.invalidate();
            owner
                .runtime
                .lock()
                .await
                .place_npcs(
                    &NpcPlacementTable {
                        npcs: vec![NpcPlacements {
                            key: "oteryn:npc.telas".into(),
                            placements: vec![NpcPlacement {
                                x: occupied.x,
                                y: occupied.y,
                                floor: occupied.floor,
                                direction: MovementFacing::South,
                            }],
                        }],
                    },
                    |_| true,
                    true,
                )
                .map_err(debug)?;
        } else {
            let drops = owner
                .root
                .read_item_loop_ground_for_scope(fence.runtime_scope)
                .await
                .map_err(debug)?;
            for wall in walls() {
                let spatial = |p: Position| {
                    crate::durability::map_item_mint_audit::map_item_spatial_position(
                        p.x, p.y, p.floor,
                    )
                };
                let count_at = |p| {
                    drops
                        .iter()
                        .filter(|drop| drop.item.ground.spatial_position == spatial(p))
                        .count()
                };
                assert_eq!(count_at(wall), 0, "wall cell {wall:?}");
                assert_eq!(
                    count_at(Position {
                        y: landing.y,
                        ..wall
                    }),
                    2
                );
            }
            let runtime = owner.runtime.lock().await;
            let occupant = runtime
                .visible_entities()
                .npcs
                .into_iter()
                .find(|npc| runtime.npc_key(npc.actor) == Some("oteryn:npc.telas"))
                .ok_or("occupant")?;
            assert_eq!(
                runtime
                    .read_scene_actor_position(occupant.actor)
                    .map_err(debug)?
                    .position(),
                fixture::local(landing)
            );
        }
    }
    Ok(())
}

fn native(p: Position) -> TilePos {
    TilePos {
        x: p.x,
        y: p.y,
        floor: p.floor,
    }
}
