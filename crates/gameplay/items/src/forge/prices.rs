//! Reference data only, not Content activation.
//! https://github.com/opentibiabr/canary/blob/04b83b512114bfd888000d6e1433ed8ecaec7c5b/data/scripts/systems/item_tiers.lua

pub const FORGE_CLASS_CAPS: [u8; 5] = [0, 1, 2, 3, 10];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ForgeReferencePrice {
    pub classification: u8,
    /// Result tier for fusion; donor tier for standard transfer gold.
    pub tier: u8,
    pub gold: u64,
    /// Indexed by receiver result tier for standard transfer.
    pub transfer_cores: u8,
}

const fn row(classification: u8, tier: u8, gold: u64, transfer_cores: u8) -> ForgeReferencePrice {
    ForgeReferencePrice {
        classification,
        tier,
        gold,
        transfer_cores,
    }
}

pub const FORGE_REFERENCE_PRICES: [ForgeReferencePrice; 16] = [
    row(1, 1, 25_000, 1),
    row(2, 1, 750_000, 1),
    row(2, 2, 5_000_000, 1),
    row(3, 1, 4_000_000, 1),
    row(3, 2, 10_000_000, 2),
    row(3, 3, 20_000_000, 3),
    row(4, 1, 8_000_000, 1),
    row(4, 2, 20_000_000, 2),
    row(4, 3, 40_000_000, 5),
    row(4, 4, 65_000_000, 10),
    row(4, 5, 100_000_000, 15),
    row(4, 6, 250_000_000, 25),
    row(4, 7, 750_000_000, 35),
    row(4, 8, 2_500_000_000, 50),
    row(4, 9, 8_000_000_000, 60),
    row(4, 10, 15_000_000_000, 85),
];
