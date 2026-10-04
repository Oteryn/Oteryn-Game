//! The ATTACK-0 §5 time-based block budget (`creature.cpp:141-145, 961-963`): one block per
//! refill period, capped, used up per blocked hit. Hits beyond the budget meet armor only.

use super::constants::BlockConstants;
use oteryn_simulation_determinism::SemanticTimeMicros;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BlockBudget {
    blocks: u8,
    /// Time accrued toward the next block.
    accrued_micros: u64,
    /// The instant the budget was last advanced to.
    at: SemanticTimeMicros,
}

impl BlockBudget {
    pub(crate) fn new(constants: &BlockConstants, now: SemanticTimeMicros) -> Self {
        Self {
            blocks: constants.initial_blocks.min(constants.max_blocks),
            accrued_micros: 0,
            at: now,
        }
    }

    pub(crate) fn blocks(&self) -> u8 {
        self.blocks
    }

    /// Accrue the time since the last advance: one block per full refill period, up to the cap.
    /// A full budget accrues nothing; a `now` before the last advance changes nothing.
    pub(crate) fn advance_to(&mut self, constants: &BlockConstants, now: SemanticTimeMicros) {
        let Ok(elapsed) = now.elapsed_since(self.at) else {
            return;
        };
        self.at = now;
        if self.blocks >= constants.max_blocks {
            self.blocks = constants.max_blocks;
            self.accrued_micros = 0;
            return;
        }
        let period = constants.refill_ms.saturating_mul(1_000).max(1);
        let total = self.accrued_micros.saturating_add(elapsed);
        let gained = total / period;
        let headroom = u64::from(constants.max_blocks - self.blocks);
        if gained >= headroom {
            self.blocks = constants.max_blocks;
            self.accrued_micros = 0;
        } else {
            // `gained < headroom <= u8::MAX`, so the cast is exact.
            self.blocks += gained as u8;
            self.accrued_micros = total % period;
        }
    }

    /// Use one block for a blockable hit at `now`; `false` when the budget is empty.
    pub(crate) fn try_block(
        &mut self,
        constants: &BlockConstants,
        now: SemanticTimeMicros,
    ) -> bool {
        self.advance_to(constants, now);
        if self.blocks == 0 {
            return false;
        }
        self.blocks -= 1;
        true
    }
}
