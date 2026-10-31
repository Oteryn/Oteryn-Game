//! Explicit candidate physical House placement in the same qualified native map.
//! Omission is Unknown; neither catalogue membership nor custody items imply that
//! the current actor stands in a House. World-global House IDs remain channel-free.
use super::{
    ProjectError, ProjectV2Declaration, ProjectV2DefinitionRef, ProjectV2Family, WorldProject,
};
use crate::content::static_cell_engine::EngineeringStaticCellScope;
use crate::content::{FirstProductionCell, LogicalCell};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
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
    /// Active fixed bed parts in this House. Both parts are named so the runtime can
    /// resolve either visible map item to one canonical reservation head.
    #[serde(default)]
    pub beds: Vec<NativeHouseBedPair>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeHouseBedPair {
    pub head_cell: String,
    pub partner_cell: String,
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
    beds: BTreeMap<LogicalCell, LogicalCell>,
}
impl QualifiedHousePlacement {
    pub fn house(&self) -> &ProjectV2DefinitionRef {
        &self.house
    }
    pub fn contains(&self, cell: LogicalCell) -> bool {
        self.cells.contains(&cell)
    }
    pub fn entry(&self) -> LogicalCell {
        self.entry
    }
    pub fn has_door(&self, id: u32) -> bool {
        self.doors.values().any(|number| *number == id)
    }
    pub fn door_at(&self, cell: LogicalCell) -> Option<u32> {
        self.doors.get(&cell).copied()
    }
    /// Resolves either reciprocal part to the canonical sleeper reservation cell.
    pub fn bed_head_at(&self, cell: LogicalCell) -> Option<LogicalCell> {
        self.beds.get(&cell).copied()
    }
}
impl QualifiedHouseTiles {
    /// Reset's HOUSE-1a seam admits only a qualified, explicitly empty catalogue.
    /// Unknown metadata and every actual House remain closed until that owner lands.
    pub fn is_known_empty(&self) -> bool {
        self.houses.as_ref().is_some_and(Vec::is_empty)
    }

    /// Test-support only: production bundle Worlds carry no authored House with beds yet, so a
    /// test installs one qualified House with one door and a single bed pair under `scope`.
    #[cfg(any(test, feature = "test-support"))]
    pub fn single_bed_house_for_test(
        scope: EngineeringStaticCellScope,
        house_key: &str,
        cells: impl IntoIterator<Item = LogicalCell>,
        entry: LogicalCell,
        door: LogicalCell,
        head: LogicalCell,
        partner: LogicalCell,
    ) -> Self {
        let house = QualifiedHousePlacement {
            house: ProjectV2DefinitionRef {
                family: ProjectV2Family::House,
                key: house_key.to_owned(),
                revision: "test-1".to_owned(),
            },
            cells: cells.into_iter().collect(),
            entry,
            doors: [(door, 1)].into_iter().collect(),
            beds: [(head, head), (partner, head)].into_iter().collect(),
        };
        Self {
            scope,
            houses: Some(vec![house]),
        }
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
            return Ok(Self {
                scope,
                houses: None,
            });
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
                || !identities.insert(&authored.house.key) || authored.cells.is_empty() || authored.cells.len()>4096 || authored.doors.len()>1024 || authored.beds.len()>1024
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
            let mut beds = BTreeMap::new();
            for bed in &authored.beds {
                let head = position(&bed.head_cell)?;
                let partner = position(&bed.partner_cell)?;
                if head == partner
                    || !placed.contains(&head)
                    || !placed.contains(&partner)
                    || beds.insert(head, head).is_some()
                    || beds.insert(partner, head).is_some()
                {
                    return Err(invalid());
                }
            }
            houses.push(QualifiedHousePlacement {
                house: authored.house.clone(),
                cells: placed,
                entry,
                doors,
                beds,
            });
        }
        Ok(Self {
            scope,
            houses: Some(houses),
        })
    }
    pub fn house_at(
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
    fn reciprocal_parts_resolve_to_one_reservation_head() {
        let head = LogicalCell {
            x: 10,
            y: 20,
            z: -7,
        };
        let partner = LogicalCell {
            x: 10,
            y: 21,
            z: -7,
        };
        let placement = QualifiedHousePlacement {
            house: ProjectV2DefinitionRef {
                family: ProjectV2Family::House,
                key: "oteryn:content.house.fixture".to_owned(),
                revision: "fixture-1".to_owned(),
            },
            cells: [head, partner].into_iter().collect(),
            entry: LogicalCell { x: 9, y: 20, z: -7 },
            doors: BTreeMap::new(),
            beds: [(head, head), (partner, head)].into_iter().collect(),
        };
        assert_eq!(placement.bed_head_at(head), Some(head));
        assert_eq!(placement.bed_head_at(partner), Some(head));
        assert_eq!(
            placement.bed_head_at(LogicalCell {
                x: 11,
                y: 20,
                z: -7
            }),
            None
        );
    }
    #[test]
    fn omission_retains_unknown_on_actual_accepted_room() {
        let world = oteryn_protocol_oteryn::WorldId::decode(&[
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
