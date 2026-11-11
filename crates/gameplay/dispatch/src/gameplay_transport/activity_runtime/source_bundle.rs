//! Materialize only a sealed, pinned donor capture through the existing bundle codec.
//! No rectangle is filled: absent donor tiles stay absent. Native Item references
//! are resolved anew against the same active generation as the qualified source room.
use crate::content::bosses::activity_template::PinnedActivitySourceRoom;
use crate::content::{ActiveGeneration, NativeEntryStart, QualifiedNativeEntryRoom};
use crate::foundation::{ChannelId, RuntimeScopeRefV1, WorldId};
use oteryn_world_bundle::{bundle, sector, writer};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[derive(Deserialize)]
struct Capture {
    source_entry: [i32; 3],
    source_bounds: [i32; 6],
    source_regions: Vec<[i32; 6]>,
    source_identity_bindings: Vec<Binding>,
    item_policies: Vec<Policy>,
    tiles: Vec<Tile>,
}
#[derive(Deserialize)]
struct Binding {
    external_id: String,
    target: Target,
}
#[derive(Deserialize)]
struct Target {
    family: String,
    key: String,
    revision: String,
}
#[derive(Deserialize)]
struct Policy {
    server_item_id: u32,
    source_top_order: u8,
    ground: bool,
    ground_speed: u16,
    block_solid: bool,
    block_projectile: bool,
    pickupable: bool,
    dynamic_kind: Option<String>,
}
#[derive(Deserialize)]
struct Tile {
    source_position: [i32; 3],
    otbm_flags: u32,
    house_id: Option<u32>,
    zones: Vec<u16>,
    items: Vec<Item>,
}
#[derive(Deserialize)]
struct Item {
    server_item_id: u32,
    depth: u8,
    attributes: BTreeMap<String, serde_json::Value>,
}

fn refusal(source: PinnedActivitySourceRoom, stage: &str, error: impl std::fmt::Debug) {
    #[cfg(test)]
    eprintln!("Source Bundle {} {stage}: {error:?}", source.name());
    let _ = (source, stage, error);
}

pub(in crate::gameplay_transport) fn qualify(
    room: &QualifiedNativeEntryRoom,
    source: PinnedActivitySourceRoom,
    active: &ActiveGeneration,
    world: WorldId,
    channel: ChannelId,
) -> Result<QualifiedNativeEntryRoom, ()> {
    qualify_for_scope(
        room,
        source,
        active,
        RuntimeScopeRefV1::channel(world, channel),
    )
}

pub(in crate::gameplay_transport) fn qualify_for_scope(
    room: &QualifiedNativeEntryRoom,
    source: PinnedActivitySourceRoom,
    active: &ActiveGeneration,
    scope: RuntimeScopeRefV1,
) -> Result<QualifiedNativeEntryRoom, ()> {
    let world = scope.world_id();
    if room.movement_cells().scope().world_id != world
        || room.map_revision_digest() != <[u8; 32]>::from(Sha256::digest(source.bytes()))
        || room.compiled().server_digest() != active.identity().server_artifact_digest()
        || room.compiled().client_digest() != active.identity().client_artifact_digest()
        || room.source_world().is_none()
    {
        return Err(());
    }
    let capture: Capture = serde_json::from_slice(source.bytes()).map_err(|_| ())?;
    if capture.source_regions.is_empty()
        || capture.tiles.iter().any(|tile| {
            let [x, y, z] = tile.source_position;
            !capture.source_regions.iter().any(|&[a, b, c, d, e, f]| {
                (a..=c).contains(&x) && (b..=d).contains(&y) && (e..=f).contains(&z)
            })
        })
    {
        return Err(());
    }
    let native = active.native_gameplay().ok_or(())?;
    let index = native.item_index().ok_or(())?;
    let bindings: BTreeMap<_, _> = capture
        .source_identity_bindings
        .iter()
        .map(|binding| {
            Ok((
                binding.external_id.parse::<u32>().map_err(|_| ())?,
                &binding.target,
            ))
        })
        .collect::<Result<_, ()>>()?;
    let policies: BTreeMap<_, _> = capture
        .item_policies
        .iter()
        .map(|policy| (policy.server_item_id, policy))
        .collect();
    let mut palette = Vec::with_capacity(policies.len());
    let mut palette_ids = BTreeMap::new();
    for (id, policy) in &policies {
        if policy.source_top_order > 3 {
            return Err(());
        }
        let target = bindings.get(id).ok_or(())?;
        let static_surface = crate::content::qualified_static_binding(
            &target.family,
            &target.key,
            &target.revision,
            *id,
        );
        let definition_id = if static_surface {
            let facts = crate::content::qualified_static_surface(&target.key).ok_or(())?;
            if facts.block_solid != policy.block_solid
                || facts.blocks_projectile != policy.block_projectile
                || facts.ground
                    != policy
                        .ground
                        .then_some((!policy.block_solid, policy.ground_speed))
                || policy.pickupable
                || policy.dynamic_kind.is_some()
            {
                refusal(
                    source,
                    "static policy mismatch",
                    (
                        *id,
                        &target.key,
                        policy.block_solid,
                        policy.block_projectile,
                        policy.ground,
                        policy.ground_speed,
                        policy.pickupable,
                        &policy.dynamic_kind,
                    ),
                );
                return Err(());
            }
            crate::content::static_surface_definition_id(&target.key).ok_or(())?
        } else {
            if target.family != "Item" {
                refusal(
                    source,
                    "unsupported binding family",
                    (*id, &target.family, &target.key),
                );
                return Err(());
            }
            index
                .definition_ref("Item", &target.key, &target.revision)
                .ok_or_else(|| {
                    refusal(
                        source,
                        "native Item binding unavailable",
                        (*id, &target.key, &target.revision),
                    )
                })?
                .get()
                - 1
        };
        palette_ids.insert(*id, u32::try_from(palette.len()).map_err(|_| ())?);
        palette.push(bundle::PaletteEntry {
            key: target.key.clone(),
            family: if static_surface {
                bundle::Family::Terrain
            } else {
                bundle::Family::Item
            },
            id: definition_id,
            terrain: if policy.ground {
                Some(bundle::Terrain {
                    kind: bundle::TerrainKind::Ground,
                    walkable: Some(!policy.block_solid),
                    ground_speed: Some(policy.ground_speed),
                    blocks_projectile: Some(policy.block_projectile),
                })
            } else {
                static_surface.then_some(bundle::Terrain {
                    kind: bundle::TerrainKind::Common,
                    walkable: None,
                    ground_speed: None,
                    blocks_projectile: Some(policy.block_projectile),
                })
            },
        });
    }
    let source_world = room.source_world().ok_or(())?;
    let mut closed_items = std::collections::BTreeSet::new();
    for tile in &capture.tiles {
        let [x, y, z] = tile.source_position;
        if source_world
            .lookup(
                room.movement_cells().scope(),
                crate::content::LogicalCell {
                    x,
                    y,
                    z: z.checked_neg().ok_or(())?,
                },
            )
            .is_err()
        {
            closed_items.extend(tile.items.iter().map(|item| item.server_item_id));
        }
    }
    // Only actually UNKNOWN placements need a conservative Item wall view.
    // Unplaced callback states must keep one unambiguous qualified definition.
    // Static surfaces retain their exact canonical Ground/Common semantics;
    // the final per-cell proof below still refuses any open UNKNOWN cell.
    let mut closed_palette_ids = palette_ids.clone();
    for id in closed_items {
        let normal = palette_ids.get(&id).ok_or(())?;
        let mut closed = palette.get(*normal as usize).ok_or(())?.clone();
        if closed.family != bundle::Family::Item {
            continue;
        }
        closed.terrain = Some(bundle::Terrain {
            kind: bundle::TerrainKind::Wall,
            walkable: None,
            ground_speed: None,
            // This is a conservative seal for an unknown dynamic source item,
            // not the raw item's proven projectile policy.
            blocks_projectile: None,
        });
        closed_palette_ids.insert(id, u32::try_from(palette.len()).map_err(|_| ())?);
        palette.push(closed);
    }
    let mut sectors = BTreeMap::<(i8, u16, u16), writer::Sector>::new();
    let mut dropped_teleports = Vec::new();
    for tile in &capture.tiles {
        let [x, y, z] = tile.source_position;
        let (x, y, floor) = (
            u16::try_from(x).map_err(|_| ())?,
            u16::try_from(y).map_err(|_| ())?,
            i8::try_from(z.checked_neg().ok_or(())?).map_err(|_| ())?,
        );
        let qualified = source_world
            .lookup(
                room.movement_cells().scope(),
                crate::content::LogicalCell {
                    x: i32::from(x),
                    y: i32::from(y),
                    z: i32::from(floor),
                },
            )
            .is_ok();
        let tile_palette = if qualified {
            &palette_ids
        } else {
            &closed_palette_ids
        };
        let mut items = Vec::with_capacity(tile.items.len());
        let mut ordinal = 0u8;
        for item in &tile.items {
            if item
                .attributes
                .keys()
                .any(|key| key != "dest" && key != "22" && key != "15")
            {
                return Err(());
            }
            let mut attrs = sector::Attrs::default();
            if let Some(count) = item.attributes.get("15") {
                attrs.count = Some(u8::try_from(count.as_u64().ok_or(())?).map_err(|_| ())?);
            }
            // OTBM attribute22 is an explicitly present u16 charge count;
            // zero remains present, rather than becoming an absent attribute.
            if let Some(charges) = item.attributes.get("22") {
                attrs.charges = Some(u16::try_from(charges.as_u64().ok_or(())?).map_err(|_| ())?);
            }
            if let Some(destination) = item.attributes.get("dest") {
                let [dx, dy, dz]: [i32; 3] =
                    serde_json::from_value(destination.clone()).map_err(|_| ())?;
                if [dx, dy, dz] == [0; 3] {
                    dropped_teleports.push(
                        crate::map::view::placement_key(
                            crate::map::overlay::TilePos { x, y, floor },
                            ordinal,
                        )
                        .ok_or(())?,
                    );
                } else {
                    attrs.teleport = Some((
                        u16::try_from(dx).map_err(|_| ())?,
                        u16::try_from(dy).map_err(|_| ())?,
                        i8::try_from(dz.checked_neg().ok_or(())?)
                            .map_err(|_| ())?
                            .to_le_bytes()[0],
                    ));
                }
            }
            items.push(sector::Item {
                palette: *tile_palette.get(&item.server_item_id).ok_or(())?,
                depth: item.depth,
                attrs,
            });
            if item.depth == 0 {
                ordinal = ordinal.checked_add(1).ok_or(())?;
            }
        }
        sectors
            .entry((floor, y / 32, x / 32))
            .or_insert_with(|| writer::Sector {
                floor,
                sx: x / 32,
                sy: y / 32,
                tiles: Vec::new(),
            })
            .tiles
            .push(sector::Tile {
                x,
                y,
                flags: tile.otbm_flags,
                house: tile.house_id.unwrap_or(0),
                zones: tile.zones.clone(),
                items,
            });
    }
    let mut floors: Vec<_> = sectors.keys().map(|key| key.0).collect();
    let [mut min_x, mut min_y, mut max_x, mut max_y, _, _] = capture.source_bounds;
    // The codec requires every retained TP coordinate to fit its metadata
    // extent. Sparse extent is not source coverage: no destination tile,
    // terrain policy, placement grant or cross-scope return is created here.
    for destination in sectors
        .values()
        .flat_map(|sector| &sector.tiles)
        .flat_map(|tile| &tile.items)
        .filter_map(|item| item.attrs.teleport)
    {
        let (x, y, floor) = destination;
        min_x = min_x.min(i32::from(x));
        min_y = min_y.min(i32::from(y));
        max_x = max_x.max(i32::from(x));
        max_y = max_y.max(i32::from(y));
        floors.push(i8::from_ne_bytes([floor]));
    }
    floors.sort_unstable();
    floors.dedup();
    dropped_teleports.sort_unstable();
    let manifest = writer::Manifest {
        subareas: Default::default(),
        format: bundle::FORMAT.into(),
        min_reader_version: bundle::VERSION,
        projection_class: "server".into(),
        compiler_version: "oteryn-pinned-activity-source-v1".into(),
        build_class: bundle::BuildClass::Production,
        identity: writer::Identity {
            content_revision: active.identity().content_revision().into(),
            required_capabilities: if palette
                .iter()
                .any(|entry| crate::content::qualified_static_surface(&entry.key).is_some())
            {
                vec!["source-static-surfaces-v1".into()]
            } else {
                Vec::new()
            },
            provenance_summary: format!(
                "zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:{}",
                source.name()
            ),
            ..Default::default()
        },
        world: writer::Extent {
            min_x: u16::try_from(min_x).map_err(|_| ())?,
            min_y: u16::try_from(min_y).map_err(|_| ())?,
            max_x: u16::try_from(max_x.checked_add(1).ok_or(())?).map_err(|_| ())?,
            max_y: u16::try_from(max_y.checked_add(1).ok_or(())?).map_err(|_| ())?,
            floors,
        },
        palette,
        draft_areas: Vec::new(),
        skipped_provisional_keys: Vec::new(),
        dropped_teleports,
        spawns: Default::default(),
        npcs: Default::default(),
    };
    let sectors: Vec<_> = sectors
        .into_values()
        .map(|mut sector| {
            sector.tiles.sort_by_key(|tile| (tile.y, tile.x));
            sector
        })
        .collect();
    let bytes = writer::write(&manifest, &sectors, &Default::default())
        .map_err(|error| refusal(source, "codec write", error))?;
    let decoded = bundle::read(&bytes).map_err(|error| refusal(source, "codec read", error))?;
    let pins = crate::map::BundlePins {
        digest: decoded.digest,
        project_format_version: manifest.identity.project_format_version.clone(),
        world_schema_version: manifest.identity.world_schema_version.clone(),
        content_revision: manifest.identity.content_revision.clone(),
        production: true,
    };
    let base =
        crate::map::load(&bytes, &pins).map_err(|error| refusal(source, "base load", error))?;
    let [x, y, z] = capture.source_entry;
    let checked = crate::map::boot::check(
        bytes.clone(),
        crate::map::boot::BootPins {
            bundle: pins,
            map_revision: crate::map::overlay::map_revision(&base),
            start: crate::map::overlay::TilePos {
                x: u16::try_from(x).map_err(|_| ())?,
                y: u16::try_from(y).map_err(|_| ())?,
                floor: i8::try_from(z.checked_neg().ok_or(())?).map_err(|_| ())?,
            },
        },
    )
    .map_err(|error| refusal(source, "map check", error))?;
    let map = checked.boot_for_scope(scope, |key| {
        let (id, target) = bindings.iter().find(|(_, target)| target.key == key)?;
        let policy = policies.get(id)?;
        let admitted = native.item_policy(&target.key, &target.revision).map(|policy| policy.record());
        let pickupable = policy.pickupable && admitted.is_some_and(|record| matches!(&record.semantics.physical,
            crate::content::ReferenceItemField::Known(physical) if physical.pickupable == crate::content::ReferenceItemField::Known(true)));
        let door = policy.dynamic_kind.as_deref() == Some("door") && admitted.is_some_and(|record| matches!(&record.semantics.classification,
            crate::content::ReferenceItemField::Known(classification) if classification.item_type == crate::content::ReferenceItemField::Known(crate::content::ReferenceItemType::Door)));
        Some(crate::map::facts::ItemDefinition { reference: index.definition_ref("Item", &target.key, &target.revision)?, solid: Some(policy.block_solid), blocks_projectile: policy.block_projectile, pickupable, door })
    }).map_err(|error|refusal(source,"map boot",error))?;
    let cells = map.movement_cells(room.movement_cells()).map_err(|_| ())?;
    #[cfg(test)]
    for tile in &capture.tiles {
        let [x, y, z] = tile.source_position;
        let before = room.movement_cells().spell_tiles().lookup(
            room.movement_cells().scope(),
            crate::content::LogicalCell { x, y, z: -z },
        );
        let at = crate::content::LogicalCell { x, y, z };
        let after = cells.spell_tiles().lookup(cells.scope(), at);
        let same = match (&before, &after) {
            (Ok(before), Ok(after)) => {
                before.flags() == after.flags()
                    && before.ground_source_id() == after.ground_source_id()
                    && before.ground_speed() == after.ground_speed()
                    && before.top_source_ids() == after.top_source_ids()
            }
            (Err(_), Ok(after)) => {
                cells.index().lookup(cells.scope(), at)
                    == Ok(crate::content::CollisionClass::Blocked)
                    && after.flags().block_projectile
            }
            (Err(_), Err(crate::content::SpellTileLookupError::Unknown)) => {
                cells.index().lookup(cells.scope(), at)
                    == Ok(crate::content::CollisionClass::Blocked)
            }
            _ => false,
        };
        if !same {
            eprintln!(
                "Source Bundle {} first sealed-cell mismatch {at:?} source={:?} bundle={:?}",
                source.name(),
                before.as_ref().map(|tile| (
                    tile.flags(),
                    tile.ground_source_id(),
                    tile.ground_speed(),
                    tile.top_source_ids().to_vec()
                )),
                after.as_ref().map(|tile| (
                    tile.flags(),
                    tile.ground_source_id(),
                    tile.ground_speed(),
                    tile.top_source_ids().to_vec()
                ))
            );
            break;
        }
    }
    let view = room
        .pinned_activity_bundle_view(
            cells,
            map.frame_binding_digest(world),
            map.map_revision_digest(),
            NativeEntryStart {
                x,
                y,
                floor: i16::try_from(z).map_err(|_| ())?,
            },
            &bytes,
        )
        .ok_or_else(|| refusal(source, "sealed Source view", ()))?;
    for tile in &capture.tiles {
        let [x, y, z] = tile.source_position;
        let source_cell = crate::content::LogicalCell {
            x,
            y,
            z: z.checked_neg().ok_or(())?,
        };
        let target_cell = crate::content::LogicalCell { x, y, z };
        match room
            .movement_cells()
            .spell_tiles()
            .lookup(room.movement_cells().scope(), source_cell)
        {
            Ok(expected) => {
                let actual = view
                    .movement_cells()
                    .spell_tiles()
                    .lookup(view.movement_cells().scope(), target_cell)
                    .map_err(|error| {
                        refusal(source, "qualified tile lookup", (target_cell, error))
                    })?;
                if actual.flags() != expected.flags() {
                    refusal(
                        source,
                        "qualified tile flags",
                        (target_cell, expected.flags(), actual.flags()),
                    );
                    return Err(());
                }
            }
            Err(_) => {
                if view
                    .movement_cells()
                    .index()
                    .lookup(view.movement_cells().scope(), target_cell)
                    == Ok(crate::content::CollisionClass::Walkable)
                {
                    refusal(source, "UNKNOWN tile became walkable", target_cell);
                    return Err(());
                }
                if view
                    .movement_cells()
                    .spell_tiles()
                    .lookup(view.movement_cells().scope(), target_cell)
                    .is_ok_and(|tile| !tile.flags().block_projectile)
                {
                    refusal(
                        source,
                        "UNKNOWN tile became projectile-transparent",
                        target_cell,
                    );
                    return Err(());
                }
            }
        }
    }
    Ok(view)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::LogicalCell;
    use crate::map::boot::BundleCollisionIndexExt;

    #[test]
    fn srx3_soul_war_actual_content_qualifies_five_recipes_and_keeps_greed_closed()
    -> Result<(), String> {
        std::thread::Builder::new()
            .name("srx3-soul-war-entry-content".into())
            .stack_size(64 * 1024 * 1024)
            .spawn(exercise_soul_war_entry)
            .map_err(|error| error.to_string())?
            .join()
            .map_err(|_| "actual Soul War content qualification panicked".to_owned())?
    }

    fn exercise_soul_war_entry() -> Result<(), String> {
        use crate::content::bosses::activity_template::{
            ActivityTemplateRefusal, PinnedActivitySourceRoom as Room,
        };
        use crate::content::native_gameplay::{
            NativeGameplayInput, test_support::try_activate_input,
        };

        fn debug(error: impl std::fmt::Debug) -> String {
            format!("{error:?}")
        }
        let mut world_bytes = [89; 16];
        world_bytes[6] = 0x70;
        world_bytes[8] = 0x80;
        let world = oteryn_protocol_oteryn::WorldId::decode(&world_bytes).map_err(debug)?;
        let mut channel_bytes = world_bytes;
        channel_bytes[0] = 90;
        let channel = ChannelId::decode(&channel_bytes).map_err(debug)?;
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(
            if env!("CARGO_PKG_NAME") == "oteryn-game-server" {
                "../../content/spells.manifest.json"
            } else {
                "../../../content/spells.manifest.json"
            },
        );
        // Keep the actual source world, profiles, source witnesses and reward rows.
        // No test policy or handwritten callback grant replaces their qualification.
        let input = NativeGameplayInput::from_manifest(&manifest).map_err(debug)?;
        let (controller, pin) = try_activate_input(world, &input).map_err(debug)?;
        let active = controller.active().ok_or("actual native activation")?;
        let native = active.native_gameplay().ok_or("actual native gameplay")?;
        let revision = native
            .training_formula()
            .ok_or("actual content revision")?
            .content_revision();
        assert!(native.qualifies_soul_war_callbacks(active.identity().server_artifact_digest()));

        // Crystal201189f lib/quests/soul_war.lua:286-478. Each role must reach its
        // own dedicated recipe, including Purple rather than a generic Mego alias.
        for (source, boss, creature, position, cooldown) in [
            (
                Room::GoshnarsMalice,
                "oteryn:boss.goshnar_s_malice",
                "oteryn:creature.goshnar_s_malice",
                [33709, 31599, 14],
                72_000,
            ),
            (
                Room::GoshnarsSpite,
                "oteryn:boss.goshnar_s_spite",
                "oteryn:creature.goshnar_s_spite",
                [33743, 31632, 14],
                72_000,
            ),
            (
                Room::GoshnarsHatred,
                "oteryn:boss.goshnar_s_hatred",
                "oteryn:creature.goshnar_s_hatred",
                [33744, 31599, 14],
                72_000,
            ),
            (
                Room::GoshnarsCruelty,
                "oteryn:boss.goshnar_s_cruelty",
                "oteryn:creature.goshnar_s_cruelty",
                [33856, 31866, 7],
                72_000,
            ),
            (
                Room::GoshnarsMegalomania,
                "oteryn:boss.goshnar_s_megalomania",
                "oteryn:creature.goshnar_s_megalomania_purple",
                [33710, 31634, 14],
                259_200,
            ),
        ] {
            let captured = active
                .qualify_activity_source_room(pin.qualified_room(), source)
                .map_err(|error| format!("actual {source:?} source room: {error:?}"))?;
            // The production entry qualifier receives physical Bundle coordinates.
            // Use its existing codec/Map owner, including the source-z conversion.
            let room = qualify(&captured, source, active, world, channel)
                .map_err(|_| format!("actual {source:?} source bundle"))?;
            let template = active
                .qualify_soul_war_activity_template(&room, revision, source)
                .map_err(|error| format!("actual {source:?} dedicated recipe: {error:?}"))?;
            assert_eq!(template.boss_key(), boss);
            assert_eq!(template.creature_key(), creature);
            let [x, y, z] = position;
            assert_eq!(template.boss_position(), LogicalCell { x, y, z });
            assert_eq!(template.room().pinned_activity_source_room(), Some(source));
            assert_eq!(template.cooldown_seconds(), cooldown);
            assert_eq!(
                (
                    template.minimum_level(),
                    template.minimum_players(),
                    template.maximum_players(),
                    template.time_limit_seconds(),
                    template.after_kill_seconds(),
                ),
                (250, 1, 5, 1200, 60)
            );
            let runtime_pin = template.issue_runtime_pin(active).map_err(debug)?;
            assert_eq!(
                runtime_pin.server_artifact_digest(),
                active.identity().server_artifact_digest()
            );
            assert_eq!(
                runtime_pin.frame_binding_digest(),
                room.frame_binding().digest()
            );
            assert_eq!(
                runtime_pin.map_revision_digest(),
                room.map_revision_digest()
            );

            assert!(matches!(
                active.qualify_soul_war_activity_template(&room, revision, Room::GoshnarsGreed),
                Err(ActivityTemplateRefusal::SourceMapUnavailable)
            ));
        }

        let greed_source = active
            .qualify_activity_source_room(pin.qualified_room(), Room::GoshnarsGreed)
            .map_err(debug)?;
        let greed_room = qualify(&greed_source, Room::GoshnarsGreed, active, world, channel)
            .map_err(|_| "actual Greed source bundle")?;
        assert!(
            matches!(
                active.qualify_soul_war_activity_template(
                    &greed_room,
                    revision,
                    Room::GoshnarsGreed
                ),
                Err(ActivityTemplateRefusal::SourceHookUnavailable)
            ),
            "actual Greed recipe must retain the unresolved Sphere15/10000/15000 health hold"
        );
        Ok(())
    }

    #[test]
    fn srx3_pinned_activity_bundle_preserves_real_tiles_and_decodes_map_window()
    -> Result<(), String> {
        let id = |seed| {
            let mut bytes = [seed; 16];
            bytes[6] = 0x70;
            bytes[8] = 0x80;
            bytes
        };
        let world = WorldId::decode(&id(1)).map_err(|error| format!("{error:?}"))?;
        let channel = ChannelId::decode(&id(2)).map_err(|error| format!("{error:?}"))?;
        let (controller, pin) =
            crate::content::native_gameplay::test_support::activated_with_loot_tables(world, None);
        let active = controller.active().ok_or("active content")?;
        for source in PinnedActivitySourceRoom::ALL {
            let room = active
                .qualify_activity_source_room(pin.qualified_room(), source)
                .map_err(|error| format!("{error:?}"))?;
            let bundle = qualify(&room, source, active, world, channel)
                .map_err(|_| format!("{} pinned source bundle", source.name()))?;
            assert_eq!(bundle.pinned_activity_source_room(), Some(source));
            let unqualified = room
                .bundle_view(
                    bundle.movement_cells().clone(),
                    bundle.frame_binding().digest(),
                    bundle.map_revision_digest(),
                    bundle.entry_start(),
                )
                .ok_or("generic bundle view")?;
            assert_eq!(
                unqualified.pinned_activity_source_room(),
                None,
                "generic map projection never carries a source encounter role"
            );
            if source == PinnedActivitySourceRoom::GoshnarsSpite {
                let native = active.native_gameplay().ok_or("actual native closure")?;
                assert!(
                    native.qualify_spite_global_event(&bundle).is_some(),
                    "sealed real Bundle retains immutable GlobalEvent DATA in its current geometry"
                );
                assert!(
                    native.qualify_spite_global_event(&unqualified).is_none(),
                    "generic Bundle projection grants no registered GlobalEvent role"
                );
            }
            if matches!(
                source,
                PinnedActivitySourceRoom::GoshnarsSpite
                    | PinnedActivitySourceRoom::GoshnarsHatred
                    | PinnedActivitySourceRoom::GoshnarsCruelty
                    | PinnedActivitySourceRoom::GoshnarsMegalomania
                    | PinnedActivitySourceRoom::CallbackRagiaz
            ) {
                let original = active
                    .qualify_ambient_xml_spawn_policy(&room)
                    .map_err(|error| format!("{} actual Source XML: {error:?}", source.name()))?;
                let projected = active
                    .qualify_ambient_xml_spawn_policy(&bundle)
                    .map_err(|error| format!("{} sealed Bundle XML: {error:?}", source.name()))?;
                assert_eq!(projected.role(), original.role());
                assert_eq!(projected.sources(), original.sources());
                assert_eq!(projected.pins()[2], bundle.frame_binding().digest());
                assert_eq!(projected.pins()[3], bundle.map_revision_digest());
                assert!(
                    active
                        .qualify_ambient_xml_spawn_policy(&unqualified)
                        .is_err(),
                    "generic projection cannot grant source XML registration"
                );
            }
            let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(index) =
                bundle.movement_cells().index()
            else {
                return Err("activity did not use the real bundle owner".into());
            };
            let capture: Capture =
                serde_json::from_slice(source.bytes()).map_err(|error| error.to_string())?;
            let references: BTreeMap<_, _> = capture
                .source_identity_bindings
                .iter()
                .map(|binding| {
                    let id = binding
                        .external_id
                        .parse::<u32>()
                        .map_err(|error| error.to_string())?;
                    if crate::content::qualified_static_binding(
                        &binding.target.family,
                        &binding.target.key,
                        &binding.target.revision,
                        id,
                    ) {
                        return Ok((
                            id,
                            crate::content::static_surface_definition_id(&binding.target.key)
                                .ok_or("static source definition")?,
                        ));
                    }
                    let reference = active
                        .native_gameplay()
                        .and_then(|native| native.item_index())
                        .and_then(|index| {
                            index.definition_ref(
                                "Item",
                                &binding.target.key,
                                &binding.target.revision,
                            )
                        })
                        .ok_or("native source Item")?;
                    Ok((id, reference.get() - 1))
                })
                .collect::<Result<_, String>>()?;
            assert_eq!(index.map().overlay.base().tile_count(), capture.tiles.len());
            for tile in &capture.tiles {
                let [x, y, z] = tile.source_position;
                let actual = index
                    .map()
                    .overlay
                    .base()
                    .tile(x as u16, y as u16, -z as i8)
                    .ok_or("captured tile omitted")?;
                assert_eq!(
                    actual.ids(),
                    tile.items
                        .iter()
                        .map(|item| references[&item.server_item_id])
                        .collect::<Vec<_>>()
                );
                assert_eq!(
                    actual.depths(),
                    tile.items.iter().map(|item| item.depth).collect::<Vec<_>>()
                );
                for item in tile.items.iter().filter(|item| item.depth == 0) {
                    let Some(destination) = item.attributes.get("dest") else {
                        continue;
                    };
                    let [dx, dy, dz]: [i32; 3] = serde_json::from_value(destination.clone())
                        .map_err(|error| error.to_string())?;
                    if [dx, dy, dz] == [0; 3] {
                        continue;
                    }
                    let from = crate::map::overlay::TilePos {
                        x: x.try_into().map_err(|_| "TPx")?,
                        y: y.try_into().map_err(|_| "TPy")?,
                        floor: (-z).try_into().map_err(|_| "TPfloor")?,
                    };
                    let to = crate::map::overlay::TilePos {
                        x: dx.try_into().map_err(|_| "TPdestx")?,
                        y: dy.try_into().map_err(|_| "TPdesty")?,
                        floor: (-dz).try_into().map_err(|_| "TPdestfloor")?,
                    };
                    assert_eq!(
                        index.map().floors.teleport(from),
                        Some(to),
                        "retained donor TP attribute"
                    );
                    if !capture
                        .tiles
                        .iter()
                        .any(|tile| tile.source_position == [dx, dy, dz])
                    {
                        assert!(
                            index
                                .map()
                                .overlay
                                .base()
                                .tile(to.x, to.y, to.floor)
                                .is_none(),
                            "sparse metadata extent adds no destination geometry"
                        );
                        assert!(
                            bundle
                                .movement_cells()
                                .index()
                                .lookup(
                                    bundle.movement_cells().scope(),
                                    LogicalCell {
                                        x: dx,
                                        y: dy,
                                        z: dz
                                    }
                                )
                                .is_err(),
                            "an uncaptured destination grants no movement/source placement"
                        );
                        assert!(
                            bundle
                                .movement_cells()
                                .spell_tiles()
                                .lookup(
                                    bundle.movement_cells().scope(),
                                    LogicalCell {
                                        x: dx,
                                        y: dy,
                                        z: dz
                                    }
                                )
                                .is_err(),
                            "an uncaptured destination remains UNKNOWN"
                        );
                        assert!(
                            room.movement_cells()
                                .index()
                                .lookup(
                                    room.movement_cells().scope(),
                                    LogicalCell {
                                        x: dx,
                                        y: dy,
                                        z: -dz
                                    }
                                )
                                .is_err(),
                            "original source coverage stays unchanged"
                        );
                    }
                }
                let source_cell = crate::content::LogicalCell { x, y, z: -z };
                let target_cell = crate::content::LogicalCell { x, y, z };
                match room
                    .movement_cells()
                    .spell_tiles()
                    .lookup(room.movement_cells().scope(), source_cell)
                {
                    Ok(expected) => assert_eq!(
                        bundle
                            .movement_cells()
                            .spell_tiles()
                            .lookup(bundle.movement_cells().scope(), target_cell)
                            .map_err(|error| format!("{error:?}"))?
                            .flags(),
                        expected.flags(),
                        "{} source tile {source_cell:?}",
                        source.name()
                    ),
                    Err(_) => {
                        assert_eq!(
                            bundle
                                .movement_cells()
                                .index()
                                .lookup(bundle.movement_cells().scope(), target_cell),
                            Ok(crate::content::CollisionClass::Blocked),
                            "a captured UNKNOWN cell must retain actual blocked geometry"
                        );
                        match bundle
                            .movement_cells()
                            .spell_tiles()
                            .lookup(bundle.movement_cells().scope(), target_cell)
                        {
                            Ok(tile) => assert!(tile.flags().block_projectile),
                            Err(crate::content::SpellTileLookupError::Unknown) => (),
                            Err(error) => {
                                return Err(format!(
                                    "captured UNKNOWN cell became absent: {error:?}"
                                ));
                            }
                        }
                    }
                }
            }
            let map = index.map();
            let facts = map.facts.pinned();
            let view = super::super::super::world_map::MapViewSource {
                overlay: &map.overlay,
                facts: &facts,
                content_generation: active.identity().client_artifact_digest(),
                reset_epoch: 1,
            };
            let mut items =
                super::super::super::item_view::SessionItemView::resume(Default::default());
            let mut session = super::super::super::world_map::SessionMapView::default();
            let [x, y, z] = capture.source_entry;
            let update = session
                .update(
                    &mut items,
                    &view,
                    super::super::super::world_spatial::ActorPosition {
                        x,
                        y,
                        floor: -z as i16,
                    },
                )
                .map_err(|error| format!("{error:?}"))?
                .ok_or("source map update")?;
            let super::super::super::world_map::MapUpdate::Snapshot(snapshot) = update else {
                return Err("first activity map update was not a snapshot".into());
            };
            let decoded =
                oteryn_protocol_oteryn::world_map::decode_world_map_snapshot(&snapshot.payload)
                    .map_err(|error| format!("{error:?}"))?;
            assert!(decoded.tiles.iter().any(|tile| !tile.items.is_empty()));
        }
        Ok(())
    }
    #[test]
    fn srx3_pinned_activity_instances_isolate_ground_and_native_cues() -> Result<(), String> {
        use crate::map::view::MapFacts;
        let id = |seed| {
            let mut b = [seed; 16];
            b[6] = 0x70;
            b[8] = 0x80;
            b
        };
        let world = WorldId::decode(&id(1)).map_err(|e| format!("{e:?}"))?;
        let channel = ChannelId::decode(&id(2)).map_err(|e| format!("{e:?}"))?;
        let (controller, pin) =
            crate::content::native_gameplay::test_support::activated_with_loot_tables(world, None);
        let active = controller.active().ok_or("active")?;
        let kind = PinnedActivitySourceRoom::GoshnarsMalice;
        let source = active
            .qualify_activity_source_room(pin.qualified_room(), kind)
            .map_err(|e| format!("{e:?}"))?;
        let canonical = qualify(&source, kind, active, world, channel)
            .map_err(|_| "canonical source bundle")?;
        let registry = super::super::ActivityRuntimeRegistry::default();
        registry
            .register_source_room(kind, &canonical)
            .map_err(|_| "register source template")?;
        let first_scope =
            RuntimeScopeRefV1::instance(world, id(3)).map_err(|error| format!("{error:?}"))?;
        let second_scope =
            RuntimeScopeRefV1::instance(world, id(4)).map_err(|error| format!("{error:?}"))?;
        assert!(
            registry
                .fresh_source_room(kind, active, RuntimeScopeRefV1::channel(world, channel))
                .is_err(),
            "a private activity factory requires an Instance scope"
        );
        let first = registry
            .fresh_source_room(kind, active, first_scope)
            .map_err(|_| "first Instance facts")?;
        let second = registry
            .fresh_source_room(kind, active, second_scope)
            .map_err(|_| "second Instance facts")?;
        fn map(room: &QualifiedNativeEntryRoom) -> Option<&crate::map::boot::BundleMap> {
            match room.movement_cells().index() {
                crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(index) => {
                    Some(index.map())
                }
                _ => None,
            }
        }
        let capture: Capture = serde_json::from_slice(kind.bytes()).map_err(|e| e.to_string())?;
        let tile = capture
            .tiles
            .iter()
            .find(|tile| {
                let [x, y, z] = tile.source_position;
                tile.items.len() == 1
                    && tile.items[0].server_item_id != 409
                    && canonical
                        .movement_cells()
                        .index()
                        .lookup(canonical.movement_cells().scope(), LogicalCell { x, y, z })
                        == Ok(crate::content::CollisionClass::Walkable)
            })
            .ok_or("source nonwhite Ground cell")?;
        let [x, y, z] = tile.source_position;
        let pos = crate::map::overlay::TilePos {
            x: x.try_into().map_err(|_| "x")?,
            y: y.try_into().map_err(|_| "y")?,
            floor: (-z).try_into().map_err(|_| "floor")?,
        };
        let key = crate::map::view::placement_key(pos, 0).ok_or("Ground placement")?;
        let first = map(&first).ok_or("first Bundle owner")?;
        let second = map(&second).ok_or("second Bundle owner")?;
        let canonical = map(&canonical).ok_or("template Bundle owner")?;
        assert_eq!(first.overlay.runtime_scope(), first_scope);
        assert_eq!(second.overlay.runtime_scope(), second_scope);
        assert_eq!(
            canonical.overlay.runtime_scope(),
            RuntimeScopeRefV1::channel(world, channel)
        );
        assert_eq!(first.facts.object_revision(key), 0);
        assert!(
            first.facts.transform_movement_item(key, 409),
            "exact pinned white Ground definition"
        );
        assert!(first.facts.emit_movement_effect(pos, 56));
        assert_eq!(first.facts.object_revision(key), 1);
        assert_eq!(
            second.facts.object_revision(key),
            0,
            "party Ground state leaked between Instances"
        );
        assert_eq!(
            canonical.facts.object_revision(key),
            0,
            "the immutable source template was mutated"
        );
        assert_eq!(first.facts.movement_effects_since(0).len(), 1);
        assert!(
            second.facts.movement_effects_since(0).is_empty(),
            "native cue leaked between Instances"
        );
        assert!(canonical.facts.movement_effects_since(0).is_empty());
        Ok(())
    }
}
