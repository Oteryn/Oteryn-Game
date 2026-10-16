//! Tier and structural forge eligibility (accepted IMBUE-FORGE-0 §8/§10).
//! These pure checks consume supplied Content facts, not current authority.
//! The writer must separately bind them to current Content and recheck distinct
//! live instances, direct main-backpack placement, imbuements and fences under
//! its locks. No recipe, roll, cost or mutation is implemented here.

use crate::content::ProjectV2ItemForgeProfile;
use crate::durability::item_transfer::{ItemDefinitionFacts, ItemStackClass};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForgeShapeError {
    MissingForgeProfile,
    InvalidClassification,
    InvalidTierCap,
    NotNonStackable,
    TierAboveCap,
    DifferentDefinition,
    InconsistentDefinitionProfile,
    DifferentTier,
    DifferentClassification,
    ConvergenceRequiresClassFour,
    TargetAlreadyTiered,
    DonorTierTooLow,
}

/// Definition/tier predicates only; this value is not an ownership, location,
/// imbuement, current-revision or mutation-authority witness. In particular an
/// authoring profile alone cannot establish its binding to a live definition.
#[derive(Debug, Clone, Copy)]
pub struct ForgeItemShape<'a> {
    item: &'a ItemDefinitionFacts,
    profile: ProjectV2ItemForgeProfile,
    tier: ItemTier,
}

impl<'a> ForgeItemShape<'a> {
    /// Validate §8 against the actual Content profile shape. Class-specific caps
    /// are supplied by Content; Canary's example caps are not executable policy.
    pub fn from_content(
        item: &'a ItemDefinitionFacts,
        profile: Option<ProjectV2ItemForgeProfile>,
        tier: ItemTier,
    ) -> Result<Self, ForgeShapeError> {
        let profile = profile.ok_or(ForgeShapeError::MissingForgeProfile)?;
        if !(1..=4).contains(&profile.classification) {
            return Err(ForgeShapeError::InvalidClassification);
        }
        if !(1..=ItemTier::MAX).contains(&profile.max_tier) {
            return Err(ForgeShapeError::InvalidTierCap);
        }
        if item.stack != ItemStackClass::NonStackable {
            return Err(ForgeShapeError::NotNonStackable);
        }
        if tier.get() > profile.max_tier {
            return Err(ForgeShapeError::TierAboveCap);
        }
        Ok(Self {
            item,
            profile,
            tier,
        })
    }

    /// Standard fusion's potential success tier, before RNG and bonuses (§10).
    /// A refusal at the cap precedes any costs or outcome draw. This does not
    /// validate convergence, distinct instances or the writer-owned live facts.
    pub fn fusion_target_tier(self, source: Self) -> Result<ItemTier, ForgeShapeError> {
        if self.item.definition != source.item.definition {
            return Err(ForgeShapeError::DifferentDefinition);
        }
        if self.profile != source.profile {
            return Err(ForgeShapeError::InconsistentDefinitionProfile);
        }
        if self.tier != source.tier {
            return Err(ForgeShapeError::DifferentTier);
        }
        self.checked_target_tier(self.tier.get() + 1)
    }

    /// Standard transfer's receiver tier (§10). Different definitions are
    /// allowed within one class; the receiver's own Content cap still applies.
    /// Convergence has different rules and must not use this calculation.
    pub fn transfer_target_tier(self, donor: Self) -> Result<ItemTier, ForgeShapeError> {
        self.validate_transfer_pair(donor)?;
        self.checked_target_tier(donor.tier.get() - 1)
    }

    /// Convergence transfer has the same transfer prerequisites (§10), requires
    /// class four on both definitions, and preserves the donor's tier instead
    /// of subtracting one. The receiver's Content cap is still enforced.
    pub fn convergence_transfer_target_tier(
        self,
        donor: Self,
    ) -> Result<ItemTier, ForgeShapeError> {
        if self.profile.classification != 4 || donor.profile.classification != 4 {
            return Err(ForgeShapeError::ConvergenceRequiresClassFour);
        }
        self.validate_transfer_pair(donor)?;
        self.checked_target_tier(donor.tier.get())
    }

    fn validate_transfer_pair(self, donor: Self) -> Result<(), ForgeShapeError> {
        if self.profile.classification != donor.profile.classification {
            return Err(ForgeShapeError::DifferentClassification);
        }
        if self.tier != ItemTier::ZERO {
            return Err(ForgeShapeError::TargetAlreadyTiered);
        }
        if donor.tier.get() < 2 {
            return Err(ForgeShapeError::DonorTierTooLow);
        }
        Ok(())
    }

    fn checked_target_tier(self, value: u8) -> Result<ItemTier, ForgeShapeError> {
        if value > self.profile.max_tier {
            return Err(ForgeShapeError::TierAboveCap);
        }
        ItemTier::new(value).ok_or(ForgeShapeError::TierAboveCap)
    }
}

#[cfg(test)]
#[path = "../durability/forge_tests.rs"]
mod tests;
