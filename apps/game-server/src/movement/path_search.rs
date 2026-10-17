//! CREATURE-MOVE-1 (CREATURE-AI-0 §5.3): the shared bounded path search of the Movement owner,
//! for a player chase (RANGED-0 §8, `combat_attack::chase`) and later creatures.
//!
//! Cardinal neighbours in the order north, east, south, west (Movement admits no diagonal step,
//! so §5.3's diagonals stay out); 10 per step plus the caller's extra tile cost (+40 over an
//! occupied tile); ties by lowest total, then lowest estimate, then neighbour order, then
//! discovery order; at most `AI01-PATH-SEARCH-WORK` (1,024) expansions and `AI01-ROUTE-STEPS`
//! (128) steps, within the caller's search distance (Chebyshev, 12 tiles for a chase). The goal
//! is the nearest admitted tile at the authored Chebyshev distance from the target (1 for
//! chase), or the exact spawn cell (walk back, within 50 tiles). A search state is a tile and the
//! steps taken to it, so a cheaper but longer label never hides a dearer, shorter one the step
//! bound still admits. Which tiles are path tiles and what they cost is the caller's; the search
//! never changes floors. How often a search may run is the caller's budget.
//!
//! Pure and runtime-only: nothing here is durable.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap};

use crate::foundation::MovementLocalPosition;
use crate::movement::CardinalStep;

/// `AI01-PATH-SEARCH-WORK` (CREATURE-AI-0 §5.3, `RESOURCE_LIMITS_REGISTRY`), for a chase
/// `RANGED0-RL-04` (RANGED-0 §8): expansions per
/// search; the 1,025th expansion fails the search.
pub(crate) const PATH_SEARCH_WORK: usize = 1_024;
/// `AI01-ROUTE-STEPS` (CREATURE-AI-0 §5.3, `RESOURCE_LIMITS_REGISTRY`): steps of a route.
pub(crate) const ROUTE_STEPS: usize = 128;
/// `RANGED0-RL-05` (RANGED-0 §8, CREATURE-AI-0 §5.3): the 12-tile search distance of a chase,
/// from the start (Chebyshev).
pub(crate) const CHASE_SEARCH_DISTANCE: i32 = 12;
/// CREATURE-MOVE-1 §1.4: walk back searches within the 50-tile leash distance.
const WALK_BACK_SEARCH_DISTANCE: i32 = 50;
/// The extra cost of a path tile another actor stands on (CREATURE-AI-0 §5.3 "+40 over a
/// creature"; a player is a creature of the path profile).
pub(crate) const CREATURE_COST: u32 = 40;

/// The cost of one step.
const STEP_COST: u32 = 10;
const NEIGHBOURS: [CardinalStep; 4] = [
    CardinalStep::North,
    CardinalStep::East,
    CardinalStep::South,
    CardinalStep::West,
];

/// A search state: a tile and the steps taken to reach it.
type State = (i32, i32, usize);
/// Per discovered state: its best cost and the step and state it was reached by.
type Discovered = BTreeMap<State, (u32, Option<(CardinalStep, State)>)>;

/// The bounds of one search.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SearchBounds {
    work: usize,
    route_steps: usize,
    distance: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SearchGoal {
    Distance(u16),
    Cell,
}

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

/// The fewest cardinal steps to the goal set, times the step cost: admissible and consistent.
fn estimate(at: MovementLocalPosition, target: MovementLocalPosition, goal: SearchGoal) -> u32 {
    let dx = (i64::from(target.x) - i64::from(at.x)).unsigned_abs();
    let dy = (i64::from(target.y) - i64::from(at.y)).unsigned_abs();
    let steps = match goal {
        SearchGoal::Cell => dx.saturating_add(dy),
        SearchGoal::Distance(distance) => {
            let distance = u64::from(distance);
            if dx.max(dy) < distance {
                distance - dx.max(dy)
            } else {
                dx.saturating_sub(distance)
                    .saturating_add(dy.saturating_sub(distance))
            }
        }
    };
    u32::try_from(steps)
        .unwrap_or(u32::MAX)
        .saturating_mul(STEP_COST)
}

fn is_goal(at: MovementLocalPosition, target: MovementLocalPosition, goal: SearchGoal) -> bool {
    at.floor == target.floor
        && match goal {
            SearchGoal::Distance(distance) => chebyshev(at, target) == i64::from(distance),
            SearchGoal::Cell => at == target,
        }
}

/// The route from `start` to the nearest tile at Chebyshev distance 1 from `target` on the same
/// floor, within `distance` tiles of `start`. `tile_cost` answers each candidate tile: `None`
/// when it is no path tile, else its extra cost ([`CREATURE_COST`] over an occupied tile).
/// `Some(empty)` when `start` already is such a tile; `None` when no route is found within the
/// bounds (a failed search).
pub(crate) fn route_next_to(
    start: MovementLocalPosition,
    target: MovementLocalPosition,
    distance: i32,
    tile_cost: impl FnMut(MovementLocalPosition) -> Option<u32>,
) -> Option<Vec<CardinalStep>> {
    search(
        start,
        target,
        SearchGoal::Distance(1),
        SearchBounds {
            work: PATH_SEARCH_WORK,
            route_steps: ROUTE_STEPS,
            distance,
        },
        tile_cost,
    )
}

/// CREATURE-MOVE-1 §1.5: fallback for a keep-distance goal, using the chase search bounds.
/// The AI owner decides when local distance steps fail or sight is unclear. Its `can_hold`
/// callback admits an endpoint (including clear sight); rejected endpoints remain traversable
/// if `tile_cost` admits them. This avoids either stopping without sight or treating a sight
/// obstruction as a Movement obstruction. Distance zero refuses without consulting callbacks.
/// No path adoption, local goal selection or runtime authority is supplied here.
pub(crate) fn route_at_distance(
    start: MovementLocalPosition,
    target: MovementLocalPosition,
    target_distance: u16,
    tile_cost: impl FnMut(MovementLocalPosition) -> Option<u32>,
    can_hold: impl FnMut(MovementLocalPosition) -> bool,
) -> Option<Vec<CardinalStep>> {
    if target_distance == 0 {
        return None;
    }
    search_with_goal_admission(
        start,
        target,
        SearchGoal::Distance(target_distance),
        SearchBounds {
            work: PATH_SEARCH_WORK,
            route_steps: ROUTE_STEPS,
            distance: CHASE_SEARCH_DISTANCE,
        },
        tile_cost,
        can_hold,
    )
}

/// Walk back to the exact spawn cell using the same bounded search and Movement-supplied
/// admission/cost callback as chase. `None` means no route within the bounds; the owning think
/// then idles in place until the next wake (CREATURE-MOVE-1 §1.5). An empty route means arrival.
/// This returns only steps: it does not adopt a path, schedule work, relocate or grant Movement
/// authority. The owner must still revalidate every step at execution.
pub(crate) fn route_to_cell(
    start: MovementLocalPosition,
    spawn: MovementLocalPosition,
    tile_cost: impl FnMut(MovementLocalPosition) -> Option<u32>,
) -> Option<Vec<CardinalStep>> {
    search(
        start,
        spawn,
        SearchGoal::Cell,
        SearchBounds {
            work: PATH_SEARCH_WORK,
            route_steps: ROUTE_STEPS,
            distance: WALK_BACK_SEARCH_DISTANCE,
        },
        tile_cost,
    )
}

fn search(
    start: MovementLocalPosition,
    target: MovementLocalPosition,
    goal: SearchGoal,
    bounds: SearchBounds,
    tile_cost: impl FnMut(MovementLocalPosition) -> Option<u32>,
) -> Option<Vec<CardinalStep>> {
    search_with_goal_admission(start, target, goal, bounds, tile_cost, |_| true)
}

fn search_with_goal_admission(
    start: MovementLocalPosition,
    target: MovementLocalPosition,
    goal: SearchGoal,
    bounds: SearchBounds,
    mut tile_cost: impl FnMut(MovementLocalPosition) -> Option<u32>,
    mut can_hold: impl FnMut(MovementLocalPosition) -> bool,
) -> Option<Vec<CardinalStep>> {
    if start.floor != target.floor {
        return None;
    }
    if is_goal(start, target, goal) && can_hold(start) {
        return Some(Vec::new());
    }
    // Open entries are ordered by (total, estimate, neighbour order, discovery order).
    let mut open = BinaryHeap::new();
    let mut best = Discovered::new();
    // Per tile: the (cost, steps) labels found; a label no better in both is dominated.
    let mut labels: BTreeMap<(i32, i32), Vec<(u32, usize)>> = BTreeMap::new();
    let dominated = |labels: &BTreeMap<(i32, i32), Vec<(u32, usize)>>,
                     tile: (i32, i32),
                     cost: u32,
                     steps: usize| {
        labels.get(&tile).is_some_and(|found| {
            found.iter().any(|&(known, known_steps)| {
                known <= cost && known_steps <= steps && (known, known_steps) != (cost, steps)
            })
        })
    };
    let mut discovered: u64 = 0;
    best.insert((start.x, start.y, 0), (0, None));
    labels.insert((start.x, start.y), vec![(0, 0)]);
    open.push(Reverse((
        estimate(start, target, goal),
        estimate(start, target, goal),
        0_u8,
        0_u64,
        start.x,
        start.y,
        0_usize,
        0_u32,
    )));
    let mut expansions = 0_usize;
    while let Some(Reverse((_, _, _, _, x, y, steps, cost))) = open.pop() {
        if best
            .get(&(x, y, steps))
            .is_some_and(|(known, _)| *known < cost)
            || dominated(&labels, (x, y), cost, steps)
        {
            continue;
        }
        let at = MovementLocalPosition {
            x,
            y,
            floor: start.floor,
        };
        if is_goal(at, target, goal) && can_hold(at) {
            return route_to(&best, (start.x, start.y, 0), (x, y, steps));
        }
        expansions += 1;
        if expansions > bounds.work {
            return None;
        }
        if steps >= bounds.route_steps {
            continue;
        }
        let next_steps = steps + 1;
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
            if chebyshev(start, next) > i64::from(bounds.distance) {
                continue;
            }
            let Some(extra) = tile_cost(next) else {
                continue;
            };
            let next_cost = cost.saturating_add(STEP_COST).saturating_add(extra);
            if labels.get(&(nx, ny)).is_some_and(|found| {
                found
                    .iter()
                    .any(|&(known, known_steps)| known <= next_cost && known_steps <= next_steps)
            }) {
                continue;
            }
            labels
                .entry((nx, ny))
                .or_default()
                .push((next_cost, next_steps));
            best.insert(
                (nx, ny, next_steps),
                (next_cost, Some((step, (x, y, steps)))),
            );
            discovered += 1;
            let h = estimate(next, target, goal);
            open.push(Reverse((
                next_cost.saturating_add(h),
                h,
                order,
                discovered,
                nx,
                ny,
                next_steps,
                next_cost,
            )));
        }
    }
    None
}

fn route_to(best: &Discovered, start: State, goal: State) -> Option<Vec<CardinalStep>> {
    let mut steps = Vec::with_capacity(goal.2);
    let mut at = goal;
    while at != start {
        let (_, Some((step, from))) = best.get(&at)? else {
            return None;
        };
        steps.push(*step);
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

    /// A chase search: the real bounds and a 12-tile search distance.
    fn chase(
        start: MovementLocalPosition,
        target: MovementLocalPosition,
        tile_cost: impl FnMut(MovementLocalPosition) -> Option<u32>,
    ) -> Option<Vec<CardinalStep>> {
        route_next_to(start, target, CHASE_SEARCH_DISTANCE, tile_cost)
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
        let route = chase(at(0, 0), at(5, 0), open).unwrap();
        assert_eq!(route, vec![CardinalStep::East; 4]);
        let diagonal = chase(at(0, 0), at(3, 3), open).unwrap();
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
        let first = chase(at(0, 0), at(3, 3), open).unwrap();
        for _ in 0..8 {
            assert_eq!(chase(at(0, 0), at(3, 3), open).unwrap(), first);
        }
        assert_eq!(first[0], CardinalStep::East, "east precedes south");
        let north = chase(at(0, 0), at(-3, -3), open).unwrap();
        assert_eq!(north[0], CardinalStep::North, "north precedes west");
    }

    #[test]
    fn an_adjacent_or_diagonal_target_needs_no_step_and_another_floor_is_no_route() {
        assert_eq!(chase(at(0, 0), at(1, 1), open), Some(Vec::new()));
        assert_eq!(chase(at(0, 0), at(0, 1), open), Some(Vec::new()));
        let below = MovementLocalPosition {
            x: 3,
            y: 0,
            floor: 8,
        };
        assert_eq!(chase(at(0, 0), below, open), None);
    }

    #[test]
    fn a_wall_is_walked_around_and_a_closed_room_is_a_failed_search() {
        // A wall on x = 2 from y = -3 to y = 3.
        let wall = |cell: MovementLocalPosition| {
            (!(cell.x == 2 && (-3..=3).contains(&cell.y))).then_some(0)
        };
        let route = chase(at(0, 0), at(4, 0), wall).unwrap();
        let end = walk(at(0, 0), &route);
        assert_eq!(chebyshev(end, at(4, 0)), 1);
        let mut position = at(0, 0);
        for step in &route {
            position = walk(position, &[*step]);
            assert!(wall(position).is_some(), "the route never enters the wall");
        }
        // The target walled in on every side.
        let boxed = |cell: MovementLocalPosition| (chebyshev(cell, at(6, 0)) > 2).then_some(0);
        assert_eq!(chase(at(0, 0), at(6, 0), boxed), None);
    }

    #[test]
    fn a_creature_costs_forty_so_a_short_detour_goes_around_it() {
        let creature = at(1, 0);
        let cost =
            |cell: MovementLocalPosition| Some(if cell == creature { CREATURE_COST } else { 0 });
        let route = chase(at(0, 0), at(4, 0), cost).unwrap();
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
    fn a_chase_searches_twelve_tiles_from_its_start_and_routes_at_most_128_steps() {
        // The goal (12, 0) lies on the search distance; (13, 0) lies beyond it.
        assert_eq!(chase(at(0, 0), at(13, 0), open).map(|r| r.len()), Some(12));
        assert_eq!(chase(at(0, 0), at(14, 0), open), None);
        assert_eq!(ROUTE_STEPS, 128);
        assert_eq!(PATH_SEARCH_WORK, 1_024);
        // Within a wide distance a route holds at most `ROUTE_STEPS` steps.
        let wide = |target| route_next_to(at(0, 0), target, i32::MAX, open).map(|r| r.len());
        assert_eq!(wide(at(129, 0)), Some(128));
        assert_eq!(wide(at(130, 0)), None);
    }

    #[test]
    fn a_dearer_shorter_label_survives_a_cheaper_longer_one_under_the_step_bound() {
        // Through the creature on (1, 0): 4 steps costing 80; around it: 6 steps costing 60,
        // beyond a 4-step bound. A tile-keyed search keeps only the cheaper label of (1, 0)'s
        // neighbours and fails; the step-aware search finds the 4-step route.
        let creature = at(1, 0);
        let cost =
            |cell: MovementLocalPosition| Some(if cell == creature { CREATURE_COST } else { 0 });
        let bounds = SearchBounds {
            work: PATH_SEARCH_WORK,
            route_steps: 4,
            distance: i32::MAX,
        };
        let route = search(at(0, 0), at(5, 0), SearchGoal::Distance(1), bounds, cost).unwrap();
        assert_eq!(route, vec![CardinalStep::East; 4]);
        assert_eq!(walk(at(0, 0), &route[..1]), creature);
    }

    #[test]
    fn the_1025th_expansion_fails_the_search() {
        // A corridor open only on y = 0: reaching (n, 0) expands (0, 0) .. (n - 1, 0) and
        // pops (n, 0) as the goal without expanding it.
        let corridor = |cell: MovementLocalPosition| (cell.y == 0).then_some(0);
        let bounds = SearchBounds {
            work: PATH_SEARCH_WORK,
            route_steps: usize::MAX,
            distance: i32::MAX,
        };
        let route = search(
            at(0, 0),
            at(1_025, 0),
            SearchGoal::Distance(1),
            bounds,
            corridor,
        )
        .map(|r| r.len());
        assert_eq!(route, Some(1_024), "1,024 expansions reach (1024, 0)");
        assert_eq!(
            search(
                at(0, 0),
                at(1_026, 0),
                SearchGoal::Distance(1),
                bounds,
                corridor
            ),
            None
        );
    }

    #[test]
    fn a_corridor_route_needing_the_full_work_bound_is_found_and_one_more_fails() {
        // `RANGED0-RL-04` / `AI01-PATH-SEARCH-WORK`: in a one-tile corridor every tile before
        // the goal is expanded once, so a target `k` tiles away costs `k - 1` expansions.
        let corridor = |cell: MovementLocalPosition| (cell.y == 0).then_some(0);
        let unbounded = |work| SearchBounds {
            work,
            route_steps: usize::MAX,
            distance: i32::MAX,
        };
        let far = |expansions: usize| at(i32::try_from(expansions + 1).unwrap(), 0);
        let route = search(
            at(0, 0),
            far(PATH_SEARCH_WORK),
            SearchGoal::Distance(1),
            unbounded(PATH_SEARCH_WORK),
            corridor,
        );
        assert_eq!(route.map(|steps| steps.len()), Some(PATH_SEARCH_WORK));
        assert_eq!(
            search(
                at(0, 0),
                far(PATH_SEARCH_WORK + 1),
                SearchGoal::Distance(1),
                unbounded(PATH_SEARCH_WORK),
                corridor,
            ),
            None,
            "the 1,025th expansion fails the search"
        );
    }

    #[test]
    fn an_unreachable_goal_fails_within_the_work_bound() {
        let mut calls = 0_usize;
        let unreachable = |cell: MovementLocalPosition| {
            calls += 1;
            (chebyshev(cell, at(12, 0)) > 1).then_some(0)
        };
        assert_eq!(chase(at(0, 0), at(12, 0), unreachable), None);
        assert!(calls <= 4 * PATH_SEARCH_WORK, "{calls} tile reads");
    }

    #[test]
    fn creature_keep_distance_approaches_retreats_and_holds_the_authored_distance() {
        for (start, target, expected) in [
            (at(0, 0), at(8, 0), vec![CardinalStep::East; 4]),
            (at(1, 0), at(0, 0), vec![CardinalStep::East; 3]),
            (at(0, 0), at(4, 0), Vec::new()),
        ] {
            let route = route_at_distance(start, target, 4, open, |_| true).unwrap();
            assert_eq!(route, expected);
            assert_eq!(chebyshev(walk(start, &route), target), 4);
        }
        // With no authored separation from the target itself, all directions tie; north wins.
        assert_eq!(
            route_at_distance(at(0, 0), at(0, 0), 4, open, |_| true),
            Some(vec![CardinalStep::North; 4])
        );
    }

    #[test]
    fn creature_keep_distance_goes_around_a_wall_with_replay_identical_ties() {
        let start = at(0, 0);
        let target = at(8, 0);
        let wall = |cell: MovementLocalPosition| {
            (!(cell.x == 1 && (-1..=1).contains(&cell.y))).then_some(0)
        };
        let expected = vec![
            CardinalStep::North,
            CardinalStep::North,
            CardinalStep::East,
            CardinalStep::East,
            CardinalStep::East,
            CardinalStep::East,
        ];
        for _ in 0..8 {
            let route = route_at_distance(start, target, 4, wall, |_| true).unwrap();
            assert_eq!(route, expected);
            let mut position = start;
            for step in route {
                position = walk(position, &[step]);
                assert!(wall(position).is_some());
            }
            assert_eq!(chebyshev(position, target), 4);
        }
    }

    #[test]
    fn creature_keep_distance_requires_an_admitted_endpoint_but_can_cross_other_ring_cells() {
        let start = at(0, 0);
        let target = at(4, 0);
        // The current distance is correct but sight is not. The owner's sight stub admits
        // only (0, -2); (0, -1) remains traversable despite not being a valid stopping tile.
        assert_eq!(
            route_at_distance(start, target, 4, open, |cell| cell == at(0, -2)),
            Some(vec![CardinalStep::North; 2])
        );
        assert_eq!(route_at_distance(start, target, 4, open, |_| false), None);
        let closed = |cell| (cell != at(0, -2)).then_some(0);
        assert_eq!(
            route_at_distance(start, target, 4, closed, |cell| cell == at(0, -2)),
            None
        );
    }

    #[test]
    fn creature_keep_distance_uses_occupancy_cost_and_the_twelve_tile_search_bound() {
        let cost = |cell| Some(if cell == at(1, 0) { CREATURE_COST } else { 0 });
        let route = route_at_distance(at(0, 0), at(8, 0), 4, cost, |_| true).unwrap();
        assert_eq!(route.len(), 5, "one extra step costs less than occupancy");
        let mut position = at(0, 0);
        for step in route {
            position = walk(position, &[step]);
            assert_ne!(position, at(1, 0));
        }
        assert_eq!(chebyshev(position, at(8, 0)), 4);
        assert_eq!(
            route_at_distance(at(0, 0), at(16, 0), 4, open, |_| true),
            Some(vec![CardinalStep::East; 12])
        );
        let mut reads = 0;
        assert_eq!(
            route_at_distance(
                at(0, 0),
                at(17, 0),
                4,
                |cell| {
                    reads += 1;
                    assert!(chebyshev(at(0, 0), cell) <= 12);
                    Some(0)
                },
                |_| true
            ),
            None
        );
        assert!(reads <= PATH_SEARCH_WORK * 4);
    }

    #[test]
    fn creature_keep_distance_refuses_zero_and_other_floors_before_reading_facts() {
        for (target, distance) in [
            (at(4, 0), 0),
            (
                MovementLocalPosition {
                    floor: 8,
                    ..at(4, 0)
                },
                4,
            ),
        ] {
            let mut movement_reads = 0;
            let mut sight_reads = 0;
            assert_eq!(
                route_at_distance(
                    at(0, 0),
                    target,
                    distance,
                    |_| {
                        movement_reads += 1;
                        Some(0)
                    },
                    |_| {
                        sight_reads += 1;
                        true
                    }
                ),
                None
            );
            assert_eq!((movement_reads, sight_reads), (0, 0));
        }
    }

    #[test]
    fn creature_keep_distance_preserves_the_shared_route_and_work_limits() {
        let corridor = |cell: MovementLocalPosition| (cell.y == 0).then_some(0);
        let bounds = SearchBounds {
            work: PATH_SEARCH_WORK,
            route_steps: ROUTE_STEPS,
            distance: i32::MAX,
        };
        assert_eq!(
            search(
                at(0, 0),
                at(132, 0),
                SearchGoal::Distance(4),
                bounds,
                corridor
            )
            .unwrap()
            .len(),
            ROUTE_STEPS
        );
        assert_eq!(
            search(
                at(0, 0),
                at(133, 0),
                SearchGoal::Distance(4),
                bounds,
                corridor
            ),
            None
        );
        let work_bounds = SearchBounds {
            route_steps: usize::MAX,
            ..bounds
        };
        assert_eq!(
            search(
                at(0, 0),
                at(1_028, 0),
                SearchGoal::Distance(4),
                work_bounds,
                corridor
            )
            .unwrap()
            .len(),
            PATH_SEARCH_WORK
        );
        assert_eq!(
            search(
                at(0, 0),
                at(1_029, 0),
                SearchGoal::Distance(4),
                work_bounds,
                corridor
            ),
            None
        );
    }

    #[test]
    fn creature_walk_back_reaches_the_cell_instead_of_stopping_adjacent() {
        assert_eq!(route_to_cell(at(0, 0), at(0, 0), open), Some(vec![]));
        assert_eq!(
            route_to_cell(at(0, 0), at(1, 0), open),
            Some(vec![CardinalStep::East])
        );
        let route = route_to_cell(at(0, 0), at(3, 0), open).unwrap();
        assert_eq!(walk(at(0, 0), &route), at(3, 0));
        // Keep the accepted player chase's adjacent goal unchanged.
        assert_eq!(chase(at(0, 0), at(3, 0), open).unwrap().len(), 2);
    }

    #[test]
    fn creature_walk_back_uses_the_same_admission_and_deterministic_tie_order() {
        let wall = |cell| (cell != at(1, 0)).then_some(0);
        let expected = vec![
            CardinalStep::North,
            CardinalStep::East,
            CardinalStep::East,
            CardinalStep::South,
        ];
        for _ in 0..8 {
            assert_eq!(
                route_to_cell(at(0, 0), at(2, 0), wall),
                Some(expected.clone())
            );
        }
        let closed_spawn = |cell| (cell != at(2, 0)).then_some(0);
        assert_eq!(route_to_cell(at(0, 0), at(2, 0), closed_spawn), None);
        assert_eq!(
            route_to_cell(
                at(0, 0),
                MovementLocalPosition {
                    floor: 8,
                    ..at(2, 0)
                },
                open
            ),
            None
        );
    }

    #[test]
    fn creature_walk_back_has_the_fifty_tile_bound_and_shared_work_limit() {
        assert_eq!(route_to_cell(at(0, 0), at(50, 0), open).unwrap().len(), 50);
        assert_eq!(route_to_cell(at(0, 0), at(51, 0), open), None);
        let mut reads = 0;
        let sealed = |cell| {
            reads += 1;
            (chebyshev(cell, at(50, 0)) > 1).then_some(0)
        };
        assert_eq!(route_to_cell(at(0, 0), at(50, 0), sealed), None);
        assert!(reads <= PATH_SEARCH_WORK * 4);
    }

    #[test]
    fn creature_field_stub_cost_is_180_in_addition_to_occupied_cost() {
        // No field owner exists yet: the caller supplies only this test stub term.
        // Two corridors: straight (3 steps) or around the rectangle (9 steps). Occupancy
        // alone costs 30 + 40 < 90; adding the field makes the detour cheaper (250 > 90).
        let cost = |field| {
            move |cell: MovementLocalPosition| {
                let corridor = (0..=3).contains(&cell.x)
                    && (-3..=0).contains(&cell.y)
                    && (cell.y == 0 || cell.y == -3 || cell.x == 0 || cell.x == 3);
                corridor.then_some(if cell == at(1, 0) {
                    CREATURE_COST + field
                } else {
                    0
                })
            }
        };
        assert_eq!(
            route_to_cell(at(0, 0), at(3, 0), cost(0)),
            Some(vec![CardinalStep::East; 3])
        );
        let route = route_to_cell(at(0, 0), at(3, 0), cost(180)).unwrap();
        assert_eq!(route.len(), 9);
        let mut position = at(0, 0);
        for step in route {
            position = walk(position, &[step]);
            assert_ne!(position, at(1, 0));
        }
        assert_eq!(position, at(3, 0));
    }

    #[test]
    fn creature_exact_goal_keeps_both_the_route_and_expansion_limits() {
        let corridor = |cell: MovementLocalPosition| (cell.y == 0).then_some(0);
        let bounds = SearchBounds {
            work: PATH_SEARCH_WORK,
            route_steps: ROUTE_STEPS,
            distance: i32::MAX,
        };
        let route = search(at(0, 0), at(128, 0), SearchGoal::Cell, bounds, corridor);
        assert_eq!(route.unwrap().len(), 128);
        assert_eq!(
            search(at(0, 0), at(129, 0), SearchGoal::Cell, bounds, corridor),
            None
        );
        // Vary only the route bound to isolate the work cap.
        let work_bounds = SearchBounds {
            route_steps: usize::MAX,
            ..bounds
        };
        assert_eq!(
            search(
                at(0, 0),
                at(1_024, 0),
                SearchGoal::Cell,
                work_bounds,
                corridor
            )
            .unwrap()
            .len(),
            1_024
        );
        assert_eq!(
            search(
                at(0, 0),
                at(1_025, 0),
                SearchGoal::Cell,
                work_bounds,
                corridor
            ),
            None
        );
    }
}
