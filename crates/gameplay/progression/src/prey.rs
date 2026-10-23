//! Nonrandom PREY0 rules. Draw admission, payment and Store delivery are separate owners.
//! Reference: Canary 52550d5e3c0ffe565f4bd1014f4e5575abf1059b,
//! `src/io/ioprey.cpp` (reloadBonusValue/parsePreyAction), `config.lua.dist:138-150`.

use oteryn_gameplay_domain::domain::premium::PremiumGate;

pub const SLOT_COUNT: u8 = 3;
pub const LIST_SIZE: usize = 9;
pub const CREATURE_KEY_BYTES: usize = 256;
pub const HUNTING_SECONDS: u32 = 7_200;
pub const FREE_REROLL_SECONDS: u64 = 72_000;
pub const GOLD_PER_LEVEL: u64 = 200;
pub const WILDCARD_CAP: u8 = 52;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreyRefusal {
    InvalidSlot,
    NotEligible,
    SlotLocked,
    InvalidBonus,
    InvalidTime,
    InvalidLevel,
    InvalidBalance,
    NotEnoughWildcards,
    InvalidOption,
}

/// Only live, same Account+World permanent facts belong here, never Store intent.
#[derive(Debug, Clone, Copy)]
pub struct SlotAccess {
    pub slot: u8,
    pub has_vocation: bool,
    pub premium: PremiumGate,
    pub permanent: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnlockedSlot(u8);

impl TryFrom<SlotAccess> for UnlockedSlot {
    type Error = PreyRefusal;

    fn try_from(facts: SlotAccess) -> Result<Self, Self::Error> {
        if !(1..=SLOT_COUNT).contains(&facts.slot) {
            return Err(PreyRefusal::InvalidSlot);
        }
        if !facts.has_vocation {
            return Err(PreyRefusal::NotEligible);
        }
        if facts.slot == 1
            || facts.permanent
            || (facts.slot == 2 && facts.premium.require().is_ok())
        {
            Ok(Self(facts.slot))
        } else {
            Err(PreyRefusal::SlotLocked)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BonusType {
    Damage,
    Reduction,
    Experience,
    Loot,
}

impl TryFrom<&str> for BonusType {
    type Error = PreyRefusal;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "DAMAGE" => Ok(Self::Damage),
            "REDUCTION" => Ok(Self::Reduction),
            "XP" => Ok(Self::Experience),
            "LOOT" => Ok(Self::Loot),
            _ => Err(PreyRefusal::InvalidBonus),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreyBonus {
    pub kind: BonusType,
    pub grade: u8,
    pub percent: u8,
}

impl TryFrom<(BonusType, u8)> for PreyBonus {
    type Error = PreyRefusal;

    fn try_from((kind, grade): (BonusType, u8)) -> Result<Self, Self::Error> {
        if !(1..=10).contains(&grade) {
            return Err(PreyRefusal::InvalidBonus);
        }
        let percent = match kind {
            BonusType::Damage => 2 * grade + 5,
            BonusType::Reduction => 2 * grade + 10,
            BonusType::Experience | BonusType::Loot => 3 * grade + 10,
        };
        Ok(Self {
            kind,
            grade,
            percent,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreyOption {
    None,
    AutoReroll,
    Lock,
}

impl TryFrom<&str> for PreyOption {
    type Error = PreyRefusal;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "NONE" => Ok(Self::None),
            "AUTO_REROLL" => Ok(Self::AutoReroll),
            "LOCK" => Ok(Self::Lock),
            _ => Err(PreyRefusal::InvalidOption),
        }
    }
}

/// A check at use only: choosing an option never reserves or spends wildcards.
impl TryFrom<(PreyOption, u8)> for PreyOption {
    type Error = PreyRefusal;

    fn try_from((option, balance): (PreyOption, u8)) -> Result<Self, Self::Error> {
        if balance > WILDCARD_CAP {
            return Err(PreyRefusal::InvalidBalance);
        }
        let required = match option {
            Self::None => 0,
            Self::AutoReroll => 1,
            Self::Lock => 5,
        };
        if balance < required {
            Err(PreyRefusal::NotEnoughWildcards)
        } else {
            Ok(option)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WildcardSpend {
    BonusReroll,
    SelectAny,
    AutoReroll,
    Lock,
}

/// Pure before/after plan. A real debit still needs the Account ledger and receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WildcardDebit {
    pub before: u8,
    pub cost: u8,
    pub after: u8,
}

impl TryFrom<(u8, WildcardSpend)> for WildcardDebit {
    type Error = PreyRefusal;

    fn try_from((before, spend): (u8, WildcardSpend)) -> Result<Self, Self::Error> {
        if before > WILDCARD_CAP {
            return Err(PreyRefusal::InvalidBalance);
        }
        let cost = match spend {
            WildcardSpend::BonusReroll | WildcardSpend::AutoReroll => 1,
            WildcardSpend::SelectAny | WildcardSpend::Lock => 5,
        };
        let after = before
            .checked_sub(cost)
            .ok_or(PreyRefusal::NotEnoughWildcards)?;
        Ok(Self {
            before,
            cost,
            after,
        })
    }
}

/// Trusted server seconds, not client time. Paid rerolls preserve the free deadline.
#[derive(Debug, Clone, Copy)]
pub struct ListReroll {
    pub now: u64,
    pub free_reroll_at: u64,
    pub level: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RerollQuote {
    pub gold: u64,
    pub free_reroll_at: u64,
}

impl TryFrom<ListReroll> for RerollQuote {
    type Error = PreyRefusal;

    fn try_from(input: ListReroll) -> Result<Self, Self::Error> {
        if input.level == 0 {
            return Err(PreyRefusal::InvalidLevel);
        }
        // Match PostgreSQL's nonnegative BIGINT timestamps, including the successor.
        let last_timestamp = i64::MAX.unsigned_abs();
        if input.now > last_timestamp || input.free_reroll_at > last_timestamp {
            return Err(PreyRefusal::InvalidTime);
        }
        if input.now >= input.free_reroll_at {
            Ok(Self {
                gold: 0,
                free_reroll_at: input
                    .now
                    .checked_add(FREE_REROLL_SECONDS)
                    .filter(|time| *time <= last_timestamp)
                    .ok_or(PreyRefusal::InvalidTime)?,
            })
        } else {
            Ok(Self {
                gold: u64::from(input.level) * GOLD_PER_LEVEL,
                free_reroll_at: input.free_reroll_at,
            })
        }
    }
}

#[cfg(test)]
#[path = "prey_tests.rs"]
mod tests;
