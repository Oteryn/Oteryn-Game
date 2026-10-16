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
use crate::movement::path_search::{CHASE_SEARCH_DISTANCE, chebyshev, delta, route_next_to};

pub(crate) use crate::movement::path_search::CREATURE_COST;

/// `RANGED0-RL-06` (RANGED-0 §8): at most one search per player per this interval.
pub(crate) const CHASE_SEARCH_INTERVAL_MICROS: u64 = 1_000_000;
/// `RANGED0-RL-07` (RANGED-0 §8): a failed search waits this long before the next one (Canary).
pub(crate) const CHASE_FAILED_SEARCH_WAIT_MICROS: u64 = 2_000_000;
/// `RANGED0-RL-03`: player chase searches per owner window per Channel, served oldest-waiting
/// request first, then actor id.
pub(crate) const CHASE_SEARCHES_PER_WINDOW: u32 = 64;
/// The owner window `RANGED0-RL-03` counts in (`CREATUREAI0-RL-09`'s 50 ms window).
pub(crate) const CHASE_WINDOW_MICROS: u64 = 50_000;
/// `RANGED0-RL-08`'s TTL: how long a cell the Movement owner refused a chase step into as
/// blocked stays out of the search.
pub(crate) const CHASE_REFUSED_CELL_MICROS: u64 = CHASE_FAILED_SEARCH_WAIT_MICROS;
/// `RANGED0-RL-08`: at most this many refused cells are remembered; the oldest is forgotten
/// first.
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
    /// `RANGED0-RL-03`: the owner time since which a due search has waited for the Channel's
    /// window budget.
    waiting_since: Option<u64>,
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
    /// Drop the route, the refused cells and a waiting search (target or mode change). The
    /// per-player search cooldown stays (`RANGED0-RL-06`, `-07`), so repeated target or mode changes never search
    /// more often than it allows.
    pub(crate) fn reset(&mut self) {
        *self = Self {
            next_search_us: self.next_search_us,
            ..Self::default()
        };
    }

    /// The held target is out of chase for now (another floor, out of sight, or the attacker
    /// re-entry protected): drop the route and a waiting search, keeping the target's refused
    /// cells (`RANGED0-RL-08` clears them only on a target or mode change) and the search
    /// cooldown. RANGED-0 §8: on a floor change the route resets and the target is kept.
    pub(crate) fn hold(&mut self) {
        self.route.clear();
        self.expected_at = None;
        self.goal = None;
        self.waiting_since = None;
    }

    /// A client step: drop the route; chase re-paths at its next 1,000 ms check (RANGED-0 §8,
    /// "client intent wins").
    pub(crate) fn yield_to_client(&mut self, now_us: u64) {
        self.route.clear();
        self.expected_at = None;
        self.goal = None;
        self.waiting_since = None;
        self.next_search_us = self
            .next_search_us
            .max(now_us.saturating_add(CHASE_SEARCH_INTERVAL_MICROS));
    }

    /// `RANGED0-RL-03`: since when a due search has waited for the Channel's window budget.
    pub(crate) const fn search_waiting_since(&self) -> Option<u64> {
        self.waiting_since
    }

    /// The chase check of `at` towards `target` at `now_us`, before the Channel's search drain:
    /// `None` holds (another floor or in reach); otherwise whether a search is due. A route is
    /// followed while the attacker holds the position it expects; it is searched again when none
    /// is held or its target moved, at most once per [`CHASE_SEARCH_INTERVAL_MICROS`]. A due
    /// search waits for the Channel's drain from the time it first became due. Until a search
    /// runs, a route whose target moved is still followed (CREATURE-AI-0 §5.3: the previous path
    /// while valid).
    pub(crate) fn prepare(
        &mut self,
        now_us: u64,
        at: MovementLocalPosition,
        target: MovementLocalPosition,
    ) -> Option<bool> {
        self.refused.retain(|&(_, expires_us)| now_us < expires_us);
        if at.floor != target.floor || chebyshev(at, target) <= 1 {
            self.hold();
            return None;
        }
        if self.expected_at != Some(at) {
            self.route.clear();
            self.expected_at = None;
            self.goal = None;
        }
        let stale = self.route.is_empty() || self.goal != Some(target);
        let due = stale && now_us >= self.next_search_us;
        if due {
            self.waiting_since.get_or_insert(now_us);
        }
        Some(due)
    }

    /// The Channel's drain runs this chase's search from `at` towards `target` at `now_us`
    /// (`RANGED0-RL-03`); `false` when none ran (another floor or in reach). The route it adopts
    /// starts at `at`.
    pub(crate) fn search(
        &mut self,
        now_us: u64,
        at: MovementLocalPosition,
        target: MovementLocalPosition,
        mut tile_cost: impl FnMut(MovementLocalPosition) -> Option<u32>,
    ) -> bool {
        if at.floor != target.floor || chebyshev(at, target) <= 1 {
            self.hold();
            return false;
        }
        self.waiting_since = None;
        self.refused.retain(|&(_, expires_us)| now_us < expires_us);
        let refused = &self.refused;
        let route = route_next_to(at, target, CHASE_SEARCH_DISTANCE, |cell| {
            if refused.iter().any(|&(blocked, _)| blocked == cell) {
                None
            } else {
                tile_cost(cell)
            }
        });
        match route {
            Some(route) if !route.is_empty() => {
                self.route = route.into();
                self.expected_at = Some(at);
                self.goal = Some(target);
                self.next_search_us = now_us.saturating_add(CHASE_SEARCH_INTERVAL_MICROS);
            }
            _ => {
                self.route.clear();
                self.expected_at = None;
                self.goal = None;
                self.next_search_us = now_us.saturating_add(CHASE_FAILED_SEARCH_WAIT_MICROS);
            }
        }
        true
    }

    /// The cell the route's next step enters, if a route is held.
    pub(crate) fn next_cell(&self) -> Option<MovementLocalPosition> {
        let (at, step) = (self.expected_at?, self.route.front()?);
        Some(stepped(at, *step))
    }

    /// The step the chase takes now from `at`: the route's next step, if any.
    pub(crate) fn next_step(&mut self, at: MovementLocalPosition) -> ChaseDecision {
        if self.expected_at != Some(at) {
            self.route.clear();
        }
        let Some(step) = self.route.pop_front() else {
            self.expected_at = None;
            return ChaseDecision::Hold;
        };
        self.expected_at = Some(stepped(at, step));
        ChaseDecision::Step(step)
    }

    /// One chase check with its own search: [`Self::prepare`], a due search when `admit` grants
    /// it, then [`Self::next_step`].
    #[cfg(test)]
    pub(crate) fn decide(
        &mut self,
        now_us: u64,
        at: MovementLocalPosition,
        target: MovementLocalPosition,
        admit: impl FnOnce() -> bool,
        tile_cost: impl FnMut(MovementLocalPosition) -> Option<u32>,
    ) -> ChaseDecision {
        let Some(due) = self.prepare(now_us, at, target) else {
            return ChaseDecision::Hold;
        };
        if due && admit() {
            self.search(now_us, at, target, tile_cost);
        }
        self.next_step(at)
    }

    /// The step just taken was refused for any reason but a blocked cell (a transient refusal):
    /// the route is dropped and nothing is remembered (`RANGED0-RL-08`); the cell stays a path
    /// tile.
    pub(crate) fn step_refused(&mut self) {
        self.route.clear();
        self.expected_at = None;
        self.goal = None;
    }

    /// The Movement owner refused the step just taken because its cell is blocked, at `now_us`:
    /// the route is dropped (RANGED-0 §8) and the cell stays out of the search for
    /// [`CHASE_REFUSED_CELL_MICROS`] (`RANGED0-RL-08`). No backoff starts: the next search, at
    /// most [`CHASE_SEARCH_INTERVAL_MICROS`] after the last, detours around the cell while it is
    /// remembered.
    pub(crate) fn step_blocked(&mut self, now_us: u64) {
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

fn stepped(at: MovementLocalPosition, step: CardinalStep) -> MovementLocalPosition {
    let (dx, dy) = delta(step);
    MovementLocalPosition {
        x: at.x.saturating_add(dx),
        y: at.y.saturating_add(dy),
        floor: at.floor,
    }
}

/// `RANGED0-RL-03`: the owner window whose player chase searches the Channel last drained.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct ChaseSearchBudget {
    drained: Option<u64>,
}

impl ChaseSearchBudget {
    /// Whether the owner window of `now_us` is still to be drained; if so it counts as drained.
    /// A caller's earlier window never reopens a drained one.
    pub(crate) fn open(&mut self, now_us: u64) -> bool {
        let window = now_us / CHASE_WINDOW_MICROS;
        if self.drained.is_some_and(|drained| window <= drained) {
            return false;
        }
        self.drained = Some(window);
        true
    }
}

/// `RANGED0-RL-03`: the order the drain of the owner window of `now_us` serves the waiting
/// searches in, each given as when it began waiting and its actor key: only those that waited
/// from an earlier window, so every search due in one window is known before any is served;
/// the oldest-waiting window first, then actor id. The drain serves at most
/// [`CHASE_SEARCHES_PER_WINDOW`] of them; the rest keep their places for the next window.
pub(crate) fn chase_search_order<K: Ord>(
    now_us: u64,
    waiting: impl IntoIterator<Item = (u64, K)>,
) -> Vec<K> {
    let window = now_us / CHASE_WINDOW_MICROS;
    let mut order: Vec<_> = waiting
        .into_iter()
        .map(|(since_us, key)| (since_us / CHASE_WINDOW_MICROS, key))
        .filter(|(since, _)| *since < window)
        .collect();
    order.sort();
    order.into_iter().map(|(_, key)| key).collect()
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
    fn a_second_search_waits_the_full_second() {
        // `RANGED0-RL-06`: the player stays put, so the route is dropped and needs a search.
        let mut chase = ChaseState::default();
        let searches = std::cell::Cell::new(0);
        let admit = || {
            searches.set(searches.get() + 1);
            true
        };
        assert_eq!(
            chase.decide(0, at(0, 0), at(4, 0), admit, open),
            ChaseDecision::Step(CardinalStep::East)
        );
        assert_eq!(
            chase.decide(999_999, at(0, 0), at(4, 0), admit, open),
            ChaseDecision::Hold
        );
        assert_eq!(searches.get(), 1, "no second search inside the second");
        assert_eq!(
            chase.decide(
                CHASE_SEARCH_INTERVAL_MICROS,
                at(0, 0),
                at(4, 0),
                admit,
                open
            ),
            ChaseDecision::Step(CardinalStep::East)
        );
        assert_eq!(searches.get(), 2);
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
        chase.step_blocked(2_000_000);
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
        chase.step_blocked(0);
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
        chase.step_blocked(0);
        assert_ne!(
            chase.decide(1_999_999, at(0, 0), at(4, 0), || true, open),
            ChaseDecision::Step(CardinalStep::East)
        );
        chase.hold();
        assert_eq!(chase.refused.len(), 1, "a hold keeps the refused cells");
        chase.reset();
        assert_eq!(chase.refused.len(), 0, "a reset forgets the refused cells");
        chase.step_blocked(0);
        assert_eq!(chase.refused.len(), 0, "no step was expected");
        for x in 0..=CHASE_REFUSED_CELLS {
            chase.expected_at = Some(at(i32::try_from(x).unwrap(), 9));
            chase.step_blocked(0);
            chase.step_blocked(0);
        }
        assert_eq!(chase.refused.len(), CHASE_REFUSED_CELLS);
        assert_eq!(chase.refused.front().map(|(cell, _)| *cell), Some(at(1, 9)));
        // The reset kept the cooldown of the search at 1,999,999 us.
        let next = 1_999_999 + CHASE_SEARCH_INTERVAL_MICROS;
        assert!(next >= CHASE_REFUSED_CELL_MICROS);
        assert_eq!(
            chase.decide(next, at(0, 0), at(4, 0), || true, open),
            ChaseDecision::Step(CardinalStep::East),
            "an expired cell is searched again"
        );
        assert_eq!(chase.refused.len(), 0);
    }

    #[test]
    fn the_refused_cell_cap_matches_its_registered_row() {
        let registry: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../docs/contracts/RESOURCE_LIMITS_REGISTRY.json"
        ))
        .unwrap();
        let entries = registry["entries"].as_array().unwrap();
        let row = entries
            .iter()
            .find(|entry| entry["id"] == "RANGED0-RL-08")
            .unwrap();
        assert_eq!(
            row["hard_maximum"].as_u64(),
            u64::try_from(CHASE_REFUSED_CELLS).ok()
        );
        assert_eq!(CHASE_REFUSED_CELL_MICROS, 2_000_000);
    }

    #[test]
    fn a_reset_keeps_the_per_player_search_cooldown() {
        let mut chase = ChaseState::default();
        assert_eq!(
            chase.decide(0, at(0, 0), at(4, 0), || true, open),
            ChaseDecision::Step(CardinalStep::East)
        );
        chase.reset();
        assert_eq!(chase.route.len(), 0);
        assert_eq!(
            chase.decide(250_000, at(1, 0), at(6, 0), || true, open),
            ChaseDecision::Hold,
            "a target change searches no sooner than 1,000 ms after the last search"
        );
        assert_eq!(
            chase.decide(
                CHASE_SEARCH_INTERVAL_MICROS,
                at(1, 0),
                at(6, 0),
                || true,
                open
            ),
            ChaseDecision::Step(CardinalStep::East)
        );
    }

    #[test]
    fn a_transient_refusal_drops_the_route_but_keeps_the_cell_a_path_tile() {
        let mut chase = ChaseState::default();
        assert_eq!(
            chase.decide(0, at(0, 0), at(4, 0), || true, open),
            ChaseDecision::Step(CardinalStep::East)
        );
        chase.step_refused();
        assert_eq!(chase.refused.len(), 0, "only a blocked cell is remembered");
        assert_eq!(chase.route.len(), 0);
        assert_eq!(
            chase.decide(1, at(0, 0), at(4, 0), || true, open),
            ChaseDecision::Hold,
            "the next search waits for the interval"
        );
        assert_eq!(
            chase.decide(
                CHASE_SEARCH_INTERVAL_MICROS,
                at(0, 0),
                at(4, 0),
                || true,
                open
            ),
            ChaseDecision::Step(CardinalStep::East),
            "the same cell is taken again"
        );
    }

    #[test]
    fn a_chase_searches_within_twelve_tiles_and_a_target_beyond_fails_the_search() {
        // The goal (12, 0) of a target on (13, 0) lies on the 12-tile search distance.
        let mut chase = ChaseState::default();
        assert_eq!(
            chase.decide(0, at(0, 0), at(13, 0), || true, open),
            ChaseDecision::Step(CardinalStep::East)
        );
        assert_eq!(chase.route.len(), 11, "12 steps, the first one taken");
        // A target on (14, 0) has no goal within it: a failed search waits 2,000 ms.
        let mut chase = ChaseState::default();
        assert_eq!(
            chase.decide(0, at(0, 0), at(14, 0), || true, open),
            ChaseDecision::Hold
        );
        assert_eq!(chase.next_search_us, CHASE_FAILED_SEARCH_WAIT_MICROS);
        // Around a wall the search may leave the straight line, but never the 12 tiles: a wall
        // across x = 1 from y = -12 to y = 12 leaves no route to (3, 0).
        let wall = |cell: MovementLocalPosition| {
            (!(cell.x == 1 && (-12..=12).contains(&cell.y))).then_some(0)
        };
        let mut chase = ChaseState::default();
        assert_eq!(
            chase.decide(0, at(0, 0), at(3, 0), || true, wall),
            ChaseDecision::Hold
        );
        assert_eq!(chase.next_search_us, CHASE_FAILED_SEARCH_WAIT_MICROS);
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
            chase.decide(10_000, at(0, 0), at(4, 0), || false, open),
            ChaseDecision::Hold
        );
        assert_eq!(
            chase.search_waiting_since(),
            Some(0),
            "a refused search keeps waiting from when it first became due"
        );
        assert_eq!(
            chase.decide(50_000, at(0, 0), at(4, 0), || true, open),
            ChaseDecision::Step(CardinalStep::East)
        );
        assert_eq!(chase.search_waiting_since(), None);
    }

    #[test]
    fn the_channel_drains_each_owner_window_once() {
        let mut budget = ChaseSearchBudget::default();
        assert!(budget.open(10_000));
        assert!(!budget.open(49_999));
        assert!(budget.open(50_000), "a new window is drained once more");
        assert!(
            !budget.open(49_999),
            "a late caller's earlier window never reopens a drained one"
        );
    }

    #[test]
    fn the_drain_serves_the_oldest_waiting_window_first_then_actor_id() {
        let order = chase_search_order(
            100_000,
            [(60_000, 1), (10_000, 3), (40_000, 2), (100_000, 0)],
        );
        assert_eq!(
            order,
            [2, 3, 1],
            "a search registered in the drained window waits for the next"
        );
    }
}
