//! Pure forge quotes and outcomes consumed by the fenced durable owner.
//!
//! Crystal 201189f player.cpp:10545-10942 and game.cpp:10990-11035.
//! Supplied Content, resources and reserved server draws are value projections;
//! the caller must resolve current custody and commit the complete plan atomically.

use oteryn_gameplay_domain::domain::death::EquipmentSlot;
use oteryn_gameplay_domain::domain::forge::{ForgeItemShape, ForgeShapeError, ItemTier};
use oteryn_gameplay_domain::domain::forge_dust::ForgeDustBalance;
use oteryn_gameplay_domain::domain::item_definition::{
    ItemDefinitionFacts, ProjectV2ItemForgeProfile,
};

mod prices;
pub use prices::{
    CONVERGENCE_FUSION_DUST, CONVERGENCE_TRANSFER_DUST, EXALTED_CORE_KEY, FORGE_CLASS_CAPS,
    FORGE_REFERENCE_PRICES, FORGE_SLIVER_KEY, ForgeReferencePrice, NORMAL_FUSION_DUST,
    NORMAL_FUSION_SUCCESS_PERCENT, NORMAL_TRANSFER_DUST,
};

/// Crystal 201189f config.lua.dist:198-204: fusion/transfer cost 100 dust;
/// base success 50%, one core adds 15 points, a protection core halves tier loss.
pub const FORGE_DUST_COST: u32 = 100;
pub const FORGE_BASE_SUCCESS_PERCENT: u8 = 50;
pub const FORGE_BOOSTED_SUCCESS_PERCENT: u8 = 65;
pub const FORGE_PROTECTION_PERCENT: u8 = 50;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForgeOperation {
    Fusion { boost: bool, protect: bool },
    ConvergenceFusion,
    Transfer,
    ConvergenceTransfer,
}

/// Supplied facts only. The live writer must resolve and lock these instances.
#[derive(Debug, Clone, Copy)]
pub struct ForgeItem<'a> {
    pub instance: [u8; 16],
    pub definition: &'a ItemDefinitionFacts,
    pub profile: ProjectV2ItemForgeProfile,
    pub tier: ItemTier,
    /// Primary equipment slot from the current admitted Content definition.
    pub slot: EquipmentSlot,
}

#[derive(Debug, Clone, Copy)]
pub struct ForgeRequest<'a> {
    pub operation: ForgeOperation,
    pub target: ForgeItem<'a>,
    pub source: ForgeItem<'a>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ForgeCost {
    pub gold: u64,
    pub dust: u32,
    pub cores: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForgeRuleError {
    SameInstance,
    InvalidClassCap,
    Shape(ForgeShapeError),
    InvalidRoll,
    InsufficientGold,
    InsufficientDust,
    InsufficientCores,
}

/// A validated quote, constructed only by checked conversion of a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ForgeQuote {
    cost: ForgeCost,
    success_percent: u8,
    operation: ForgeOperation,
    original_tier: ItemTier,
    target_tier: ItemTier,
    classification: u8,
    max_tier: u8,
}

impl AsRef<ForgeCost> for ForgeQuote {
    fn as_ref(&self) -> &ForgeCost {
        &self.cost
    }
}

impl TryFrom<ForgeRequest<'_>> for ForgeQuote {
    type Error = ForgeRuleError;

    fn try_from(request: ForgeRequest<'_>) -> Result<Self, Self::Error> {
        if request.target.instance == request.source.instance {
            return Err(ForgeRuleError::SameInstance);
        }
        let target = shape(request.target)?;
        let source = shape(request.source)?;
        let (target_tier, price_tier, cores, success_percent) = match request.operation {
            ForgeOperation::Fusion { boost, protect } => {
                let tier = target
                    .fusion_target_tier(source)
                    .map_err(ForgeRuleError::Shape)?;
                (
                    tier,
                    tier.get(),
                    u8::from(boost) + u8::from(protect),
                    if boost {
                        FORGE_BOOSTED_SUCCESS_PERCENT
                    } else {
                        FORGE_BASE_SUCCESS_PERCENT
                    },
                )
            }
            ForgeOperation::ConvergenceFusion => {
                let tier = target
                    .convergence_fusion_target_tier(source)
                    .map_err(ForgeRuleError::Shape)?;
                check_slots(request.target.slot, request.source.slot)?;
                (tier, tier.get(), 0, 100)
            }
            ForgeOperation::Transfer => {
                let tier = target
                    .transfer_target_tier(source)
                    .map_err(ForgeRuleError::Shape)?;
                check_slots(request.target.slot, request.source.slot)?;
                // Crystal player.cpp:10843-10846 indexes both by RESULT tier.
                // Canary 47dfd51:11603 instead charges donor-tier gold.
                let price = price(request.target.profile.classification, tier.get())?;
                (tier, tier.get(), price.transfer_cores, 100)
            }
            ForgeOperation::ConvergenceTransfer => {
                let tier = target
                    .convergence_transfer_target_tier(source)
                    .map_err(ForgeRuleError::Shape)?;
                let row = price(request.target.profile.classification, tier.get())?;
                (tier, tier.get(), row.transfer_cores, 100)
            }
        };
        let row = price(request.target.profile.classification, price_tier)?;
        let (gold, dust) = match request.operation {
            ForgeOperation::ConvergenceFusion => {
                (row.convergence_fusion_gold, CONVERGENCE_FUSION_DUST)
            }
            ForgeOperation::ConvergenceTransfer => {
                (row.convergence_transfer_gold, CONVERGENCE_TRANSFER_DUST)
            }
            ForgeOperation::Fusion { .. } => (row.gold, NORMAL_FUSION_DUST),
            ForgeOperation::Transfer => (row.gold, NORMAL_TRANSFER_DUST),
        };
        Ok(Self {
            cost: ForgeCost { gold, dust, cores },
            success_percent,
            operation: request.operation,
            original_tier: request.target.tier,
            target_tier,
            classification: request.target.profile.classification,
            max_tier: request.target.profile.max_tier,
        })
    }
}

fn check_slots(target: EquipmentSlot, source: EquipmentSlot) -> Result<(), ForgeRuleError> {
    // Crystal protocolgame.cpp:6813-6816,6890-6917 normalizes two-handed
    // weapons to the hand group; either hand is the same forging category.
    let normalize = |slot| match slot {
        EquipmentSlot::LeftHand => EquipmentSlot::RightHand,
        other => other,
    };
    if normalize(target) == normalize(source) {
        Ok(())
    } else {
        Err(ForgeRuleError::Shape(
            ForgeShapeError::DifferentEquipmentSlot,
        ))
    }
}

fn shape(item: ForgeItem<'_>) -> Result<ForgeItemShape<'_>, ForgeRuleError> {
    let cap = FORGE_CLASS_CAPS
        .get(usize::from(item.profile.classification))
        .copied()
        .filter(|cap| *cap != 0)
        .ok_or(ForgeRuleError::InvalidClassCap)?;
    if item.profile.max_tier > cap {
        return Err(ForgeRuleError::InvalidClassCap);
    }
    ForgeItemShape::from_content(item.definition, Some(item.profile), item.tier)
        .map_err(ForgeRuleError::Shape)
}

fn price(classification: u8, tier: u8) -> Result<ForgeReferencePrice, ForgeRuleError> {
    FORGE_REFERENCE_PRICES
        .iter()
        .find(|row| row.classification == classification && row.tier == tier)
        .copied()
        .ok_or(ForgeRuleError::InvalidClassCap)
}

/// Dust is the existing Character asset; cores/slivers remain native items.
/// These counts are a checked arithmetic projection, never a second wallet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ForgeResources {
    pub gold: u64,
    pub dust: ForgeDustBalance,
    pub slivers: u64,
    pub cores: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct ForgeAttempt {
    pub quote: ForgeQuote,
    pub resources: ForgeResources,
    /// Independent server draws in 0..100, reserved with the occurrence.
    pub success_roll: u8,
    /// Crystal player.cpp:10736: 0..50 loses a tier with protection, 50..100 keeps it.
    pub protection_roll: u8,
    /// Crystal game.cpp:11015 draws uniformly in the inclusive range 0..=10_000.
    pub bonus_roll: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ForgePlan {
    pub success: bool,
    pub target_tier: ItemTier,
    /// None means BURN; Some(0) preserves an untiered instance.
    pub source_tier: Option<ItemTier>,
    /// Crystal result bonus codes 0..=8, including 8 for protection on failure.
    pub bonus: u8,
    /// Actual consumption after any successful-fusion resource bonus.
    pub cost: ForgeCost,
    pub resources_after: ForgeResources,
}

impl TryFrom<ForgeAttempt> for ForgePlan {
    type Error = ForgeRuleError;

    fn try_from(attempt: ForgeAttempt) -> Result<Self, Self::Error> {
        if attempt.success_roll >= 100
            || attempt.protection_roll >= 100
            || attempt.bonus_roll > 10_000
        {
            return Err(ForgeRuleError::InvalidRoll);
        }
        let quote = attempt.quote;
        let resources = attempt.resources;
        // Crystal player.cpp:10560-10572 checks nominal dust and cores before
        // awarding their possible refunds. Gold is charged only after the bonus.
        if u32::from(resources.dust.balance()) < quote.cost.dust {
            return Err(ForgeRuleError::InsufficientDust);
        }
        if resources.cores < u64::from(quote.cost.cores) {
            return Err(ForgeRuleError::InsufficientCores);
        }
        let success = attempt.success_roll < quote.success_percent;
        let protected = !success
            && matches!(
                quote.operation,
                ForgeOperation::Fusion { protect: true, .. }
            )
            && attempt.protection_roll >= FORGE_PROTECTION_PERCENT;
        let bonus = if success && matches!(quote.operation, ForgeOperation::Fusion { .. }) {
            fusion_bonus(attempt.bonus_roll)
        } else if protected {
            8
        } else {
            0
        };
        let cost = ForgeCost {
            gold: if bonus == 3 { 0 } else { quote.cost.gold },
            dust: if bonus == 1 { 0 } else { quote.cost.dust },
            cores: if bonus == 2 { 0 } else { quote.cost.cores },
        };
        let gold = resources
            .gold
            .checked_sub(cost.gold)
            .ok_or(ForgeRuleError::InsufficientGold)?;
        let dust = if cost.dust == 0 {
            resources.dust
        } else {
            resources
                .dust
                .spend(cost.dust)
                .map_err(|_| ForgeRuleError::InsufficientDust)?
                .after
        };
        let cores = resources
            .cores
            .checked_sub(u64::from(cost.cores))
            .ok_or(ForgeRuleError::InsufficientCores)?;
        let source_tier = if success {
            match bonus {
                4 => ItemTier::new(quote.original_tier.get().saturating_sub(1)),
                5 => Some(quote.original_tier),
                6 => Some(quote.target_tier),
                _ => None,
            }
        } else if protected {
            Some(quote.original_tier)
        } else {
            quote
                .original_tier
                .get()
                .checked_sub(1)
                .and_then(ItemTier::new)
        };
        let target_tier = if !success {
            quote.original_tier
        } else if bonus == 7
            && quote.original_tier.get() + 2 <= quote.classification
            && quote.original_tier.get() + 2 <= quote.max_tier
        {
            // Preserve Crystal player.cpp:10722's classification check: even
            // class four gains two tiers only when the result is at most four.
            ItemTier::new(quote.original_tier.get() + 2).ok_or(ForgeRuleError::InvalidClassCap)?
        } else {
            quote.target_tier
        };
        Ok(Self {
            success,
            target_tier,
            source_tier,
            bonus,
            cost,
            resources_after: ForgeResources {
                gold,
                dust,
                cores,
                ..resources
            },
        })
    }
}

/// Crystal 201189f src/utils/tools.cpp:1879-1910. The draw has 10_001
/// possible values, so the final bonus includes both 9_975 and 10_000.
fn fusion_bonus(roll: u16) -> u8 {
    match roll {
        0..7400 => 0,
        7400..9000 => 1,
        9000..9500 => 2,
        9500..9525 => 3,
        9525..9550 => 4,
        9550..9950 => 5,
        9950..9975 => 6,
        _ => 7,
    }
}

#[cfg(test)]
#[path = "forge_property_tests.rs"]
mod property_tests;
#[cfg(test)]
#[path = "forge_tests.rs"]
mod tests;
