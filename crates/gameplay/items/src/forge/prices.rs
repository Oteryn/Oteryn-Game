//! Crystal 201189f data/scripts/systems/item_tiers.lua:1-90 prices and
//! config.lua.dist:193-204 resource costs. Canary 47dfd51 has identical prices;
//! its standard-transfer index differs and is intentionally not followed.

pub const FORGE_CLASS_CAPS: [u8; 5] = [0, 1, 2, 3, 10];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ForgeReferencePrice {
    pub classification: u8,
    /// Result tier for both fusion and transfer gold (Crystal player.cpp:10843-10846).
    pub tier: u8,
    pub gold: u64,
    /// Indexed by receiver result tier for standard transfer.
    pub transfer_cores: u8,
    /// Guaranteed class-four fusion price; zero for classes without convergence.
    pub convergence_fusion_gold: u64,
    /// Guaranteed class-four transfer price; zero for classes without convergence.
    pub convergence_transfer_gold: u64,
}

const fn row(classification: u8, tier: u8, gold: u64, transfer_cores: u8) -> ForgeReferencePrice {
    ForgeReferencePrice {
        classification,
        tier,
        gold,
        transfer_cores,
        convergence_fusion_gold: 0,
        convergence_transfer_gold: 0,
    }
}

const fn convergence(
    tier: u8,
    gold: u64,
    transfer_cores: u8,
    fusion_gold: u64,
    transfer_gold: u64,
) -> ForgeReferencePrice {
    ForgeReferencePrice {
        classification: 4,
        tier,
        gold,
        transfer_cores,
        convergence_fusion_gold: fusion_gold,
        convergence_transfer_gold: transfer_gold,
    }
}

pub const FORGE_REFERENCE_PRICES: [ForgeReferencePrice; 16] = [
    row(1, 1, 25_000, 1),
    row(2, 1, 750_000, 1),
    row(2, 2, 5_000_000, 1),
    row(3, 1, 4_000_000, 1),
    row(3, 2, 10_000_000, 2),
    row(3, 3, 20_000_000, 3),
    convergence(1, 8_000_000, 1, 55_000_000, 65_000_000),
    convergence(2, 20_000_000, 2, 110_000_000, 165_000_000),
    convergence(3, 40_000_000, 5, 170_000_000, 375_000_000),
    convergence(4, 65_000_000, 10, 300_000_000, 800_000_000),
    convergence(5, 100_000_000, 15, 875_000_000, 2_000_000_000),
    convergence(6, 250_000_000, 25, 2_350_000_000, 5_250_000_000),
    convergence(7, 750_000_000, 35, 6_950_000_000, 14_500_000_000),
    convergence(8, 2_500_000_000, 50, 21_250_000_000, 42_500_000_000),
    convergence(9, 8_000_000_000, 60, 50_000_000_000, 100_000_000_000),
    convergence(10, 15_000_000_000, 85, 125_000_000_000, 300_000_000_000),
];

/// Native resource identities shared with the P19 influenced-creature supply path.
pub const FORGE_SLIVER_KEY: &str = "oteryn:item.tibia.i37109";
pub const EXALTED_CORE_KEY: &str = "oteryn:item.tibia.i37110";
pub const NORMAL_FUSION_DUST: u32 = 100;
pub const CONVERGENCE_FUSION_DUST: u32 = 130;
pub const NORMAL_TRANSFER_DUST: u32 = 100;
pub const CONVERGENCE_TRANSFER_DUST: u32 = 160;
pub const NORMAL_FUSION_SUCCESS_PERCENT: u8 = 50;
