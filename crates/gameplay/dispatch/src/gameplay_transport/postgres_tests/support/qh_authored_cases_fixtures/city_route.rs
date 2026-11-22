use super::{AdmittedSession, ComposedFreshAdmission, IRIANA, JIMMY, Position, TestResult, debug};
use crate::content::{CollisionClass, LogicalCell};
use crate::map::floor::{BundleDoors, Cell, CellBlocker, Ground, resolve_landing, table_of};
use crate::movement::CardinalStep;
use oteryn_protocol_oteryn::world_spatial::StepDirection;
use std::collections::{BTreeMap, VecDeque};

pub(super) async fn walk_to_jimmy(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult<Position> {
    let actor = admitted.runtime_actor.ok_or("city actor")?;
    let origin = owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?
        .position();
    let start = (origin.x, origin.y, i32::from(origin.floor));
    let npc_cell = |p: Position| (i32::from(p.x), i32::from(p.y), -i32::from(p.floor));
    assert_eq!(start, npc_cell(super::city_start()));
    let occupied = [npc_cell(IRIANA), npc_cell(JIMMY)];
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

    // The roofs and their approaches lie in this part of the real Thais region.
    let mut queue = VecDeque::from([start]);
    let mut previous = BTreeMap::<Cell, Option<(Cell, StepDirection)>>::from([(start, None)]);
    let stop = loop {
        let at = queue
            .pop_front()
            .ok_or("no source STEP route from Iriana to Jimmy")?;
        if at.2 == -i32::from(JIMMY.floor)
            && at
                .0
                .abs_diff(i32::from(JIMMY.x))
                .max(at.1.abs_diff(i32::from(JIMMY.y)))
                == 1
        {
            break at;
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
    for ((x, y, z), direction) in path.into_iter().rev() {
        super::super::desert_earned_route_fixtures::step(owner, admitted, direction).await?;
        let actual = owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?
            .position();
        assert_eq!((actual.x, actual.y, i32::from(actual.floor)), (x, y, z));
    }
    Ok(Position {
        x: u16::try_from(stop.0)?,
        y: u16::try_from(stop.1)?,
        floor: -i8::try_from(stop.2)?,
    })
}
