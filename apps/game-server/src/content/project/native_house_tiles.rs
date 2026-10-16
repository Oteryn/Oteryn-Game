//! Explicit candidate physical House placement in the same qualified native map.
//! Omission is Unknown; neither catalogue membership nor custody items imply that
//! the current actor stands in a House. World-global House IDs remain channel-free.
use super::{
    ProjectError, ProjectV2Declaration, ProjectV2DefinitionRef, ProjectV2Family, WorldProject,
};
use crate::content::static_cell_engine::EngineeringStaticCellScope;
use crate::content::{FirstProductionCell, LogicalCell};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
/// D1241 canonical opaque placement bytes, extracted exactly from death_reward:
/// signed x i32BE || signed y i32BE || floor i16BE. No frame conversion or clamp.
pub(crate) fn encode_spatial_position(
    position: crate::foundation::MovementLocalPosition,
) -> Vec<u8> {
    let mut spatial_position = Vec::with_capacity(10);
    spatial_position.extend_from_slice(&position.x.to_be_bytes());
    spatial_position.extend_from_slice(&position.y.to_be_bytes());
    spatial_position.extend_from_slice(&position.floor.to_be_bytes());
    spatial_position
}

/// Checked conversion into the actual movement carrier. x/y are already i32;
/// LogicalCell z must fit the carrier's i16 floor. Source frame qualification
/// remains the caller's responsibility; this changes neither floor sign nor origin.
pub(crate) fn checked_movement_position(
    cell: LogicalCell,
) -> Option<crate::foundation::MovementLocalPosition> {
    Some(crate::foundation::MovementLocalPosition {
        x: cell.x,
        y: cell.y,
        floor: i16::try_from(cell.z).ok()?,
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeHouseTileDocument {
    pub schema: String,
    pub houses: Vec<NativeHousePlacement>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeHousePlacement {
    pub house: ProjectV2DefinitionRef,
    pub cells: Vec<String>,
    pub entry_cell: String,
    /// Source door numbers are scoped to one House; native physical cells are
    /// already qualified by this map. No unqualified source coordinates survive.
    pub doors: BTreeMap<u32, String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QualifiedHouseTiles {
    scope: EngineeringStaticCellScope,
    houses: Option<Vec<QualifiedHousePlacement>>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QualifiedHousePlacement {
    house: ProjectV2DefinitionRef,
    cells: BTreeSet<LogicalCell>,
    entry: LogicalCell,
    doors: BTreeMap<LogicalCell, u32>,
}
impl QualifiedHousePlacement {
    pub(crate) fn house(&self) -> &ProjectV2DefinitionRef {
        &self.house
    }
    pub(crate) fn contains(&self, cell: LogicalCell) -> bool {
        self.cells.contains(&cell)
    }
    pub(crate) fn entry(&self) -> LogicalCell {
        self.entry
    }
    pub(crate) fn has_door(&self, id: u32) -> bool {
        self.doors.values().any(|number| *number == id)
    }
    pub(crate) fn door_at(&self, cell: LogicalCell) -> Option<u32> {
        self.doors.get(&cell).copied()
    }
}
impl QualifiedHouseTiles {
    /// Explicit Unknown for this actual owner's scope; never inherit an entry catalogue.
    pub(super) fn unknown_for_scope(scope: EngineeringStaticCellScope) -> Self {
        Self {
            scope,
            houses: None,
        }
    }

    /// D1245 canonical placement identity only, never current activation/0047 authority.
    /// Unknown metadata has no digest; explicit proven-empty metadata has a digest.
    /// SHA256(domain/version,house-count u64BE, houses sorted by key/revision).
    /// Each house: length-prefixed UTF8 key/revision, cell-count u64BE + sorted
    /// (x,y,z)i32BE cells, entry(x,y,z)i32BE, door-count u64BE + sorted
    /// (cell(x,y,z)i32BE,door-number u32BE). Counts/string lengths are u64BE.
    pub(crate) fn placement_digest(&self) -> Option<[u8; 32]> {
        let houses = self.houses.as_ref()?;
        let mut ordered: Vec<_> = houses.iter().collect();
        ordered.sort_by(|a, b| {
            (&a.house.key, &a.house.revision).cmp(&(&b.house.key, &b.house.revision))
        });
        let mut hash = Sha256::new();
        hash.update(b"OTERYN_QUALIFIED_HOUSE_PLACEMENTS/v1\0");
        hash.update((ordered.len() as u64).to_be_bytes());
        let cell = |hash: &mut Sha256, position: LogicalCell| {
            hash.update(position.x.to_be_bytes());
            hash.update(position.y.to_be_bytes());
            hash.update(position.z.to_be_bytes());
        };
        for placement in ordered {
            for text in [&placement.house.key, &placement.house.revision] {
                hash.update((text.len() as u64).to_be_bytes());
                hash.update(text.as_bytes());
            }
            hash.update((placement.cells.len() as u64).to_be_bytes());
            for position in &placement.cells {
                cell(&mut hash, *position);
            }
            cell(&mut hash, placement.entry);
            hash.update((placement.doors.len() as u64).to_be_bytes());
            for (position, number) in &placement.doors {
                cell(&mut hash, *position);
                hash.update(number.to_be_bytes());
            }
        }
        Some(hash.finalize().into())
    }

    pub(super) fn from_source_world(
        world: &super::native_spell_world::QualifiedNativeSpellWorld,
    ) -> Self {
        // The strict source-world qualifier rejects any House tile in this exact
        // captured region. This is proven absence, not a fake allocated House.
        Self {
            scope: world.scope().clone(),
            houses: Some(Vec::new()),
        }
    }
    pub(super) fn qualify(
        project: &WorldProject,
        cells: &[FirstProductionCell],
        scope: EngineeringStaticCellScope,
        document: Option<&NativeHouseTileDocument>,
    ) -> Result<Self, ProjectError> {
        let invalid = || ProjectError::InvalidProject("invalid native House placement");
        let Some(document) = document else {
            return Ok(Self::unknown_for_scope(scope));
        };
        if document.schema != "OTERYN_NATIVE_HOUSE_TILES/v1" || document.houses.len() > 1024 {
            return Err(invalid());
        }
        let state = project.v2().ok_or_else(invalid)?;
        let source = project.lower_reference_source()?;
        if source.world_id != scope.world_id
            || source.coordinate_frame != scope.coordinate_frame
            || source.content_lock != scope.content_lock
        {
            return Err(invalid());
        }
        let position = |key: &str| -> Result<LogicalCell, ProjectError> {
            let c = cells
                .iter()
                .find(|c| {
                    c.key.as_str() == key && c.collision == crate::content::CollisionClass::Walkable
                })
                .ok_or_else(invalid)?;
            Ok(LogicalCell {
                x: c.x,
                y: c.y,
                z: i32::from(c.z),
            })
        };
        let mut occupied = BTreeSet::new();
        let mut identities = BTreeSet::new();
        let mut houses = Vec::new();
        for authored in &document.houses {
            if authored.house.family!=ProjectV2Family::House || !authored.house.key.starts_with("oteryn:content.house.")
                || !identities.insert(&authored.house.key) || authored.cells.is_empty() || authored.cells.len()>4096 || authored.doors.len()>1024
                || !state.declarations.iter().any(|d|matches!(d,ProjectV2Declaration::House{identity,..} if identity.key==authored.house.key && identity.revision==authored.house.revision)) { return Err(invalid()); }
            let mut placed = BTreeSet::new();
            for key in &authored.cells {
                let cell = position(key)?;
                if !placed.insert(cell) || !occupied.insert(cell) {
                    return Err(invalid());
                }
            }
            let entry = position(&authored.entry_cell)?;
            if placed.contains(&entry) {
                return Err(invalid());
            }
            let mut doors = BTreeMap::new();
            for (number, key) in &authored.doors {
                let cell = position(key)?;
                if *number == 0 || !placed.contains(&cell) || doors.insert(cell, *number).is_some()
                {
                    return Err(invalid());
                }
            }
            houses.push(QualifiedHousePlacement {
                house: authored.house.clone(),
                cells: placed,
                entry,
                doors,
            });
        }
        Ok(Self {
            scope,
            houses: Some(houses),
        })
    }
    pub(crate) fn house_at(
        &self,
        scope: &EngineeringStaticCellScope,
        cell: LogicalCell,
    ) -> Result<Option<&QualifiedHousePlacement>, &'static str> {
        if scope != &self.scope {
            return Err("House map scope mismatch");
        }
        let houses = self
            .houses
            .as_ref()
            .ok_or("House placement metadata unavailable")?;
        Ok(houses.iter().find(|house| house.contains(cell)))
    }
}
#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    #[test]
    fn omission_retains_unknown_on_actual_accepted_room() {
        let world = crate::foundation::WorldId::decode(&[
            0x01, 0x9a, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 1,
        ])
        .unwrap();
        let room = super::super::qualify_native_entry_room(world).unwrap();
        let cells = room.movement_cells();
        let start = room.entry_start();
        assert_eq!(
            cells.house_tiles().house_at(
                cells.scope(),
                LogicalCell {
                    x: start.x,
                    y: start.y,
                    z: i32::from(start.floor)
                }
            ),
            Err("House placement metadata unavailable")
        );
    }
}

#[cfg(test)]
mod spatial_position_encoding_tests {
    use super::{checked_movement_position, encode_spatial_position};
    use crate::content::LogicalCell;

    #[test]
    fn canonical_placement_bytes_preserve_signed_axes_and_check_floor_narrowing()
    -> Result<(), &'static str> {
        let position = checked_movement_position(LogicalCell {
            x: -2,
            y: 0x01020304,
            z: -7,
        })
        .ok_or("valid signed position")?;
        assert_eq!(
            encode_spatial_position(position),
            [0xff, 0xff, 0xff, 0xfe, 1, 2, 3, 4, 0xff, 0xf9]
        );
        for z in [i32::from(i16::MIN), i32::from(i16::MAX)] {
            let position = checked_movement_position(LogicalCell {
                x: i32::MIN,
                y: i32::MAX,
                z,
            })
            .ok_or("floor boundary")?;
            assert_eq!(position.x, i32::MIN);
            assert_eq!(position.y, i32::MAX);
            assert_eq!(i32::from(position.floor), z);
        }
        for z in [
            i32::MIN,
            i32::from(i16::MIN) - 1,
            i32::from(i16::MAX) + 1,
            i32::MAX,
        ] {
            assert!(checked_movement_position(LogicalCell { x: 0, y: 0, z }).is_none());
        }
        Ok(())
    }
}

#[cfg(test)]
mod placement_digest_tests {
    use super::super::{
        CanonicalProjectDocuments, NativeFirstEntryDocument, ProjectSnapshot, ProjectV2Identity,
        native_entry_first_slice_limits,
    };
    use super::*;
    type TestResult<T> = Result<T, Box<dyn std::error::Error>>;

    fn world() -> TestResult<crate::foundation::WorldId> {
        Ok(crate::foundation::WorldId::decode(&[
            1, 0, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 1,
        ])?)
    }
    fn document() -> NativeHouseTileDocument {
        NativeHouseTileDocument {
            schema: "OTERYN_NATIVE_HOUSE_TILES/v1".into(),
            houses: vec![
                NativeHousePlacement {
                    house: ProjectV2DefinitionRef {
                        family: ProjectV2Family::House,
                        key: "oteryn:content.house.fixture_a".into(),
                        revision: "oteryn:rev/fixture-house-r1".into(),
                    },
                    cells: vec![
                        "oteryn:cell/entry-east".into(),
                        "oteryn:cell/entry-door".into(),
                    ],
                    entry_cell: "oteryn:cell/entry-start".into(),
                    doors: [(1, "oteryn:cell/entry-door".into())].into_iter().collect(),
                },
                NativeHousePlacement {
                    house: ProjectV2DefinitionRef {
                        family: ProjectV2Family::House,
                        key: "oteryn:content.house.fixture_b".into(),
                        revision: "oteryn:rev/fixture-house-r1".into(),
                    },
                    cells: vec![
                        "oteryn:cell/entry-den".into(),
                        "oteryn:cell/entry-den-north".into(),
                    ],
                    entry_cell: "oteryn:cell/entry-start".into(),
                    doors: BTreeMap::new(),
                },
            ],
        }
    }
    fn qualify_overlay(
        document: Option<NativeHouseTileDocument>,
    ) -> TestResult<QualifiedHouseTiles> {
        // Test-only authored overlay, not a House allocation or current authority witness.
        let base = crate::content::native_entry_room_documents(world()?)?;
        let baseline = ProjectSnapshot::new(
            base.documents().clone(),
            native_entry_first_slice_limits().project,
        )?
        .parse_native_entry()?;
        let mut draft = baseline.project().migrate_to_v2();
        if let Some(document) = &document {
            for house in &document.houses {
                draft.state.declarations.push(ProjectV2Declaration::House {
                    identity: ProjectV2Identity {
                        key: house.house.key.clone(),
                        revision: house.house.revision.clone(),
                    },
                    fields: Vec::new(),
                });
            }
        }
        let raw: serde_json::Value =
            serde_json::from_slice(super::super::NATIVE_ENTRY_ROOM_SOURCE)?;
        let mut overlay: NativeFirstEntryDocument = serde_json::from_value(
            raw.get("native_first_entry")
                .ok_or("missing actual overlay")?
                .clone(),
        )?;
        overlay.house_tiles = document.clone();
        // Canonical serialization and parse_native_entry run first. Its qualify_for
        // automatically validates movement_cells at the compiled generation. The
        // returned test catalogue is separately requalified below from this parsed
        // source and its derived scope; no selected-manifest producer is claimed.
        let canonical = CanonicalProjectDocuments::from_native_entry_draft(draft, overlay)?;
        let project = ProjectSnapshot::new(
            canonical.documents().clone(),
            native_entry_first_slice_limits().project,
        )?
        .parse_native_entry()?;
        let source = project.source();
        let reference = project.project().lower_reference_source()?;
        let compiled = crate::content::compile_first_production(
            source,
            crate::content::FirstProductionCompileTarget::OrdinaryRelease,
        )?;
        let scope = EngineeringStaticCellScope {
            world_id: source.world_id,
            coordinate_frame: reference.coordinate_frame,
            map_revision: crate::content::MapRevisionRef::new(source.revisions.map.as_str())?,
            generation_digest: compiled.server_digest(),
            content_lock: source.content_lock.clone(),
        };
        Ok(QualifiedHouseTiles::qualify(
            project.project(),
            &source.cells,
            scope,
            document.as_ref(),
        )?)
    }
    #[test]
    fn qualified_overlay_digest_is_order_independent_and_definition_sensitive() -> TestResult<()> {
        let source = document();
        let original = qualify_overlay(Some(source.clone()))?
            .placement_digest()
            .ok_or("qualified digest")?;
        let mut reordered = source.clone();
        reordered.houses.reverse();
        for house in &mut reordered.houses {
            house.cells.reverse();
        }
        assert_eq!(
            qualify_overlay(Some(reordered))?.placement_digest(),
            Some(original)
        );
        let mut revised = source.clone();
        revised.houses[0].house.revision = "oteryn:rev/fixture-house-r2".into();
        assert_ne!(
            qualify_overlay(Some(revised))?.placement_digest(),
            Some(original)
        );
        let mut changed_cells = source.clone();
        changed_cells.houses[1].cells.pop();
        assert_ne!(
            qualify_overlay(Some(changed_cells))?.placement_digest(),
            Some(original)
        );
        let mut changed_key = source.clone();
        changed_key.houses[0].house.key = "oteryn:content.house.fixture_c".into();
        assert_ne!(
            qualify_overlay(Some(changed_key))?.placement_digest(),
            Some(original)
        );
        let mut changed_entry = source.clone();
        changed_entry.houses[0].entry_cell = "oteryn:cell/entry-den".into();
        assert_ne!(
            qualify_overlay(Some(changed_entry))?.placement_digest(),
            Some(original)
        );
        let mut changed_door = source;
        changed_door.houses[0].doors = [(2, "oteryn:cell/entry-door".into())].into_iter().collect();
        assert_ne!(
            qualify_overlay(Some(changed_door))?.placement_digest(),
            Some(original)
        );
        Ok(())
    }
    #[test]
    fn unknown_metadata_differs_from_explicit_empty_and_invalid_overlay_refuses() -> TestResult<()>
    {
        assert_eq!(qualify_overlay(None)?.placement_digest(), None);
        // Independently computed SHA256 of literal domain bytes + zero u64BE count.
        // Fixed vector pins the durable codec; expected bytes do not call the implementation.
        assert_eq!(
            qualify_overlay(Some(NativeHouseTileDocument {
                schema: "OTERYN_NATIVE_HOUSE_TILES/v1".into(),
                houses: Vec::new()
            }))?
            .placement_digest(),
            Some([
                0x48, 0xc7, 0x40, 0xc2, 0xce, 0x12, 0xd1, 0x33, 0x4b, 0x35, 0x98, 0xbf, 0x5a, 0xa4,
                0x47, 0xeb, 0xb5, 0x5d, 0x3f, 0xdd, 0x2e, 0xd8, 0xbb, 0xe9, 0xf7, 0x2d, 0x74, 0x60,
                0xde, 0x33, 0x7d, 0x25
            ])
        );
        let mut invalid = document();
        invalid.houses[0].cells[0] = "oteryn:cell/missing".into();
        assert!(qualify_overlay(Some(invalid)).is_err());
        Ok(())
    }
}
