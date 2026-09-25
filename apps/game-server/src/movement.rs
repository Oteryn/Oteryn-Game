//! Unactivated production Rust proof of one local cardinal Movement decision.
//!
//! Engineering Content scope and synthetic collision are caller supplied, not admitted
//! Reference data or independently verified active-generation authority. No owner loop,
//! external dispatch, client legality, or creature movement is composed here.

use crate::content::static_cell_engine::{
    EngineeringStaticCellIndex, EngineeringStaticCellScope, StaticCellEngineError,
};
use crate::content::{CollisionClass, LogicalCell};
use crate::foundation::{
    CarrierError, CurrentOwnerMovementPosition, ExactActorRef, MovementLocalPosition,
    MovementPositionContext, MovementPositionSnapshot,
};

pub(crate) const LOCAL_STEP_CANDIDATES_PER_DECISION: usize = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CardinalStep {
    North,
    East,
    South,
    West,
}

impl CardinalStep {
    const fn delta(self) -> (i32, i32) {
        match self {
            Self::North => (0, -1),
            Self::East => (1, 0),
            Self::South => (0, 1),
            Self::West => (-1, 0),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MovementError {
    Actor(CarrierError),
    SnapshotMismatch,
    ContextMismatch,
    ScopeMismatch,
    CoordinateOverflow,
    Cell(StaticCellEngineError),
    Blocked,
    CapacityExceeded,
    NotQualified,
}

impl std::fmt::Display for MovementError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}
impl std::error::Error for MovementError {}

/// The owner context is separately supplied engineering binding; matching a stored context
/// does not prove a Reference generation is active or clear a July-28 field claim.
pub(crate) struct MovementEngineeringSelection<'a> {
    pub(crate) owner_context: MovementPositionContext,
    pub(crate) content_scope: &'a EngineeringStaticCellScope,
}

/// No mutation or result publication is possible until the entire decision has passed its
/// candidate budget. An over-budget attempt poisons this decision even after a prior WALKABLE.
pub(crate) struct MovementDecision<'a> {
    expected: MovementPositionSnapshot,
    scope: &'a EngineeringStaticCellScope,
    destination: MovementLocalPosition,
    cell: LogicalCell,
    attempted: usize,
    real_content_lookups: usize,
    qualified: bool,
    failure: Option<MovementError>,
}

impl<'a> MovementDecision<'a> {
    pub(crate) fn begin(
        owner: &CurrentOwnerMovementPosition<'_>,
        actor: ExactActorRef,
        expected: MovementPositionSnapshot,
        selection: &MovementEngineeringSelection<'a>,
        direction: CardinalStep,
    ) -> Result<Self, MovementError> {
        let current = owner.read(actor).map_err(MovementError::Actor)?;
        if current != expected {
            return Err(MovementError::SnapshotMismatch);
        }
        if current.context() != selection.owner_context {
            return Err(MovementError::ContextMismatch);
        }
        if current.world_id() != selection.content_scope.world_id {
            return Err(MovementError::ScopeMismatch);
        }
        let position = current.position();
        let (dx, dy) = direction.delta();
        let x = position
            .x
            .checked_add(dx)
            .ok_or(MovementError::CoordinateOverflow)?;
        let y = position
            .y
            .checked_add(dy)
            .ok_or(MovementError::CoordinateOverflow)?;
        let destination = MovementLocalPosition {
            x,
            y,
            floor: position.floor,
        };
        Ok(Self {
            expected,
            scope: selection.content_scope,
            destination,
            cell: LogicalCell {
                x,
                y,
                z: i32::from(position.floor),
            },
            attempted: 0,
            real_content_lookups: 0,
            qualified: false,
            failure: None,
        })
    }

    /// One direct lookup into the integrated Content index. The attempt counter advances
    /// before touching that index, so max+1 cannot issue a second key lookup or owner write.
    pub(crate) fn attempt_candidate(
        &mut self,
        index: &EngineeringStaticCellIndex,
    ) -> Result<(), MovementError> {
        if let Some(error) = self.failure {
            return Err(error);
        }
        self.attempted = self
            .attempted
            .checked_add(1)
            .ok_or(MovementError::CapacityExceeded)?;
        if self.attempted > LOCAL_STEP_CANDIDATES_PER_DECISION {
            self.failure = Some(MovementError::CapacityExceeded);
            return Err(MovementError::CapacityExceeded);
        }
        self.real_content_lookups += 1;
        match index.lookup(self.scope, self.cell) {
            Ok(CollisionClass::Walkable) => {
                self.qualified = true;
                Ok(())
            }
            Ok(CollisionClass::Blocked) => {
                self.failure = Some(MovementError::Blocked);
                Err(MovementError::Blocked)
            }
            Err(error) => {
                self.failure = Some(MovementError::Cell(error));
                Err(MovementError::Cell(error))
            }
        }
    }

    pub(crate) const fn attempted(&self) -> usize {
        self.attempted
    }
    pub(crate) const fn real_content_lookups(&self) -> usize {
        self.real_content_lookups
    }

    pub(crate) fn commit(
        self,
        owner: &mut CurrentOwnerMovementPosition<'_>,
    ) -> Result<MovementPositionSnapshot, MovementError> {
        if let Some(error) = self.failure {
            return Err(error);
        }
        if !self.qualified {
            return Err(MovementError::NotQualified);
        }
        owner
            .commit_cardinal(self.expected, self.destination)
            .map_err(MovementError::Actor)
    }
}

pub(crate) fn step_cardinal(
    owner: &mut CurrentOwnerMovementPosition<'_>,
    actor: ExactActorRef,
    expected: MovementPositionSnapshot,
    selection: &MovementEngineeringSelection<'_>,
    index: &EngineeringStaticCellIndex,
    direction: CardinalStep,
) -> Result<MovementPositionSnapshot, MovementError> {
    let mut decision = MovementDecision::begin(owner, actor, expected, selection, direction)?;
    decision.attempt_candidate(index)?;
    decision.commit(owner)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::static_cell_engine::{
        EngineeringCollisionClaim, EngineeringStaticCellClaim,
    };
    use crate::content::{
        ContentLockBinding, ContentLockEntry, CoordinateFrameRef, MapRevisionRef, ProductionAtom,
        ProductionKey, Sha256HexDigest,
    };
    use crate::foundation::MovementActorFixture;
    use std::error::Error;

    fn fixture(x: i32, y: i32, creature: bool) -> Result<MovementActorFixture, CarrierError> {
        MovementActorFixture::new(MovementLocalPosition { x, y, floor: 7 }, creature)
    }
    fn scope(fixture: &MovementActorFixture) -> Result<EngineeringStaticCellScope, Box<dyn Error>> {
        Ok(EngineeringStaticCellScope {
            world_id: fixture.world_id(),
            coordinate_frame: CoordinateFrameRef::new("movement-engineering-frame")?,
            map_revision: MapRevisionRef::new("movement-engineering-map")?,
            generation_digest: [7; 32],
            content_lock: ContentLockBinding {
                revision_digest_token: ProductionAtom::new("lock", "movement-engineering-lock")?,
                entries: vec![ContentLockEntry::exact(
                    ProductionKey::new("engineering:movement")?,
                    ProductionAtom::new("revision", "movement-engineering-r1")?,
                    Sha256HexDigest::new(&"a".repeat(64))?,
                )],
            },
        })
    }
    fn index(
        scope: &EngineeringStaticCellScope,
        cells: &[(i32, i32, i32, EngineeringCollisionClaim)],
    ) -> Result<EngineeringStaticCellIndex, StaticCellEngineError> {
        EngineeringStaticCellIndex::from_claims(
            cells
                .iter()
                .map(|(x, y, z, collision)| EngineeringStaticCellClaim {
                    scope: scope.clone(),
                    cell: LogicalCell {
                        x: *x,
                        y: *y,
                        z: *z,
                    },
                    collision: *collision,
                })
                .collect(),
        )
    }
    fn selection<'a>(
        snapshot: MovementPositionSnapshot,
        scope: &'a EngineeringStaticCellScope,
    ) -> MovementEngineeringSelection<'a> {
        MovementEngineeringSelection {
            owner_context: snapshot.context(),
            content_scope: scope,
        }
    }
    fn walkable() -> EngineeringCollisionClaim {
        EngineeringCollisionClaim::Qualified(CollisionClass::Walkable)
    }

    #[test]
    fn real_movement_all_cardinals_use_exact_content_and_owner() -> Result<(), Box<dyn Error>> {
        for (direction, x, y) in [
            (CardinalStep::North, 4, 4),
            (CardinalStep::East, 5, 5),
            (CardinalStep::South, 4, 6),
            (CardinalStep::West, 3, 5),
        ] {
            let mut fixture = self::fixture(4, 5, false)?;
            let scope = self::scope(&fixture)?;
            let index = self::index(&scope, &[(x, y, 7, walkable()), (x + 10, y, 7, walkable())])?;
            let before = fixture.current_position()?;
            let actor = fixture.actor();
            let result = step_cardinal(
                &mut fixture.borrow_position(),
                actor,
                before,
                &selection(before, &scope),
                &index,
                direction,
            )?;
            assert_eq!(result.position(), MovementLocalPosition { x, y, floor: 7 });
            assert_eq!(result.revision(), before.revision() + 1);
            assert_eq!(
                fixture.raw_position()?,
                (result.position(), result.revision())
            );
        }
        Ok(())
    }

    #[test]
    fn real_movement_max_plus_one_poisons_same_decision_before_second_lookup()
    -> Result<(), Box<dyn Error>> {
        let mut fixture = self::fixture(4, 5, false)?;
        let scope = self::scope(&fixture)?;
        let index = self::index(&scope, &[(5, 5, 7, walkable()), (6, 5, 7, walkable())])?;
        let before = fixture.current_position()?;
        let actor = fixture.actor();
        let mut owner = fixture.borrow_position();
        let mut decision = MovementDecision::begin(
            &owner,
            actor,
            before,
            &selection(before, &scope),
            CardinalStep::East,
        )?;
        assert_eq!(decision.attempted(), 0);
        assert_eq!(decision.real_content_lookups(), 0);
        assert_eq!(decision.attempt_candidate(&index), Ok(()));
        assert_eq!(decision.attempted(), 1);
        assert_eq!(decision.real_content_lookups(), 1);
        assert_eq!(
            decision.attempt_candidate(&index),
            Err(MovementError::CapacityExceeded)
        );
        assert_eq!(decision.attempted(), 2);
        assert_eq!(decision.real_content_lookups(), 1);
        assert_eq!(
            decision.commit(&mut owner),
            Err(MovementError::CapacityExceeded)
        );
        assert_eq!(owner.read(actor), Ok(before));
        Ok(())
    }

    #[test]
    fn real_movement_collision_absence_and_scope_reject_unchanged() -> Result<(), Box<dyn Error>> {
        for (cells, expected) in [
            (
                vec![(
                    5,
                    5,
                    7,
                    EngineeringCollisionClaim::Qualified(CollisionClass::Blocked),
                )],
                MovementError::Blocked,
            ),
            (
                vec![(6, 5, 7, walkable())],
                MovementError::Cell(StaticCellEngineError::Absent),
            ),
            (
                vec![(5, 5, 7, EngineeringCollisionClaim::Unqualified)],
                MovementError::Cell(StaticCellEngineError::Unqualified),
            ),
            (
                vec![(5, 5, 7, EngineeringCollisionClaim::Conflict)],
                MovementError::Cell(StaticCellEngineError::Conflict),
            ),
            (
                vec![(5, 5, 8, walkable())],
                MovementError::Cell(StaticCellEngineError::Absent),
            ),
        ] {
            let mut fixture = self::fixture(4, 5, false)?;
            let scope = self::scope(&fixture)?;
            let index = self::index(&scope, &cells)?;
            let before = fixture.current_position()?;
            let actor = fixture.actor();
            assert_eq!(
                step_cardinal(
                    &mut fixture.borrow_position(),
                    actor,
                    before,
                    &selection(before, &scope),
                    &index,
                    CardinalStep::East
                ),
                Err(expected)
            );
            assert_eq!(
                fixture.raw_position()?,
                (before.position(), before.revision())
            );
        }
        let mut fixture = self::fixture(4, 5, false)?;
        let scope = self::scope(&fixture)?;
        let before = fixture.current_position()?;
        let actor = fixture.actor();
        for mutation in 0..5 {
            let mut other = scope.clone();
            match mutation {
                0 => other.coordinate_frame = CoordinateFrameRef::new("other-frame")?,
                1 => other.map_revision = MapRevisionRef::new("other-map")?,
                2 => other.generation_digest[0] ^= 1,
                3 => {
                    other.content_lock.revision_digest_token =
                        ProductionAtom::new("lock", "other-lock")?
                }
                _ => {
                    other.content_lock.entries[0].package_revision =
                        ProductionAtom::new("revision", "other-r2")?
                }
            }
            let index = index(&other, &[(5, 5, 7, walkable())])?;
            let mut owner = fixture.borrow_position();
            let mut decision = MovementDecision::begin(
                &owner,
                actor,
                before,
                &selection(before, &scope),
                CardinalStep::East,
            )?;
            assert_eq!(
                decision.attempt_candidate(&index),
                Err(MovementError::Cell(StaticCellEngineError::ScopeMismatch))
            );
            assert_eq!(decision.real_content_lookups(), 1);
            assert_eq!(
                decision.commit(&mut owner),
                Err(MovementError::Cell(StaticCellEngineError::ScopeMismatch))
            );
            assert_eq!(owner.read(actor), Ok(before));
        }
        assert_eq!(
            fixture.raw_position()?,
            (before.position(), before.revision())
        );
        Ok(())
    }

    #[test]
    fn real_movement_overflow_and_context_reject_before_content_lookup()
    -> Result<(), Box<dyn Error>> {
        for (x, y, direction) in [
            (i32::MAX, 5, CardinalStep::East),
            (i32::MIN, 5, CardinalStep::West),
            (5, i32::MAX, CardinalStep::South),
            (5, i32::MIN, CardinalStep::North),
        ] {
            let mut fixture = self::fixture(x, y, false)?;
            let scope = self::scope(&fixture)?;
            let before = fixture.current_position()?;
            let actor = fixture.actor();
            assert_eq!(
                MovementDecision::begin(
                    &fixture.borrow_position(),
                    actor,
                    before,
                    &selection(before, &scope),
                    direction
                )
                .err(),
                Some(MovementError::CoordinateOverflow)
            );
            assert_eq!(
                fixture.raw_position()?,
                (before.position(), before.revision())
            );
        }
        let mut fixture = self::fixture(4, 5, false)?;
        let scope = self::scope(&fixture)?;
        let before = fixture.current_position()?;
        let actor = fixture.actor();
        let wrong = MovementEngineeringSelection {
            owner_context: fixture.wrong_context()?,
            content_scope: &scope,
        };
        assert_eq!(
            MovementDecision::begin(
                &fixture.borrow_position(),
                actor,
                before,
                &wrong,
                CardinalStep::East
            )
            .err(),
            Some(MovementError::ContextMismatch)
        );
        let mut other = scope.clone();
        let mut uuid = [2u8; 16];
        uuid[6] = 0x72;
        uuid[8] = 0x82;
        other.world_id = crate::foundation::WorldId::decode(&uuid)?;
        assert_eq!(
            MovementDecision::begin(
                &fixture.borrow_position(),
                actor,
                before,
                &selection(before, &other),
                CardinalStep::East
            )
            .err(),
            Some(MovementError::ScopeMismatch)
        );
        assert_eq!(
            fixture.raw_position()?,
            (before.position(), before.revision())
        );
        Ok(())
    }

    #[test]
    fn real_movement_stale_actor_owner_snapshot_commit_and_replay_fail_closed()
    -> Result<(), Box<dyn Error>> {
        let mut fixture = self::fixture(4, 5, false)?;
        let scope = self::scope(&fixture)?;
        let index = self::index(&scope, &[(5, 5, 7, walkable())])?;
        let before = fixture.current_position()?;
        let old_actor = fixture.actor();
        fixture.recycle()?;
        assert_eq!(
            MovementDecision::begin(
                &fixture.borrow_position(),
                old_actor,
                before,
                &selection(before, &scope),
                CardinalStep::East
            )
            .err(),
            Some(MovementError::Actor(CarrierError::StaleActorGeneration))
        );
        let after_recycle = fixture.current_position()?;
        assert_eq!(
            fixture.raw_position()?,
            (after_recycle.position(), after_recycle.revision())
        );
        let actor = fixture.actor();
        fixture.intervening_write()?;
        assert_eq!(
            MovementDecision::begin(
                &fixture.borrow_position(),
                actor,
                after_recycle,
                &selection(after_recycle, &scope),
                CardinalStep::East
            )
            .err(),
            Some(MovementError::SnapshotMismatch)
        );
        let fresh = fixture.current_position()?;
        let mut decision = MovementDecision::begin(
            &fixture.borrow_position(),
            actor,
            fresh,
            &selection(fresh, &scope),
            CardinalStep::East,
        )?;
        decision.attempt_candidate(&index)?;
        fixture.intervening_write()?;
        let changed = fixture.current_position()?;
        assert_eq!(
            decision.commit(&mut fixture.borrow_position()),
            Err(MovementError::Actor(CarrierError::PositionSnapshotMismatch))
        );
        assert_eq!(fixture.current_position()?, changed);
        let committed = step_cardinal(
            &mut fixture.borrow_position(),
            actor,
            changed,
            &selection(changed, &scope),
            &index,
            CardinalStep::East,
        )?;
        assert_eq!(
            MovementDecision::begin(
                &fixture.borrow_position(),
                actor,
                changed,
                &selection(changed, &scope),
                CardinalStep::East
            )
            .err(),
            Some(MovementError::SnapshotMismatch)
        );
        assert_eq!(fixture.current_position()?, committed);
        fixture.advance_owner()?;
        assert_eq!(
            MovementDecision::begin(
                &fixture.borrow_position(),
                actor,
                committed,
                &selection(committed, &scope),
                CardinalStep::East
            )
            .err(),
            Some(MovementError::Actor(CarrierError::WrongScope))
        );
        Ok(())
    }

    #[test]
    fn real_movement_creature_and_revision_ceiling_reject_no_mutation() -> Result<(), Box<dyn Error>>
    {
        let mut creature = self::fixture(4, 5, true)?;
        let actor = creature.actor();
        assert_eq!(
            creature.borrow_position().read(actor),
            Err(CarrierError::MovementCreatureUnavailable)
        );
        assert_eq!(
            creature.raw_position()?,
            (
                MovementLocalPosition {
                    x: 4,
                    y: 5,
                    floor: 7
                },
                1
            )
        );
        let mut fixture = self::fixture(4, 5, false)?;
        let scope = self::scope(&fixture)?;
        let index = self::index(&scope, &[(5, 5, 7, walkable())])?;
        let actor = fixture.actor();
        let before = fixture.current_position()?;
        let mut decision = MovementDecision::begin(
            &fixture.borrow_position(),
            actor,
            before,
            &selection(before, &scope),
            CardinalStep::East,
        )?;
        decision.attempt_candidate(&index)?;
        fixture.become_creature()?;
        let creature_actor = fixture.actor();
        let creature_before = fixture.raw_position()?;
        assert_eq!(
            decision.commit(&mut fixture.borrow_position()),
            Err(MovementError::Actor(
                CarrierError::MovementCreatureUnavailable
            ))
        );
        assert_eq!(fixture.raw_position()?, creature_before);
        assert_eq!(
            fixture.borrow_position().read(creature_actor),
            Err(CarrierError::MovementCreatureUnavailable)
        );
        let mut fixture = self::fixture(4, 5, false)?;
        let scope = self::scope(&fixture)?;
        let index = self::index(&scope, &[(5, 5, 7, walkable())])?;
        fixture.exhaust_position_revision()?;
        let snapshot = fixture.current_position()?;
        let actor = fixture.actor();
        assert_eq!(
            step_cardinal(
                &mut fixture.borrow_position(),
                actor,
                snapshot,
                &selection(snapshot, &scope),
                &index,
                CardinalStep::East
            ),
            Err(MovementError::Actor(
                CarrierError::PositionRevisionExhausted
            ))
        );
        assert_eq!(fixture.current_position()?, snapshot);
        Ok(())
    }
}
