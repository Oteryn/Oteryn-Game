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
