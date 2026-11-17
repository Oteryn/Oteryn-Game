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

pub(super) fn world() -> TestResult<crate::map::boot::BundleWorld> {
    use oteryn_world_bundle::{bundle, sector};
    let (bytes, mut pins) = crate::map::boot::tests::bundle();
    let mut data = bundle::read(&bytes)?;
    data.manifest.palette.truncate(1);
    data.manifest.palette[0].key = "oteryn:terrain.tibia.i351".into();
    for (id, appearance, kind) in [
        (1, 2772, bundle::TerrainKind::Common),
        (2, 1271, bundle::TerrainKind::Wall),
        (3, 2321, bundle::TerrainKind::Common),
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
    for (pos, extra) in [
        (START, None),
        (LEVER, Some(1)),
        (PEDESTAL, Some(3)),
        (WALL, Some(2)),
        (RECEIVER, Some(3)),
    ] {
        let mut items = vec![item(0)];
        items.extend(extra.map(item));
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
        .map(|((floor, sy, sx), tiles)| bundle::Sector {
            floor,
            sx,
            sy,
            tiles,
        })
        .collect();
    data.manifest.world = bundle::Extent {
        min_x: START.x,
        min_y: LEVER.y,
        max_x: RECEIVER.x + 1,
        max_y: RECEIVER.y + 1,
        floors: vec![-10, -9],
    };
    let bytes = oteryn_world_bundle::writer::write(&data.manifest, &sectors, &Default::default())?;
    pins.digest = bundle::read(&bytes)?.digest;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    crate::map::boot::boot(
        &bytes,
        &crate::map::boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start: START,
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
