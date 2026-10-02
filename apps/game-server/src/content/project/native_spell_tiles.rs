//! Spell tile semantics authored in the qualified native WorldProject overlay.
//!
//! Collision is not a substitute for these fields. An omitted document is unknown,
//! never an implicit floor, empty stack or false flag. Source item ids are usable
//! only through the project's exact admitted source-identity bindings. This module
//! does not activate a generation or grant entry/movement authority.

use super::{ProjectError, ProjectV2Family, ProjectV2SourceIdentityBinding, WorldProject};
use crate::content::static_cell_engine::EngineeringStaticCellScope;
use crate::content::{FirstProductionCell, LogicalCell};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const NATIVE_SPELL_TILE_SCHEMA: &str = "OTERYN_NATIVE_SPELL_TILES/v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeSpellTileDocument {
    pub schema: String,
    pub tiles: Vec<NativeSpellTileRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeSpellTileRecord {
    /// The exact placed cell in the same admitted native overlay.
    pub cell_key: String,
    /// None means explicitly authored absence of ground, not unavailable metadata.
    #[serde(deserialize_with = "required_option")]
    pub ground: Option<NativeSpellTileGround>,
    /// Explicit None retains unknown speed; Some(0) is a known zero value.
    #[serde(deserialize_with = "required_option")]
    pub ground_speed: Option<u16>,
    pub top_items: Vec<NativeSpellTileItem>,
    /// All fields are required. No collision-derived defaults are applied.
    pub flags: NativeSpellTileFlags,
}

/// Explicit native terrain binding; a legacy numeric alias is optional and may
/// only be used if the same admitted project binds it to this Terrain identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeSpellTileGround {
    pub terrain_key: String,
    #[serde(deserialize_with = "required_option")]
    pub source_binding: Option<ProjectV2SourceIdentityBinding>,
}

fn required_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeSpellTileItem {
    /// Must equal an existing source binding verbatim. The binding's target must
    /// be an admitted Item definition and external id a canonical positive u32.
    pub source_binding: ProjectV2SourceIdentityBinding,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeSpellTileFlags {
    pub block_solid: bool,
    pub block_projectile: bool,
    pub immovable_block_solid: bool,
    pub immovable_block_item: bool,
    pub immovable_nonfield_block_item: bool,
    pub floor_change: bool,
    pub protection_zone: bool,
}

/// Fields are private so arbitrary caller snapshots cannot masquerade as
/// source-qualified tile semantics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QualifiedSpellTile {
    ground_present: bool,
    ground_source_id: Option<u32>,
    ground_speed: Option<u16>,
    top_source_ids: Vec<u32>,
    flags: NativeSpellTileFlags,
    source_step: Option<QualifiedSourceStepTile>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum SourceFloorChange {
    Down,
    East,
    Eastalt,
    North,
    South,
    Southalt,
    West,
}
/// Explicit source-loader facts. Missing means unknown, not height0/no stairs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct QualifiedSourceStepTile {
    pub(crate) height_count: u16,
    pub(crate) floor_changes: Vec<SourceFloorChange>,
    no_pvp_zone: bool,
    pvp_zone: bool,
}

impl QualifiedSpellTile {
    pub(super) fn from_source_world(
        ground: Option<u32>,
        ground_speed: Option<u16>,
        top_source_ids: Vec<u32>,
        flags: NativeSpellTileFlags,
        height_count: u16,
        floor_changes: Vec<SourceFloorChange>,
        no_pvp_zone: bool,
        pvp_zone: bool,
    ) -> Self {
        Self {
            ground_present: ground.is_some(),
            ground_source_id: ground,
            ground_speed,
            top_source_ids,
            flags,
            source_step: Some(QualifiedSourceStepTile {
                height_count,
                floor_changes,
                no_pvp_zone,
                pvp_zone,
            }),
        }
    }
    pub(crate) const fn ground_present(&self) -> bool {
        self.ground_present
    }
    pub(crate) const fn ground_speed(&self) -> Option<u16> {
        self.ground_speed
    }
    pub(crate) const fn ground_source_id(&self) -> Option<u32> {
        self.ground_source_id
    }
    pub(crate) fn top_source_ids(&self) -> &[u32] {
        &self.top_source_ids
    }
    pub(crate) const fn flags(&self) -> NativeSpellTileFlags {
        self.flags
    }
    pub(crate) fn no_pvp_zone(&self) -> Option<bool> {
        self.source_step.as_ref().map(|facts| facts.no_pvp_zone)
    }
    pub(crate) fn pvp_zone(&self) -> Option<bool> {
        self.source_step.as_ref().map(|facts| facts.pvp_zone)
    }
    pub(crate) fn source_step(&self) -> Option<&QualifiedSourceStepTile> {
        self.source_step.as_ref()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SpellTileLookupError {
    ScopeMismatch,
    Unknown,
    Absent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QualifiedSpellTiles {
    scope: EngineeringStaticCellScope,
    /// None retains unknown semantics for the existing entry-r1 source.
    tiles: Option<BTreeMap<LogicalCell, QualifiedSpellTile>>,
    unknown_cells: BTreeSet<LogicalCell>,
    source_bounds: Option<[i32; 6]>,
}

impl QualifiedSpellTiles {
    pub(super) fn from_source_world(
        world: &super::native_spell_world::QualifiedNativeSpellWorld,
    ) -> Self {
        let mut tiles = BTreeMap::new();
        let mut unknown_cells = BTreeSet::new();
        for (cell, tile) in world.qualified_cells() {
            match tile {
                Some(tile) => {
                    tiles.insert(*cell, tile.clone());
                }
                None => {
                    unknown_cells.insert(*cell);
                }
            }
        }
        Self {
            scope: world.scope().clone(),
            tiles: Some(tiles),
            unknown_cells,
            source_bounds: Some(world.native_bounds()),
        }
    }
    /// Only the native project qualification path can create this index. The
    /// caller must include this document in its canonical package provenance,
    /// before binding the compiled server generation to `scope`.
    pub(super) fn qualify(
        project: &WorldProject,
        cells: &[FirstProductionCell],
        scope: EngineeringStaticCellScope,
        document: Option<&NativeSpellTileDocument>,
    ) -> Result<Self, ProjectError> {
        let source = project.lower_reference_source()?;
        if source.world_id != scope.world_id
            || source.coordinate_frame != scope.coordinate_frame
            || source.content_lock != scope.content_lock
        {
            return Err(ProjectError::InvalidProject(
                "native spell tiles scope does not match the admitted project",
            ));
        }
        let Some(document) = document else {
            return Ok(Self {
                scope,
                tiles: None,
                unknown_cells: BTreeSet::new(),
                source_bounds: None,
            });
        };
        if document.schema != NATIVE_SPELL_TILE_SCHEMA {
            return Err(ProjectError::InvalidProject(
                "native spell tile schema mismatch",
            ));
        }
        if document.tiles.len() != cells.len() {
            return Err(ProjectError::InvalidProject(
                "native spell tiles must cover every placed cell",
            ));
        }
        let state = project.v2().ok_or(ProjectError::InvalidProject(
            "native spell tiles require v2 source bindings",
        ))?;
        let qualify_binding = |binding: &ProjectV2SourceIdentityBinding,
                               family: ProjectV2Family|
         -> Result<u32, ProjectError> {
            if binding.target.family != family || !state.source_identity_bindings.contains(binding)
            {
                return Err(ProjectError::InvalidProject(
                    "native spell tile has no exact source binding to the expected definition family",
                ));
            }
            let value = binding
                .external_id
                .parse::<u32>()
                .ok()
                .filter(|id| *id != 0 && id.to_string() == binding.external_id)
                .ok_or(ProjectError::InvalidProject(
                    "native spell tile item id is not a canonical positive u32",
                ))?;
            // Same numeric ids from different namespaces are not interchangeable.
            if binding.identity_namespace != "ots/item_server_id" {
                return Err(ProjectError::InvalidProject(
                    "native spell tile source namespace is not ots/item_server_id",
                ));
            }
            Ok(value)
        };
        let mut tiles = BTreeMap::new();
        let mut keys = BTreeSet::new();
        for record in &document.tiles {
            if !keys.insert(&record.cell_key) {
                return Err(ProjectError::InvalidProject(
                    "duplicate native spell tile cell",
                ));
            }
            let source = cells
                .iter()
                .find(|cell| cell.key.as_str() == record.cell_key)
                .ok_or(ProjectError::InvalidProject(
                    "native spell tile does not name a placed cell",
                ))?;
            if record.ground.is_none() && record.ground_speed.is_some() {
                return Err(ProjectError::InvalidProject(
                    "ground speed requires explicitly authored ground",
                ));
            }
            let ground_source_id = if let Some(ground) = &record.ground {
                if ground.terrain_key != source.terrain_key.as_str() {
                    return Err(ProjectError::InvalidProject(
                        "native spell ground does not bind the placed Terrain",
                    ));
                }
                ground
                    .source_binding
                    .as_ref()
                    .map(|binding| {
                        if binding.target.key != ground.terrain_key {
                            return Err(ProjectError::InvalidProject(
                                "native spell ground alias targets a different Terrain",
                            ));
                        }
                        qualify_binding(binding, ProjectV2Family::Terrain)
                    })
                    .transpose()?
            } else {
                None
            };
            let tile = QualifiedSpellTile {
                ground_present: record.ground.is_some(),
                ground_speed: record.ground_speed,
                ground_source_id,
                top_source_ids: record
                    .top_items
                    .iter()
                    .map(|item| qualify_binding(&item.source_binding, ProjectV2Family::Item))
                    .collect::<Result<_, _>>()?,
                flags: record.flags,
                source_step: None,
            };
            let cell = LogicalCell {
                x: source.x,
                y: source.y,
                z: i32::from(source.z),
            };
            if tiles.insert(cell, tile).is_some() {
                return Err(ProjectError::InvalidProject(
                    "native spell tile positions collide",
                ));
            }
        }
        Ok(Self {
            scope,
            tiles: Some(tiles),
            unknown_cells: BTreeSet::new(),
            source_bounds: None,
        })
    }

    /// The scope is independently checked against the current runtime pin by the
    /// Movement owner before lookup. Matching these stored bytes alone is not
    /// evidence that the generation remains active.
    pub(crate) fn lookup(
        &self,
        active_scope: &EngineeringStaticCellScope,
        cell: LogicalCell,
    ) -> Result<&QualifiedSpellTile, SpellTileLookupError> {
        if active_scope != &self.scope {
            return Err(SpellTileLookupError::ScopeMismatch);
        }
        if self.unknown_cells.contains(&cell)
            || self
                .source_bounds
                .is_some_and(|[min_x, min_y, max_x, max_y, min_z, max_z]| {
                    cell.x < min_x
                        || cell.x > max_x
                        || cell.y < min_y
                        || cell.y > max_y
                        || cell.z < min_z
                        || cell.z > max_z
                })
        {
            return Err(SpellTileLookupError::Unknown);
        }
        self.tiles
            .as_ref()
            .ok_or(SpellTileLookupError::Unknown)?
            .get(&cell)
            .ok_or(SpellTileLookupError::Absent)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use crate::content::{native_entry_room_documents, qualify_native_entry_room};
    use crate::foundation::WorldId;
    use serde_json::json;

    fn world() -> WorldId {
        WorldId::decode(&[0, 0, 0, 0, 0, 1, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 1]).unwrap()
    }

    #[test]
    fn omitted_source_metadata_is_unknown_not_a_walkable_floor() {
        let room = qualify_native_entry_room(world()).unwrap();
        let documents = native_entry_room_documents(world()).unwrap();
        let project = super::super::ProjectSnapshot::new(
            documents.documents().clone(),
            super::super::native_entry_first_slice_limits().project,
        )
        .unwrap()
        .parse_native_entry()
        .unwrap();
        let tiles = QualifiedSpellTiles::qualify(
            project.project(),
            &project.source().cells,
            room.movement_cells().scope().clone(),
            None,
        )
        .unwrap();
        assert_eq!(
            tiles.lookup(
                room.movement_cells().scope(),
                LogicalCell { x: 0, y: 0, z: 0 }
            ),
            Err(SpellTileLookupError::Unknown)
        );
        let mut foreign = room.movement_cells().scope().clone();
        foreign.generation_digest = [9; 32];
        assert_eq!(
            tiles.lookup(&foreign, LogicalCell { x: 0, y: 0, z: 0 }),
            Err(SpellTileLookupError::ScopeMismatch)
        );
    }

    #[test]
    fn proposed_duplicate_cells_and_unbound_source_ids_refuse() {
        let room = qualify_native_entry_room(world()).unwrap();
        let documents = native_entry_room_documents(world()).unwrap();
        let project = super::super::ProjectSnapshot::new(
            documents.documents().clone(),
            super::super::native_entry_first_slice_limits().project,
        )
        .unwrap()
        .parse_native_entry()
        .unwrap();
        let flags = serde_json::from_value(json!({"block_solid":false,"block_projectile":false,
            "immovable_block_solid":false,"immovable_block_item":false,
            "immovable_nonfield_block_item":false,"floor_change":false,"protection_zone":false}))
        .unwrap();
        let mut document = NativeSpellTileDocument {
            schema: NATIVE_SPELL_TILE_SCHEMA.into(),
            tiles: project
                .source()
                .cells
                .iter()
                .map(|cell| NativeSpellTileRecord {
                    cell_key: cell.key.as_str().into(),
                    ground: None,
                    ground_speed: None,
                    top_items: vec![],
                    flags,
                })
                .collect(),
        };
        // These are rejected proposed documents, never authority fixtures for a cast.
        document.tiles[1].cell_key = document.tiles[0].cell_key.clone();
        assert!(
            QualifiedSpellTiles::qualify(
                project.project(),
                &project.source().cells,
                room.movement_cells().scope().clone(),
                Some(&document)
            )
            .is_err()
        );
        document.tiles[1].cell_key = project.source().cells[1].key.as_str().into();
        document.tiles[0].ground = Some(NativeSpellTileGround {
            terrain_key: project.source().cells[0].terrain_key.as_str().into(),
            source_binding: Some(
                serde_json::from_value(json!({
                    "source_key":"oteryn:source/canary-items", "source_revision":"source-pin",
                    "identity_namespace":"ots/item_server_id", "external_id":"386",
                    "target":{"family":"Terrain","key":"oteryn:terrain/stone-floor",
                        "revision":"oteryn:rev/entry-r1"}, "disposition":"EXACT"
                }))
                .unwrap(),
            ),
        });
        assert!(
            QualifiedSpellTiles::qualify(
                project.project(),
                &project.source().cells,
                room.movement_cells().scope().clone(),
                Some(&document)
            )
            .is_err()
        );
    }

    #[test]
    fn flags_are_required_and_unknown_fields_refuse() {
        let incomplete = json!({"cell_key":"oteryn:cell/entry-start","ground":null,
            "top_items":[], "flags":{"block_solid":false}});
        assert!(serde_json::from_value::<NativeSpellTileRecord>(incomplete).is_err());
        let complete = json!({"block_solid":false,"block_projectile":false,
            "immovable_block_solid":false,"immovable_block_item":false,
            "immovable_nonfield_block_item":false,"floor_change":false,"protection_zone":false});
        assert!(serde_json::from_value::<NativeSpellTileFlags>(complete.clone()).is_ok());
        let mut unknown = complete;
        unknown["rope_spot"] = json!(true);
        assert!(serde_json::from_value::<NativeSpellTileFlags>(unknown).is_err());
    }
}
