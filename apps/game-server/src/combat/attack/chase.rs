//! CHASE-1: the player chase runtime state and its search budget (RANGED-0 §8,
//! `docs/architecture/reviews/OTERYN_GAME_RANGED0_DISTANCE_WEAPONS_AMMUNITION_WANDS_AND_CHASE_DECISION_2026-10-01.md`).
//!
//! Server-driven (owner decision 2026-10-14): the route comes from the Movement owner's shared
//! path search (`movement::path_search`, CREATURE-MOVE-1). A player pays no harmful-field cost.
//! Which tiles are path tiles is the wiring's: floor-change, teleport and protection-zone tiles
//! never are. A client step always wins over a chase step.
//!
//! Runtime-only: nothing here is durable.

use std::collections::VecDeque;

use crate::foundation::MovementLocalPosition;
use crate::movement::CardinalStep;
use crate::movement::path_search::{chebyshev, delta, route_next_to};

pub(crate) use crate::movement::path_search::CREATURE_COST;

/// `RANGED0-RL-06`: at most one search per player per this interval.
pub(crate) const CHASE_SEARCH_INTERVAL_MICROS: u64 = 1_000_000;
/// `RANGED0-RL-07`: a failed search waits this long before the next one (Canary).
pub(crate) const CHASE_FAILED_SEARCH_WAIT_MICROS: u64 = 2_000_000;
/// `RANGED0-RL-03`: player chase searches per owner window per Channel.
pub(crate) const CHASE_SEARCHES_PER_WINDOW: u32 = 64;
/// The owner window `RANGED0-RL-03` counts in (`CREATUREAI0-RL-09`'s 50 ms window).
pub(crate) const CHASE_WINDOW_MICROS: u64 = 50_000;
/// How long a cell whose chase step the Movement owner refused stays out of the search.
pub(crate) const CHASE_REFUSED_CELL_MICROS: u64 = CHASE_FAILED_SEARCH_WAIT_MICROS;
/// At most this many refused cells are remembered; the oldest is forgotten first.
pub(crate) const CHASE_REFUSED_CELLS: usize = 8;

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
    /// Cells whose chase step the Movement owner refused, each with the owner time it expires.
    /// The owner alone judges passability from committed facts (a blocking item has no
    /// in-memory projection), so the search avoids what it just refused instead of re-reading
    /// them.
    refused: VecDeque<(MovementLocalPosition, u64)>,
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
        mut tile_cost: impl FnMut(MovementLocalPosition) -> Option<u32>,
    ) -> ChaseDecision {
        self.refused.retain(|&(_, expires_us)| now_us < expires_us);
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
            let refused = &self.refused;
            let route = route_next_to(at, target, |cell| {
                if refused.iter().any(|&(blocked, _)| blocked == cell) {
                    None
                } else {
                    tile_cost(cell)
                }
            });
            match route {
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

    /// The step just taken was refused at `now_us`: the route is dropped (RANGED-0 §8) and the
    /// refused cell stays out of the search for [`CHASE_REFUSED_CELL_MICROS`].
    pub(crate) fn step_refused(&mut self, now_us: u64) {
        if let Some(cell) = self.expected_at {
            self.refused.retain(|&(blocked, _)| blocked != cell);
            if self.refused.len() >= CHASE_REFUSED_CELLS {
                self.refused.pop_front();
            }
            self.refused
                .push_back((cell, now_us.saturating_add(CHASE_REFUSED_CELL_MICROS)));
        }
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
        chase.step_refused(2_000_000);
        // The player never reached the expected tile: the route is gone; the next search waits
        // for the interval.
        assert_eq!(
            chase.decide(2_100_000, at(0, 0), at(4, 0), || true, open),
            ChaseDecision::Hold
        );
        assert_ne!(
            chase.decide(3_000_000, at(0, 0), at(4, 0), || true, open),
            ChaseDecision::Step(CardinalStep::East),
            "the refused cell stays out of the search"
        );
    }

    #[test]
    fn a_blocking_item_the_owner_refuses_on_the_shortest_route_is_detoured() {
        // A committed blocking item on (1, 0): the search cannot see it, the Movement owner
        // refuses the step onto it.
        let item = at(1, 0);
        let target = at(4, 0);
        let mut chase = ChaseState::default();
        assert_eq!(
            chase.decide(0, at(0, 0), target, || true, open),
            ChaseDecision::Step(CardinalStep::East)
        );
        chase.step_refused(0);
        let mut position = at(0, 0);
        for now in CHASE_SEARCH_INTERVAL_MICROS..CHASE_SEARCH_INTERVAL_MICROS + 8 {
            match chase.decide(now, position, target, || true, open) {
                ChaseDecision::Step(step) => {
                    position = walk(position, &[step]);
                    assert_ne!(
                        position, item,
                        "the chase never steps onto the refused item"
                    );
                }
                ChaseDecision::Hold => break,
            }
        }
        assert_eq!(
            chebyshev(position, target),
            1,
            "the detour reaches the target"
        );
    }

    #[test]
    fn refused_cells_expire_after_two_seconds_and_at_most_eight_are_kept() {
        let mut chase = ChaseState::default();
        assert_eq!(
            chase.decide(0, at(0, 0), at(4, 0), || true, open),
            ChaseDecision::Step(CardinalStep::East)
        );
        chase.step_refused(0);
        assert_ne!(
            chase.decide(1_999_999, at(0, 0), at(4, 0), || true, open),
            ChaseDecision::Step(CardinalStep::East)
        );
        chase.reset();
        assert_eq!(chase.refused.len(), 0, "a reset forgets the refused cells");
        chase.step_refused(0);
        assert_eq!(chase.refused.len(), 0, "no step was expected");
        for x in 0..=CHASE_REFUSED_CELLS {
            chase.expected_at = Some(at(i32::try_from(x).unwrap(), 9));
            chase.step_refused(0);
            chase.step_refused(0);
        }
        assert_eq!(chase.refused.len(), CHASE_REFUSED_CELLS);
        assert_eq!(chase.refused.front().map(|(cell, _)| *cell), Some(at(1, 9)));
        assert_eq!(
            chase.decide(CHASE_REFUSED_CELL_MICROS, at(0, 0), at(4, 0), || true, open),
            ChaseDecision::Step(CardinalStep::East),
            "an expired cell is searched again"
        );
        assert_eq!(chase.refused.len(), 0);
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
