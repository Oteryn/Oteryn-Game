use super::{
    AdmittedSession, ComposedFreshAdmission, IRIANA, JIMMY, Position, TestResult, debug, target,
    use_target,
};
use crate::content::{CollisionClass, LogicalCell};
use crate::map::floor::{BundleDoors, Cell, CellBlocker, Ground, resolve_landing, table_of};
use crate::map::overlay::TilePos;
use crate::movement::CardinalStep;
use oteryn_protocol_oteryn::world_object::UseDisposition;
use oteryn_protocol_oteryn::world_spatial::StepDirection;
use std::collections::{BTreeMap, VecDeque};

const LADDER_UP: TilePos = TilePos { x: 32833, y: 31331, floor: -6 };
const JIMMY_DOOR: TilePos = TilePos { x: 32858, y: 31374, floor: -6 };

pub(super) async fn walk_to_jimmy(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult<Position> {
    assert_eq!(actor_cell(owner, admitted).await?, cell(super::city_start()));

    // Walk onto Iriana's north-facing stair (Crystal items.xml:14781-14783).
    walk_to(owner, admitted, (32845, 31300, 7)).await?;
    step_to(owner, admitted, StepDirection::East, (32846, 31299, 6)).await?;

    // TibiaWiki Mission One, Step One: cross the roofs and USE the ladder up.
    walk_to(owner, admitted, (32834, 31331, 6)).await?;
    use_placement(owner, admitted, LADDER_UP).await?;
    assert_eq!(actor_cell(owner, admitted).await?, (32833, 31332, 5));

    // Step Two: cross the upper roofs and descend onto the ladder below.
    // The top is a walk-on trapdoor (Crystal items.xml:744-747).
    walk_to(owner, admitted, (32856, 31362, 5)).await?;
    step_to(owner, admitted, StepDirection::South, (32856, 31363, 6)).await?;

    let door_approach = (32858, 31373, 6);
    walk_to(owner, admitted, door_approach).await?;
    use_placement(owner, admitted, JIMMY_DOOR).await?;
    assert_eq!(actor_cell(owner, admitted).await?, door_approach);
    step_to(owner, admitted, StepDirection::South, (32858, 31374, 6)).await?;

    let stop = Position { x: 32858, y: 31382, floor: JIMMY.floor };
    walk_to(owner, admitted, cell(stop)).await?;
    Ok(stop)
}

fn cell(position: Position) -> Cell {
    (i32::from(position.x), i32::from(position.y), -i32::from(position.floor))
}

async fn actor_cell(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult<Cell> {
    let actor = admitted.runtime_actor.ok_or("city actor")?;
    let origin = owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?
        .position();
    Ok((origin.x, origin.y, i32::from(origin.floor)))
}

async fn use_placement(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    at: TilePos,
) -> TestResult {
    let target = target(owner, at)?;
    let command = admitted.continuity.next_command_id;
    assert_eq!(
        use_target(owner, admitted, command, &target).await?,
        UseDisposition::Committed,
        "USE at {at:?}"
    );
    Ok(())
}

async fn step_to(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    direction: StepDirection,
    expected: Cell,
) -> TestResult {
    super::super::desert_earned_route_fixtures::step(owner, admitted, direction).await?;
    assert_eq!(actor_cell(owner, admitted).await?, expected, "STEP {direction:?}");
    Ok(())
}

async fn walk_to(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    stop: Cell,
) -> TestResult {
    let start = actor_cell(owner, admitted).await?;
    let occupied = [cell(IRIANA), cell(JIMMY)];
    let stairs = table_of(&owner.movement_cells).ok_or("city floor changes")?;
    let doors = BundleDoors(owner.bundle_map().ok_or("city map")?.facts.doors());
    let ground = |(x, y, z)| {
        let cell = LogicalCell { x, y, z };
        if doors.blocks(cell) || occupied.contains(&(x, y, z)) {
            return Ground::Blocked;
        }
        match owner
            .movement_cells
            .index()
            .lookup(owner.movement_cells.scope(), cell)
        {
            Ok(CollisionClass::Walkable) => Ground::Walkable,
            Ok(CollisionClass::Blocked) => Ground::Blocked,
            Err(_) => Ground::Absent,
        }
    };

    // Keep the search within the Yalahar roofs and their approaches.
    let mut queue = VecDeque::from([start]);
    let mut previous = BTreeMap::<Cell, Option<(Cell, StepDirection)>>::from([(start, None)]);
    loop {
        let at = queue
            .pop_front()
            .ok_or_else(|| format!("no source STEP route from {start:?} to {stop:?}"))?;
        if at == stop {
            break;
        }
        for (direction, wire) in [
            (CardinalStep::North, StepDirection::North),
            (CardinalStep::East, StepDirection::East),
            (CardinalStep::South, StepDirection::South),
            (CardinalStep::West, StepDirection::West),
        ] {
            let Ok(next) = resolve_landing(stairs, ground, |_| true, at, direction) else {
                continue;
            };
            if !(32795..=32900).contains(&next.0)
                || !(31235..=31405).contains(&next.1)
                || !(4..=10).contains(&next.2)
            {
                continue;
            }
            if let std::collections::btree_map::Entry::Vacant(entry) = previous.entry(next) {
                entry.insert(Some((at, wire)));
                queue.push_back(next);
            }
        }
    };
    let mut path = Vec::new();
    let mut cursor = stop;
    while let Some(&(parent, direction)) = previous.get(&cursor).and_then(Option::as_ref) {
        path.push((cursor, direction));
        cursor = parent;
    }
    for (expected, direction) in path.into_iter().rev() {
        step_to(owner, admitted, direction, expected).await?;
    }
    Ok(())
}
