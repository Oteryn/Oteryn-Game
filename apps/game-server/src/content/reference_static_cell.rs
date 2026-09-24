//! Non-shipping structural predecessor for a Reference static-cell read.
//!
//! This module is only compiled into unit tests. It demonstrates that one immutable static fact
//! can be read by its complete scope and cell key without scanning a collection. It does not
//! connect to Reference activation, a production artifact, OTS material, or a runtime owner.

use super::{CollisionClass, CoordinateFrameRef, MapRevisionRef, ProductionAtom};
use crate::foundation::WorldId;
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

const REFERENCE_STATIC_CELL_FIXTURE_MAX_CELLS: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct StaticCellScope {
    world_id: WorldId,
    coordinate_frame: CoordinateFrameRef,
    generation: ProductionAtom,
    map_revision: MapRevisionRef,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct StaticCellPosition {
    x: i32,
    y: i32,
    floor: i16,
}

impl StaticCellPosition {
    pub(crate) fn checked_from_wide(x: i64, y: i64, floor: i32) -> Result<Self, StaticCellError> {
        Ok(Self {
            x: i32::try_from(x).map_err(|_| StaticCellError::CoordinateOutOfRange)?,
            y: i32::try_from(y).map_err(|_| StaticCellError::CoordinateOutOfRange)?,
            floor: i16::try_from(floor).map_err(|_| StaticCellError::CoordinateOutOfRange)?,
        })
    }
}

impl StaticCellScope {
    pub(crate) fn fixture(
        world_id: WorldId,
        coordinate_frame: CoordinateFrameRef,
        generation: ProductionAtom,
        map_revision: MapRevisionRef,
    ) -> Self {
        Self {
            world_id,
            coordinate_frame,
            generation,
            map_revision,
        }
    }
}

impl StaticCellPosition {
    pub(crate) const fn fixture(x: i32, y: i32, floor: i16) -> Self {
        Self { x, y, floor }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct StaticCellKey {
    scope: StaticCellScope,
    position: StaticCellPosition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StaticCellFact {
    Qualified(CollisionClass),
    Unqualified,
    Conflict,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StaticCellFixtureRecord {
    key: StaticCellKey,
    fact: StaticCellFact,
}

impl StaticCellFixtureRecord {
    pub(crate) fn fixture(
        scope: &StaticCellScope,
        position: StaticCellPosition,
        fact: StaticCellFact,
    ) -> Self {
        Self {
            key: StaticCellKey {
                scope: scope.clone(),
                position,
            },
            fact,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StaticCellError {
    Absent,
    Unqualified,
    Conflict,
    Duplicate,
    TooManyFixtureCells,
    CoordinateOutOfRange,
}

impl fmt::Display for StaticCellError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl Error for StaticCellError {}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct ReferenceStaticCellIndex {
    cells: BTreeMap<StaticCellKey, StaticCellFact>,
}

impl ReferenceStaticCellIndex {
    /// Fixture-only construction. The bound is a local test fixture bound, not a production
    /// resource limit or content-profile decision.
    pub(crate) fn from_fixture(
        records: Vec<StaticCellFixtureRecord>,
    ) -> Result<Self, StaticCellError> {
        if records.len() > REFERENCE_STATIC_CELL_FIXTURE_MAX_CELLS {
            return Err(StaticCellError::TooManyFixtureCells);
        }

        let mut cells = BTreeMap::new();
        for record in records {
            if cells.insert(record.key, record.fact).is_some() {
                return Err(StaticCellError::Duplicate);
            }
        }
        Ok(Self { cells })
    }

    /// Reads exactly one complete key from the ordered map. No collection iteration or spatial
    /// fallback is performed when the scope, map revision, floor, or position differs.
    pub(crate) fn lookup(
        &self,
        scope: &StaticCellScope,
        position: StaticCellPosition,
    ) -> Result<CollisionClass, StaticCellError> {
        let key = StaticCellKey {
            scope: scope.clone(),
            position,
        };
        match self
            .cells
            .get(&key)
            .copied()
            .ok_or(StaticCellError::Absent)?
        {
            StaticCellFact::Qualified(collision) => Ok(collision),
            StaticCellFact::Unqualified => Err(StaticCellError::Unqualified),
            StaticCellFact::Conflict => Err(StaticCellError::Conflict),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::foundation::WorldId;

    fn uuid_v7(seed: u8) -> [u8; 16] {
        let mut bytes = [seed; 16];
        bytes[6] = 0x70 | (bytes[6] & 0x0f);
        bytes[8] = 0x80 | (bytes[8] & 0x3f);
        bytes
    }

    fn scope() -> Result<StaticCellScope, Box<dyn Error>> {
        Ok(StaticCellScope::fixture(
            WorldId::decode(&uuid_v7(1))?,
            CoordinateFrameRef::new("local-frame-r1")?,
            ProductionAtom::new("fixture generation", "generation-7")?,
            MapRevisionRef::new("map-r3")?,
        ))
    }

    fn position(x: i32, y: i32, floor: i16) -> StaticCellPosition {
        StaticCellPosition::fixture(x, y, floor)
    }

    fn record(
        scope: &StaticCellScope,
        position: StaticCellPosition,
        fact: StaticCellFact,
    ) -> StaticCellFixtureRecord {
        StaticCellFixtureRecord::fixture(scope, position, fact)
    }

    #[test]
    fn reads_one_walkable_or_blocked_fact_by_exact_key() -> Result<(), Box<dyn Error>> {
        let scope = scope()?;
        let walkable = position(10, 20, 7);
        let blocked = position(11, 20, 7);
        let index = ReferenceStaticCellIndex::from_fixture(vec![
            record(
                &scope,
                walkable,
                StaticCellFact::Qualified(CollisionClass::Walkable),
            ),
            record(
                &scope,
                blocked,
                StaticCellFact::Qualified(CollisionClass::Blocked),
            ),
        ])?;

        assert_eq!(index.lookup(&scope, walkable), Ok(CollisionClass::Walkable));
        assert_eq!(index.lookup(&scope, blocked), Ok(CollisionClass::Blocked));
        assert_eq!(
            index.lookup(&scope, position(12, 20, 7)),
            Err(StaticCellError::Absent)
        );
        Ok(())
    }

    #[test]
    fn world_frame_generation_map_revision_and_floor_are_all_bound() -> Result<(), Box<dyn Error>> {
        let scope = scope()?;
        let cell = position(10, 20, 7);
        let index = ReferenceStaticCellIndex::from_fixture(vec![record(
            &scope,
            cell,
            StaticCellFact::Qualified(CollisionClass::Walkable),
        )])?;

        let mut wrong_world = scope.clone();
        wrong_world.world_id = WorldId::decode(&uuid_v7(2))?;
        let mut wrong_frame = scope.clone();
        wrong_frame.coordinate_frame = CoordinateFrameRef::new("other-frame-r1")?;
        let mut wrong_generation = scope.clone();
        wrong_generation.generation = ProductionAtom::new("fixture generation", "generation-8")?;
        let mut wrong_map_revision = scope.clone();
        wrong_map_revision.map_revision = MapRevisionRef::new("map-r4")?;

        for mismatched_scope in [
            wrong_world,
            wrong_frame,
            wrong_generation,
            wrong_map_revision,
        ] {
            assert_eq!(
                index.lookup(&mismatched_scope, cell),
                Err(StaticCellError::Absent)
            );
        }
        assert_eq!(
            index.lookup(&scope, position(10, 20, 8)),
            Err(StaticCellError::Absent)
        );
        Ok(())
    }

    #[test]
    fn absent_unqualified_conflicting_and_duplicate_facts_fail_closed() -> Result<(), Box<dyn Error>>
    {
        let scope = scope()?;
        let unqualified = position(10, 20, 7);
        let conflict = position(11, 20, 7);
        let duplicate = position(12, 20, 7);
        let index = ReferenceStaticCellIndex::from_fixture(vec![
            record(&scope, unqualified, StaticCellFact::Unqualified),
            record(&scope, conflict, StaticCellFact::Conflict),
        ])?;

        assert_eq!(
            index.lookup(&scope, position(9, 20, 7)),
            Err(StaticCellError::Absent)
        );
        assert_eq!(
            index.lookup(&scope, unqualified),
            Err(StaticCellError::Unqualified)
        );
        assert_eq!(
            index.lookup(&scope, conflict),
            Err(StaticCellError::Conflict)
        );
        assert_eq!(
            ReferenceStaticCellIndex::from_fixture(vec![
                record(
                    &scope,
                    duplicate,
                    StaticCellFact::Qualified(CollisionClass::Walkable)
                ),
                record(
                    &scope,
                    duplicate,
                    StaticCellFact::Qualified(CollisionClass::Blocked)
                ),
            ]),
            Err(StaticCellError::Duplicate)
        );
        Ok(())
    }

    #[test]
    fn fixture_collection_accepts_max_and_rejects_max_plus_one() -> Result<(), Box<dyn Error>> {
        let scope = scope()?;
        let mut records = Vec::new();
        for x in 0..REFERENCE_STATIC_CELL_FIXTURE_MAX_CELLS {
            records.push(record(
                &scope,
                position(i32::try_from(x)?, 20, 7),
                StaticCellFact::Qualified(CollisionClass::Walkable),
            ));
        }
        assert!(ReferenceStaticCellIndex::from_fixture(records.clone()).is_ok());

        let mut over_limit = records;
        over_limit.push(record(
            &scope,
            position(
                i32::try_from(REFERENCE_STATIC_CELL_FIXTURE_MAX_CELLS)?,
                20,
                7,
            ),
            StaticCellFact::Qualified(CollisionClass::Walkable),
        ));
        assert_eq!(
            ReferenceStaticCellIndex::from_fixture(over_limit),
            Err(StaticCellError::TooManyFixtureCells)
        );
        Ok(())
    }

    #[test]
    fn coordinate_conversion_rejects_i32_and_i16_overflow() -> Result<(), Box<dyn Error>> {
        assert_eq!(
            StaticCellPosition::checked_from_wide(i64::from(i32::MAX) + 1, 0, 7),
            Err(StaticCellError::CoordinateOutOfRange)
        );
        assert_eq!(
            StaticCellPosition::checked_from_wide(0, i64::from(i32::MIN) - 1, 7),
            Err(StaticCellError::CoordinateOutOfRange)
        );
        assert_eq!(
            StaticCellPosition::checked_from_wide(0, 0, i32::from(i16::MAX) + 1),
            Err(StaticCellError::CoordinateOutOfRange)
        );
        assert_eq!(
            StaticCellPosition::checked_from_wide(i64::from(i32::MIN), i64::from(i32::MAX), 7),
            Ok(position(i32::MIN, i32::MAX, 7))
        );
        Ok(())
    }
}
