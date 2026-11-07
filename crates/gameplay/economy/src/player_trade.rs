//! PLAYER-TRADE0 leaf exchange policy. These values establish no runtime authority.
//! Container trees are BAGS-TRADE-1; this slice offers exactly one whole leaf per side.

mod lifecycle;
pub use lifecycle::{Accept, CloseReason, Invalidation, Phase, Trade};

pub const IDLE_TIMEOUT_MS: u64 = 120_000;
pub const REFERENCE_ITEM_LIMIT: u32 = 100;
/// GAMEITEM01 stack ceiling (D82); container item count is a separate rule.
pub const LEAF_QUANTITY_MAX: u32 = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    Rejected,
    Stale,
    TooFar,
    NotTradeable,
    NoRoom,
    Overweight,
    UnknownWeight,
    Transferring,
    ClockReversed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TradeFlag {
    Unknown,
    Known(bool),
    Ambiguous,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LeafFacts {
    pub direct_backpack_entry: bool,
    pub is_main_backpack: bool,
    pub has_contents: bool,
    pub pickupable: bool,
    pub tradeable: TradeFlag,
    pub quantity: u32,
}

impl LeafFacts {
    pub fn check(self) -> Result<(), Refusal> {
        if !self.direct_backpack_entry
            || self.is_main_backpack
            || self.has_contents
            || !self.pickupable
            || self.quantity == 0
            || self.quantity > LEAF_QUANTITY_MAX
            || !matches!(self.tradeable, TradeFlag::Unknown | TradeFlag::Known(true))
        {
            return Err(Refusal::NotTradeable);
        }
        Ok(())
    }
}

/// Unknown visibility/LOS/map facts must be refused by the runtime before this pure rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PairFacts {
    pub same_character: bool,
    pub same_account: bool,
    pub same_channel: bool,
    pub positions: [(u16, u16, i8); 2],
    pub mutually_visible: bool,
    pub line_of_sight: bool,
}

impl PairFacts {
    pub fn check(self) -> Result<(), Refusal> {
        if self.same_character || self.same_account || !self.same_channel {
            return Err(Refusal::Rejected);
        }
        let [a, b] = self.positions;
        if a.2 != b.2 || a.0.abs_diff(b.0).max(a.1.abs_diff(b.1)) > 2 {
            return Err(Refusal::TooFar);
        }
        if !self.mutually_visible || !self.line_of_sight {
            return Err(Refusal::Rejected);
        }
        Ok(())
    }
}

/// Admission happens before either source is removed: no freed-entry/weight credit,
/// and no auto-stack merge. Weight is in centi-oz, matching B3-3.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Destination {
    pub entries: u32,
    pub entry_limit: u32,
    pub highest_ordinal: u64,
    pub carried_weight: u64,
    pub capacity: u64,
    pub incoming_unit_weight: Option<u32>,
    pub incoming_quantity: u32,
}

impl Destination {
    pub fn next_entry(self) -> Result<u64, Refusal> {
        if self.entry_limit == 0
            || self.entries >= self.entry_limit.min(20)
            || self.incoming_quantity == 0
        {
            return Err(Refusal::NoRoom);
        }
        let unit = self.incoming_unit_weight.ok_or(Refusal::UnknownWeight)?;
        let total =
            u128::from(self.carried_weight) + u128::from(unit) * u128::from(self.incoming_quantity);
        if total > u128::from(self.capacity) {
            return Err(Refusal::Overweight);
        }
        self.highest_ordinal.checked_add(1).ok_or(Refusal::NoRoom)
    }
}

#[cfg(test)]
mod player_trade_tests;
