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
    CarrierError, ChannelRuntimeV1, CurrentOwnerMovementPosition, ExactActorRef,
    MovementLocalPosition, MovementPositionContext, MovementPositionSnapshot,
};
use std::num::NonZeroUsize;

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

/// Explicit engineering work budget for one exclusive Channel owner borrow. The caller
/// retains inputs returned as Deferred and decides which actor to offer next turn.
/// No production maximum, queue, command outcome, or scheduling authority is implied.
/// Fairness remains an obligation of the future owner scheduler.
pub(crate) struct MovementOwnerTurn<'a> {
    owner: CurrentOwnerMovementPosition<'a>,
    max_inputs: NonZeroUsize,
    processed: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MovementTurnOutcome {
    Applied(MovementPositionSnapshot),
    Deferred,
}

impl<'a> MovementOwnerTurn<'a> {
    pub(crate) fn begin(runtime: &'a mut ChannelRuntimeV1, max_inputs: NonZeroUsize) -> Self {
        Self {
            owner: runtime.borrow_movement_position(),
            max_inputs,
            processed: 0,
        }
    }

    pub(crate) const fn processed(&self) -> usize {
        self.processed
    }

    /// All admitted attempts, including a blocked or stale input, consume one unit.
    /// At max+1 no Content lookup or owner read/write occurs; the input stays with
    /// the caller for a subsequent turn and must be revalidated against that turn.
    pub(crate) fn try_step(
        &mut self,
        actor: ExactActorRef,
        expected: MovementPositionSnapshot,
        selection: &MovementEngineeringSelection<'_>,
        index: &EngineeringStaticCellIndex,
        direction: CardinalStep,
    ) -> Result<MovementTurnOutcome, MovementError> {
        if self.processed >= self.max_inputs.get() {
            return Ok(MovementTurnOutcome::Deferred);
        }
        self.processed += 1;
        step_cardinal(
            &mut self.owner,
            actor,
            expected,
            selection,
            index,
            direction,
        )
        .map(MovementTurnOutcome::Applied)
    }

    pub(crate) fn read(
        &self,
        actor: ExactActorRef,
    ) -> Result<MovementPositionSnapshot, CarrierError> {
        self.owner.read(actor)
    }
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
    use crate::foundation::{ChannelId, GameSessionId, MovementActorFixture, NodeId, WorldId};
    use std::error::Error;
    use std::time::Instant;

    fn fixture(x: i32, y: i32, creature: bool) -> Result<MovementActorFixture, CarrierError> {
        MovementActorFixture::new(MovementLocalPosition { x, y, floor: 7 }, creature)
    }
    fn scope(fixture: &MovementActorFixture) -> Result<EngineeringStaticCellScope, Box<dyn Error>> {
        scope_for_world(fixture.world_id())
    }
    fn scope_for_world(world_id: WorldId) -> Result<EngineeringStaticCellScope, Box<dyn Error>> {
        Ok(EngineeringStaticCellScope {
            world_id,
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

    fn uuid_v7(raw: u64) -> [u8; 16] {
        let mut bytes = [0; 16];
        bytes[8..].copy_from_slice(&raw.to_be_bytes());
        bytes[6] = 0x70;
        bytes[8] = (bytes[8] & 0x3f) | 0x80;
        bytes
    }

    fn runtime() -> Result<ChannelRuntimeV1, Box<dyn Error>> {
        Ok(ChannelRuntimeV1::from_committed_assignment(
            WorldId::decode(&uuid_v7(20))?,
            ChannelId::decode(&uuid_v7(21))?,
            NodeId::decode(&uuid_v7(22))?,
            7,
            3,
            11,
            "runtime-scope-assignment:11",
            2,
        )?)
    }

    fn admitted_at(
        runtime: &mut ChannelRuntimeV1,
        session: u64,
        x: i32,
    ) -> Result<(ExactActorRef, MovementPositionSnapshot), Box<dyn Error>> {
        let reserved = runtime.reserve_fresh_session(GameSessionId::decode(&uuid_v7(session))?)?;
        let actor = runtime.commit_fresh_session(reserved)?;
        let snapshot = runtime.initialize_movement_test_position(
            actor,
            MovementLocalPosition { x, y: 5, floor: 7 },
        )?;
        Ok((actor, snapshot))
    }

    #[test]
    fn composed_owner_turn_counts_attempts_defers_and_revalidates_replay()
    -> Result<(), Box<dyn Error>> {
        let mut runtime = runtime()?;
        let (actor_a, start_a) = admitted_at(&mut runtime, 30, 4)?;
        let (actor_b, start_b) = admitted_at(&mut runtime, 31, 14)?;
        let scope = scope_for_world(start_a.world_id())?;
        let mut other_scope = scope.clone();
        other_scope.map_revision = MapRevisionRef::new("other-map")?;
        let wrong_index = index(&other_scope, &[(15, 5, 7, walkable())])?;
        let index = index(
            &scope,
            &[
                (5, 5, 7, walkable()),
                (14, 5, 7, walkable()),
                (15, 5, 7, walkable()),
            ],
        )?;
        let binding = selection(start_a, &scope);
        let limit = NonZeroUsize::new(2).ok_or("nonzero engineering limit")?;

        let after_a;
        {
            let mut turn = MovementOwnerTurn::begin(&mut runtime, limit);
            assert_eq!(turn.processed(), 0);
            after_a = match turn.try_step(actor_a, start_a, &binding, &index, CardinalStep::East)? {
                MovementTurnOutcome::Applied(snapshot) => snapshot,
                MovementTurnOutcome::Deferred => return Err("input within budget deferred".into()),
            };
            assert_eq!(
                turn.try_step(actor_a, start_a, &binding, &index, CardinalStep::East),
                Err(MovementError::SnapshotMismatch)
            );
            assert_eq!(
                turn.processed(),
                2,
                "rejected input consumes the same work budget"
            );
            // A real lookup into this wrong-scope index would reject. Deferral
            // precedes the decision and leaves B's owner snapshot untouched.
            assert_eq!(
                turn.try_step(actor_b, start_b, &binding, &wrong_index, CardinalStep::East)?,
                MovementTurnOutcome::Deferred
            );
            assert_eq!(turn.processed(), 2);
            assert_eq!(turn.read(actor_b)?, start_b);
        }

        // An explicitly reoffered input can progress next turn. No scheduler
        // is composed here, so this does not prove starvation freedom.
        {
            let mut turn = MovementOwnerTurn::begin(&mut runtime, limit);
            let after_b =
                match turn.try_step(actor_b, start_b, &binding, &index, CardinalStep::East)? {
                    MovementTurnOutcome::Applied(snapshot) => snapshot,
                    MovementTurnOutcome::Deferred => return Err("deferred input starved".into()),
                };
            assert_eq!(after_b.position().x, 15);
            assert_eq!(
                turn.try_step(actor_a, start_a, &binding, &index, CardinalStep::East),
                Err(MovementError::SnapshotMismatch)
            );
            assert_eq!(turn.read(actor_a)?, after_a);
            assert_eq!(turn.processed(), 2);
        }
        Ok(())
    }

    #[test]
    fn composed_owner_turn_overflow_consumes_budget_without_writing() -> Result<(), Box<dyn Error>>
    {
        let mut runtime = runtime()?;
        let (actor, before) = admitted_at(&mut runtime, 30, i32::MAX)?;
        let scope = scope_for_world(before.world_id())?;
        let index = index(&scope, &[(0, 5, 7, walkable())])?;
        let binding = selection(before, &scope);
        let limit = NonZeroUsize::new(1).ok_or("nonzero engineering limit")?;
        let mut turn = MovementOwnerTurn::begin(&mut runtime, limit);
        assert_eq!(
            turn.try_step(actor, before, &binding, &index, CardinalStep::East),
            Err(MovementError::CoordinateOverflow)
        );
        assert_eq!(turn.processed(), 1);
        assert_eq!(
            turn.try_step(actor, before, &binding, &index, CardinalStep::East)?,
            MovementTurnOutcome::Deferred
        );
        assert_eq!(turn.read(actor)?, before);
        Ok(())
    }

    #[test]
    fn composed_owner_turn_measures_engineering_workload() -> Result<(), Box<dyn Error>> {
        let mut runtime = runtime()?;
        let (actor, mut snapshot) = admitted_at(&mut runtime, 30, 4)?;
        let scope = scope_for_world(snapshot.world_id())?;
        let index = index(&scope, &[(4, 5, 7, walkable()), (5, 5, 7, walkable())])?;
        let binding = selection(snapshot, &scope);
        const TURNS: usize = 1024;
        const INPUTS_PER_TURN: usize = 8; // Engineering fixture, not a production maximum.
        let limit = NonZeroUsize::new(INPUTS_PER_TURN).ok_or("nonzero engineering limit")?;
        let mut turn_latencies_ns = Vec::with_capacity(TURNS);
        let start = Instant::now();
        for _ in 0..TURNS {
            let turn_start = Instant::now();
            let mut turn = MovementOwnerTurn::begin(&mut runtime, limit);
            for _ in 0..INPUTS_PER_TURN {
                let direction = if snapshot.position().x == 4 {
                    CardinalStep::East
                } else {
                    CardinalStep::West
                };
                snapshot = match turn.try_step(actor, snapshot, &binding, &index, direction)? {
                    MovementTurnOutcome::Applied(next) => next,
                    MovementTurnOutcome::Deferred => {
                        return Err("within-budget input deferred".into());
                    }
                };
            }
            assert_eq!(turn.processed(), INPUTS_PER_TURN);
            assert_eq!(
                turn.try_step(actor, snapshot, &binding, &index, CardinalStep::East)?,
                MovementTurnOutcome::Deferred
            );
            turn_latencies_ns.push(turn_start.elapsed().as_nanos());
        }
        let elapsed = start.elapsed();
        turn_latencies_ns.sort_unstable();
        assert_eq!(snapshot.revision(), 1 + (TURNS * INPUTS_PER_TURN) as u64);
        assert_eq!(snapshot.position().x, 4);
        eprintln!(
            "movement_owner_turn_engineering_sample: os={} arch={} logical_parallelism={:?} turns={} inputs={} elapsed_ns={} ns_per_input={} inputs_per_sec={:.0} turn_p50_ns={} turn_p95_ns={} turn_max_ns={} fixture=one_actor_two_walkable_cells_east_west_no_network_or_scheduler",
            std::env::consts::OS,
            std::env::consts::ARCH,
            std::thread::available_parallelism()
                .ok()
                .map(NonZeroUsize::get),
            TURNS,
            TURNS * INPUTS_PER_TURN,
            elapsed.as_nanos(),
            elapsed.as_nanos() / (TURNS * INPUTS_PER_TURN) as u128,
            (TURNS * INPUTS_PER_TURN) as f64 / elapsed.as_secs_f64(),
            turn_latencies_ns[TURNS / 2],
            turn_latencies_ns[TURNS * 95 / 100],
            turn_latencies_ns[TURNS - 1],
        );
        Ok(())
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
