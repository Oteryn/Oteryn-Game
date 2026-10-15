//! Item tier representation (accepted IMBUE-FORGE-0 §8). This value does not
//! qualify classification, the class cap, current Content or forge eligibility.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ItemTier(u8);

impl ItemTier {
    /// An existing ItemInstance with no durable tier row.
    pub const ZERO: Self = Self(0);
    pub const MAX: u8 = 10;

    #[must_use]
    pub const fn new(value: u8) -> Option<Self> {
        if value <= Self::MAX {
            Some(Self(value))
        } else {
            None
        }
    }

    #[must_use]
    pub const fn get(self) -> u8 {
        self.0
    }
}
