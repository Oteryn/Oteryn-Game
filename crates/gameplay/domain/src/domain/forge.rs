//! Tier and structural forge eligibility (accepted IMBUE-FORGE-0 §8/§10).
//! These pure checks consume supplied Content facts, not current authority.
//! The writer must separately bind them to current Content and recheck distinct
//! live instances, direct main-backpack placement, imbuements and fences under
//! its locks. D1230 adds only pure checked dust quotes/conversions below; no
//! item recipe, roll, current account balance, durable writer or mutation is implemented.

use super::item_definition::{ItemDefinitionFacts, ItemStackClass, ProjectV2ItemForgeProfile};

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

/// CP D1230 dust policy only; no recipe, account balance, mutation or runtime authority.
/// GUI/CIPSOFT S1 establish base100, step20 and initial next cost25. The general
/// cost25+n and maximum225 upgrades (4600) are separately accepted PARITY_WIKI W1.
/// D1230f: a future GUI shows only current_limit(), never the wiki maximum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ForgeDustLimit {
    upgrades: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForgeDustError {
    InvalidUpgradeCount,
    MaximumLimitReached,
    InsufficientDust,
    InsufficientSlivers,
    ArithmeticOverflow,
}

impl ForgeDustLimit {
    pub const INITIAL: Self = Self { upgrades: 0 };
    // Accepted enforcement ceiling, PARITY_WIKI W1; not a UI display value.
    const MAX_UPGRADES: u16 = super::forge_dust::DUST_LIMIT_UPGRADES_MAX;

    pub const fn new(upgrades: u16) -> Result<Self, ForgeDustError> {
        if upgrades <= Self::MAX_UPGRADES {
            Ok(Self { upgrades })
        } else {
            Err(ForgeDustError::InvalidUpgradeCount)
        }
    }

    /// Current unlocked dust limit, not the wiki maximum or a carried balance.
    pub fn current_limit(self) -> Result<u32, ForgeDustError> {
        super::forge_dust::DustLimit::from_upgrade_count(self.upgrades)
            .map(|limit| u32::from(limit.get()))
            .map_err(|_| ForgeDustError::InvalidUpgradeCount)
    }

    /// Quote exactly the next accepted limit upgrade. This does not spend dust.
    /// The native DustLimit receipt owner rechecks the quoted stage and debits atomically;
    /// this value-level API itself writes nothing.
    pub fn next_upgrade(self) -> Result<(u32, Self), ForgeDustError> {
        if self.upgrades == Self::MAX_UPGRADES {
            return Err(ForgeDustError::MaximumLimitReached);
        }
        let current = super::forge_dust::DustLimit::from_upgrade_count(self.upgrades)
            .map_err(|_| ForgeDustError::InvalidUpgradeCount)?;
        let (cost, next) = current
            .next_upgrade()
            .map_err(|_| ForgeDustError::ArithmeticOverflow)?;
        Ok((cost, Self::new(next.upgrade_count())?))
    }
}

/// Pure checked projection of one GUI-accepted conversion, not persisted balances.
/// No gold-to-dust operation exists (CP D1230f); no other recipe is inferred.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ForgeDustResources {
    pub dust: u64,
    pub slivers: u64,
    pub cores: u64,
}

impl ForgeDustResources {
    /// GUI: exactly60 dust ->3 slivers, once.
    pub fn dust_to_slivers(self) -> Result<Self, ForgeDustError> {
        let dust = self
            .dust
            .checked_sub(60)
            .ok_or(ForgeDustError::InsufficientDust)?;
        let slivers = self
            .slivers
            .checked_add(3)
            .ok_or(ForgeDustError::ArithmeticOverflow)?;
        Ok(Self {
            dust,
            slivers,
            ..self
        })
    }

    /// GUI: exactly50 slivers ->1 core, once.
    pub fn slivers_to_core(self) -> Result<Self, ForgeDustError> {
        let slivers = self
            .slivers
            .checked_sub(50)
            .ok_or(ForgeDustError::InsufficientSlivers)?;
        let cores = self
            .cores
            .checked_add(1)
            .ok_or(ForgeDustError::ArithmeticOverflow)?;
        Ok(Self {
            slivers,
            cores,
            ..self
        })
    }
}
