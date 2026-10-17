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
use crate::movement::path_search::{
    CHASE_SEARCH_DISTANCE, chebyshev, delta, route_next_to_counted,
};

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
/// CREATURE-AI0 §5.3 / D1219: one shared Channel window, including player reserve.
pub(crate) const WINDOW_UNITS: u32 = 65_536;
pub(crate) const PLAYER_WAIT_RESERVE: u32 = 16_384;
pub(crate) const UNITS_PER_SEARCH: u32 = 1_024;
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
    /// The window and starting cell of the adopted search, while its first step awaits Movement.
    search_origin: Option<(u64, MovementLocalPosition)>,
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
    /// The first step a search planned and `next_step` returned: window, starting cell and
    /// destination. Survives [`Self::reset`] (target cleanup) because Movement may still commit
    /// it; it only counts while the actor stands on the starting cell in that window.
    in_flight: Option<(u64, MovementLocalPosition, MovementLocalPosition)>,
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
            in_flight: self.in_flight,
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
        self.search_origin = None;
        self.goal = None;
        self.waiting_since = None;
    }

    /// A client step: drop the route; chase re-paths at its next 1,000 ms check (RANGED-0 §8,
    /// "client intent wins").
    pub(crate) fn yield_to_client(&mut self, now_us: u64) {
        self.route.clear();
        self.expected_at = None;
        self.search_origin = None;
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
            self.search_origin = None;
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
    /// (`RANGED0-RL-03`); `None` when none ran (another floor or in reach), else the search's
    /// actual expansion count. The route it adopts starts at `at`.
    pub(crate) fn search(
        &mut self,
        now_us: u64,
        at: MovementLocalPosition,
        target: MovementLocalPosition,
        mut tile_cost: impl FnMut(MovementLocalPosition) -> Option<u32>,
    ) -> Option<usize> {
        if at.floor != target.floor || chebyshev(at, target) <= 1 {
            self.hold();
            return None;
        }
        self.waiting_since = None;
        self.refused.retain(|&(_, expires_us)| now_us < expires_us);
        let refused = &self.refused;
        let (route, work) = route_next_to_counted(at, target, CHASE_SEARCH_DISTANCE, |cell| {
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
                self.search_origin = Some((now_us / CHASE_WINDOW_MICROS, at));
                self.goal = Some(target);
                self.next_search_us = now_us.saturating_add(CHASE_SEARCH_INTERVAL_MICROS);
            }
            _ => {
                self.route.clear();
                self.expected_at = None;
                self.search_origin = None;
                self.goal = None;
                self.next_search_us = now_us.saturating_add(CHASE_FAILED_SEARCH_WAIT_MICROS);
            }
        }
        Some(work)
    }

    /// The cell the route's next step enters, if a route is held.
    pub(crate) fn next_cell(&self) -> Option<MovementLocalPosition> {
        let (at, step) = (self.expected_at?, self.route.front()?);
        Some(stepped(at, *step))
    }

    /// The first destination selected by this window's earlier drain, before Movement commits.
    /// After the first step is consumed, `expected_at` is that in-flight destination, never the
    /// route's following step. A changed current position uses standing occupancy instead.
    pub(crate) fn planned_search_cell(
        &self,
        now_us: u64,
        at: MovementLocalPosition,
    ) -> Option<MovementLocalPosition> {
        if self.search_origin != Some((now_us / CHASE_WINDOW_MICROS, at)) {
            return self
                .in_flight
                .filter(|&(window, origin, _)| {
                    window == now_us / CHASE_WINDOW_MICROS && origin == at
                })
                .map(|(_, _, destination)| destination);
        }
        let expected = self.expected_at?;
        if expected == at {
            self.next_cell()
        } else {
            Some(expected)
        }
    }

    /// Whether a step already returned by [`Self::next_step`] this window from `at` may still
    /// commit, whatever the target does meanwhile.
    pub(crate) fn has_in_flight_step(&self, now_us: u64, at: MovementLocalPosition) -> bool {
        self.in_flight.is_some_and(|(window, origin, _)| {
            window == now_us / CHASE_WINDOW_MICROS && origin == at
        })
    }

    /// The step the chase takes now from `at`: the route's next step, if any.
    pub(crate) fn next_step(&mut self, at: MovementLocalPosition) -> ChaseDecision {
        if self.expected_at != Some(at) {
            self.route.clear();
            self.search_origin = None;
        }
        let Some(step) = self.route.pop_front() else {
            self.expected_at = None;
            self.search_origin = None;
            return ChaseDecision::Hold;
        };
        if self.search_origin.is_some_and(|(_, origin)| origin != at) {
            self.search_origin = None;
        }
        self.in_flight = self
            .search_origin
            .map(|(window, origin)| (window, origin, stepped(at, step)));
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
        self.in_flight = None;
        self.route.clear();
        self.expected_at = None;
        self.search_origin = None;
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
        self.search_origin = None;
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
    window: Option<u64>,
    player_units_used: u32,
    creature_units_used: u32,
    /// Units the creature queue declared for its already-pending searches (CREATURE-MOVE-1
    /// packet: pending creature searches go before player chase). Not reset by a new window, so
    /// a player drain that runs before the creature drain in that window cannot take them.
    creature_pending_units: u32,
    /// Player chase searches the drain ran in the current window ([`CHASE_SEARCHES_PER_WINDOW`]
    /// counts across a resumed drain).
    player_searches: u32,
    /// The current window's player drain stopped on the shared work budget, not on its own
    /// queue or search count; it resumes once a full search fits again (creature refunds).
    drain_suspended: bool,
}

impl ChaseSearchBudget {
    fn advance(&mut self, now_us: u64) -> bool {
        let window = now_us / CHASE_WINDOW_MICROS;
        match self.window {
            Some(current) if window < current => false,
            Some(current) if window == current => true,
            _ => {
                self.window = Some(window);
                self.player_units_used = 0;
                self.creature_units_used = 0;
                self.drained = None;
                self.player_searches = 0;
                self.drain_suspended = false;
                true
            }
        }
    }

    /// Units of the current window held for declared pending creature work. A charging player
    /// is waiting, so the hold never cuts into the player reserve: creatures stay within
    /// `WINDOW_UNITS - PLAYER_WAIT_RESERVE` while a player waits.
    fn creature_hold(&self) -> u32 {
        self.creature_pending_units
            .min((WINDOW_UNITS - PLAYER_WAIT_RESERVE).saturating_sub(self.creature_units_used))
    }

    /// Declare the total units the creature queue's pending searches need (for example
    /// pending searches times [`UNITS_PER_SEARCH`]); it replaces the earlier declaration.
    /// [`Self::charge_creature`] consumes it; the owner re-declares when its queue changes.
    #[allow(dead_code, reason = "D1219 creature consumer is composed by #2200")]
    pub(crate) fn declare_creature_pending(&mut self, units: u32) {
        self.creature_pending_units = units;
    }

    /// The most player work the current window still admits.
    fn player_limit(&self) -> u32 {
        WINDOW_UNITS - self.creature_units_used - self.creature_hold()
    }

    pub(crate) fn charge_player(&mut self, now_us: u64, units: u32) -> bool {
        if !self.advance(now_us) {
            return false;
        }
        let limit = self.player_limit();
        let Some(after) = self
            .player_units_used
            .checked_add(units)
            .filter(|used| *used <= limit)
        else {
            return false;
        };
        self.player_units_used = after;
        true
    }

    /// Return `units` of an earlier [`Self::charge_player`] in the same window: admission
    /// reserved the per-search cap, and only the work the search actually did stays debited.
    pub(crate) fn refund_player(&mut self, now_us: u64, units: u32) {
        if self.window == Some(now_us / CHASE_WINDOW_MICROS) {
            self.player_units_used = self.player_units_used.saturating_sub(units);
        }
    }

    /// Creature counterpart of [`Self::refund_player`]: return the unused part of an earlier
    /// [`Self::charge_creature`] in the same window once the search's real work is known.
    #[allow(dead_code, reason = "D1219 creature consumer is composed by #2200")]
    pub(crate) fn refund_creature(&mut self, now_us: u64, units: u32) {
        if self.window == Some(now_us / CHASE_WINDOW_MICROS) {
            self.creature_units_used = self.creature_units_used.saturating_sub(units);
        }
    }

    #[allow(dead_code, reason = "D1219 creature consumer is composed by #2200")]
    pub(crate) fn charge_creature(
        &mut self,
        now_us: u64,
        units: u32,
        player_waiting: bool,
    ) -> bool {
        if !self.advance(now_us) {
            return false;
        }
        let reserve = if player_waiting {
            PLAYER_WAIT_RESERVE.saturating_sub(self.player_units_used)
        } else {
            0
        };
        let Some(after) = self
            .creature_units_used
            .checked_add(units)
            .filter(|used| *used <= WINDOW_UNITS - reserve - self.player_units_used)
        else {
            return false;
        };
        self.creature_units_used = after;
        self.creature_pending_units = self.creature_pending_units.saturating_sub(units);
        true
    }

    #[allow(
        dead_code,
        reason = "D1219 current shared-window reader is consumed by #2200"
    )]
    pub(crate) fn remaining(&mut self, now_us: u64) -> u32 {
        if self.advance(now_us) {
            WINDOW_UNITS - self.player_units_used - self.creature_units_used
        } else {
            0
        }
    }

    /// Whether the owner window of `now_us` is still to be drained; if so it counts as drained.
    /// A caller's earlier window never reopens a drained one. A drain [`Self::suspend_drain`]
    /// stopped on the shared budget reopens in its window once a full search fits again, so
    /// work creature refunds return stays usable by waiting players.
    pub(crate) fn open(&mut self, now_us: u64) -> bool {
        if !self.advance(now_us) {
            return false;
        }
        let window = now_us / CHASE_WINDOW_MICROS;
        if self.drained.is_some_and(|drained| window <= drained) {
            let resumable = self.drain_suspended
                && self.drained == Some(window)
                && self.player_searches < CHASE_SEARCHES_PER_WINDOW
                && self.player_limit().saturating_sub(self.player_units_used) >= UNITS_PER_SEARCH;
            if resumable {
                self.drain_suspended = false;
            }
            return resumable;
        }
        self.drained = Some(window);
        true
    }

    /// The drain of the window of `now_us` stopped because [`Self::charge_player`] refused a
    /// search: it is not complete, and [`Self::open`] resumes it once capacity returns.
    pub(crate) fn suspend_drain(&mut self, now_us: u64) {
        if self.window == Some(now_us / CHASE_WINDOW_MICROS) {
            self.drain_suspended = true;
        }
    }

    /// Player chase searches run in the current window, across a resumed drain.
    pub(crate) const fn player_searches(&self) -> u32 {
        self.player_searches
    }

    /// Count one player chase search run in the window of `now_us`.
    pub(crate) fn note_player_search(&mut self, now_us: u64) {
        if self.window == Some(now_us / CHASE_WINDOW_MICROS) {
            self.player_searches = self.player_searches.saturating_add(1);
        }
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
    fn a_returned_step_stays_in_flight_for_its_window_and_cell() {
        let mut chase = ChaseState::default();
        let decision = chase.decide(0, at(0, 0), at(4, 0), || true, open);
        assert_eq!(decision, ChaseDecision::Step(CardinalStep::East));
        assert!(chase.has_in_flight_step(0, at(0, 0)));
        assert!(!chase.has_in_flight_step(0, at(1, 0)));
        assert!(!chase.has_in_flight_step(CHASE_WINDOW_MICROS, at(0, 0)));
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
    fn many_trivial_searches_do_not_exhaust_the_window_early() {
        let mut budget = ChaseSearchBudget::default();
        let mut chase = ChaseState::default();
        // Twice the fixed-charge capacity (64): each admission needs the 1,024 cap, but only
        // the actual (tiny) expansion count stays debited.
        for _ in 0..128 {
            assert!(budget.charge_player(0, UNITS_PER_SEARCH));
            let work = chase.search(0, at(0, 0), at(3, 0), open).unwrap();
            assert!(work < 8, "{work}");
            budget.refund_player(0, UNITS_PER_SEARCH - u32::try_from(work).unwrap());
            chase.next_search_us = 0;
        }
        assert!(budget.remaining(0) > WINDOW_UNITS - 128 * 8);
        // Admission still needs the worst case: a nearly full window refuses another search.
        assert!(budget.charge_player(0, WINDOW_UNITS - budget.player_units_used));
        assert!(!budget.charge_player(0, 1));
    }

    #[test]
    fn refunded_creature_reservations_keep_only_actual_work_debited() {
        let mut budget = ChaseSearchBudget::default();
        for _ in 0..128 {
            assert!(budget.charge_creature(0, UNITS_PER_SEARCH, true));
            budget.refund_creature(0, UNITS_PER_SEARCH - 4);
        }
        assert_eq!(budget.creature_units_used, 128 * 4);
        budget.refund_creature(50_000, 4);
        assert_eq!(budget.creature_units_used, 128 * 4, "other window ignored");
    }

    #[test]
    fn creature_work_leaves_the_exact_player_reserve_and_does_not_drain_players() {
        let mut budget = ChaseSearchBudget::default();
        for _ in 0..48 {
            assert!(budget.charge_creature(0, UNITS_PER_SEARCH, true));
        }
        assert_eq!(budget.remaining(0), PLAYER_WAIT_RESERVE);
        assert!(!budget.charge_creature(0, 1, true));
        assert!(
            budget.open(0),
            "creature charges never consume player drain"
        );
        for _ in 0..16 {
            assert!(budget.charge_player(0, UNITS_PER_SEARCH));
        }
        assert_eq!(budget.remaining(0), 0);
        assert!(!budget.charge_player(0, 1));
        assert!(!budget.open(0));
    }

    #[test]
    fn pending_creature_work_stays_admissible_when_players_charge_first() {
        let mut budget = ChaseSearchBudget::default();
        assert!(budget.charge_creature(0, UNITS_PER_SEARCH, true));
        budget.declare_creature_pending(4 * UNITS_PER_SEARCH);
        // A new window: the player drain wins the lock before the creature drain.
        let mut players = 0;
        while budget.charge_player(50_000, UNITS_PER_SEARCH) {
            players += 1;
        }
        assert_eq!(players, 60);
        for _ in 0..4 {
            assert!(budget.charge_creature(50_000, UNITS_PER_SEARCH, true));
        }
        assert_eq!(budget.creature_pending_units, 0);
        assert_eq!(budget.remaining(50_000), 0);
        // A declaration over the creature share never cuts into the player reserve.
        budget.declare_creature_pending(WINDOW_UNITS);
        let mut players = 0;
        while budget.charge_player(100_000, UNITS_PER_SEARCH) {
            players += 1;
        }
        assert_eq!(players, 16);
        for _ in 0..48 {
            assert!(budget.charge_creature(100_000, UNITS_PER_SEARCH, true));
        }
        assert!(!budget.charge_creature(100_000, 1, true));
    }

    #[test]
    fn a_budget_stopped_player_drain_resumes_after_creature_refunds() {
        let mut budget = ChaseSearchBudget::default();
        // 48 pending creature searches hold their full reservations in a new window.
        budget.declare_creature_pending(48 * UNITS_PER_SEARCH);
        assert!(budget.open(50_000));
        // The first player searches do their full bounded work, so nothing is refunded.
        let mut players = 0;
        while budget.charge_player(50_000, UNITS_PER_SEARCH) {
            budget.note_player_search(50_000);
            players += 1;
        }
        assert_eq!(players, 16);
        budget.suspend_drain(50_000);
        assert!(!budget.open(50_000), "no capacity returned yet");
        // The creature drain runs and each search does a single expansion.
        for _ in 0..48 {
            assert!(budget.charge_creature(50_000, UNITS_PER_SEARCH, true));
            budget.refund_creature(50_000, UNITS_PER_SEARCH - 1);
        }
        assert!(
            budget.open(50_000),
            "the refunded units reopen the suspended player drain"
        );
        assert!(!budget.open(50_000), "a resumed drain is drained once more");
        // The resumed searches are trivial: only one expansion each stays debited.
        while budget.player_searches() < CHASE_SEARCHES_PER_WINDOW
            && budget.charge_player(50_000, UNITS_PER_SEARCH)
        {
            budget.refund_player(50_000, UNITS_PER_SEARCH - 1);
            budget.note_player_search(50_000);
            players += 1;
        }
        assert_eq!(players, 64, "the per-window search count spans the resume");
        // A complete drain (not suspended) never reopens, even with capacity left.
        let mut budget = ChaseSearchBudget::default();
        assert!(budget.open(0));
        assert!(!budget.open(0));
        budget.suspend_drain(50_000);
        assert!(!budget.open(0), "another window's suspension is ignored");
    }

    #[test]
    fn without_pending_creature_work_players_can_use_the_full_window() {
        let mut budget = ChaseSearchBudget::default();
        for _ in 0..64 {
            assert!(budget.charge_player(0, UNITS_PER_SEARCH));
        }
        assert_eq!(budget.remaining(0), 0);
        assert!(!budget.charge_player(0, 1));
        // Charged creature work consumed its declaration, so it no longer holds capacity.
        budget.declare_creature_pending(2 * UNITS_PER_SEARCH);
        assert!(budget.charge_creature(50_000, 2 * UNITS_PER_SEARCH, false));
        assert!(budget.charge_player(100_000, WINDOW_UNITS));
    }

    #[test]
    fn both_drains_share_one_total_and_stale_windows_cannot_reset_it() {
        let mut budget = ChaseSearchBudget::default();
        assert!(budget.open(50_000));
        assert!(budget.charge_player(50_000, 16 * UNITS_PER_SEARCH));
        assert!(budget.charge_creature(50_000, 32 * UNITS_PER_SEARCH, true));
        assert!(!budget.charge_player(49_999, 1));
        assert!(!budget.charge_creature(49_999, 1, false));
        assert_eq!(budget.remaining(49_999), 0);
        assert_eq!(budget.remaining(50_000), PLAYER_WAIT_RESERVE);
        assert!(budget.charge_creature(50_000, PLAYER_WAIT_RESERVE, false));
        assert_eq!(budget.remaining(50_000), 0);
        assert!(budget.charge_creature(100_000, UNITS_PER_SEARCH, true));
        assert_eq!(budget.player_units_used, 0);
        assert_eq!(budget.creature_units_used, UNITS_PER_SEARCH);
        assert_eq!(budget.remaining(100_000), WINDOW_UNITS - UNITS_PER_SEARCH);
        assert!(budget.open(100_000));
        assert!(!budget.open(50_000));
    }

    #[test]
    fn an_overflowing_charge_preserves_the_shared_window() {
        let mut budget = ChaseSearchBudget::default();
        assert!(budget.charge_player(0, UNITS_PER_SEARCH));
        assert!(!budget.charge_player(0, u32::MAX));
        assert!(!budget.charge_creature(0, u32::MAX, false));
        assert_eq!(budget.remaining(0), WINDOW_UNITS - UNITS_PER_SEARCH);
    }

    #[test]
    fn no_player_waiters_release_the_entire_shared_window_to_creatures() {
        let mut budget = ChaseSearchBudget::default();
        for _ in 0..64 {
            assert!(budget.charge_creature(0, UNITS_PER_SEARCH, false));
        }
        assert_eq!(budget.remaining(0), 0);
        assert!(!budget.charge_creature(0, 1, false));
        assert!(
            budget.open(0),
            "the player drain marker is still independent"
        );
        assert!(!budget.charge_player(0, 1));
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
