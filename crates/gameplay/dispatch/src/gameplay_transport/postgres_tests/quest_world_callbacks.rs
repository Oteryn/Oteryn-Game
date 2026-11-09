//! QUESTCB2: literal commands exercise the real callback, map, quest and item owners.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::foundation::{
    ChannelRuntimeV1, MessageType, MovementLocalPosition, decode_wire_envelope,
};
use crate::gameplay_transport::connection::{AdmittedSession, FirstEntryOutcome};
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
use std::collections::BTreeMap;

fn callback(name: &str) -> TestResult<Callback> {
    if name == "triangle-tower-lever" {
        // This source-equivalent adapter is deliberately absent from production:
        // the established Triangle callback owns the physical lever selector.
        let fixtures: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../tools/content-schema/quest-authoring/samples/qengine-access-test-fixtures.json"
        )))?;
        let row = fixtures["callbacks"]
            .as_array()
            .ok_or("access fixtures")?
            .iter()
            .find(|row| row["key"].as_str().is_some_and(|key| key.ends_with(name)))
            .ok_or("item-cycle fixture")?;
        return Ok(serde_json::from_value(row.clone())?);
    }
    oteryn_gameplay_quests::quest::world_callbacks::load()?
        .into_iter()
        .find(|c| c.key.ends_with(name))
        .ok_or_else(|| "callback".into())
}
pub(super) fn local(p: Position) -> MovementLocalPosition {
    MovementLocalPosition {
        x: i32::from(p.x),
        y: i32::from(p.y),
        floor: -i16::from(p.floor),
    }
}

async fn offering(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    fence: crate::durability::item_transfer::CurrentCharacterItemFence,
    offering: &oteryn_gameplay_quests::quest::world_callbacks::Offering,
    n: u64,
) -> TestResult {
    input_item(owner, fence, offering, n, false).await
}

pub(super) async fn input_item(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    fence: crate::durability::item_transfer::CurrentCharacterItemFence,
    offering: &oteryn_gameplay_quests::quest::world_callbacks::Offering,
    n: u64,
    carried: bool,
) -> TestResult {
    use crate::durability::{
        item_loop_move::{LoopDestination, LoopGround, LoopMoveRequest},
        item_mint::{
            GroundPlacement, ItemMintCause, ItemMintOutcome, ItemMintRequest, TypedDefinitionRef,
        },
        item_transfer::{ItemTransferDestination, ItemTransferRequest},
    };
    use crate::gameplay_transport::item_ref_admission::{NativeItemWeights, generation_item_facts};
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let definition = TypedDefinitionRef {
        family: "Item".into(),
        production_key: offering.item.clone(),
        revision_ref: "definition-r1".into(),
    };
    let facts = generation_item_facts(native, &definition).map_err(debug)?;
    let map = owner.bundle_map().ok_or("map")?;
    let (digest, frame) = {
        let r = owner.runtime.lock().await;
        (
            r.content_pin().server_artifact_digest(),
            r.content_pin().frame_binding_digest(),
        )
    };
    let revision = format!(
        "sha256:{}",
        digest
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    );
    let mut candidate = owner
        .root
        .freeze_item_mint(
            owner.character,
            owner.holder,
            ItemMintRequest {
                cause: ItemMintCause::for_test(
                    owner.world_id,
                    owner.channel_id,
                    fence.scope_ownership_generation,
                    1,
                    n,
                    TypedDefinitionRef {
                        family: "LootTable".into(),
                        production_key: "fixture:quest-callbacks".into(),
                        revision_ref: "definition-r1".into(),
                    },
                    "fixture:quest-callbacks".into(),
                    0,
                ),
                item: definition,
                quantity: u32::from(offering.count),
                ground: GroundPlacement {
                    spatial_position:
                        crate::durability::map_item_mint_audit::map_item_spatial_position(
                            offering.position.x,
                            offering.position.y,
                            offering.position.floor,
                        ),
                    corpse_ref: id(3).to_vec(),
                    map_revision: crate::map::overlay::map_revision(map.overlay.base()),
                    content_revision: revision.clone(),
                    native_room_placement_context: frame.to_vec(),
                },
                content_revision: "content-1".into(),
                ruleset_revision: "ruleset-1".into(),
                sim_revision: "sim-1".into(),
            },
        )
        .await
        .map_err(debug)?;
    let permit = owner.spell_lane_permit().await.ok_or("lane")?;
    let ItemMintOutcome::Committed(item) = owner
        .root
        .commit_item_mint(&permit, owner.character, owner.holder, &mut candidate)
        .await
        .map_err(debug)?
    else {
        return Err("mint".into());
    };
    let command = |offset| -> TestResult<_> {
        Ok(crate::foundation::CommandRef::new(
            fence.game_session_id,
            crate::foundation::CommandId::new(1000 + n * 10 + offset).map_err(debug)?,
        ))
    };
    let mut transfer = owner
        .root
        .freeze_item_transfer(
            owner.character,
            owner.holder,
            fence,
            ItemTransferRequest {
                weight_generation_digest: native.source_digest(),
                command: command(0)?,
                source_item_instance_id: item.item_instance_id,
                destination: ItemTransferDestination::MainBackpack,
                item: facts,
                backpack: Some(
                    generation_item_facts(
                        native,
                        &TypedDefinitionRef {
                            family: "Item".into(),
                            production_key: "oteryn:item.tibia.i2854".into(),
                            revision_ref: "definition-r1".into(),
                        },
                    )
                    .map_err(debug)?,
                ),
                content_revision: "content-1".into(),
                ruleset_revision: "ruleset-1".into(),
                sim_revision: "sim-1".into(),
            },
            &NativeItemWeights(native),
        )
        .await
        .map_err(debug)?;
    owner
        .root
        .commit_item_transfer(
            &permit,
            owner.character,
            owner.holder,
            fence,
            &mut transfer,
            &NativeItemWeights(native),
        )
        .await
        .map_err(debug)?;
    if carried {
        return Ok(());
    }
    let moved = owner
        .root
        .commit_item_loop_move(
            &permit,
            owner.character,
            owner.holder,
            &fence,
            command(1)?,
            &NativeItemWeights(native),
            LoopMoveRequest {
                item: item.item_instance_id,
                count: None,
                pickup_cell: None,
                destination: LoopDestination::Ground(LoopGround {
                    spatial_position:
                        crate::durability::map_item_mint_audit::map_item_spatial_position(
                            offering.position.x,
                            offering.position.y,
                            offering.position.floor,
                        ),
                    map_revision: crate::map::overlay::map_revision(map.overlay.base()),
                    content_revision: revision,
                    placement_context: frame.to_vec(),
                }),
            },
        )
        .await
        .map_err(debug)?;
    assert!(moved.is_ok(), "drop: {moved:?}");
    Ok(())
}

pub(super) fn world(
    callback: &Callback,
    trigger: Position,
) -> TestResult<crate::map::boot::BundleWorld> {
    use oteryn_world_bundle::{bundle, sector};
    let (bytes, mut pins) = crate::map::boot::tests::bundle();
    let mut data = bundle::read(&bytes)?;
    data.manifest.palette.truncate(1);
    let mut cells = BTreeMap::new();
    for x in trigger.x - 2..=trigger.x + 8 {
        for y in trigger.y - 1..=trigger.y + 1 {
            cells.insert((trigger.floor, x, y), Vec::new());
        }
    }
    if callback.key.starts_with("oteryn:quest-callback/unnatural-selection-dance-") {
        for (x, y, floor) in oteryn_gameplay_quests::quest::unnatural_selection::TILES {
            cells.entry((floor, x, y)).or_default();
        }
    }
    for position in callback
        .passages
        .iter()
        .map(|p| p.position)
        .chain(callback.offerings.iter().map(|p| p.position))
        .chain(callback.teleport)
    {
        cells
            .entry((position.floor, position.x, position.y))
            .or_default();
    }
    for passage in &callback.passages {
        if passage.replacement == Some(7767) {
            let p = passage.position;
            cells.entry((p.floor, p.x, p.y - 1)).or_default();
            cells.entry((p.floor - 1, p.x, p.y)).or_default();
        }
    }
    let mut effect_positions = Vec::new();
    if callback.key == oteryn_gameplay_quests::quest::dreamer_fire_bug::IGNITE {
        effect_positions.push(oteryn_gameplay_quests::quest::dreamer_fire_bug::DESTINATION);
        effect_positions.extend(oteryn_gameplay_quests::quest::dreamer_fire_bug::IGNITION_POSITIONS
            .into_iter().map(|(x,y)|Position{x,y,floor:-9}));
    }
    if let Some(pair) = &callback.effects.portals {
        for portal in &pair.portals {
            effect_positions.extend([portal.position, portal.destination]);
        }
    }
    if let Some(cycle) = &callback.effects.item_cycle {
        effect_positions.push(cycle.position);
    }
    if let Some(encounter) = &callback.effects.encounter {
        for player in &encounter.players {
            effect_positions.extend([player.from, player.to]);
        }
        effect_positions.extend(encounter.monsters.iter().map(|m| m.position));
    }
    for p in effect_positions {
        for x in p.x - 1..=p.x + 1 {
            for y in p.y - 1..=p.y + 1 {
                cells.entry((p.floor, x, y)).or_default();
            }
        }
    }
    if callback.key == "oteryn:quest-callback/l6-carried-step-fixture"
        && let Some(recipe) = oteryn_gameplay_quests::quest::carried_step::RECIPES
            .iter()
            .find(|recipe| Some(recipe.unique) == callback.selector.unique)
    {
        for p in [recipe.success, recipe.missing] {
            cells.entry((p.floor, p.x, p.y)).or_default();
        }
    }
    for passage in &callback.passages {
        if passage.replacement == Some(7767) {
            let p = passage.position;
            cells.entry((p.floor, p.x, p.y - 1)).or_default();
            cells.entry((p.floor - 1, p.x, p.y)).or_default();
        }
    }
    if let Some(presence) = &callback.effects.item_presence {
        let p = presence.position;
        cells.entry((p.floor, p.x, p.y)).or_default();
        cells.entry((p.floor + 1, p.x, p.y + 1)).or_default();
    }
    let mut effect_positions = Vec::new();
    if let Some(pair) = &callback.effects.portals {
        for portal in &pair.portals {
            effect_positions.extend([portal.position, portal.destination]);
        }
    }
    if let Some(cycle) = &callback.effects.item_cycle {
        effect_positions.push(cycle.position);
    }
    if let Some(encounter) = &callback.effects.encounter {
        for player in &encounter.players {
            effect_positions.extend([player.from, player.to]);
        }
        effect_positions.extend(encounter.monsters.iter().map(|m| m.position));
    }
    for p in effect_positions {
        for x in p.x - 1..=p.x + 1 {
            for y in p.y - 1..=p.y + 1 {
                cells.entry((p.floor, x, y)).or_default();
            }
        }
    }
    let appearance = callback.selector.appearance.unwrap_or(100);
    data.manifest.palette.push(bundle::PaletteEntry {
        key: format!("oteryn:terrain.tibia.i{appearance}"),
        family: bundle::Family::Terrain,
        id: 1,
        terrain: Some(bundle::Terrain {
            kind: bundle::TerrainKind::Common,
            walkable: None,
            ground_speed: None,
            blocks_projectile: None,
        }),
    });
    if let Some(appearance) = callback
        .item_on_target
        .as_ref()
        .and_then(|i| i.target_replacement)
    {
        data.manifest.palette.push(bundle::PaletteEntry {
            key: format!("oteryn:terrain.tibia.i{appearance}"),
            family: bundle::Family::Terrain,
            id: u32::try_from(data.manifest.palette.len())?,
            terrain: Some(bundle::Terrain {
                kind: bundle::TerrainKind::Common,
                walkable: None,
                ground_speed: None,
                blocks_projectile: None,
            }),
        });
    }
    cells
        .entry((trigger.floor, trigger.x, trigger.y))
        .or_default()
        .push(sector::Item {
            palette: 1,
            depth: 0,
            attrs: sector::Attrs {
                unique: if callback.key == oteryn_gameplay_quests::quest::dreamer_fire_bug::IGNITE {
                    None // Retained B3 has the bare ashes at the source's literal position.
                } else { callback.selector.unique },
                action: callback.selector.action,
                ..Default::default()
            },
        });
    if callback.key == oteryn_gameplay_quests::quest::dreamer_death_ring::RESET {
        // Synthetic literal UID2244; no production-map position correspondence is claimed.
        cells.entry((trigger.floor,trigger.x-1,trigger.y+1)).or_default().push(sector::Item {
            palette:1,depth:0,attrs:sector::Attrs {unique:Some(2244),..Default::default()}
        });
    }
    if callback.key == "oteryn:quest-callback/l6-shared-gate-fixture" {
        let (action, _, _) =
            super::l6_shared_gates::spec(callback.selector.unique.ok_or("gate UID")?)?;
        for x in trigger.x - 2..=trigger.x + 18 {
            for y in trigger.y - 1..=trigger.y + 1 {
                cells.entry((trigger.floor, x, y)).or_default();
            }
        }
        let off = u32::try_from(data.manifest.palette.len())?;
        for appearance in [2772, 2773] {
            let index = u32::try_from(data.manifest.palette.len())?;
            data.manifest.palette.push(bundle::PaletteEntry {
                key: format!("oteryn:terrain.tibia.i{appearance}"),
                family: bundle::Family::Terrain,
                id: index,
                terrain: Some(bundle::Terrain {
                    kind: bundle::TerrainKind::Common,
                    walkable: None,
                    ground_speed: None,
                    blocks_projectile: None,
                }),
            });
        }
        for n in 0..8 {
            cells
                .entry((trigger.floor, trigger.x + 1 + 2 * n, trigger.y + 1))
                .or_default()
                .push(sector::Item {
                    palette: off,
                    depth: 0,
                    attrs: sector::Attrs {
                        action: Some(action),
                        ..Default::default()
                    },
                });
        }
    }
    if callback.key == "oteryn:quest-callback/l6-instrument-fixture" {
        // A bounded open-room fixture retains the actual B3 instrument positions.
        // It qualifies the selector and transaction, not the tomb's collision path.
        for x in 33186..=33193 {
            for y in 32659..=32670 {
                cells.entry((-15, x, y)).or_default();
            }
        }
        for (appearance, position) in super::l6_ancient_tombs::INSTRUMENTS.into_iter().skip(1) {
            let palette = u32::try_from(data.manifest.palette.len())?;
            data.manifest.palette.push(bundle::PaletteEntry {
                key: format!("oteryn:terrain.tibia.i{appearance}"),
                family: bundle::Family::Terrain,
                id: palette,
                terrain: Some(bundle::Terrain {
                    kind: bundle::TerrainKind::Common,
                    walkable: None,
                    ground_speed: None,
                    blocks_projectile: None,
                }),
            });
            cells
                .entry((position.floor, position.x, position.y))
                .or_default()
                .push(sector::Item {
                    palette,
                    depth: 0,
                    attrs: Default::default(),
                });
        }
    }
    if callback.key == "oteryn:quest-callback/monk-guidance" {
        for shrine in oteryn_gameplay_quests::quest::world_callbacks::load()?
            .into_iter()
            .filter(|c| c.quest == callback.quest && c.key != callback.key)
        {
            let p = shrine.selector.position.ok_or("shrine position")?;
            for x in p.x - 2..=p.x + 1 {
                for y in p.y - 1..=p.y + 1 {
                    cells.entry((p.floor, x, y)).or_default();
                }
            }
            cells
                .entry((p.floor, p.x, p.y))
                .or_default()
                .push(sector::Item {
                    palette: 1,
                    depth: 0,
                    attrs: Default::default(),
                });
        }
    }
    if let Some(pair) = callback.lever {
        for appearance in pair {
            let key = format!("oteryn:terrain.tibia.i{appearance}");
            if !data.manifest.palette.iter().any(|p| p.key == key) {
                let id = u32::try_from(data.manifest.palette.len())?;
                data.manifest.palette.push(bundle::PaletteEntry {
                    key,
                    family: bundle::Family::Terrain,
                    id,
                    terrain: Some(bundle::Terrain {
                        kind: bundle::TerrainKind::Common,
                        walkable: None,
                        ground_speed: None,
                        blocks_projectile: None,
                    }),
                });
            }
        }
    }
    for passage in &callback.passages {
        let appearance_key = format!("oteryn:terrain.tibia.i{}", passage.appearance);
        let index = if let Some(i) = data
            .manifest
            .palette
            .iter()
            .position(|p| p.key == appearance_key)
        {
            u32::try_from(i)?
        } else {
            let index = u32::try_from(data.manifest.palette.len())?;
            data.manifest.palette.push(bundle::PaletteEntry {
                key: appearance_key,
                family: bundle::Family::Terrain,
                id: index,
                terrain: Some(bundle::Terrain {
                    kind: bundle::TerrainKind::Wall,
                    walkable: None,
                    ground_speed: None,
                    blocks_projectile: None,
                }),
            });
            index
        };
        if let Some(replacement) = passage.replacement.or(passage.create_on_close) {
            let replacement_key = format!("oteryn:terrain.tibia.i{replacement}");
            if !data
                .manifest
                .palette
                .iter()
                .any(|p| p.key == replacement_key)
            {
                let id = u32::try_from(data.manifest.palette.len())?;
                data.manifest.palette.push(bundle::PaletteEntry {
                    key: replacement_key,
                    family: bundle::Family::Terrain,
                    id,
                    terrain: Some(bundle::Terrain {
                        kind: if passage.create_on_close.is_some() {
                            bundle::TerrainKind::Wall
                        } else {
                            bundle::TerrainKind::Ground
                        },
                        walkable: if passage.create_on_close.is_some() {
                            None
                        } else {
                            Some(true)
                        },
                        ground_speed: if passage.create_on_close.is_some() {
                            None
                        } else {
                            Some(150)
                        },
                        blocks_projectile: None,
                    }),
                });
            }
        }
        cells
            .entry((
                passage.position.floor,
                passage.position.x,
                passage.position.y,
            ))
            .or_default()
            .push(sector::Item {
                palette: index,
                depth: 0,
                attrs: Default::default(),
            });
    }
    let mut effect_appearances = Vec::new();
    effect_appearances.extend(callback.effects.transform);
    if callback.effects.transform.is_some() {
        effect_appearances.extend([430, 431]);
    }
    if let Some(pair) = &callback.effects.portals {
        effect_appearances.push(pair.appearance);
    }
    if let Some(presence) = &callback.effects.item_presence {
        effect_appearances.push(presence.appearance);
    }
    if let Some(cycle) = &callback.effects.item_cycle {
        effect_appearances.extend([cycle.remove, cycle.create]);
    }
    for appearance in effect_appearances {
        let key = format!("oteryn:terrain.tibia.i{appearance}");
        if data.manifest.palette.iter().any(|entry| entry.key == key) {
            continue;
        }
        let id = u32::try_from(data.manifest.palette.len())?;
        data.manifest.palette.push(bundle::PaletteEntry {
            key,
            family: bundle::Family::Terrain,
            id,
            terrain: Some(bundle::Terrain {
                kind: if [1270, 1025].contains(&appearance) {
                    bundle::TerrainKind::Wall
                } else {
                    bundle::TerrainKind::Common
                },
                walkable: None,
                ground_speed: None,
                blocks_projectile: None,
            }),
        });
    }
    if let Some(cycle) = &callback.effects.item_cycle {
        let index = data
            .manifest
            .palette
            .iter()
            .position(|p| p.key == format!("oteryn:terrain.tibia.i{}", cycle.remove))
            .ok_or("cycle palette")?;
        let p = cycle.position;
        cells
            .entry((p.floor, p.x, p.y))
            .or_default()
            .push(sector::Item {
                palette: u32::try_from(index)?,
                depth: 0,
                attrs: Default::default(),
            });
    }
    if callback.key == "oteryn:quest-callback/triangle-tower-lever" {
        // Preserve the native basement exit rather than authoring a second callback.
        let region = oteryn_world_bundle_compiler::b3::decode_region(
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/placements/region-z08-x127-y125.b3"
            )),
            bundle::TILE_LIMITS,
            &mut sector::Budget {
                tiles: 65536,
                entries: 1_000_000,
            },
        )?;
        let source = region
            .sectors
            .iter()
            .flat_map(|(_, _, tiles)| tiles)
            .find(|t| t.x == 32566 && t.y == 32120)
            .ok_or("basement exit")?;
        let item = source
            .items
            .iter()
            .find(|i| i.depth == 0 && i.attrs.teleport.is_some())
            .ok_or("native exit teleport")?;
        let destination = item.attrs.teleport.ok_or("destination")?;
        assert_eq!(destination, (32569, 32110, 7));
        let palette = u32::try_from(data.manifest.palette.len())?;
        data.manifest.palette.push(bundle::PaletteEntry {
            key: "oteryn:terrain.tibia.i1949".into(),
            family: bundle::Family::Terrain,
            id: palette,
            terrain: Some(bundle::Terrain {
                kind: bundle::TerrainKind::Common,
                walkable: None,
                ground_speed: None,
                blocks_projectile: None,
            }),
        });
        cells
            .entry((-8, 32566, 32120))
            .or_default()
            .push(sector::Item {
                palette,
                depth: 0,
                attrs: {
                    let mut attrs = item.attrs.clone();
                    // B3 stores legacy z; bundles store the native signed floor byte.
                    attrs.teleport = Some((
                        destination.0,
                        destination.1,
                        (-i8::try_from(destination.2)?) as u8,
                    ));
                    attrs
                },
            });
        cells.entry((-8, 32565, 32120)).or_default();
        cells
            .entry((-i8::try_from(destination.2)?, destination.0, destination.1))
            .or_default();
    }
    let floors: std::collections::BTreeSet<_> = cells.keys().map(|(f, _, _)| *f).collect();
    data.manifest.world = bundle::Extent {
        min_x: cells.keys().map(|(_, x, _)| *x).min().ok_or("min")?,
        min_y: cells.keys().map(|(_, _, y)| *y).min().ok_or("min")?,
        max_x: cells.keys().map(|(_, x, _)| *x).max().ok_or("max")? + 1,
        max_y: cells.keys().map(|(_, _, y)| *y).max().ok_or("max")? + 1,
        floors: floors.into_iter().collect(),
    };
    let mut sectors: BTreeMap<(i8, u16, u16), Vec<sector::Tile>> = BTreeMap::new();
    for ((floor, x, y), mut items) in cells {
        items.insert(
            0,
            sector::Item {
                palette: 0,
                depth: 0,
                attrs: Default::default(),
            },
        );
        sectors
            .entry((floor, x / 32, y / 32))
            .or_default()
            .push(sector::Tile {
                x,
                y,
                flags: 0,
                house: 0,
                zones: vec![],
                items,
            });
    }
    let mut sectors = sectors
        .into_iter()
        .map(|((floor, sx, sy), mut tiles)| {
            tiles.sort_by_key(|t| (t.y, t.x));
            bundle::Sector {
                floor,
                sx,
                sy,
                tiles,
            }
        })
        .collect::<Vec<_>>();
    sectors.sort_by_key(bundle::order);
    let bytes = oteryn_world_bundle::writer::write(&data.manifest, &sectors, &Default::default())?;
    pins.digest = bundle::read(&bytes)?.digest;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    crate::map::boot::boot(
        &bytes,
        &crate::map::boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start: crate::map::overlay::TilePos {
                x: trigger.x - 1,
                y: trigger.y,
                floor: trigger.floor,
            },
        },
        crate::foundation::WorldId::decode(&id(42)).map_err(debug)?,
        crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
        |_| None,
    )
    .map_err(debug)
}

pub(super) async fn wire(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    id: u64,
    kind: u32,
    payload: Vec<u8>,
) -> TestResult<Vec<u8>> {
    use crate::gameplay_transport::connection::{ConnectionEnd, IDLE_LIVENESS, serve_admitted};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let frame = oteryn_protocol_oteryn::encode_client_command(
        1,
        &oteryn_protocol_oteryn::ClientCommandValue {
            command_id: id,
            command_type: kind,
            payload: &payload,
        },
    )
    .map_err(debug)?;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    client
        .write_all(&u32::try_from(frame.len())?.to_be_bytes())
        .await?;
    client.write_all(&frame).await?;
    // Keep the transport alive while a paced command is buffered. Closing its write half
    // before the result correctly disconnects a real client before the queued step runs.
    let receiving = async move {
        let mut result = None;
        loop {
            let mut prefix = [0; 4];
            match client.read_exact(&mut prefix).await {
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(error) => return Err(error.into()),
            }
            let mut frame = vec![0; u32::from_be_bytes(prefix) as usize];
            client.read_exact(&mut frame).await?;
            let envelope = decode_wire_envelope(&frame).map_err(debug)?;
            if envelope.message_type() == MessageType::CommandResult {
                let command = oteryn_protocol_oteryn::decode_command_result(envelope.payload())
                    .map_err(debug)?;
                assert_eq!(command.command_id, id);
                result = Some(command.payload.to_vec());
                client.shutdown().await?;
            }
        }
        result.ok_or_else(|| "no command result".into())
    };
    let reader = tokio::spawn(async move {
        let result: TestResult<_> = receiving.await;
        result.map_err(|error| error.to_string())
    });
    let end = serve_admitted(&mut server, *admitted, owner, IDLE_LIVENESS).await;
    if let ConnectionEnd::AdmittedThenClosed(_, error) = &end {
        eprintln!("quest wire command={id} type={kind} closed={error:?}");
    }
    drop(server);
    let result = reader.await?.map_err(Into::into);
    if let ConnectionEnd::AdmittedThenDisconnected(a) | ConnectionEnd::AdmittedThenClosed(a, _) =
        end
    {
        *admitted = a;
    }
    result
}

#[path = "quest_world_callbacks/actions.rs"]
mod actions;
#[path = "quest_world_callbacks/assertions.rs"]
mod assertions;
#[path = "quest_world_callbacks/lions_rock.rs"]
mod lions_rock;
#[path = "quest_world_callbacks/rotten_blood.rs"]
mod rotten_blood;
#[path = "quest_world_callbacks/scenario.rs"]
mod scenario;
#[path = "quest_world_callbacks/stateless.rs"]
mod stateless;
#[path = "quest_world_callbacks/unnatural_selection.rs"]
mod unnatural_selection;
pub(super) use scenario::scenario;
#[allow(
    unused_imports,
    reason = "per-quest stateless PG cases are independently picked after the common harness"
)]
pub(super) use scenario::stateless_scenario;
