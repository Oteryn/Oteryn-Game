use crate::foundation::{ChannelId, ChannelRuntimeV1, NodeId, WorldId};
use crate::map::boot::{self, BootPins};
use crate::map::overlay::TilePos;
use crate::node::serve::{
    bundle_npcs::{NpcBoot, TimedNpcSpawns, place},
    daily_world_changes::bind_layers,
};
use oteryn_gameplay_entities::{MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements};
use oteryn_gameplay_world_change::{
    daily::DailyWorldChanges,
    seasonal::{KINGSDAY, LAST_CREEP, SPRING, at_save},
};
use oteryn_world_bundle::{bundle, layers::Layer, writer};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn id(seed: u8) -> [u8; 16] {
    [
        seed, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, seed,
    ]
}

#[test]
fn seasonal_occurrences_compose_spring_and_kingsday_layers_only_on_active_saves() -> TestResult {
    let world = WorldId::decode(&id(42))?;
    let channel = ChannelId::decode(&id(43))?;
    let (bytes, mut pins) = boot::tests::bundle();
    let mut data = bundle::read(&bytes)?;
    let sector = data
        .sectors
        .iter()
        .find(|sector| sector.floor == -7)
        .ok_or("fixture floor")?;
    for (key, x) in [(KINGSDAY, 1), (SPRING, 2)] {
        let mut tile = sector
            .tiles
            .iter()
            .find(|tile| tile.x == x && tile.y == 0)
            .ok_or("fixture layer tile")?
            .clone();
        tile.flags ^= 1;
        data.layers.push(Layer {
            key: key.into(),
            palette: data.manifest.palette.clone(),
            sectors: vec![bundle::Sector {
                floor: sector.floor,
                sx: sector.sx,
                sy: sector.sy,
                tiles: vec![tile],
            }],
        });
    }
    let bytes = writer::write_with_layers(
        &data.manifest,
        &data.sectors,
        &data.spawns,
        &data.npcs,
        &data.layers,
    )?;
    pins.digest.copy_from_slice(&bytes[bytes.len() - 32..]);
    let pins = BootPins {
        map_revision: format!(
            "sha256:{}",
            pins.digest
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        ),
        bundle: pins,
        start: TilePos {
            x: 3,
            y: 0,
            floor: -7,
        },
    };
    let boot_at = |epoch, month, day| {
        let checked = boot::check(bytes.clone(), pins.clone())?;
        let daily = DailyWorldChanges::roll(epoch, month, day, |_| u64::MAX);
        let occurrences = at_save(world.as_bytes(), epoch, 2026, month, day);
        bind_layers(checked, &daily, &occurrences)?.boot(world, channel, boot::tests::items)
    };
    let active = boot_at(21, 4, 16)?;
    let closed = boot_at(22, 5, 2)?;
    assert_eq!(active.base().active_layers(), &[KINGSDAY, SPRING]);
    assert!(closed.base().active_layers().is_empty());
    for x in [1, 2] {
        let during = active.base().tile(x, 0, -7).ok_or("active layer tile")?;
        let after = closed.base().tile(x, 0, -7).ok_or("closed layer tile")?;
        assert_ne!(during.protection_zone(), after.protection_zone());
    }
    Ok(())
}

#[test]
fn seasonal_occurrences_place_casper_once_and_leave_offseason_channels_empty() -> TestResult {
    let world = WorldId::decode(&id(42))?;
    let channel = ChannelId::decode(&id(43))?;
    let node = NodeId::decode(&id(1))?;
    for (day, count, authored) in [(20, 1, false), (25, 0, false), (20, 1, true), (25, 0, true)] {
        // The retained Casper source gives Tibia z=7. The bundle stores -z,
        // while runtime actors report the original z on both placement paths.
        let (x, y, z) = (if authored { 33262 } else { 33261 }, 32363, 7);
        let position = TilePos { x, y, floor: -z };
        let placement = NpcPlacements {
            key: "oteryn:npc.casper".to_owned(),
            placements: vec![NpcPlacement {
                x: position.x,
                y: position.y,
                floor: position.floor,
                direction: MovementFacing::South,
            }],
        };
        let boot = NpcBoot::Place {
            table: NpcPlacementTable {
                npcs: authored.then_some(placement).into_iter().collect(),
            },
            catalogue_sha256: String::new(),
            admitted: ["oteryn:npc.casper".to_owned()].into(),
            production: true,
            talk: Default::default(),
        };
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world,
            channel,
            node,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            8,
            oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin::test(world),
        )
        .map_err(|error| format!("{error:?}"))?;
        let events = at_save(world.as_bytes(), 2, 2026, 5, day);
        assert_eq!(
            events.iter().any(|event| event.key == LAST_CREEP),
            count == 1
        );
        let keys = events
            .into_iter()
            .map(|event| event.key)
            .collect::<Vec<_>>();
        place(&mut runtime, Some(&boot), &keys).map_err(|error| format!("{error:?}"))?;
        let mut spawns =
            TimedNpcSpawns::from_boot(Some(&boot)).with_seasonal(Some(&boot), &keys, |tile| {
                tile == TilePos {
                    x: 33261,
                    y: 32363,
                    floor: -7,
                }
            });
        for _ in 0..2 {
            spawns
                .tick(
                    &mut runtime,
                    crate::world_clock::DEFAULT_WORLD_CLOCK_EPOCH_UTC_MS,
                )
                .ok_or("timed NPC clock")?
                .map_err(|error| format!("{error:?}"))?;
        }
        let visible = runtime.visible_entities();
        assert_eq!(visible.npcs.len(), count);
        for npc in visible.npcs {
            assert_eq!(runtime.npc_key(npc.actor), Some("oteryn:npc.casper"));
            assert_eq!(
                (npc.position.x, npc.position.y, npc.position.floor),
                (
                    i32::from(position.x),
                    i32::from(position.y),
                    i16::from(z)
                )
            );
        }
    }
    Ok(())
}
