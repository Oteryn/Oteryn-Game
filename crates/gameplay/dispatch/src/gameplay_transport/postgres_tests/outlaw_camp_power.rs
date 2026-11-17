//! Native Outlaw Camp switch through world-object USE and the fenced Ground writer.
use super::fixture::{Harness, debug, id};
use super::questpc8_authored_programmes::{object_target, use_target};
use super::{TestResult, quest_world_callbacks};
use crate::gameplay_transport::{ComposedFreshAdmission, connection::AdmittedSession};
use crate::map::{
    overlay::TilePos,
    view::{MapFacts, placement_key},
};
use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};
use oteryn_protocol_oteryn::world_object::UseDisposition;

pub(super) const SWITCH_CASE: &str = "authored-outlaw-camp-power-switch";
pub(super) const BURN_CASE: &str = "authored-outlaw-camp-power-burn";
const RING: &str = "oteryn:item.tibia.i3050";
const LEVER: TilePos = TilePos {
    x: 32594,
    y: 32212,
    floor: -9,
};
const START: TilePos = TilePos {
    x: LEVER.x - 1,
    ..LEVER
};
const PEDESTAL: TilePos = TilePos { y: 32214, ..LEVER };
const WALL: TilePos = TilePos {
    x: 32604,
    y: 32216,
    ..LEVER
};
const RECEIVER: TilePos = TilePos {
    x: 32613,
    y: 32220,
    floor: -10,
};

#[test]
fn outlaw_camp_power_switch_moves_exact_ring_and_opens_wall_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run(SWITCH_CASE, move || {
        quest_world_callbacks::scenario_with_admin(
            SWITCH_CASE,
            Position {
                x: LEVER.x,
                y: LEVER.y,
                floor: LEVER.floor,
            },
            admin,
        )
    })
}

const BURN_LEVER: TilePos = TilePos {
    x: 32616,
    y: 32222,
    floor: -10,
};
const BURN_START: TilePos = TilePos {
    x: BURN_LEVER.x - 1,
    ..BURN_LEVER
};
const BURN_WALL: TilePos = TilePos {
    x: 32614,
    y: 32205,
    ..BURN_LEVER
};
const STONE: TilePos = TilePos {
    y: 32206,
    ..BURN_WALL
};
const BARREL: TilePos = TilePos {
    y: 32209,
    ..BURN_WALL
};
const SECOND_FIRE: TilePos = TilePos {
    y: 32221,
    ..BURN_START
};

#[test]
fn outlaw_camp_power_burn_uses_ring_only_with_barrel_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run(BURN_CASE, move || {
        quest_world_callbacks::scenario_with_admin(
            BURN_CASE,
            Position {
                x: BURN_LEVER.x,
                y: BURN_LEVER.y,
                floor: BURN_LEVER.floor,
            },
            admin,
        )
    })
}

pub(super) fn world(burn: bool) -> TestResult<crate::map::boot::BundleWorld> {
    use oteryn_world_bundle::{bundle, sector};
    let (bytes, mut pins) = crate::map::boot::tests::bundle();
    let mut data = bundle::read(&bytes)?;
    data.manifest.palette.truncate(1);
    data.manifest.palette[0].key = "oteryn:terrain.tibia.i351".into();
    for (id, appearance, kind) in [
        (1, 2772, bundle::TerrainKind::Common),
        (2, 1271, bundle::TerrainKind::Wall),
        (3, 2321, bundle::TerrainKind::Common),
        (4, 1270, bundle::TerrainKind::Wall),
        (5, 1791, bundle::TerrainKind::Common),
        (6, 6477, bundle::TerrainKind::Common),
        (7, 2118, bundle::TerrainKind::Field),
        (8, 2119, bundle::TerrainKind::Field),
        (9, 2120, bundle::TerrainKind::Field),
    ] {
        data.manifest.palette.push(bundle::PaletteEntry {
            key: format!("oteryn:terrain.tibia.i{appearance}"),
            family: bundle::Family::Terrain,
            id,
            terrain: Some(bundle::Terrain {
                kind,
                walkable: None,
                ground_speed: None,
                blocks_projectile: None,
            }),
        });
    }
    let item = |palette| sector::Item {
        palette,
        depth: 0,
        attrs: Default::default(),
    };
    let mut grouped = std::collections::BTreeMap::<(i8, u16, u16), Vec<sector::Tile>>::new();
    let (start, fixture) = if burn {
        (
            BURN_START,
            vec![
                (BURN_START, vec![]),
                (BURN_LEVER, vec![6, 1]),
                (BURN_WALL, vec![4]),
                (STONE, vec![5]),
                (BARREL, vec![]),
                (RECEIVER, vec![3]),
                (SECOND_FIRE, vec![]),
            ],
        )
    } else {
        (
            START,
            vec![
                (START, vec![]),
                (LEVER, vec![1]),
                (PEDESTAL, vec![3]),
                (WALL, vec![2]),
                (RECEIVER, vec![3]),
            ],
        )
    };
    for (pos, extra) in fixture {
        let mut items = vec![item(0)];
        items.extend(extra.into_iter().map(item));
        grouped
            .entry((pos.floor, pos.y / 32, pos.x / 32))
            .or_default()
            .push(sector::Tile {
                x: pos.x,
                y: pos.y,
                flags: 0,
                house: 0,
                zones: Vec::new(),
                items,
            });
    }
    let sectors: Vec<_> = grouped
        .into_iter()
        .map(|((floor, sy, sx), mut tiles)| {
            tiles.sort_by_key(|tile| (tile.y, tile.x));
            bundle::Sector {
                floor,
                sx,
                sy,
                tiles,
            }
        })
        .collect();
    data.manifest.world = bundle::Extent {
        min_x: START.x,
        min_y: BURN_WALL.y,
        max_x: BURN_LEVER.x + 1,
        max_y: BURN_LEVER.y + 1,
        floors: if burn { vec![-10] } else { vec![-10, -9] },
    };
    let bytes = oteryn_world_bundle::writer::write(&data.manifest, &sectors, &Default::default())?;
    pins.digest = bundle::read(&bytes)?.digest;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    crate::map::boot::boot(
        &bytes,
        &crate::map::boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start,
        },
        crate::foundation::WorldId::decode(&id(42)).map_err(debug)?,
        crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
        |_| None,
    )
    .map_err(debug)
}

pub(super) async fn exercise_switch(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    harness: &Harness,
) -> TestResult {
    let wall = placement_key(WALL, 1).ok_or("wall placement")?;
    let fence = admitted.item_fence.ok_or("Ground fence")?;
    let hidden = |owner: &ComposedFreshAdmission<'_, '_, '_>| -> TestResult<bool> {
        Ok(owner.bundle_map().ok_or("map")?.facts.base_hidden(wall))
    };
    for command in [1, 2] {
        let target = object_target(owner, LEVER, 1)?;
        assert_eq!(
            use_target(owner, admitted, command, &target).await?,
            UseDisposition::Committed
        );
        assert!(!hidden(owner)?);
    }
    quest_world_callbacks::input_item(
        owner,
        fence,
        &Offering {
            position: Position {
                x: PEDESTAL.x,
                y: PEDESTAL.y,
                floor: PEDESTAL.floor,
            },
            item: RING.into(),
            count: 1,
        },
        31,
        false,
    )
    .await?;
    let original: Vec<u8> = sqlx::query_scalar("SELECT uuid_send(item_instance_id) FROM game_item_instances WHERE definition_production_key=$1")
        .bind(RING).fetch_one(&harness.pool).await?;
    let target = object_target(owner, LEVER, 1)?;
    assert_eq!(
        use_target(owner, admitted, 3, &target).await?,
        UseDisposition::Committed
    );
    assert!(hidden(owner)?);
    let moved: (Vec<u8>, Vec<u8>) = sqlx::query_as("SELECT uuid_send(i.item_instance_id), g.spatial_position FROM game_item_instances i JOIN game_item_ground_locations g USING(item_instance_id) WHERE i.definition_production_key=$1")
        .bind(RING).fetch_one(&harness.pool).await?;
    assert_eq!(
        moved,
        (
            original,
            crate::durability::map_item_mint_audit::map_item_spatial_position(
                RECEIVER.x,
                RECEIVER.y,
                RECEIVER.floor
            )
        )
    );
    assert_eq!(
        use_target(owner, admitted, 3, &target).await?,
        UseDisposition::Committed
    );
    assert!(hidden(owner)?);
    Ok(())
}

pub(super) async fn exercise_burn(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    harness: &Harness,
) -> TestResult {
    let wall = placement_key(BURN_WALL, 1).ok_or("wall placement")?;
    let fence = admitted.item_fence.ok_or("Ground fence")?;
    quest_world_callbacks::input_item(
        owner,
        fence,
        &Offering {
            position: Position {
                x: RECEIVER.x,
                y: RECEIVER.y,
                floor: RECEIVER.floor,
            },
            item: RING.into(),
            count: 1,
        },
        31,
        false,
    )
    .await?;
    for command in [1, 2] {
        let target = object_target(owner, BURN_LEVER, 2)?;
        assert_eq!(
            use_target(owner, admitted, command, &target).await?,
            UseDisposition::Committed
        );
        assert!(!owner.bundle_map().ok_or("map")?.facts.base_hidden(wall));
    }
    let (digest, frame) = {
        let runtime = owner.runtime.lock().await;
        (
            runtime.content_pin().server_artifact_digest(),
            runtime.content_pin().frame_binding_digest(),
        )
    };
    let revision =
        crate::map::overlay::map_revision(owner.bundle_map().ok_or("map")?.overlay.base());
    super::outlaw_camp_rope::seed_barrel(harness, &revision, digest, frame, BARREL).await?;
    owner.states.loop_ground.invalidate();
    let target = object_target(owner, BURN_LEVER, 2)?;
    assert_eq!(
        use_target(owner, admitted, 3, &target).await?,
        UseDisposition::Committed
    );
    let map = owner.bundle_map().ok_or("map")?;
    assert!(map.facts.base_hidden(wall));
    let stone = map
        .overlay
        .base()
        .tile(STONE.x, STONE.y, STONE.floor)
        .ok_or("stone tile")?;
    assert_eq!(
        map.facts
            .base_entry(STONE, 1, stone.ids()[1])
            .map(|entry| entry.appearance_id),
        Some(1270)
    );
    for position in [RECEIVER, SECOND_FIRE] {
        assert_eq!(
            map.facts
                .movement_items(position)
                .iter()
                .filter(|entry| entry.facts.appearance_id == 2118)
                .count(),
            1
        );
    }
    let consumed: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_quest_callback_consumptions WHERE transition_key=$1",
    )
    .bind(format!("oteryn:quest-callback/{BURN_CASE}"))
    .fetch_one(&harness.pool)
    .await?;
    assert_eq!(consumed, 1);
    let active_roots: Vec<String> = sqlx::query_scalar("SELECT i.definition_production_key FROM game_item_instances i JOIN game_item_ground_locations g USING (item_instance_id) WHERE i.definition_production_key IN ($1, $2)")
        .bind(RING).bind("oteryn:item.tibia.i2523").fetch_all(&harness.pool).await?;
    assert_eq!(active_roots, ["oteryn:item.tibia.i2523"]);
    assert_eq!(
        use_target(owner, admitted, 3, &target).await?,
        UseDisposition::Committed
    );
    let map = owner.bundle_map().ok_or("map")?;
    let deadline = map.facts.next_movement_decay_time().ok_or("fire decay")?;
    for (time, expected) in [
        (deadline, Some(2119)),
        (deadline + 148_000_000, Some(2120)),
        (deadline + 246_000_000, None),
    ] {
        assert_eq!(map.facts.expire_movement_items(time), 1);
        assert_eq!(
            map.facts
                .quest_scene_field(RECEIVER)
                .map(|(_, facts)| facts.appearance_id),
            expected
        );
        assert_eq!(
            map.facts
                .quest_scene_field(SECOND_FIRE)
                .map(|(_, facts)| facts.appearance_id),
            Some(2118)
        );
    }
    Ok(())
}
