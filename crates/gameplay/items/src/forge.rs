//! ITEM-FORGE-1 pure standard fusion/transfer plans (IMBUE-FORGE0 §8–10).
//!
//! Reference prices are not an admitted Content recipe. A plan neither grants
//! authority nor spends value. The eventual writer must bind current Content,
//! session fences, custody, imbuements, server RNG and the whole native receipt.
//! Convergence and success bonuses require their own Content rules.

use oteryn_gameplay_domain::domain::forge::{ForgeItemShape, ForgeShapeError, ItemTier};
use oteryn_gameplay_domain::domain::forge_dust::ForgeDustBalance;
use oteryn_gameplay_domain::domain::item_definition::{
    ItemDefinitionFacts, ProjectV2ItemForgeProfile,
};

mod prices;
pub use prices::{FORGE_CLASS_CAPS, FORGE_REFERENCE_PRICES, ForgeReferencePrice};

/// Canary 04b83b51 config.lua.dist: fusion/transfer both cost 100 dust;
/// base success 50%, one core adds 15 points, protection prevents loss at 50%.
pub const FORGE_DUST_COST: u32 = 100;
pub const FORGE_BASE_SUCCESS_PERCENT: u8 = 50;
pub const FORGE_BOOSTED_SUCCESS_PERCENT: u8 = 65;
pub const FORGE_PROTECTION_PERCENT: u8 = 50;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForgeOperation {
    Fusion { boost: bool, protect: bool },
    Transfer,
}

/// Supplied facts only. The live writer must resolve and lock these instances.
#[derive(Debug, Clone, Copy)]
pub struct ForgeItem<'a> {
    pub instance: [u8; 16],
    pub definition: &'a ItemDefinitionFacts,
    pub profile: ProjectV2ItemForgeProfile,
    pub tier: ItemTier,
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
            ForgeOperation::Transfer => {
                let tier = target
                    .transfer_target_tier(source)
                    .map_err(ForgeRuleError::Shape)?;
                // The fee is indexed by DONOR tier; cores by RESULT tier.
                let price = price(request.target.profile.classification, tier.get())?;
                (tier, request.source.tier.get(), price.transfer_cores, 100)
            }
        };
        Ok(Self {
            cost: ForgeCost {
                gold: price(request.target.profile.classification, price_tier)?.gold,
                dust: FORGE_DUST_COST,
                cores,
            },
            success_percent,
            operation: request.operation,
            original_tier: request.target.tier,
            target_tier,
        })
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
    pub protection_roll: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ForgePlan {
    pub success: bool,
    pub target_tier: ItemTier,
    /// None means BURN; Some(0) preserves an untiered instance.
    pub source_tier: Option<ItemTier>,
    pub resources_after: ForgeResources,
}

impl TryFrom<ForgeAttempt> for ForgePlan {
    type Error = ForgeRuleError;

    fn try_from(attempt: ForgeAttempt) -> Result<Self, Self::Error> {
        if attempt.success_roll >= 100 || attempt.protection_roll >= 100 {
            return Err(ForgeRuleError::InvalidRoll);
        }
        let quote = attempt.quote;
        let resources = attempt.resources;
        // All costs are paid even on failure. Input values remain unchanged on refusal.
        let gold = resources
            .gold
            .checked_sub(quote.cost.gold)
            .ok_or(ForgeRuleError::InsufficientGold)?;
        let dust = resources
            .dust
            .spend(quote.cost.dust)
            .map_err(|_| ForgeRuleError::InsufficientDust)?
            .after;
        let cores = resources
            .cores
            .checked_sub(u64::from(quote.cost.cores))
            .ok_or(ForgeRuleError::InsufficientCores)?;
        let success = attempt.success_roll < quote.success_percent;
        let source_tier = if success {
            None
        } else if matches!(
            quote.operation,
            ForgeOperation::Fusion { protect: true, .. }
        ) && attempt.protection_roll < FORGE_PROTECTION_PERCENT
        {
            Some(quote.original_tier)
        } else {
            quote
                .original_tier
                .get()
                .checked_sub(1)
                .and_then(ItemTier::new)
        };
        Ok(Self {
            success,
            target_tier: if success {
                quote.target_tier
            } else {
                quote.original_tier
            },
            source_tier,
            resources_after: ForgeResources {
                gold,
                dust,
                cores,
                ..resources
            },
        })
    }
}

#[cfg(test)]
#[path = "forge_property_tests.rs"]
mod property_tests;
#[cfg(test)]
#[path = "forge_tests.rs"]
mod tests;
