//! Forge dust (IMBUE-FORGE-0 §9, FORGE-1a): a per-Character balance from 0 to the dust limit,
//! and the D1230 limit `100 + 20*n` for `n` in 0..=225. The general next cost `25+n`
//! and maximum are PARITY_WIKI; the base and increment are GUI/CIPSOFT evidence.
//! Gain and Spend preserve the unlocked limit. LimitRaise is a checked value plan;
//! only its native receipt/ledger owner may commit it.

use std::error::Error;
use std::fmt::{self, Display, Formatter};

/// `IMBFORGE0-RL-08`: the initial and lowest dust limit.
pub const DUST_LIMIT_MIN: u16 = 100;
/// `IMBFORGE0-RL-08`: the highest dust limit.
pub const DUST_LIMIT_MAX: u16 = 4_600;
pub const DUST_LIMIT_UPGRADES_MAX: u16 = 225;
const DUST_LIMIT_STEP: u16 = 20;
const DUST_LIMIT_FIRST_COST: u32 = 25;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForgeDustError {
    /// The limit is outside the exact D1230 progression or its count is invalid.
    InvalidLimit,
    MaximumLimitReached,
    /// The balance is above the limit.
    BalanceAboveLimit,
    /// A gain or spend of zero.
    ZeroAmount,
    /// A spend above the balance.
    InsufficientDust,
}

impl Display for ForgeDustError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidLimit => "forge dust limit must be an exact D1230 stage from 100 to 4600",
            Self::MaximumLimitReached => "forge dust limit is already at its maximum",
            Self::BalanceAboveLimit => "forge dust balance must not exceed its limit",
            Self::ZeroAmount => "forge dust amount must be positive",
            Self::InsufficientDust => "forge dust balance is below the amount",
        })
    }
}

impl Error for ForgeDustError {}

/// A dust limit within `IMBFORGE0-RL-08`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct DustLimit(u16);

impl DustLimit {
    pub const INITIAL: Self = Self(DUST_LIMIT_MIN);

    pub fn new(value: u16) -> Result<Self, ForgeDustError> {
        if (DUST_LIMIT_MIN..=DUST_LIMIT_MAX).contains(&value)
            && (value - DUST_LIMIT_MIN).is_multiple_of(DUST_LIMIT_STEP)
        {
            Ok(Self(value))
        } else {
            Err(ForgeDustError::InvalidLimit)
        }
    }

    pub fn get(self) -> u16 {
        self.0
    }

    pub fn from_upgrade_count(upgrades: u16) -> Result<Self, ForgeDustError> {
        if upgrades > DUST_LIMIT_UPGRADES_MAX {
            return Err(ForgeDustError::InvalidLimit);
        }
        let limit = upgrades
            .checked_mul(DUST_LIMIT_STEP)
            .and_then(|increase| DUST_LIMIT_MIN.checked_add(increase))
            .ok_or(ForgeDustError::InvalidLimit)?;
        Self::new(limit)
    }

    /// Only a validated D1230 value has a count. Historical persisted rows must
    /// first pass the migration's complete legacy-history check; never round them.
    pub fn upgrade_count(self) -> u16 {
        (self.0 - DUST_LIMIT_MIN) / DUST_LIMIT_STEP
    }

    pub fn next_upgrade(self) -> Result<(u32, Self), ForgeDustError> {
        let count = self.upgrade_count();
        if count == DUST_LIMIT_UPGRADES_MAX {
            return Err(ForgeDustError::MaximumLimitReached);
        }
        let next = count.checked_add(1).ok_or(ForgeDustError::InvalidLimit)?;
        let cost = DUST_LIMIT_FIRST_COST
            .checked_add(u32::from(count))
            .ok_or(ForgeDustError::InvalidLimit)?;
        Ok((cost, Self::from_upgrade_count(next)?))
    }
}

/// A Character's dust balance and limit; the balance never exceeds the limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ForgeDustBalance {
    balance: u16,
    limit: DustLimit,
}

impl Default for ForgeDustBalance {
    /// No balance row: balance 0 at the initial limit.
    fn default() -> Self {
        Self {
            balance: 0,
            limit: DustLimit::INITIAL,
        }
    }
}

impl ForgeDustBalance {
    pub fn new(balance: u16, limit: DustLimit) -> Result<Self, ForgeDustError> {
        if balance > limit.get() {
            return Err(ForgeDustError::BalanceAboveLimit);
        }
        Ok(Self { balance, limit })
    }

    pub fn balance(self) -> u16 {
        self.balance
    }

    pub fn limit(self) -> DustLimit {
        self.limit
    }

    /// Credit `amount` up to the limit; the rest is lost.
    pub fn gain(self, amount: u32) -> Result<DustGain, ForgeDustError> {
        if amount == 0 {
            return Err(ForgeDustError::ZeroAmount);
        }
        let room = self.limit.get() - self.balance;
        let credited = u16::try_from(amount).map_or(room, |amount| amount.min(room));
        let after = Self {
            balance: self.balance + credited,
            limit: self.limit,
        };
        Ok(DustGain {
            before: self,
            after,
            amount,
            lost: amount - u32::from(credited),
        })
    }

    /// Debit `amount`, refused when it is above the balance.
    pub fn spend(self, amount: u32) -> Result<DustSpend, ForgeDustError> {
        if amount == 0 {
            return Err(ForgeDustError::ZeroAmount);
        }
        let amount_u16 = u16::try_from(amount).map_err(|_| ForgeDustError::InsufficientDust)?;
        let balance = self
            .balance
            .checked_sub(amount_u16)
            .ok_or(ForgeDustError::InsufficientDust)?;
        Ok(DustSpend {
            before: self,
            after: Self {
                balance,
                limit: self.limit,
            },
            amount,
        })
    }

    /// Quote one indivisible debit and +20 limit transition. The caller cannot
    /// supply a cost, resulting limit, or a fabricated earlier unlock count.
    pub fn raise_limit(self) -> Result<DustLimitRaise, ForgeDustError> {
        let (amount, limit) = self.limit.next_upgrade()?;
        let spent = self.spend(amount)?;
        Ok(DustLimitRaise {
            before: self,
            after: Self {
                balance: spent.after.balance,
                limit,
            },
            amount,
        })
    }
}

/// One planned gain: `amount` requested, `amount - lost` credited.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DustGain {
    pub before: ForgeDustBalance,
    pub after: ForgeDustBalance,
    pub amount: u32,
    pub lost: u32,
}

/// One planned spend of `amount`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DustSpend {
    pub before: ForgeDustBalance,
    pub after: ForgeDustBalance,
    pub amount: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DustLimitRaise {
    pub before: ForgeDustBalance,
    pub after: ForgeDustBalance,
    pub amount: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limit_accepts_only_exact_d1230_stages() {
        assert_eq!(DustLimit::new(99), Err(ForgeDustError::InvalidLimit));
        assert_eq!(DustLimit::new(100).map(DustLimit::get), Ok(100));
        assert_eq!(DustLimit::new(4_600).map(DustLimit::get), Ok(4_600));
        assert_eq!(DustLimit::new(101), Err(ForgeDustError::InvalidLimit));
        assert_eq!(DustLimit::new(225), Err(ForgeDustError::InvalidLimit));
        assert_eq!(DustLimit::new(226), Err(ForgeDustError::InvalidLimit));
        assert_eq!(DustLimit::new(4_601), Err(ForgeDustError::InvalidLimit));
        assert_eq!(DustLimit::INITIAL.get(), 100);
    }

    #[test]
    fn balance_stays_within_limit() {
        let limit = DustLimit::INITIAL;
        assert!(ForgeDustBalance::new(100, limit).is_ok());
        assert_eq!(
            ForgeDustBalance::new(101, limit),
            Err(ForgeDustError::BalanceAboveLimit)
        );
        assert_eq!(ForgeDustBalance::default().balance(), 0);
    }

    #[test]
    fn gain_above_limit_credits_to_limit_and_loses_the_rest() -> Result<(), ForgeDustError> {
        let gain = ForgeDustBalance::new(90, DustLimit::INITIAL)?.gain(25)?;
        assert_eq!(gain.after.balance(), 100);
        assert_eq!(gain.lost, 15);
        let full = gain.after.gain(u32::MAX)?;
        assert_eq!((full.after.balance(), full.lost), (100, u32::MAX));
        let exact = ForgeDustBalance::default().gain(100)?;
        assert_eq!((exact.after.balance(), exact.lost), (100, 0));
        assert_eq!(
            ForgeDustBalance::default().gain(0),
            Err(ForgeDustError::ZeroAmount)
        );
        Ok(())
    }

    #[test]
    fn spend_above_balance_is_refused() -> Result<(), ForgeDustError> {
        let balance = ForgeDustBalance::new(40, DustLimit::new(220)?)?;
        assert_eq!(balance.spend(40)?.after.balance(), 0);
        assert_eq!(balance.spend(41), Err(ForgeDustError::InsufficientDust));
        assert_eq!(
            balance.spend(u32::MAX),
            Err(ForgeDustError::InsufficientDust)
        );
        assert_eq!(balance.spend(0), Err(ForgeDustError::ZeroAmount));
        Ok(())
    }
}
