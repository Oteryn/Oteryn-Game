//! Bounded placement DATA for Crystal Map::placeCreature (map.cpp:234-313).
//! Callers retain their source-specific spawn authority and supply current,
//! qualified map/actor facts immediately before each physical admission.
use super::MovementLocalPosition;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourcePlacementCell {
    pub exists: bool,
    pub query_add: bool,
    pub protection_zone: bool,
    pub teleport: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourcePlacementTerrain {
    pub exists: bool,
    pub ground: bool,
    pub immovable_solid: bool,
    pub protection_zone: bool,
    pub floor_change: bool,
    pub teleport: bool,
}
pub use oteryn_gameplay_entities::carrier::NativeSourcePlacementOccupant as SourcePlacementOccupant;
impl SourcePlacementTerrain {
    /// Crystal tile.cpp:601-698 under Map::placeCreature's IGNOREBLOCKITEM and
    /// IGNOREFIELDDAMAGE flags. These are facts, never an actor admission grant.
    pub fn for_masterless_monster(
        self,
        familiar: bool,
        push_creatures: bool,
        occupants: &[SourcePlacementOccupant],
    ) -> SourcePlacementCell {
        let occupancy = oteryn_gameplay_entities::carrier::native_source_monster_occupancy_admits(
            push_creatures,
            occupants,
        );
        SourcePlacementCell {
            exists: self.exists,
            query_add: self.exists
                && self.ground
                && !self.immovable_solid
                && (familiar || !(self.protection_zone || self.floor_change || self.teleport))
                && occupancy,
            protection_zone: self.protection_zone,
            teleport: self.teleport,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourcePlacementError {
    Unavailable,
    InvalidDraw,
    CoordinateOverflow,
}
#[derive(Debug, Clone)]
pub struct SourcePlacementPlan {
    candidates: Vec<MovementLocalPosition>,
    extended: bool,
    forced: bool,
}
impl SourcePlacementPlan {
    pub fn prepare(
        center: MovementLocalPosition,
        extended: bool,
        forced: bool,
        mut draw: impl FnMut(u32) -> u32,
    ) -> Result<Self, SourcePlacementError> {
        let mut offsets = if extended {
            vec![
                (0, -2),
                (-1, -1),
                (0, -1),
                (1, -1),
                (-2, 0),
                (-1, 0),
                (1, 0),
                (2, 0),
                (-1, 1),
                (0, 1),
                (1, 1),
                (0, 2),
            ]
        } else {
            vec![
                (-1, -1),
                (0, -1),
                (1, -1),
                (-1, 0),
                (1, 0),
                (-1, 1),
                (0, 1),
                (1, 1),
            ]
        };
        fn shuffle(
            values: &mut [(i32, i32)],
            draw: &mut impl FnMut(u32) -> u32,
        ) -> Result<(), SourcePlacementError> {
            for i in (1..values.len()).rev() {
                let bound = u32::try_from(i + 1).map_err(|_| SourcePlacementError::InvalidDraw)?;
                let value = draw(bound);
                if value >= bound {
                    return Err(SourcePlacementError::InvalidDraw);
                }
                values.swap(i, value as usize);
            }
            Ok(())
        }
        if extended {
            shuffle(&mut offsets[..4], &mut draw)?;
            shuffle(&mut offsets[4..], &mut draw)?;
        } else {
            shuffle(&mut offsets, &mut draw)?;
        }
        let mut candidates = Vec::with_capacity(offsets.len() + 1);
        candidates.push(center);
        for (dx, dy) in offsets {
            candidates.push(MovementLocalPosition {
                x: center
                    .x
                    .checked_add(dx)
                    .ok_or(SourcePlacementError::CoordinateOverflow)?,
                y: center
                    .y
                    .checked_add(dy)
                    .ok_or(SourcePlacementError::CoordinateOverflow)?,
                floor: center.floor,
            });
        }
        Ok(Self {
            candidates,
            extended,
            forced,
        })
    }
    pub fn candidates(&self) -> &[MovementLocalPosition] {
        &self.candidates
    }
    pub fn select(
        &self,
        mut cell: impl FnMut(MovementLocalPosition) -> Option<SourcePlacementCell>,
        mut sight: impl FnMut(MovementLocalPosition, MovementLocalPosition) -> Option<bool>,
    ) -> Result<Option<MovementLocalPosition>, SourcePlacementError> {
        let center = self.candidates[0];
        let center_cell = cell(center).ok_or(SourcePlacementError::Unavailable)?;
        if center_cell.exists && (self.forced || center_cell.query_add) {
            return Ok(Some(center));
        }
        let preserve_pz = center_cell.exists && center_cell.protection_zone;
        for at in self.candidates.iter().copied().skip(1) {
            let facts = cell(at).ok_or(SourcePlacementError::Unavailable)?;
            if !facts.exists
                || (preserve_pz && !facts.protection_zone)
                || facts.teleport
                || !facts.query_add
            {
                continue;
            }
            if self.extended && !sight(center, at).ok_or(SourcePlacementError::Unavailable)? {
                continue;
            }
            return Ok(Some(at));
        }
        Ok(None)
    }
}
