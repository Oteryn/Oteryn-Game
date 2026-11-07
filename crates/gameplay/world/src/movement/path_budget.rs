//! RL-09: one expansion ledger shared by creature paths and player chase in a Channel.
use super::path_search::PATH_SEARCH_WORK;

pub const PATH_WINDOW_US: u64 = 50_000;
const WINDOW_WORK: usize = 65_536;
const PLAYER_RESERVE: usize = 16_384;

#[derive(Debug, Default)]
pub struct ChannelPathBudget {
    window: Option<u64>,
    used: usize,
    player_used: usize,
    creature_pending: usize,
}

impl ChannelPathBudget {
    fn advance(&mut self, now: u64) -> bool {
        let window = now / PATH_WINDOW_US;
        if self.window.is_some_and(|old| old > window) {
            return false;
        }
        if self.window != Some(window) {
            self.window = Some(window);
            self.used = 0;
            self.player_used = 0;
        }
        true
    }

    /// Declare the existing creature queue before either owner's drain. A player cannot
    /// consume work already owed to pending creatures, except its accepted reserve.
    pub fn declare_creatures(&mut self, pending: usize) {
        self.creature_pending = pending;
    }

    /// Admit a whole bounded search; actual work is charged after it runs. Budget refusal
    /// keeps the request pending and never becomes an unreachable result.
    pub fn admits(&mut self, now: u64, player: bool, player_waiting: bool) -> bool {
        if !self.advance(now) {
            return false;
        }
        let remaining = WINDOW_WORK.saturating_sub(self.used);
        if player {
            let protected = self
                .creature_pending
                .saturating_mul(PATH_SEARCH_WORK)
                .min(remaining.saturating_sub(PLAYER_RESERVE.saturating_sub(self.player_used)));
            remaining.saturating_sub(protected) >= PATH_SEARCH_WORK
        } else {
            let reserve = if player_waiting {
                PLAYER_RESERVE.saturating_sub(self.player_used)
            } else {
                0
            };
            remaining.saturating_sub(reserve) >= PATH_SEARCH_WORK
        }
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn charged_work(&self) -> usize {
        self.used
    }

    pub fn charge(&mut self, player: bool, expansions: usize) {
        debug_assert!(expansions <= PATH_SEARCH_WORK);
        self.used += expansions;
        if player {
            self.player_used += expansions;
        } else {
            self.creature_pending = self.creature_pending.saturating_sub(1);
        }
    }
}

#[cfg(test)]
mod path_budget_tests;
