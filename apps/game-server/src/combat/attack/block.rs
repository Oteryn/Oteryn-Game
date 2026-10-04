//! The ATTACK-0 §5 time-based block budget (`creature.cpp:141-145, 961-963`): one block per
//! refill period, capped, used up per blocked hit. Hits beyond the budget meet armor only.

use super::constants::BlockConstants;
use oteryn_simulation_determinism::SemanticTimeMicros;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BlockBudget {
    blocks: u8,
    /// The next refill deadline. Refills keep a fixed phase from the budget's start, also while
    /// the budget is full, so how often it is polled never shifts the cadence.
    next_refill: SemanticTimeMicros,
}

impl BlockBudget {
    pub(crate) fn new(constants: &BlockConstants, now: SemanticTimeMicros) -> Self {
        Self {
            blocks: constants.initial_blocks.min(constants.max_blocks),
            next_refill: SemanticTimeMicros::from_micros(
                now.get().saturating_add(refill_period(constants)),
            ),
        }
    }

    pub(crate) fn blocks(&self) -> u8 {
        self.blocks
    }

    /// Apply every refill deadline up to `now`: one block each, up to the cap. A `now` before
    /// the next deadline changes nothing.
    pub(crate) fn advance_to(&mut self, constants: &BlockConstants, now: SemanticTimeMicros) {
        let Ok(late) = now.elapsed_since(self.next_refill) else {
            return;
        };
        let period = refill_period(constants);
        let refills = late / period + 1;
        let headroom = u64::from(constants.max_blocks.saturating_sub(self.blocks));
        // `min(refills, headroom) <= u8::MAX`, so the cast is exact.
        self.blocks = self
            .blocks
            .saturating_add(refills.min(headroom) as u8)
            .min(constants.max_blocks);
        self.next_refill = SemanticTimeMicros::from_micros(
            self.next_refill
                .get()
                .saturating_add(refills.saturating_mul(period)),
        );
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

fn refill_period(constants: &BlockConstants) -> u64 {
    constants.refill_ms.saturating_mul(1_000).max(1)
}
