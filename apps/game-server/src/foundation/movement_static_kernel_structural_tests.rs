//! Non-shipping structural Movement fixture. These local markers and synthetic cells do not
//! activate Reference, establish its parity, or qualify production MOVE-RL-03.
use super::*;
extern crate oteryn_game_server as game_library;
use game_library::content::{CollisionClass, CoordinateFrameRef, MapRevisionRef, ProductionAtom};

// Some integration targets include Foundation's source as their own crate root. Compile the
// existing test-only Content index source here so all such test crates exercise one implementation,
// not a second cell-index design. This child remains reachable only through `#[cfg(test)]`.
#[path = "../content/reference_static_cell.rs"]
mod reference_static_cell;
use reference_static_cell::{
    ReferenceStaticCellIndex, StaticCellError, StaticCellFact, StaticCellFixtureRecord,
    StaticCellPosition, StaticCellScope,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Step {
    North,
    East,
    South,
    West,
}

impl Step {
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
enum MoveError {
    Actor(CarrierError),
    SnapshotMismatch,
    PositionContextMismatch,
    CellScopeMismatch,
    CoordinateOverflow,
    Cell(StaticCellError),
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Proposal {
    expected: PositionSnapshot,
    destination: LocalPosition,
}

/// A single lookup path. The counter makes zero-before-lookup and one-candidate assertions
/// physical; no iteration, alternate key, or fallback API is exposed to this kernel.
struct CellProbe<'a> {
    index: &'a ReferenceStaticCellIndex,
    lookups: usize,
}

impl CellProbe<'_> {
    fn lookup(
        &mut self,
        scope: &StaticCellScope,
        position: StaticCellPosition,
    ) -> Result<CollisionClass, StaticCellError> {
        self.lookups += 1;
        assert_eq!(self.lookups, 1, "a step has one complete-key candidate");
        self.index.lookup(scope, position)
    }
}

/// This pair is fixture-local only. Numeric carrier markers are not Content activation evidence.
struct FixtureBinding<'a> {
    context: PreProductionPositionContext,
    static_scope: &'a StaticCellScope,
}

fn prepare_step(
    carrier: &ChannelActorCarrier,
    owner: &NamespaceContinuityGuard,
    expected: PositionSnapshot,
    requested_scope: &StaticCellScope,
    binding: &FixtureBinding<'_>,
    probe: &mut CellProbe<'_>,
    step: Step,
) -> Result<Proposal, MoveError> {
    let current = carrier
        .read_position(owner, expected.actor_ref)
        .map_err(MoveError::Actor)?;
    if current != expected {
        return Err(MoveError::SnapshotMismatch);
    }
    if current.version.context != binding.context {
        return Err(MoveError::PositionContextMismatch);
    }
    if requested_scope != binding.static_scope {
        return Err(MoveError::CellScopeMismatch);
    }
    let (dx, dy) = step.delta();
    let source = current.version.position;
    let x = source
        .x
        .checked_add(dx)
        .ok_or(MoveError::CoordinateOverflow)?;
    let y = source
        .y
        .checked_add(dy)
        .ok_or(MoveError::CoordinateOverflow)?;
    let destination = LocalPosition {
        x,
        y,
        floor: source.floor,
    };
    let cell =
        StaticCellPosition::checked_from_wide(i64::from(x), i64::from(y), i32::from(source.floor))
            .map_err(MoveError::Cell)?;
    match probe
        .lookup(requested_scope, cell)
        .map_err(MoveError::Cell)?
    {
        CollisionClass::Walkable => Ok(Proposal {
            expected,
            destination,
        }),
        CollisionClass::Blocked => Err(MoveError::Blocked),
    }
}

fn commit_step(
    carrier: &mut ChannelActorCarrier,
    owner: &NamespaceContinuityGuard,
    proposal: Proposal,
) -> Result<PositionSnapshot, MoveError> {
    carrier
        .compare_commit_position(
            owner,
            proposal.expected,
            proposal.expected.version.context,
            proposal.destination,
        )
        .map_err(MoveError::Actor)
}

fn step_once(
    carrier: &mut ChannelActorCarrier,
    owner: &NamespaceContinuityGuard,
    expected: PositionSnapshot,
    requested_scope: &StaticCellScope,
    binding: &FixtureBinding<'_>,
    probe: &mut CellProbe<'_>,
    step: Step,
) -> Result<PositionSnapshot, MoveError> {
    let proposal = prepare_step(
        carrier,
        owner,
        expected,
        requested_scope,
        binding,
        probe,
        step,
    )?;
    commit_step(carrier, owner, proposal)
}

fn uuid_v7(seed: u64) -> [u8; 16] {
    let mut bytes = [0; 16];
    bytes[8..].copy_from_slice(&seed.to_be_bytes());
    bytes[6] = 0x70;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    bytes
}

struct Harness {
    owner: NamespaceContinuityGuard,
    carrier: ChannelActorCarrier,
    actor: ActorRef,
    snapshot: PositionSnapshot,
    scope: StaticCellScope,
}

impl Harness {
    fn new(position: LocalPosition) -> Self {
        let grant = PreProductionContinuityGrant {
            world_id: WorldId::decode(&uuid_v7(10)).expect("world fixture"),
            channel_id: ChannelId::decode(&uuid_v7(11)).expect("channel fixture"),
            scope_generation: ScopeOwnershipGeneration::new(1).expect("scope fixture"),
        };
        let mut owner = NamespaceContinuityGuard::from_pre_production_grant(grant);
        let mut carrier = ChannelActorCarrier::bootstrap_pre_production(&mut owner, 1)
            .expect("one fixture actor slot");
        let actor = carrier
            .admit(&owner, ActorState(1))
            .expect("admit fixture actor");
        let context = PreProductionPositionContext {
            world_id: owner.world_id,
            channel_id: owner.channel_id,
            scope_generation: owner.current_generation,
            coordinate_frame_marker: 11,
            map_revision_marker: 12,
            content_generation_marker: 13,
        };
        let snapshot = carrier
            .initialize_position(&owner, actor, context, position)
            .expect("initialize fixture position");
        let scope = StaticCellScope::fixture(
            owner.world_id,
            CoordinateFrameRef::new("movement-fixture-frame").expect("frame fixture"),
            ProductionAtom::new("fixture generation", "movement-fixture-generation")
                .expect("generation fixture"),
            MapRevisionRef::new("movement-fixture-map").expect("map fixture"),
        );
        Self {
            owner,
            carrier,
            actor,
            snapshot,
            scope,
        }
    }

    fn index(&self, destination: LocalPosition, fact: StaticCellFact) -> ReferenceStaticCellIndex {
        ReferenceStaticCellIndex::from_fixture(vec![StaticCellFixtureRecord::fixture(
            &self.scope,
            StaticCellPosition::fixture(destination.x, destination.y, destination.floor),
            fact,
        )])
        .expect("one synthetic cell")
    }
}

#[test]
fn each_cardinal_step_uses_one_exact_candidate_and_commits_on_the_owner() {
    for (direction, x, y) in [
        (Step::North, 4, 4),
        (Step::East, 5, 5),
        (Step::South, 4, 6),
        (Step::West, 3, 5),
    ] {
        let mut fixture = Harness::new(LocalPosition {
            x: 4,
            y: 5,
            floor: 7,
        });
        let destination = LocalPosition { x, y, floor: 7 };
        let index = fixture.index(
            destination,
            StaticCellFact::Qualified(CollisionClass::Walkable),
        );
        let mut probe = CellProbe {
            index: &index,
            lookups: 0,
        };
        let binding = FixtureBinding {
            context: fixture.snapshot.version.context,
            static_scope: &fixture.scope,
        };
        let committed = step_once(
            &mut fixture.carrier,
            &fixture.owner,
            fixture.snapshot,
            &fixture.scope,
            &binding,
            &mut probe,
            direction,
        )
        .expect("walkable cardinal step");
        assert_eq!(probe.lookups, 1);
        assert_eq!(committed.version.position, destination);
        assert_eq!(
            committed.version.revision,
            fixture.snapshot.version.revision + 1
        );
        assert_eq!(
            fixture.carrier.read_position(&fixture.owner, fixture.actor),
            Ok(committed)
        );
    }
}

#[test]
fn blocked_absent_unqualified_and_conflict_reject_without_position_or_revision_change() {
    for (fact, expected_error) in [
        (
            Some(StaticCellFact::Qualified(CollisionClass::Blocked)),
            MoveError::Blocked,
        ),
        (None, MoveError::Cell(StaticCellError::Absent)),
        (
            Some(StaticCellFact::Unqualified),
            MoveError::Cell(StaticCellError::Unqualified),
        ),
        (
            Some(StaticCellFact::Conflict),
            MoveError::Cell(StaticCellError::Conflict),
        ),
    ] {
        let mut fixture = Harness::new(LocalPosition {
            x: 4,
            y: 5,
            floor: 7,
        });
        let destination = LocalPosition {
            x: 5,
            y: 5,
            floor: 7,
        };
        let records = fact.map_or_else(Vec::new, |fact| {
            vec![StaticCellFixtureRecord::fixture(
                &fixture.scope,
                StaticCellPosition::fixture(destination.x, destination.y, destination.floor),
                fact,
            )]
        });
        let index = ReferenceStaticCellIndex::from_fixture(records).expect("synthetic facts");
        let mut probe = CellProbe {
            index: &index,
            lookups: 0,
        };
        let binding = FixtureBinding {
            context: fixture.snapshot.version.context,
            static_scope: &fixture.scope,
        };
        assert_eq!(
            step_once(
                &mut fixture.carrier,
                &fixture.owner,
                fixture.snapshot,
                &fixture.scope,
                &binding,
                &mut probe,
                Step::East
            ),
            Err(expected_error)
        );
        assert_eq!(probe.lookups, 1);
        assert_eq!(
            fixture.carrier.read_position(&fixture.owner, fixture.actor),
            Ok(fixture.snapshot)
        );
    }
}

#[test]
fn cell_in_another_scope_or_floor_cannot_supply_the_candidate() {
    let mut fixture = Harness::new(LocalPosition {
        x: 4,
        y: 5,
        floor: 7,
    });
    let other_scope = StaticCellScope::fixture(
        fixture.owner.world_id,
        CoordinateFrameRef::new("other-frame").expect("frame"),
        ProductionAtom::new("fixture generation", "movement-fixture-generation")
            .expect("generation"),
        MapRevisionRef::new("movement-fixture-map").expect("map"),
    );
    for (scope, floor) in [(&other_scope, 7), (&fixture.scope, 8)] {
        let index = ReferenceStaticCellIndex::from_fixture(vec![StaticCellFixtureRecord::fixture(
            scope,
            StaticCellPosition::fixture(5, 5, floor),
            StaticCellFact::Qualified(CollisionClass::Walkable),
        )])
        .expect("mismatched synthetic cell");
        let mut probe = CellProbe {
            index: &index,
            lookups: 0,
        };
        let binding = FixtureBinding {
            context: fixture.snapshot.version.context,
            static_scope: &fixture.scope,
        };
        assert_eq!(
            step_once(
                &mut fixture.carrier,
                &fixture.owner,
                fixture.snapshot,
                &fixture.scope,
                &binding,
                &mut probe,
                Step::East
            ),
            Err(MoveError::Cell(StaticCellError::Absent))
        );
        assert_eq!(probe.lookups, 1);
        assert_eq!(
            fixture.carrier.read_position(&fixture.owner, fixture.actor),
            Ok(fixture.snapshot)
        );
    }
}

#[test]
fn stale_snapshot_wrong_scope_context_and_coordinate_overflow_reject_before_lookup() {
    let fixture = Harness::new(LocalPosition {
        x: 4,
        y: 5,
        floor: 7,
    });
    let index = fixture.index(
        LocalPosition {
            x: 5,
            y: 5,
            floor: 7,
        },
        StaticCellFact::Qualified(CollisionClass::Walkable),
    );
    let mut probe = CellProbe {
        index: &index,
        lookups: 0,
    };
    let mut stale = fixture.snapshot;
    stale.version.revision += 1;
    let binding = FixtureBinding {
        context: fixture.snapshot.version.context,
        static_scope: &fixture.scope,
    };
    assert_eq!(
        prepare_step(
            &fixture.carrier,
            &fixture.owner,
            stale,
            &fixture.scope,
            &binding,
            &mut probe,
            Step::East
        ),
        Err(MoveError::SnapshotMismatch)
    );
    let wrong_scope = StaticCellScope::fixture(
        fixture.owner.world_id,
        CoordinateFrameRef::new("other-frame").expect("frame"),
        ProductionAtom::new("fixture generation", "movement-fixture-generation")
            .expect("generation"),
        MapRevisionRef::new("movement-fixture-map").expect("map"),
    );
    assert_eq!(
        prepare_step(
            &fixture.carrier,
            &fixture.owner,
            fixture.snapshot,
            &wrong_scope,
            &binding,
            &mut probe,
            Step::East
        ),
        Err(MoveError::CellScopeMismatch)
    );
    let wrong_binding = FixtureBinding {
        context: PreProductionPositionContext {
            coordinate_frame_marker: 99,
            ..binding.context
        },
        static_scope: &fixture.scope,
    };
    assert_eq!(
        prepare_step(
            &fixture.carrier,
            &fixture.owner,
            fixture.snapshot,
            &fixture.scope,
            &wrong_binding,
            &mut probe,
            Step::East
        ),
        Err(MoveError::PositionContextMismatch)
    );
    let mut wrong_actor = fixture.actor;
    wrong_actor.channel_id = ChannelId::decode(&uuid_v7(99)).expect("other channel");
    let wrong_snapshot = PositionSnapshot {
        actor_ref: wrong_actor,
        ..fixture.snapshot
    };
    assert_eq!(
        prepare_step(
            &fixture.carrier,
            &fixture.owner,
            wrong_snapshot,
            &fixture.scope,
            &binding,
            &mut probe,
            Step::East
        ),
        Err(MoveError::Actor(CarrierError::WrongScope))
    );
    assert_eq!(probe.lookups, 0);
    assert_eq!(
        fixture.carrier.read_position(&fixture.owner, fixture.actor),
        Ok(fixture.snapshot)
    );

    for (position, direction) in [
        (
            LocalPosition {
                x: i32::MAX,
                y: 5,
                floor: 7,
            },
            Step::East,
        ),
        (
            LocalPosition {
                x: i32::MIN,
                y: 5,
                floor: 7,
            },
            Step::West,
        ),
        (
            LocalPosition {
                x: 4,
                y: i32::MAX,
                floor: 7,
            },
            Step::South,
        ),
        (
            LocalPosition {
                x: 4,
                y: i32::MIN,
                floor: 7,
            },
            Step::North,
        ),
    ] {
        let mut edge = Harness::new(position);
        let index = ReferenceStaticCellIndex::from_fixture(vec![]).expect("empty index");
        let mut probe = CellProbe {
            index: &index,
            lookups: 0,
        };
        let binding = FixtureBinding {
            context: edge.snapshot.version.context,
            static_scope: &edge.scope,
        };
        assert_eq!(
            step_once(
                &mut edge.carrier,
                &edge.owner,
                edge.snapshot,
                &edge.scope,
                &binding,
                &mut probe,
                direction
            ),
            Err(MoveError::CoordinateOverflow)
        );
        assert_eq!(probe.lookups, 0);
        assert_eq!(
            edge.carrier.read_position(&edge.owner, edge.actor),
            Ok(edge.snapshot)
        );
    }
}

#[test]
fn recycled_actor_and_stale_owner_reject_before_cell_lookup() {
    let mut fixture = Harness::new(LocalPosition {
        x: 4,
        y: 5,
        floor: 7,
    });
    let stale_actor = fixture.actor;
    let stale_snapshot = fixture.snapshot;
    fixture
        .carrier
        .remove(&fixture.owner, stale_actor)
        .expect("remove");
    fixture.actor = fixture
        .carrier
        .admit(&fixture.owner, ActorState(2))
        .expect("reuse");
    fixture.snapshot = fixture
        .carrier
        .initialize_position(
            &fixture.owner,
            fixture.actor,
            stale_snapshot.version.context,
            stale_snapshot.version.position,
        )
        .expect("new actor position");
    let index = fixture.index(
        LocalPosition {
            x: 5,
            y: 5,
            floor: 7,
        },
        StaticCellFact::Qualified(CollisionClass::Walkable),
    );
    let mut probe = CellProbe {
        index: &index,
        lookups: 0,
    };
    let binding = FixtureBinding {
        context: fixture.snapshot.version.context,
        static_scope: &fixture.scope,
    };
    assert_eq!(
        prepare_step(
            &fixture.carrier,
            &fixture.owner,
            stale_snapshot,
            &fixture.scope,
            &binding,
            &mut probe,
            Step::East
        ),
        Err(MoveError::Actor(CarrierError::StaleActorGeneration))
    );
    assert_eq!(probe.lookups, 0);
    assert_eq!(
        fixture.carrier.read_position(&fixture.owner, fixture.actor),
        Ok(fixture.snapshot)
    );
    fixture
        .owner
        .advance(PreProductionContinuityGrant {
            world_id: fixture.owner.world_id,
            channel_id: fixture.owner.channel_id,
            scope_generation: ScopeOwnershipGeneration::new(2).expect("new owner generation"),
        })
        .expect("advance owner");
    assert_eq!(
        prepare_step(
            &fixture.carrier,
            &fixture.owner,
            fixture.snapshot,
            &fixture.scope,
            &binding,
            &mut probe,
            Step::East
        ),
        Err(MoveError::Actor(CarrierError::WrongScope))
    );
    assert_eq!(probe.lookups, 0);
    assert_eq!(
        fixture.carrier.slots[0],
        Slot::Occupied {
            generation: fixture.actor.actor_local_generation.0,
            actor: ActorState(2),
            position: Some(fixture.snapshot.version),
        }
    );
}

#[test]
fn compare_commit_rejects_intervening_write_and_replayed_proposal() {
    let mut fixture = Harness::new(LocalPosition {
        x: 4,
        y: 5,
        floor: 7,
    });
    let destination = LocalPosition {
        x: 5,
        y: 5,
        floor: 7,
    };
    let index = fixture.index(
        destination,
        StaticCellFact::Qualified(CollisionClass::Walkable),
    );
    let mut probe = CellProbe {
        index: &index,
        lookups: 0,
    };
    let binding = FixtureBinding {
        context: fixture.snapshot.version.context,
        static_scope: &fixture.scope,
    };
    let proposal = prepare_step(
        &fixture.carrier,
        &fixture.owner,
        fixture.snapshot,
        &fixture.scope,
        &binding,
        &mut probe,
        Step::East,
    )
    .expect("qualified proposal");
    assert_eq!(probe.lookups, 1);
    let intervening = fixture
        .carrier
        .compare_commit_position(
            &fixture.owner,
            fixture.snapshot,
            fixture.snapshot.version.context,
            LocalPosition {
                x: 4,
                y: 6,
                floor: 7,
            },
        )
        .expect("intervening owner commit");
    assert_eq!(
        commit_step(&mut fixture.carrier, &fixture.owner, proposal),
        Err(MoveError::Actor(CarrierError::PositionSnapshotMismatch))
    );
    assert_eq!(
        fixture.carrier.read_position(&fixture.owner, fixture.actor),
        Ok(intervening)
    );
    let mut replay_probe = CellProbe {
        index: &index,
        lookups: 0,
    };
    assert_eq!(
        prepare_step(
            &fixture.carrier,
            &fixture.owner,
            fixture.snapshot,
            &fixture.scope,
            &binding,
            &mut replay_probe,
            Step::East
        ),
        Err(MoveError::SnapshotMismatch)
    );
    assert_eq!(replay_probe.lookups, 0);
    assert_eq!(
        fixture.carrier.read_position(&fixture.owner, fixture.actor),
        Ok(intervening)
    );

    let fresh_index = fixture.index(
        LocalPosition {
            x: 5,
            y: 6,
            floor: 7,
        },
        StaticCellFact::Qualified(CollisionClass::Walkable),
    );
    let mut fresh_probe = CellProbe {
        index: &fresh_index,
        lookups: 0,
    };
    let fresh = prepare_step(
        &fixture.carrier,
        &fixture.owner,
        intervening,
        &fixture.scope,
        &binding,
        &mut fresh_probe,
        Step::East,
    )
    .expect("fresh proposal");
    let committed = commit_step(&mut fixture.carrier, &fixture.owner, fresh).expect("commit once");
    assert_eq!(committed.version.revision, intervening.version.revision + 1);
    assert_eq!(
        commit_step(&mut fixture.carrier, &fixture.owner, fresh),
        Err(MoveError::Actor(CarrierError::PositionSnapshotMismatch))
    );
    assert_eq!(
        fixture.carrier.read_position(&fixture.owner, fixture.actor),
        Ok(committed)
    );
}
