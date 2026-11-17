//! The native Outlaw Camp oven through the world-object USE owner.
use super::fixture::{debug, id};
use super::questpc8_authored_programmes::{object_target, use_target};
use super::{TestResult, quest_world_callbacks};
use crate::content::{CollisionClass, LogicalCell};
use crate::gameplay_transport::{ComposedFreshAdmission, connection::AdmittedSession};
use crate::map::{
    overlay::TilePos,
    view::{MapFacts, placement_key},
};
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::world_object::UseDisposition;

const LEVER: TilePos = TilePos {
    x: 32614,
    y: 32173,
    floor: -9,
};
const START: TilePos = TilePos {
    x: LEVER.x - 1,
    ..LEVER
};
pub(super) const CASE: &str = "authored-outlaw-camp-oven";
pub(super) const LIT_CASE: &str = "authored-outlaw-camp-oven-lit";
const NORTH: TilePos = TilePos {
    x: 32623,
    y: 32188,
    floor: -9,
};
const SOUTH: TilePos = TilePos { y: 32189, ..NORTH };

#[test]
fn outlaw_camp_oven_use_moves_and_restores_native_oven_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_case(CASE, admin)
}

#[test]
fn outlaw_camp_oven_use_preserves_lit_appearance_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_case(LIT_CASE, admin)
}

fn run_case(name: &'static str, admin: String) -> TestResult {
    super::kill_reward_e2e::scenario_thread::run(name, move || {
        quest_world_callbacks::scenario_with_admin(
            name,
            Position {
                x: LEVER.x,
                y: LEVER.y,
                floor: LEVER.floor,
            },
            admin,
        )
    })
}

pub(super) fn world(appearance: u16) -> TestResult<crate::map::boot::BundleWorld> {
    use oteryn_world_bundle::{bundle, sector};
    let (bytes, mut pins) = crate::map::boot::tests::bundle();
    let mut data = bundle::read(&bytes)?;
    data.manifest.palette.truncate(1);
    data.manifest.palette[0].key = "oteryn:terrain.tibia.i351".into();
    for (id, appearance, kind) in [
        (1, 2772, bundle::TerrainKind::Common),
        (2, appearance, bundle::TerrainKind::Wall),
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
    let mut grouped = std::collections::BTreeMap::<(u16, u16), Vec<sector::Tile>>::new();
    for (pos, extra) in [
        (START, None),
        (LEVER, Some(1)),
        (NORTH, Some(2)),
        (SOUTH, None),
    ] {
        let mut items = vec![item(0)];
        items.extend(extra.map(item));
        grouped
            .entry((pos.y / 32, pos.x / 32))
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
        .map(|((sy, sx), tiles)| bundle::Sector {
            floor: LEVER.floor,
            sx,
            sy,
            tiles,
        })
        .collect();
    data.manifest.world = bundle::Extent {
        min_x: START.x,
        min_y: 32173,
        max_x: 32624,
        max_y: 32190,
        floors: vec![-9],
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

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    appearance: u16,
) -> TestResult {
    let original = placement_key(NORTH, 1).ok_or("oven placement")?;
    let assert_oven = |owner: &ComposedFreshAdmission<'_, '_, '_>, moved| -> TestResult {
        let map = owner.bundle_map().ok_or("map")?;
        assert_eq!(map.facts.base_hidden(original), moved);
        let cells = owner.movement_cells();
        for (pos, blocked) in [(NORTH, !moved), (SOUTH, moved)] {
            assert_eq!(
                cells.index().lookup(
                    cells.scope(),
                    LogicalCell {
                        x: i32::from(pos.x),
                        y: i32::from(pos.y),
                        z: -i32::from(pos.floor)
                    },
                ),
                Ok(if blocked {
                    CollisionClass::Blocked
                } else {
                    CollisionClass::Walkable
                }),
            );
        }
        assert_eq!(
            map.facts
                .movement_items(SOUTH)
                .into_iter()
                .filter(|entry| entry.facts.appearance_id == appearance)
                .count(),
            usize::from(moved),
        );
        Ok(())
    };
    assert_oven(owner, false)?;
    for (command, moved) in [(1, true), (2, false)] {
        let lever = object_target(owner, LEVER, 1)?;
        assert_eq!(
            use_target(owner, admitted, command, &lever).await?,
            UseDisposition::Committed,
        );
        assert_oven(owner, moved)?;
    }
    Ok(())
}
