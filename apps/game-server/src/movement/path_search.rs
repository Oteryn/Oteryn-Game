//! CREATURE-MOVE-1 (CREATURE-AI-0 §5.3): the shared bounded path search of the Movement owner,
//! for a player chase (RANGED-0 §8, `combat_attack::chase`) and later creatures.
//!
//! Cardinal neighbours in the order north, east, south, west; 10 per step plus the caller's extra
//! tile cost (+40 over a creature); ties by lowest total, then lowest estimate, then neighbour
//! order, then discovery order; at most `AI01-PATH-SEARCH-WORK` (`RANGED0-RL-04`) expansions and
//! `AI01-ROUTE-STEPS` (`RANGED0-RL-05`) steps. The goal is the nearest tile at Chebyshev distance
//! 1 from the target. Which tiles are path tiles and what they cost is the caller's; the search
//! never changes floors. How often a search may run is the caller's budget.
//!
//! Pure and runtime-only: nothing here is durable.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap};

use crate::foundation::MovementLocalPosition;
use crate::movement::CardinalStep;

/// `AI01-PATH-SEARCH-WORK` (`RANGED0-RL-04`): expansions per search.
pub(crate) const PATH_SEARCH_WORK: usize = 1_024;
/// `AI01-ROUTE-STEPS` (`RANGED0-RL-05`): the path length bound in tiles.
pub(crate) const ROUTE_STEPS: usize = 12;

/// The cost of one step.
const STEP_COST: u32 = 10;
/// Per discovered tile: its best cost and the step and tile it was reached by.
type Discovered = BTreeMap<(i32, i32), (u32, Option<(CardinalStep, (i32, i32))>)>;
/// The extra cost of a path tile a creature stands on.
pub(crate) const CREATURE_COST: u32 = 40;
const NEIGHBOURS: [CardinalStep; 4] = [
    CardinalStep::North,
    CardinalStep::East,
    CardinalStep::South,
    CardinalStep::West,
];

pub(crate) const fn delta(step: CardinalStep) -> (i32, i32) {
    match step {
        CardinalStep::North => (0, -1),
        CardinalStep::East => (1, 0),
        CardinalStep::South => (0, 1),
        CardinalStep::West => (-1, 0),
    }
}

pub(crate) fn chebyshev(from: MovementLocalPosition, to: MovementLocalPosition) -> i64 {
    let dx = (i64::from(to.x) - i64::from(from.x)).abs();
    let dy = (i64::from(to.y) - i64::from(from.y)).abs();
    dx.max(dy)
}

/// The fewest cardinal steps from `at` to a tile at Chebyshev distance 1 from `target`, times
/// the step cost: admissible and consistent for the goal set.
fn estimate(at: MovementLocalPosition, target: MovementLocalPosition) -> u32 {
    let dx = (i64::from(target.x) - i64::from(at.x)).unsigned_abs();
    let dy = (i64::from(target.y) - i64::from(at.y)).unsigned_abs();
    let steps = if dx == 0 && dy == 0 {
        1
    } else {
        dx.saturating_sub(1).saturating_add(dy.saturating_sub(1))
    };
    u32::try_from(steps)
        .unwrap_or(u32::MAX)
        .saturating_mul(STEP_COST)
}

fn is_goal(at: MovementLocalPosition, target: MovementLocalPosition) -> bool {
    at.floor == target.floor && chebyshev(at, target) == 1
}

/// The route from `start` to the nearest tile at Chebyshev distance 1 from `target` on the same
/// floor. `tile_cost` answers each candidate tile: `None` when it is no path tile, else its
/// extra cost ([`CREATURE_COST`] over a creature). `Some(empty)` when `start` already is such a
/// tile; `None` when no route is found within the bounds (a failed search).
pub(crate) fn route_next_to(
    start: MovementLocalPosition,
    target: MovementLocalPosition,
    mut tile_cost: impl FnMut(MovementLocalPosition) -> Option<u32>,
) -> Option<Vec<CardinalStep>> {
    if start.floor != target.floor {
        return None;
    }
    if is_goal(start, target) {
        return Some(Vec::new());
    }
    // Open entries are ordered by (total, estimate, neighbour order, discovery order).
    let mut open = BinaryHeap::new();
    // Per discovered tile: its best cost and how it was reached.
    let mut best = Discovered::new();
    let mut discovered: u64 = 0;
    best.insert((start.x, start.y), (0, None));
    open.push(Reverse((
        estimate(start, target),
        estimate(start, target),
        0_u8,
        0_u64,
        start.x,
        start.y,
        0_u32,
        0_usize,
    )));
    let mut expansions = 0_usize;
    while let Some(Reverse((_, _, _, _, x, y, cost, steps))) = open.pop() {
        if best.get(&(x, y)).is_some_and(|(known, _)| *known < cost) {
            continue;
        }
        let at = MovementLocalPosition {
            x,
            y,
            floor: start.floor,
        };
        if is_goal(at, target) {
            return route_to(&best, (start.x, start.y), (x, y));
        }
        expansions += 1;
        if expansions > PATH_SEARCH_WORK {
            return None;
        }
        // `AI01-ROUTE-STEPS`: no route is longer than the path length bound.
        if steps >= ROUTE_STEPS {
            continue;
        }
        for (order, step) in (0_u8..).zip(NEIGHBOURS) {
            let (dx, dy) = delta(step);
            let (Some(nx), Some(ny)) = (x.checked_add(dx), y.checked_add(dy)) else {
                continue;
            };
            let next = MovementLocalPosition {
                x: nx,
                y: ny,
                floor: start.floor,
            };
            let Some(extra) = tile_cost(next) else {
                continue;
            };
            let next_cost = cost.saturating_add(STEP_COST).saturating_add(extra);
            if best
                .get(&(nx, ny))
                .is_some_and(|(known, _)| *known <= next_cost)
            {
                continue;
            }
            best.insert((nx, ny), (next_cost, Some((step, (x, y)))));
            discovered += 1;
            let h = estimate(next, target);
            open.push(Reverse((
                next_cost.saturating_add(h),
                h,
                order,
                discovered,
                nx,
                ny,
                next_cost,
                steps + 1,
            )));
        }
    }
    None
}

fn route_to(best: &Discovered, start: (i32, i32), goal: (i32, i32)) -> Option<Vec<CardinalStep>> {
    let mut steps = Vec::new();
    let mut at = goal;
    while at != start {
        let (_, Some((step, from))) = best.get(&at)? else {
            return None;
        };
        steps.push(*step);
        if steps.len() > ROUTE_STEPS {
            return None;
        }
        at = *from;
    }
    steps.reverse();
    Some(steps)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn at(x: i32, y: i32) -> MovementLocalPosition {
        MovementLocalPosition { x, y, floor: 7 }
    }

    fn open(_: MovementLocalPosition) -> Option<u32> {
        Some(0)
    }

    /// Where `route` from `start` ends.
    fn walk(start: MovementLocalPosition, route: &[CardinalStep]) -> MovementLocalPosition {
        route.iter().fold(start, |at, step| {
            let (dx, dy) = delta(*step);
            MovementLocalPosition {
                x: at.x + dx,
                y: at.y + dy,
                floor: at.floor,
            }
        })
    }

    #[test]
    fn an_open_field_route_ends_next_to_the_target_in_the_fewest_steps() {
        let route = route_next_to(at(0, 0), at(5, 0), open).unwrap();
        assert_eq!(route, vec![CardinalStep::East; 4]);
        let diagonal = route_next_to(at(0, 0), at(3, 3), open).unwrap();
        assert_eq!(
            diagonal.len(),
            4,
            "(2, 2) is the nearest tile next to (3, 3)"
        );
        assert_eq!(walk(at(0, 0), &diagonal), at(2, 2));
    }

    #[test]
    fn equal_routes_break_ties_by_estimate_then_neighbour_order_deterministically() {
        // Every shortest route to (2, 2) costs the same; the lower estimate and then the
        // neighbour order N, E, S, W pick one, the same one every time.
        let first = route_next_to(at(0, 0), at(3, 3), open).unwrap();
        for _ in 0..8 {
            assert_eq!(route_next_to(at(0, 0), at(3, 3), open).unwrap(), first);
        }
        assert_eq!(first[0], CardinalStep::East, "east precedes south");
        let north = route_next_to(at(0, 0), at(-3, -3), open).unwrap();
        assert_eq!(north[0], CardinalStep::North, "north precedes west");
    }

    #[test]
    fn an_adjacent_or_diagonal_target_needs_no_step_and_another_floor_is_no_route() {
        assert_eq!(route_next_to(at(0, 0), at(1, 1), open), Some(Vec::new()));
        assert_eq!(route_next_to(at(0, 0), at(0, 1), open), Some(Vec::new()));
        let below = MovementLocalPosition {
            x: 3,
            y: 0,
            floor: 8,
        };
        assert_eq!(route_next_to(at(0, 0), below, open), None);
    }

    #[test]
    fn a_wall_is_walked_around_and_a_closed_room_is_a_failed_search() {
        // A wall on x = 2 from y = -3 to y = 3.
        let wall = |cell: MovementLocalPosition| {
            (!(cell.x == 2 && (-3..=3).contains(&cell.y))).then_some(0)
        };
        let route = route_next_to(at(0, 0), at(4, 0), wall).unwrap();
        let end = walk(at(0, 0), &route);
        assert_eq!(chebyshev(end, at(4, 0)), 1);
        let mut position = at(0, 0);
        for step in &route {
            position = walk(position, &[*step]);
            assert!(wall(position).is_some(), "the route never enters the wall");
        }
        // The target walled in on every side.
        let boxed = |cell: MovementLocalPosition| (chebyshev(cell, at(6, 0)) > 2).then_some(0);
        assert_eq!(route_next_to(at(0, 0), at(6, 0), boxed), None);
    }

    #[test]
    fn a_creature_costs_forty_so_a_short_detour_goes_around_it() {
        let creature = at(1, 0);
        let cost =
            |cell: MovementLocalPosition| Some(if cell == creature { CREATURE_COST } else { 0 });
        let route = route_next_to(at(0, 0), at(4, 0), cost).unwrap();
        let mut position = at(0, 0);
        for step in &route {
            position = walk(position, &[*step]);
            assert_ne!(position, creature, "a 2-step detour is cheaper than +40");
        }
        assert_eq!(
            route.len(),
            4,
            "around it to a diagonal neighbour of the target"
        );
    }

    #[test]
    fn the_search_stays_within_twelve_tiles_and_its_work_bound() {
        // Thirteen tiles away the goal tiles lie beyond the 12-step path length.
        assert_eq!(route_next_to(at(0, 0), at(14, 0), open), None);
        assert_eq!(
            route_next_to(at(0, 0), at(13, 0), open).map(|r| r.len()),
            Some(12)
        );
        // A maze-free but unreachable goal expands at most the work bound and fails.
        let mut calls = 0_usize;
        let unreachable = |cell: MovementLocalPosition| {
            calls += 1;
            (chebyshev(cell, at(12, 0)) > 1).then_some(0)
        };
        assert_eq!(route_next_to(at(0, 0), at(12, 0), unreachable), None);
        assert!(calls <= 4 * PATH_SEARCH_WORK, "{calls} tile reads");
    }
}
