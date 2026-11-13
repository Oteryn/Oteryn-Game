//! Bounded callback geometry provenance, verified against a complete actual bundle.
//! It grants no entry, actor, Item custody or callback rule authority.
use super::{PinnedActivitySourceRoom, SourceTile, SourceWorldDocument};
use oteryn_world_bundle::{bundle, sector};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallbackBundleBasis {
    source: PinnedActivitySourceRoom,
    bundle_digest: [u8; 32],
    item_references: Vec<(String, String, u32)>,
}
impl CallbackBundleBasis {
    pub const fn source(&self) -> PinnedActivitySourceRoom {
        self.source
    }
    pub const fn bundle_digest(&self) -> [u8; 32] {
        self.bundle_digest
    }
    pub fn protects_item_references(
        &self,
        index: &crate::content::item_ref::ItemDefinitionIndex,
    ) -> bool {
        let Some(expected) = expected_scenes()
            .ok()
            .and_then(|scenes| scenes.iter().find(|scene| scene.source == self.source))
        else {
            return false;
        };
        expected
            .document
            .source_identity_bindings
            .iter()
            .all(|binding| {
                crate::content::qualified_static_surface(&binding.target.key).is_some_and(|facts| {
                    binding.external_id.parse::<u16>().ok() == Some(facts.source_id)
                        && binding.target.revision == "definition-r1"
                }) || (binding.target.family == super::ProjectV2Family::Item
                    && index
                        .definition_ref("Item", &binding.target.key, &binding.target.revision)
                        .is_some())
            })
            && self.item_references.iter().all(|(key, revision, id)| {
                index
                    .definition_ref("Item", key, revision)
                    .is_some_and(|reference| reference.get().checked_sub(1) == Some(*id))
            })
    }
}

impl PinnedActivitySourceRoom {
    /// Only fixed-global W01 callback recipes can qualify a larger served map.
    /// The independent W07/W10/W11 admission recipes cannot use this path.
    pub const fn is_callback_context(self) -> bool {
        matches!(
            self,
            Self::BrainHead
                | Self::CallbackAnomaly
                | Self::CallbackLadyTenebris
                | Self::CallbackMazoran
                | Self::CallbackOutburst
                | Self::CallbackPlagirath
                | Self::CallbackProfessorMaxxen
                | Self::CallbackRagiaz
                | Self::CallbackRazzagorn
                | Self::CallbackShulgrax
                | Self::CallbackTimeGuardian
                | Self::CallbackZamulosh
        )
    }
}
struct ExpectedScene {
    source: PinnedActivitySourceRoom,
    document: SourceWorldDocument,
    positions: BTreeMap<(i8, u16, u16), usize>,
    sectors: BTreeSet<(i8, u16, u16)>,
    bindings: BTreeMap<u32, (String, String)>,
}
fn expected_scenes() -> Result<&'static [ExpectedScene], oteryn_world_bundle::Error> {
    static EXPECTED: OnceLock<Result<Vec<ExpectedScene>, String>> = OnceLock::new();
    EXPECTED
        .get_or_init(|| {
            PinnedActivitySourceRoom::ALL
                .into_iter()
                .filter(|source| source.is_callback_context())
                .map(|source| {
                    let document: SourceWorldDocument = serde_json::from_slice(source.bytes())
                        .map_err(|error| error.to_string())?;
                    if document.tiles.is_empty()
                        || document.tiles.len() > 4096
                        || document.native_frame.floor_transform != "native_floor=-source_z"
                        || document.native_frame.origin_x != 0
                        || document.native_frame.origin_y != 0
                    {
                        return Err("invalid bounded source callback capture".into());
                    }
                    let mut positions = BTreeMap::new();
                    let mut sectors = BTreeSet::new();
                    for (index, tile) in document.tiles.iter().enumerate() {
                        let [x, y, z] = tile.source_position;
                        let floor = i8::try_from(z)
                            .ok()
                            .filter(|z| (0..=15).contains(z))
                            .and_then(i8::checked_neg)
                            .ok_or("source callback floor")?;
                        let x = u16::try_from(x).map_err(|_| "source callback x")?;
                        let y = u16::try_from(y).map_err(|_| "source callback y")?;
                        if positions.insert((floor, x, y), index).is_some() {
                            return Err("duplicate source callback cell".into());
                        }
                        sectors.insert((floor, x / sector::SECTOR_SIZE, y / sector::SECTOR_SIZE));
                    }
                    let mut bindings = BTreeMap::new();
                    for binding in &document.source_identity_bindings {
                        let id = binding
                            .external_id
                            .parse::<u32>()
                            .map_err(|_| "source callback Item identity")?;
                        let static_surface = crate::content::qualified_static_surface(
                            &binding.target.key,
                        )
                        .filter(|facts| {
                            u32::from(facts.source_id) == id
                                && binding.target.revision == "definition-r1"
                        });
                        let expected_family =
                            if static_surface.is_some_and(|facts| facts.ground.is_some()) {
                                super::ProjectV2Family::Terrain
                            } else if static_surface.is_some() {
                                super::ProjectV2Family::WorldObject
                            } else {
                                super::ProjectV2Family::Item
                            };
                        if binding.target.family != expected_family
                            || (static_surface.is_none()
                                && binding.target.key != format!("oteryn:item.tibia.i{id}"))
                            || bindings
                                .insert(
                                    id,
                                    (binding.target.key.clone(), binding.target.revision.clone()),
                                )
                                .is_some()
                        {
                            return Err("source callback protected Item reference".into());
                        }
                    }
                    Ok(ExpectedScene {
                        source,
                        document,
                        positions,
                        sectors,
                        bindings,
                    })
                })
                .collect()
        })
        .as_ref()
        .map(|scenes| scenes.as_slice())
        .map_err(|error| oteryn_world_bundle::Error::Format(error.clone()))
}

fn source_attrs(expected: &super::SourceItem, family: bundle::Family) -> Option<sector::Attrs> {
    let mut attrs = sector::Attrs::default();
    for (key, value) in &expected.attributes {
        match key.as_str() {
            "15" => attrs.count = Some(u8::try_from(value.as_u64()?).ok()?),
            "22" => attrs.charges = Some(u16::try_from(value.as_u64()?).ok()?),
            "dest" => {
                let destination: [i32; 3] = serde_json::from_value(value.clone()).ok()?;
                if destination != [0; 3] {
                    let floor = i8::try_from(destination[2]).ok()?.checked_neg()?;
                    attrs.teleport = Some((
                        u16::try_from(destination[0]).ok()?,
                        u16::try_from(destination[1]).ok()?,
                        floor.to_ne_bytes()[0],
                    ));
                }
            }
            _ => return None,
        }
    }
    // Mirror the compiler's explicit Item materialization adapter; raw source bytes
    // and the actual bundle attributes remain unchanged.
    Some(if family == bundle::Family::Item {
        attrs.with_legacy_default_item_count()
    } else {
        attrs
    })
}
fn matches_tile(
    expected: &SourceTile,
    actual: &sector::Tile,
    manifest: &bundle::Manifest,
    scene: &ExpectedScene,
    refs: &mut BTreeSet<(String, String, u32)>,
) -> bool {
    if expected.otbm_flags != actual.flags
        || expected.house_id.unwrap_or(0) != actual.house
        || expected.zones != actual.zones
        || expected.items.len() != actual.items.len()
    {
        return false;
    }
    expected
        .items
        .iter()
        .zip(&actual.items)
        .all(|(expected, actual)| {
            let Some((key, revision)) = scene.bindings.get(&expected.server_item_id) else {
                return false;
            };
            let Some(palette) = manifest.palette.get(actual.palette as usize) else {
                return false;
            };
            let static_surface = crate::content::qualified_static_surface(key);
            let expected_family = if static_surface.is_some() {
                bundle::Family::Terrain
            } else {
                bundle::Family::Item
            };
            if palette.family != expected_family
                || palette.key != *key
                || (static_surface.is_some()
                    && crate::content::static_surface_definition_id(key) != Some(palette.id))
                || u16::from(actual.depth) != expected.depth
                || source_attrs(expected, expected_family).as_ref() != Some(&actual.attrs)
            {
                return false;
            }
            if static_surface.is_none() {
                refs.insert((key.clone(), revision.clone(), palette.id));
            }
            true
        })
}

/// Authentication of the complete repository-published World artifact. It says
/// nothing about source Lua rules, current actors, mutable tiles or admission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishedWorldBundleBasis {
    digest: [u8; 32],
}
impl PublishedWorldBundleBasis {
    pub const fn bundle_digest(&self) -> [u8; 32] {
        self.digest
    }
}
#[derive(Debug, Clone)]
pub struct QualifiedBundleSourceBases {
    pub callbacks: Vec<CallbackBundleBasis>,
    pub published_world: Option<PublishedWorldBundleBasis>,
}

/// Validate the complete real bundle before sealing any bounded geometry proof.
/// Every captured cell, full ordered stack and protected attribute must match.
/// Source-UNKNOWN collision remains UNKNOWN to callback execution; this proof
/// never supplies current occupancy, mutable ground, flags or a walkability grant.
pub fn qualify_callback_bundle_basis(
    data: &[u8],
) -> Result<Vec<CallbackBundleBasis>, oteryn_world_bundle::Error> {
    Ok(qualify_bundle_source_bases(data)?.callbacks)
}

/// The World loader retains these sealed bases beside the actual loaded map.
pub fn qualify_bundle_source_bases(
    data: &[u8],
) -> Result<QualifiedBundleSourceBases, oteryn_world_bundle::Error> {
    let scenes = expected_scenes()?;
    let mut matched = vec![0usize; scenes.len()];
    let mut refused = vec![false; scenes.len()];
    let mut references = vec![BTreeSet::new(); scenes.len()];
    let visited = bundle::visit(data, |manifest, sector| {
        for (index, scene) in scenes.iter().enumerate() {
            if refused[index]
                || !scene
                    .sectors
                    .contains(&(sector.floor, sector.sx, sector.sy))
            {
                continue;
            }
            for actual in &sector.tiles {
                let Some(expected) = scene
                    .positions
                    .get(&(sector.floor, actual.x, actual.y))
                    .map(|index| &scene.document.tiles[*index])
                else {
                    continue;
                };
                if !matches_tile(expected, actual, manifest, scene, &mut references[index]) {
                    refused[index] = true;
                    break;
                }
                matched[index] += 1;
            }
        }
        Ok(())
    })?;
    let published_pin: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../../../../../content/world/pins/oteryn.json"
    ))
    .map_err(|error| oteryn_world_bundle::Error::Format(error.to_string()))?;
    let actual_hex = visited
        .digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let published_world = (published_pin
        .get("digest")
        .and_then(serde_json::Value::as_str)
        == Some(actual_hex.as_str()))
    .then_some(PublishedWorldBundleBasis {
        digest: visited.digest,
    });
    let callbacks = scenes
        .iter()
        .enumerate()
        .filter(|(index, scene)| !refused[*index] && matched[*index] == scene.positions.len())
        .map(|(index, scene)| CallbackBundleBasis {
            source: scene.source,
            bundle_digest: visited.digest,
            item_references: std::mem::take(&mut references[index]).into_iter().collect(),
        })
        .collect();
    Ok(QualifiedBundleSourceBases {
        callbacks,
        published_world,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn captured_shulgrax_zero_counts_follow_only_item_materialization()
    -> Result<(), Box<dyn std::error::Error>> {
        let document: SourceWorldDocument =
            serde_json::from_slice(PinnedActivitySourceRoom::CallbackShulgrax.bytes())?;
        let zero_items: Vec<_> = document
            .tiles
            .iter()
            .flat_map(|tile| &tile.items)
            .filter(|item| item.attributes.get("15") == Some(&serde_json::json!(0)))
            .collect();
        assert_eq!(zero_items.len(), 9, "complete captured legacy-zero cohort");
        for item in zero_items {
            let binding = document
                .source_identity_bindings
                .iter()
                .find(|binding| binding.external_id == item.server_item_id.to_string())
                .ok_or("captured Item binding")?;
            assert_eq!(binding.target.family, super::super::ProjectV2Family::Item);
            assert!(crate::content::qualified_static_surface(&binding.target.key).is_none());
            assert_eq!(
                item.attributes,
                BTreeMap::from([("15".into(), serde_json::json!(0))]),
                "raw captured zero remains explicit"
            );
            assert_eq!(
                source_attrs(item, bundle::Family::Item),
                Some(sector::Attrs::default())
            );
            assert_eq!(
                source_attrs(item, bundle::Family::Terrain),
                Some(sector::Attrs {
                    count: Some(0),
                    ..sector::Attrs::default()
                }),
                "a non-Item source keeps exact zero presence"
            );
        }
        let explicit_one = document
            .tiles
            .iter()
            .flat_map(|tile| &tile.items)
            .find(|item| item.attributes.get("15") == Some(&serde_json::json!(1)))
            .ok_or("captured explicit one")?;
        assert_eq!(explicit_one.server_item_id, 10876);
        for family in [bundle::Family::Item, bundle::Family::Terrain] {
            assert_eq!(
                source_attrs(explicit_one, family),
                Some(sector::Attrs {
                    count: Some(1),
                    ..sector::Attrs::default()
                })
            );
        }
        Ok(())
    }

    #[test]
    fn callback_item_count_adapter_preserves_other_attributes_and_refusals()
    -> Result<(), Box<dyn std::error::Error>> {
        let document: SourceWorldDocument =
            serde_json::from_slice(PinnedActivitySourceRoom::CallbackShulgrax.bytes())?;
        let mut item = document
            .tiles
            .iter()
            .flat_map(|tile| &tile.items)
            .find(|item| item.attributes.get("15") == Some(&serde_json::json!(0)))
            .ok_or("captured zero Item")?
            .clone();
        item.attributes.insert("22".into(), serde_json::json!(0));
        item.attributes
            .insert("dest".into(), serde_json::json!([33319, 32318, 13]));
        let expected = sector::Attrs {
            charges: Some(0),
            teleport: Some((33319, 32318, (-13i8).to_ne_bytes()[0])),
            ..sector::Attrs::default()
        };
        assert_eq!(
            source_attrs(&item, bundle::Family::Item),
            Some(expected.clone())
        );
        assert_eq!(
            source_attrs(&item, bundle::Family::Terrain),
            Some(sector::Attrs {
                count: Some(0),
                ..expected
            })
        );
        for family in [bundle::Family::Item, bundle::Family::Terrain] {
            for value in [
                serde_json::json!(true),
                serde_json::json!(-1),
                serde_json::json!(256),
                serde_json::json!("0"),
            ] {
                let mut changed = item.clone();
                changed.attributes.insert("15".into(), value);
                assert!(source_attrs(&changed, family).is_none());
            }
            let mut changed = item.clone();
            changed
                .attributes
                .insert("unknown".into(), serde_json::json!(0));
            assert!(source_attrs(&changed, family).is_none());
            let mut changed = item.clone();
            changed
                .attributes
                .insert("22".into(), serde_json::json!(65536));
            assert!(source_attrs(&changed, family).is_none());
        }
        Ok(())
    }

    #[test]
    fn materialized_shulgrax_tile_rejects_an_actual_typed_zero_count()
    -> Result<(), Box<dyn std::error::Error>> {
        let document: SourceWorldDocument =
            serde_json::from_slice(PinnedActivitySourceRoom::CallbackShulgrax.bytes())?;
        let mut bindings = BTreeMap::new();
        for binding in &document.source_identity_bindings {
            bindings.insert(
                binding.external_id.parse()?,
                (binding.target.key.clone(), binding.target.revision.clone()),
            );
        }
        let scene = ExpectedScene {
            source: PinnedActivitySourceRoom::CallbackShulgrax,
            document,
            positions: BTreeMap::new(),
            sectors: BTreeSet::new(),
            bindings,
        };
        let expected = scene
            .document
            .tiles
            .iter()
            .find(|tile| tile.source_position == [33477, 32791, 13])
            .ok_or("captured two-entry zero-count tile")?;
        assert_eq!(expected.items.len(), 2);
        assert_eq!(expected.items[0].server_item_id, 19277);
        assert!(expected.items[0].attributes.is_empty());
        assert_eq!(expected.items[1].server_item_id, 3115);
        assert_eq!(
            expected.items[1].attributes,
            BTreeMap::from([("15".into(), serde_json::json!(0))])
        );
        let mut palette = Vec::new();
        for (id, item) in expected.items.iter().enumerate() {
            let (key, _) = scene.bindings.get(&item.server_item_id).ok_or("binding")?;
            assert!(crate::content::qualified_static_surface(key).is_none());
            palette.push(bundle::PaletteEntry {
                key: key.clone(),
                family: bundle::Family::Item,
                id: u32::try_from(id)?,
                terrain: None,
            });
        }
        let manifest = bundle::Manifest {
            subareas: Default::default(),
            format: bundle::FORMAT.into(),
            min_reader_version: bundle::VERSION,
            projection_class: "server".into(),
            compiler_version: "callback-tile-fixture".into(),
            build_class: bundle::BuildClass::NonProduction,
            identity: Default::default(),
            world: bundle::Extent {
                min_x: 33477,
                min_y: 32791,
                max_x: 33478,
                max_y: 32792,
                floors: vec![-13],
            },
            palette,
            draft_areas: Vec::new(),
            skipped_provisional_keys: Vec::new(),
            dropped_teleports: Vec::new(),
            spawns: Default::default(),
            npcs: Default::default(),
        };
        let actual = sector::Tile {
            x: 33477,
            y: 32791,
            flags: expected.otbm_flags,
            house: expected.house_id.unwrap_or(0),
            zones: expected.zones.clone(),
            items: vec![
                sector::Item {
                    palette: 0,
                    depth: 0,
                    attrs: sector::Attrs::default(),
                },
                sector::Item {
                    palette: 1,
                    depth: 0,
                    attrs: sector::Attrs::default(),
                },
            ],
        };
        assert!(matches_tile(
            expected,
            &actual,
            &manifest,
            &scene,
            &mut BTreeSet::new()
        ));
        for count in [0, 1, 2] {
            let mut changed = actual.clone();
            changed.items[1].attrs.count = Some(count);
            assert!(
                !matches_tile(expected, &changed, &manifest, &scene, &mut BTreeSet::new()),
                "actual typed count {count} must not be normalized or ignored"
            );
        }
        let mut changed = actual.clone();
        changed.items[1].attrs.charges = Some(0);
        assert!(!matches_tile(
            expected,
            &changed,
            &manifest,
            &scene,
            &mut BTreeSet::new()
        ));
        Ok(())
    }

    #[test]
    fn default_built_bundle_qualifies_literal_callback_contexts()
    -> Result<(), Box<dyn std::error::Error>> {
        let start = std::time::Instant::now();
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let bytes = if let Some(path) = std::env::var_os("SRX3_DEFAULT_WORLD_BUNDLE") {
            std::fs::read(path)?
        } else {
            use oteryn_world_bundle_compiler::{bundle::BuildClass, compile, source};
            let project = source::project(&root)?;
            let (registry, palette) = source::registry(&root, &project)?;
            let identity = serde_json::from_slice(&std::fs::read(
                root.join("content/world/pins/oteryn.identity.json"),
            )?)?;
            compile::compile(
                &compile::Input {
                    regions: &project.regions,
                    palette: &palette,
                    identity,
                    world: source::world_extent(&root)?,
                    build_class: BuildClass::NonProduction,
                    draft_areas: oteryn_world_bundle_compiler::project::draft_areas(
                        &project.index,
                    )?,
                    families: &project.families,
                },
                &registry,
            )?
            .bytes
        };
        let pin: serde_json::Value =
            serde_json::from_slice(&std::fs::read(root.join("content/world/pins/oteryn.json"))?)?;
        let expected_digest: [u8; 32] = bytes[bytes.len() - 32..].try_into()?;
        let digest: String = expected_digest.iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(
            Some(digest.as_str()),
            pin["digest"].as_str(),
            "actual shipped default bundle"
        );
        assert!(bytes.len() > 30_000_000);
        let bases = qualify_bundle_source_bases(&bytes)?;
        assert_eq!(
            bases
                .published_world
                .as_ref()
                .map(PublishedWorldBundleBasis::bundle_digest),
            Some(expected_digest)
        );
        assert!(
            qualify_bundle_source_bases(&bytes[..bytes.len() - 1]).is_err(),
            "full artifact validation is required"
        );
        let qualified = bases.callbacks;
        for source in [
            PinnedActivitySourceRoom::CallbackAnomaly,
            PinnedActivitySourceRoom::CallbackLadyTenebris,
            PinnedActivitySourceRoom::CallbackMazoran,
            PinnedActivitySourceRoom::CallbackOutburst,
            PinnedActivitySourceRoom::CallbackPlagirath,
            PinnedActivitySourceRoom::CallbackProfessorMaxxen,
            PinnedActivitySourceRoom::CallbackRagiaz,
            PinnedActivitySourceRoom::CallbackRazzagorn,
            PinnedActivitySourceRoom::CallbackShulgrax,
            PinnedActivitySourceRoom::CallbackTimeGuardian,
            PinnedActivitySourceRoom::CallbackZamulosh,
        ] {
            assert!(
                qualified
                    .iter()
                    .any(|basis| basis.source() == source
                        && basis.bundle_digest() == expected_digest),
                "actual default map must prove {source:?}"
            );
        }
        eprintln!(
            "SRX3_DEFAULT_BUILT_BUNDLE_QUALIFICATION_EXECUTED duration={:?}",
            start.elapsed()
        );
        Ok(())
    }
}

/// Read-only conversion data from the actual World loader's legacy-z frame.
/// Copying it grants no map, rule, actor or content authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PublishedWorldSourceFrame {
    _sealed: (),
}
impl PublishedWorldSourceFrame {
    pub fn physical_position(self, at: [i32; 3]) -> Option<(i32, i32, i16)> {
        let [x, y, z] = at;
        let floor = i16::try_from(z).ok()?;
        self.source_position((x, y, floor))?;
        Some((x, y, floor))
    }
    pub fn source_position(self, at: (i32, i32, i16)) -> Option<[i32; 3]> {
        let (x, y, z) = at;
        (u16::try_from(x).is_ok() && u16::try_from(y).is_ok() && (0..=15).contains(&z)).then_some([
            x,
            y,
            i32::from(z),
        ])
    }
}
#[derive(Debug, Clone)]
pub struct QualifiedPublishedWorldContext {
    index: crate::content::native_cell_lookup::BundleCollisionIndex,
    world: oteryn_protocol_oteryn::WorldId,
    activation: u64,
    server: [u8; 32],
    client: [u8; 32],
    frame: [u8; 32],
    map: [u8; 32],
}
impl QualifiedPublishedWorldContext {
    pub const fn source_frame(&self) -> PublishedWorldSourceFrame {
        PublishedWorldSourceFrame { _sealed: () }
    }
    pub fn matches(
        &self,
        room: &crate::content::QualifiedNativeEntryRoom,
        pin: &crate::content::channel_content_pin::ChannelContentPin,
    ) -> bool {
        let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(index) =
            room.movement_cells().index()
        else {
            return false;
        };
        std::sync::Arc::ptr_eq(&self.index.0, &index.0)
            && self.world == pin.world_id()
            && self.activation == pin.activation_sequence()
            && self.server == pin.server_artifact_digest()
            && self.client == pin.client_artifact_digest()
            && self.frame == pin.frame_binding_digest()
            && self.map == pin.map_revision_digest()
            && room.frame_binding().digest() == self.frame
            && room.map_revision_digest() == self.map
            && room.compiled().server_digest() == self.server
            && room.compiled().client_digest() == self.client
    }
}
impl crate::content::ActiveGeneration {
    pub fn qualify_published_world_context(
        &self,
        room: &crate::content::QualifiedNativeEntryRoom,
        pin: &crate::content::channel_content_pin::ChannelContentPin,
    ) -> Option<QualifiedPublishedWorldContext> {
        let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(index) =
            room.movement_cells().index()
        else {
            return None;
        };
        let digest = index
            .0
            .source_bases()?
            .published_world
            .as_ref()?
            .bundle_digest();
        self.qualify_current_bundle_context(room, pin, index, digest)
    }
    fn qualify_current_bundle_context(
        &self,
        room: &crate::content::QualifiedNativeEntryRoom,
        pin: &crate::content::channel_content_pin::ChannelContentPin,
        index: &crate::content::native_cell_lookup::BundleCollisionIndex,
        digest: [u8; 32],
    ) -> Option<QualifiedPublishedWorldContext> {
        let scope = room.movement_cells().scope();
        if scope.coordinate_frame.as_str() != "oteryn:frame/world-bundle-v3"
            || scope.world_id != pin.world_id()
            || scope.generation_digest != self.identity().server_artifact_digest()
            || self.activation_sequence() != pin.activation_sequence()
            || self.identity().server_artifact_digest() != pin.server_artifact_digest()
            || self.identity().client_artifact_digest() != pin.client_artifact_digest()
        {
            return None;
        }
        let digest = digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let revision = format!("sha256:{digest}");
        if scope.map_revision.as_str() != revision {
            return None;
        }
        let mut frame = b"OTERYN_WORLD_BUNDLE_FRAME_BINDING/v1\0".to_vec();
        frame.extend_from_slice(pin.world_id().as_bytes());
        for part in [scope.coordinate_frame.as_str(), revision.as_str()] {
            frame.extend_from_slice(&(part.len() as u64).to_be_bytes());
            frame.extend_from_slice(part.as_bytes());
        }
        if crate::content::digest::sha256(&frame) != pin.frame_binding_digest()
            || crate::content::digest::sha256(revision.as_bytes()) != pin.map_revision_digest()
        {
            return None;
        }
        let proof = QualifiedPublishedWorldContext {
            index: index.clone(),
            world: pin.world_id(),
            activation: pin.activation_sequence(),
            server: pin.server_artifact_digest(),
            client: pin.client_artifact_digest(),
            frame: pin.frame_binding_digest(),
            map: pin.map_revision_digest(),
        };
        proof.matches(room, pin).then_some(proof)
    }
}

/// Source geometry proof for one closed callback recipe on the actual current
/// Bundle map. Current mutable facts and callback registration remain separate.
#[derive(Debug, Clone)]
pub struct QualifiedCallbackSourceContext {
    identity: QualifiedPublishedWorldContext,
    source: PinnedActivitySourceRoom,
}
impl QualifiedCallbackSourceContext {
    pub const fn source(&self) -> PinnedActivitySourceRoom {
        self.source
    }
    pub const fn source_frame(&self) -> PublishedWorldSourceFrame {
        self.identity.source_frame()
    }
    pub fn matches(
        &self,
        room: &crate::content::QualifiedNativeEntryRoom,
        pin: &crate::content::channel_content_pin::ChannelContentPin,
    ) -> bool {
        self.identity.matches(room, pin)
    }
}
impl crate::content::ActiveGeneration {
    pub fn qualify_callback_source_context(
        &self,
        room: &crate::content::QualifiedNativeEntryRoom,
        pin: &crate::content::channel_content_pin::ChannelContentPin,
        source: PinnedActivitySourceRoom,
    ) -> Option<QualifiedCallbackSourceContext> {
        if !source.is_callback_context() {
            return None;
        }
        let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(index) =
            room.movement_cells().index()
        else {
            return None;
        };
        let basis = index
            .0
            .source_bases()?
            .callbacks
            .iter()
            .find(|basis| basis.source() == source)?;
        let native = self.native_gameplay()?;
        if !basis.protects_item_references(native.item_index()?) {
            return None;
        }
        let identity =
            self.qualify_current_bundle_context(room, pin, index, basis.bundle_digest())?;
        Some(QualifiedCallbackSourceContext { identity, source })
    }
}
