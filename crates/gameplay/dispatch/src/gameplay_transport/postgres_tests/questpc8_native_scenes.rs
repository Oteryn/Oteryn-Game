//! Source-literal scene edits through the normal PostgreSQL-backed USE route.
use super::fixture::{debug, id};
use super::{TestResult, quest_world_callbacks};
use crate::gameplay_transport::{ComposedFreshAdmission, connection::AdmittedSession};
use crate::map::{overlay::TilePos, view::MapFacts};
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::world_object::{
    UseDisposition, WorldObjectTarget, decode_use_result, encode_use_intent,
};

const LEVER: TilePos = TilePos {
    x: 32600,
    y: 32105,
    floor: -14,
};

#[test]
fn questpc8_mintwallin_lever_edits_all_walls_once_and_restores_with_floor() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("questpc8-mintwallin-scene", || {
        quest_world_callbacks::scenario(
            "mintwallin-cyclops-scene",
            Position {
                x: LEVER.x,
                y: LEVER.y,
                floor: LEVER.floor,
            },
            false,
        )
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED questpc8_mintwallin_scene duration={:?}",
        started.elapsed()
    );
    Ok(())
}

/// The literal source room, including prototypes for both created walls.
/// Rebuilding this source floor uses the production bundle boot boundary;
/// there is no invented reset timer or persistent quest flag for the lever.
pub(super) fn world() -> TestResult<crate::map::boot::BundleWorld> {
    use oteryn_world_bundle::{bundle, sector};
    let (bytes, mut pins) = crate::map::boot::tests::bundle();
    let mut data = bundle::read(&bytes)?;
    data.manifest.palette.truncate(1);
    for (index, appearance) in [2772, 1271, 1270].into_iter().enumerate() {
        data.manifest.palette.push(bundle::PaletteEntry {
            key: format!("oteryn:terrain.tibia.i{appearance}"),
            family: bundle::Family::Terrain,
            id: u32::try_from(index + 1)?,
            terrain: Some(bundle::Terrain {
                blocks_projectile: None,
                kind: if appearance == 2772 {
                    bundle::TerrainKind::Common
                } else {
                    bundle::TerrainKind::Wall
                },
                walkable: None,
                ground_speed: None,
            }),
        });
    }
    let item = |palette| sector::Item {
        palette,
        depth: 0,
        attrs: Default::default(),
    };
    let mut tiles = Vec::new();
    for y in 32103..=32107 {
        for x in 32591..=32602 {
            let mut items = vec![item(0)];
            if (x, y) == (LEVER.x, LEVER.y) {
                items.push(item(1));
            }
            if y == 32104 && (32593..=32601).contains(&x) {
                items.push(item(2));
            }
            tiles.push(sector::Tile {
                x,
                y,
                flags: 0,
                house: 0,
                zones: Vec::new(),
                items,
            });
        }
    }
    let mut grouped = std::collections::BTreeMap::new();
    for tile in tiles {
        grouped
            .entry((tile.x / 32, tile.y / 32))
            .or_insert_with(Vec::new)
            .push(tile);
    }
    let sectors: Vec<_> = grouped
        .into_iter()
        .map(|((sx, sy), tiles)| bundle::Sector {
            floor: LEVER.floor,
            sx,
            sy,
            tiles,
        })
        .collect();
    data.manifest.world = bundle::Extent {
        min_x: 32591,
        min_y: 32103,
        max_x: 32603,
        max_y: 32108,
        floors: vec![LEVER.floor],
    };
    let bytes = oteryn_world_bundle::writer::write(&data.manifest, &sectors, &Default::default())?;
    pins.digest = bundle::read(&bytes)?.digest;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    crate::map::boot::boot(
        &bytes,
        &crate::map::boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start: TilePos {
                x: LEVER.x - 1,
                ..LEVER
            },
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
    world: &crate::map::boot::BundleWorld,
) -> TestResult {
    let key = crate::map::view::placement_key(LEVER, 1).ok_or("lever")?;
    let mut placement = world.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let target = WorldObjectTarget {
        placement,
        expected_revision: 0,
    };
    let stale = WorldObjectTarget {
        expected_revision: 1,
        ..target.clone()
    };
    assert_eq!(
        use_target(owner, admitted, 1, &stale).await?,
        UseDisposition::StaleState
    );
    assert!(!world.facts().base_hidden(key));
    assert_eq!(
        use_target(owner, admitted, 2, &target).await?,
        UseDisposition::Committed
    );
    let map = owner.bundle_map().ok_or("map")?;
    assert!(
        map.facts.base_hidden(key),
        "the source removes its own lever"
    );
    for x in 32593..=32601 {
        let at = TilePos {
            x,
            y: 32104,
            floor: -14,
        };
        let wall = crate::map::view::placement_key(at, 1).ok_or("wall")?;
        assert!(
            map.facts.base_hidden(wall),
            "source wall {x} was not removed"
        );
        assert!(
            world.enterable(at),
            "removed wall must also release collision"
        );
    }
    for y in [32105, 32106] {
        let at = TilePos {
            x: 32592,
            y,
            floor: -14,
        };
        assert_eq!(
            map.facts
                .movement_items(at)
                .iter()
                .filter(|e| e.facts.appearance_id == 1270)
                .count(),
            1
        );
        assert!(!world.enterable(at), "created wall must block movement");
    }
    assert_eq!(
        use_target(owner, admitted, 2, &target).await?,
        UseDisposition::Committed
    );
    assert_ne!(
        use_target(owner, admitted, 3, &target).await?,
        UseDisposition::Committed
    );
    let reset = self::world()?;
    assert!(!reset.facts().base_hidden(key));
    for x in 32593..=32601 {
        assert!(!reset.enterable(TilePos {
            x,
            y: 32104,
            floor: -14
        }));
    }
    for y in [32105, 32106] {
        assert!(reset.enterable(TilePos {
            x: 32592,
            y,
            floor: -14
        }));
    }
    Ok(())
}

async fn use_target(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    command: u64,
    target: &WorldObjectTarget,
) -> TestResult<UseDisposition> {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let result = quest_world_callbacks::wire(
        owner,
        admitted,
        command,
        2,
        encode_use_intent(target).map_err(debug)?,
    )
    .await?;
    decode_use_result(&result).map_err(debug)
}
