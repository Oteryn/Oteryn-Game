//! CHASE-1: the player chase path search and its runtime state (RANGED-0 §8,
//! `docs/architecture/reviews/OTERYN_GAME_RANGED0_DISTANCE_WEAPONS_AMMUNITION_WANDS_AND_CHASE_DECISION_2026-10-01.md`).
//!
//! The path profile is CREATURE-AI-0 §5.3's: cardinal neighbours in the order north, east, south,
//! west; 10 per step and +40 over a creature; ties by lowest total, then lowest estimate, then
//! neighbour order, then discovery order; at most `AI01-PATH-SEARCH-WORK` expansions and
//! `AI01-ROUTE-STEPS` steps, within a 12-tile chase distance of the start (Canary searches fully,
//! `PARITY_PENDING`). The goal is the nearest tile at Chebyshev distance 1 from the target. A
//! player pays no harmful-field cost. Which tiles are path tiles is the wiring's: floor-change,
//! teleport and protection-zone tiles never are.
//!
//! Runtime-only: nothing here is durable.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap, VecDeque};

use crate::foundation::MovementLocalPosition;
use crate::movement::CardinalStep;

/// `AI01-PATH-SEARCH-WORK`: expansions per search.
pub(crate) const CHASE_SEARCH_WORK: usize = 1_024;
/// `AI01-ROUTE-STEPS`: steps of an adopted route.
pub(crate) const CHASE_ROUTE_STEPS: usize = 128;
/// The chase search distance in tiles from the start (Chebyshev).
pub(crate) const CHASE_SEARCH_DISTANCE: i32 = 12;
/// At most one search per player per this interval.
pub(crate) const CHASE_SEARCH_INTERVAL_MICROS: u64 = 1_000_000;
/// A failed search waits this long before the next one (Canary).
pub(crate) const CHASE_FAILED_SEARCH_WAIT_MICROS: u64 = 2_000_000;
/// `RANGED0-RL-03`: player chase searches per owner window per Channel.
pub(crate) const CHASE_SEARCHES_PER_WINDOW: u32 = 64;
/// The owner window `RANGED0-RL-03` counts in (`CREATUREAI0-RL-09`'s 50 ms window).
pub(crate) const CHASE_WINDOW_MICROS: u64 = 50_000;

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

const fn delta(step: CardinalStep) -> (i32, i32) {
    match step {
        CardinalStep::North => (0, -1),
        CardinalStep::East => (1, 0),
        CardinalStep::South => (0, 1),
        CardinalStep::West => (-1, 0),
    }
}

fn chebyshev(from: MovementLocalPosition, to: MovementLocalPosition) -> i64 {
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
pub(crate) fn chase_route(
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
    )));
    let mut expansions = 0_usize;
    while let Some(Reverse((_, _, _, _, x, y, cost))) = open.pop() {
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
        if expansions > CHASE_SEARCH_WORK {
            return None;
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
            if chebyshev(start, next) > i64::from(CHASE_SEARCH_DISTANCE) {
                continue;
            }
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
        if steps.len() > CHASE_ROUTE_STEPS {
            return None;
        }
        at = *from;
    }
    steps.reverse();
    Some(steps)
}

/// The chase of one attacker: the adopted route and when the next search may run.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ChaseState {
    /// The adopted route and the position the attacker must hold to take its next step.
    route: VecDeque<CardinalStep>,
    expected_at: Option<MovementLocalPosition>,
    /// The target position the route was searched for.
    goal: Option<MovementLocalPosition>,
    /// No search before this owner time.
    next_search_us: u64,
}

/// What a chase check decided.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ChaseDecision {
    /// Take this step through the Movement owner.
    Step(CardinalStep),
    /// Nothing to do now: in reach, no route yet, or a search is not allowed yet.
    Hold,
}

impl ChaseState {
    /// Drop the route; the next search may run at once (target or mode change).
    pub(crate) fn reset(&mut self) {
        *self = Self::default();
    }

    /// A client step: drop the route; chase re-paths at its next 1,000 ms check (RANGED-0 §8,
    /// "client intent wins").
    pub(crate) fn yield_to_client(&mut self, now_us: u64) {
        self.route.clear();
        self.expected_at = None;
        self.goal = None;
        self.next_search_us = self
            .next_search_us
            .max(now_us.saturating_add(CHASE_SEARCH_INTERVAL_MICROS));
    }

    /// The step the chase takes from `at` towards `target` at `now_us`. A route is followed while
    /// the attacker holds the position it expects; it is searched again when none is held or its
    /// target moved, at most once per [`CHASE_SEARCH_INTERVAL_MICROS`] and only when `admit`
    /// grants a search from the Channel's window budget. Until a search runs, a route whose
    /// target moved is still followed (CREATURE-AI-0 §5.3: the previous path while valid).
    pub(crate) fn decide(
        &mut self,
        now_us: u64,
        at: MovementLocalPosition,
        target: MovementLocalPosition,
        admit: impl FnOnce() -> bool,
        tile_cost: impl FnMut(MovementLocalPosition) -> Option<u32>,
    ) -> ChaseDecision {
        if at.floor != target.floor || chebyshev(at, target) <= 1 {
            self.route.clear();
            self.expected_at = None;
            self.goal = None;
            return ChaseDecision::Hold;
        }
        if self.expected_at != Some(at) {
            self.route.clear();
            self.expected_at = None;
            self.goal = None;
        }
        let stale = self.route.is_empty() || self.goal != Some(target);
        if stale && now_us >= self.next_search_us && admit() {
            match chase_route(at, target, tile_cost) {
                Some(route) if !route.is_empty() => {
                    self.route = route.into();
                    self.goal = Some(target);
                    self.next_search_us = now_us.saturating_add(CHASE_SEARCH_INTERVAL_MICROS);
                }
                _ => {
                    self.route.clear();
                    self.goal = None;
                    self.next_search_us = now_us.saturating_add(CHASE_FAILED_SEARCH_WAIT_MICROS);
                }
            }
        }
        let Some(step) = self.route.pop_front() else {
            self.expected_at = None;
            return ChaseDecision::Hold;
        };
        let (dx, dy) = delta(step);
        self.expected_at = Some(MovementLocalPosition {
            x: at.x.saturating_add(dx),
            y: at.y.saturating_add(dy),
            floor: at.floor,
        });
        ChaseDecision::Step(step)
    }

    /// The step just taken was refused: the route is dropped (RANGED-0 §8).
    pub(crate) fn step_refused(&mut self) {
        self.route.clear();
        self.expected_at = None;
        self.goal = None;
    }
}

/// `RANGED0-RL-03`: the Channel's player chase searches in the current owner window.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct ChaseSearchBudget {
    window: u64,
    used: u32,
}

impl ChaseSearchBudget {
    /// Whether one more search fits the owner window of `now_us`; if so it is counted.
    pub(crate) fn admit(&mut self, now_us: u64) -> bool {
        let window = now_us / CHASE_WINDOW_MICROS;
        if window != self.window {
            self.window = window;
            self.used = 0;
        }
        if self.used >= CHASE_SEARCHES_PER_WINDOW {
            return false;
        }
        self.used += 1;
        true
    }
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
        let route = chase_route(at(0, 0), at(5, 0), open).unwrap();
        assert_eq!(route, vec![CardinalStep::East; 4]);
        let diagonal = chase_route(at(0, 0), at(3, 3), open).unwrap();
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
        let first = chase_route(at(0, 0), at(3, 3), open).unwrap();
        for _ in 0..8 {
            assert_eq!(chase_route(at(0, 0), at(3, 3), open).unwrap(), first);
        }
        assert_eq!(first[0], CardinalStep::East, "east precedes south");
        let north = chase_route(at(0, 0), at(-3, -3), open).unwrap();
        assert_eq!(north[0], CardinalStep::North, "north precedes west");
    }

    #[test]
    fn an_adjacent_or_diagonal_target_needs_no_step_and_another_floor_is_no_route() {
        assert_eq!(chase_route(at(0, 0), at(1, 1), open), Some(Vec::new()));
        assert_eq!(chase_route(at(0, 0), at(0, 1), open), Some(Vec::new()));
        let below = MovementLocalPosition {
            x: 3,
            y: 0,
            floor: 8,
        };
        assert_eq!(chase_route(at(0, 0), below, open), None);
    }

    #[test]
    fn a_wall_is_walked_around_and_a_closed_room_is_a_failed_search() {
        // A wall on x = 2 from y = -3 to y = 3.
        let wall = |cell: MovementLocalPosition| {
            (!(cell.x == 2 && (-3..=3).contains(&cell.y))).then_some(0)
        };
        let route = chase_route(at(0, 0), at(4, 0), wall).unwrap();
        let end = walk(at(0, 0), &route);
        assert_eq!(chebyshev(end, at(4, 0)), 1);
        let mut position = at(0, 0);
        for step in &route {
            position = walk(position, &[*step]);
            assert!(wall(position).is_some(), "the route never enters the wall");
        }
        // The target walled in on every side.
        let boxed = |cell: MovementLocalPosition| (chebyshev(cell, at(6, 0)) > 2).then_some(0);
        assert_eq!(chase_route(at(0, 0), at(6, 0), boxed), None);
    }

    #[test]
    fn a_creature_costs_forty_so_a_short_detour_goes_around_it() {
        let creature = at(1, 0);
        let cost =
            |cell: MovementLocalPosition| Some(if cell == creature { CREATURE_COST } else { 0 });
        let route = chase_route(at(0, 0), at(4, 0), cost).unwrap();
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
        // Thirteen tiles away the goal tiles lie outside the search distance.
        assert_eq!(chase_route(at(0, 0), at(14, 0), open), None);
        assert_eq!(
            chase_route(at(0, 0), at(13, 0), open).map(|r| r.len()),
            Some(12)
        );
        // A maze-free but unreachable goal expands at most the work bound and fails.
        let mut calls = 0_usize;
        let unreachable = |cell: MovementLocalPosition| {
            calls += 1;
            (chebyshev(cell, at(12, 0)) > 1).then_some(0)
        };
        assert_eq!(chase_route(at(0, 0), at(12, 0), unreachable), None);
        assert!(calls <= 4 * CHASE_SEARCH_WORK, "{calls} tile reads");
    }

    #[test]
    fn a_route_is_followed_and_searched_at_most_once_per_second() {
        let mut chase = ChaseState::default();
        let searches = std::cell::Cell::new(0);
        let admit = || {
            searches.set(searches.get() + 1);
            true
        };
        let decision = chase.decide(0, at(0, 0), at(4, 0), admit, open);
        assert_eq!(decision, ChaseDecision::Step(CardinalStep::East));
        // The route is followed from the expected position without a new search, even when the
        // target moved meanwhile, until a search is allowed again.
        let decision = chase.decide(300_000, at(1, 0), at(4, 1), admit, open);
        assert_eq!(decision, ChaseDecision::Step(CardinalStep::East));
        assert_eq!(searches.get(), 1);
        // At 1,000 ms the moved target is searched again.
        let decision = chase.decide(1_000_000, at(2, 0), at(5, 1), admit, open);
        assert_eq!(decision, ChaseDecision::Step(CardinalStep::East));
        assert_eq!(searches.get(), 2);
        // In reach: nothing to do and the route is dropped.
        assert_eq!(
            chase.decide(1_100_000, at(4, 1), at(5, 1), admit, open),
            ChaseDecision::Hold
        );
        assert_eq!(chase.route.len(), 0);
    }

    #[test]
    fn a_failed_search_waits_two_seconds_and_a_refused_step_drops_the_route() {
        let mut chase = ChaseState::default();
        let none = |_: MovementLocalPosition| None;
        assert_eq!(
            chase.decide(0, at(0, 0), at(4, 0), || true, none),
            ChaseDecision::Hold
        );
        assert_eq!(
            chase.decide(1_999_999, at(0, 0), at(4, 0), || true, open),
            ChaseDecision::Hold,
            "a failed search waits 2,000 ms"
        );
        assert_eq!(
            chase.decide(2_000_000, at(0, 0), at(4, 0), || true, open),
            ChaseDecision::Step(CardinalStep::East)
        );
        chase.step_refused();
        // The player never reached the expected tile: the route is gone; the next search waits
        // for the interval.
        assert_eq!(
            chase.decide(2_100_000, at(0, 0), at(4, 0), || true, open),
            ChaseDecision::Hold
        );
        assert_eq!(
            chase.decide(3_000_000, at(0, 0), at(4, 0), || true, open),
            ChaseDecision::Step(CardinalStep::East)
        );
    }

    #[test]
    fn a_client_step_drops_the_route_and_chase_repaths_a_second_later() {
        let mut chase = ChaseState::default();
        assert_eq!(
            chase.decide(0, at(0, 0), at(4, 0), || true, open),
            ChaseDecision::Step(CardinalStep::East)
        );
        chase.yield_to_client(200_000);
        // The client stepped south instead.
        assert_eq!(
            chase.decide(500_000, at(0, 1), at(4, 0), || true, open),
            ChaseDecision::Hold
        );
        assert_eq!(
            chase.decide(1_200_000, at(0, 1), at(4, 0), || true, open),
            ChaseDecision::Step(CardinalStep::East)
        );
    }

    #[test]
    fn a_search_refused_by_the_channel_budget_is_retried_without_a_wait() {
        let mut chase = ChaseState::default();
        assert_eq!(
            chase.decide(0, at(0, 0), at(4, 0), || false, open),
            ChaseDecision::Hold
        );
        assert_eq!(
            chase.decide(50_000, at(0, 0), at(4, 0), || true, open),
            ChaseDecision::Step(CardinalStep::East)
        );
    }

    #[test]
    fn the_channel_admits_sixty_four_searches_per_owner_window() {
        let mut budget = ChaseSearchBudget::default();
        for _ in 0..CHASE_SEARCHES_PER_WINDOW {
            assert!(budget.admit(10_000));
        }
        assert!(!budget.admit(49_999));
        assert!(budget.admit(50_000), "a new window starts its own count");
    }
}
