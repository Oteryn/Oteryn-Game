//! GUILD0 §6.2 arithmetic used by the native SQL custody participant.
//! Inputs are locked balances/custody, not current authority or an auction decision.

/// BANK0-RL-01 and HOUSEOWN0-RL-13. No new balance ceiling.
pub const GUILD_BALANCE_MAX: u64 = 999_999_999_999;
pub const ACCOUNT_BALANCE_MAX: u64 = 9_000_000_000_000_000;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EscrowParts {
    pub guild: u64,
    pub account: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SplitError {
    InvalidStoredValue,
    InsufficientBalance,
    BalanceLimit,
    NoChange,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EscrowChange {
    pub before: EscrowParts,
    pub after: EscrowParts,
    pub guild_balance_before: u64,
    pub guild_balance_after: u64,
    pub account_balance_before: u64,
    pub account_balance_after: u64,
}
impl EscrowParts {
    pub fn total(self) -> Result<u64, SplitError> {
        if self.guild > GUILD_BALANCE_MAX || self.account > ACCOUNT_BALANCE_MAX {
            return Err(SplitError::InvalidStoredValue);
        }
        self.guild
            .checked_add(self.account)
            .ok_or(SplitError::InvalidStoredValue)
    }
    /// Release original custody, including an already-empty HELD bid.
    pub fn release(
        self,
        guild_balance: u64,
        account_balance: u64,
    ) -> Result<EscrowChange, SplitError> {
        match self.resize(0, guild_balance, account_balance) {
            Err(SplitError::NoChange) => Ok(EscrowChange {
                before: self,
                after: self,
                guild_balance_before: guild_balance,
                guild_balance_after: guild_balance,
                account_balance_before: account_balance,
                account_balance_after: account_balance,
            }),
            result => result,
        }
    }
    /// Grow guild-first; lower Account-first; `total=0` returns each original part.
    /// A caller separately decides whether a lowering/release is an admitted House operation.
    pub fn resize(
        self,
        total: u64,
        guild_balance: u64,
        account_balance: u64,
    ) -> Result<EscrowChange, SplitError> {
        let old = self.total()?;
        if guild_balance > GUILD_BALANCE_MAX || account_balance > ACCOUNT_BALANCE_MAX {
            return Err(SplitError::InvalidStoredValue);
        }
        if guild_balance
            .checked_add(self.guild)
            .is_none_or(|n| n > GUILD_BALANCE_MAX)
            || account_balance
                .checked_add(self.account)
                .is_none_or(|n| n > ACCOUNT_BALANCE_MAX)
        {
            return Err(SplitError::BalanceLimit);
        }
        let mut after = self;
        let (next_guild, next_account) = if total > old {
            let delta = total - old;
            let from_guild = delta.min(guild_balance);
            let from_account = delta - from_guild;
            if from_account > account_balance {
                return Err(SplitError::InsufficientBalance);
            }
            after.guild = self
                .guild
                .checked_add(from_guild)
                .ok_or(SplitError::BalanceLimit)?;
            after.account = self
                .account
                .checked_add(from_account)
                .ok_or(SplitError::BalanceLimit)?;
            (guild_balance - from_guild, account_balance - from_account)
        } else if total < old {
            let delta = old - total;
            let to_account = delta.min(self.account);
            let to_guild = delta - to_account;
            after.account -= to_account;
            after.guild -= to_guild;
            (guild_balance + to_guild, account_balance + to_account)
        } else {
            return Err(SplitError::NoChange);
        };
        after.total()?;
        Ok(EscrowChange {
            before: self,
            after,
            guild_balance_before: guild_balance,
            guild_balance_after: next_guild,
            account_balance_before: account_balance,
            account_balance_after: next_account,
        })
    }
}

impl std::fmt::Display for SplitError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "guildhall split: {self:?}")
    }
}
impl std::error::Error for SplitError {}
