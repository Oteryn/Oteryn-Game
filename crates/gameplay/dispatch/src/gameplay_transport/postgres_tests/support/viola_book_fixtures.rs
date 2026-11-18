use super::fixture::{debug, id};

use super::TestResult;

use crate::durability::item_mint::TypedDefinitionRef;

use crate::foundation::{CommandId, CommandRef};

use crate::gameplay_transport::item_ref_admission::NativeItemWeights;

use oteryn_protocol_oteryn::world_object::WorldObjectTarget;

pub(super) fn definition(id: u16) -> TypedDefinitionRef {
    TypedDefinitionRef {
        family: "Item".into(),
        production_key: format!("oteryn:item.tibia.i{id}"),
        revision_ref: "definition-r1".into(),
    }
}

pub(super) async fn free_one(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    fence: crate::durability::item_transfer::CurrentCharacterItemFence,
    command_id: u64,
) -> TestResult {
    let native = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .ok_or("native")?;
    let bag = owner
        .root
        .read_character_backpack(owner.character, fence.character_id)
        .await
        .map_err(debug)?
        .ok_or("bag")?;
    let filler = bag
        .entries
        .iter()
        .find(|entry| entry.item.definition.production_key == "oteryn:item.tibia.i3031")
        .ok_or("filler")?;
    let (digest, frame) = {
        let runtime = owner.runtime.lock().await;
        (
            runtime.content_pin().server_artifact_digest(),
            runtime.content_pin().frame_binding_digest(),
        )
    };
    let revision = format!(
        "sha256:{}",
        digest
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    );
    let permit = owner.spell_lane_permit().await.ok_or("lane permit")?;
    let moved = owner
        .root
        .commit_item_loop_move(
            &permit,
            owner.character,
            owner.holder,
            &fence,
            CommandRef::new(
                fence.game_session_id,
                CommandId::new(command_id).map_err(debug)?,
            ),
            &NativeItemWeights(native),
            crate::durability::item_loop_move::LoopMoveRequest {
                item: filler.item.item_instance_id,
                count: None,
                pickup_cell: None,
                destination: crate::durability::item_loop_move::LoopDestination::Ground(
                    crate::durability::item_loop_move::LoopGround {
                        spatial_position:
                            crate::durability::map_item_mint_audit::map_item_spatial_position(
                                32613, 32419, -7,
                            ),
                        map_revision: crate::map::overlay::map_revision(
                            owner.bundle_map().ok_or("bundle map")?.overlay.base(),
                        ),
                        content_revision: revision,
                        placement_context: frame.to_vec(),
                    },
                ),
            },
        )
        .await
        .map_err(debug)?;
    assert!(moved.is_ok(), "free capacity: {moved:?}");
    Ok(())
}

/// Exact committed source stack; the marker points one tile south of the chest.
pub(super) fn source_world() -> TestResult<crate::map::boot::BundleWorld> {
    use oteryn_world_bundle::{bundle, sector};
    let (bytes, mut pins) = crate::map::boot::tests::bundle();
    let mut data = bundle::read(&bytes)?;
    data.manifest.palette.clear();
    for (id, appearance) in [16485, 26996, 2472].into_iter().enumerate() {
        data.manifest.palette.push(bundle::PaletteEntry {
            key: format!("oteryn:terrain.tibia.i{appearance}"),
            family: bundle::Family::Terrain,
            id: u32::try_from(id)?,
            terrain: Some(bundle::Terrain {
                blocks_projectile: None,
                kind: if id == 0 {
                    bundle::TerrainKind::Ground
                } else {
                    bundle::TerrainKind::Common
                },
                walkable: (id == 0).then_some(true),
                ground_speed: (id == 0).then_some(150),
            }),
        });
    }
    data.manifest.world = bundle::Extent {
        min_x: 32570,
        min_y: 32419,
        max_x: 32615,
        max_y: 32509,
        floors: vec![-7],
    };
    let mut tiles = Vec::new();
    for y in 32419..=32420 {
        for x in 32612..=32614 {
            let palettes = if (x, y) == (32614, 32419) {
                vec![0, 1, 2]
            } else {
                vec![0]
            };
            tiles.push(sector::Tile {
                x,
                y,
                flags: 0,
                house: 0,
                zones: Vec::new(),
                items: palettes
                    .into_iter()
                    .map(|palette| sector::Item {
                        palette,
                        depth: 0,
                        attrs: sector::Attrs::default(),
                    })
                    .collect(),
            });
        }
    }
    // Retain Viola's real source placement, with a walkable approach in her sector.
    for y in 32506..=32508 {
        for x in 32570..=32572 {
            tiles.push(sector::Tile {
                x,
                y,
                flags: 0,
                house: 0,
                zones: Vec::new(),
                items: vec![sector::Item {
                    palette: 0,
                    depth: 0,
                    attrs: sector::Attrs::default(),
                }],
            });
        }
    }
    let mut grouped = std::collections::BTreeMap::<(u16, u16), Vec<sector::Tile>>::new();
    for tile in tiles {
        grouped
            .entry((tile.y / 32, tile.x / 32))
            .or_default()
            .push(tile);
    }
    let sectors = grouped
        .into_iter()
        .map(|((sy, sx), mut tiles)| {
            tiles.sort_by_key(|tile| (tile.y, tile.x));
            bundle::Sector {
                floor: -7,
                sx,
                sy,
                tiles,
            }
        })
        .collect::<Vec<_>>();
    let bytes = oteryn_world_bundle::writer::write(&data.manifest, &sectors, &Default::default())?;
    pins.digest = bundle::read(&bytes)?.digest;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    crate::map::boot::boot(
        &bytes,
        &crate::map::boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start: crate::map::overlay::TilePos {
                x: 32613,
                y: 32419,
                floor: -7,
            },
        },
        crate::foundation::WorldId::decode(&id(42)).map_err(debug)?,
        crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
        |_| None,
    )
    .map_err(debug)
}

pub(super) fn target(world: &crate::map::boot::BundleWorld) -> TestResult<WorldObjectTarget> {
    let key =
        oteryn_world_bundle::bundle::placement_key(-7, 32614, 32419, 2).ok_or("book placement")?;
    let mut placement = world.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    Ok(WorldObjectTarget {
        placement,
        expected_revision: 0,
    })
}
