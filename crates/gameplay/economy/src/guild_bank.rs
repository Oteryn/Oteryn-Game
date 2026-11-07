//! GUILD0 §5: transfers conserve owned value; disband custody never burns overflow.
use crate::bank::StarterIslandFact;

pub const GUILD_BALANCE_MAX: u64 = 999_999_999_999;
pub const ACCOUNT_HARD_MAX: u64 = 9_000_000_000_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferDirection {
    Deposit,
    Withdraw,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    NotAllowed,
    Junior,
    InsufficientBalance,
    BalanceLimit,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransferPlan {
    pub bank_after: u64,
    pub guild_after: u64,
}

/// Current rank and Junior status are independently loaded by the fenced writer.
pub fn plan_transfer(
    direction: TransferDirection,
    rank: u8,
    junior: StarterIslandFact,
    bank: u64,
    guild: u64,
    amount: u64,
    credit_headroom: u64,
) -> Result<TransferPlan, Refusal> {
    if !(1..=20).contains(&rank)
        || (direction == TransferDirection::Withdraw && rank > 2)
        || amount == 0
        || amount > GUILD_BALANCE_MAX
        || bank > ACCOUNT_HARD_MAX
        || guild > GUILD_BALANCE_MAX
    {
        return Err(Refusal::NotAllowed);
    }
    if crate::bank::is_junior(junior) {
        return Err(Refusal::Junior);
    }
    let (source, target) = match direction {
        TransferDirection::Deposit => (bank, guild),
        TransferDirection::Withdraw => (guild, bank),
    };
    let source = source
        .checked_sub(amount)
        .ok_or(Refusal::InsufficientBalance)?;
    let target = target
        .checked_add(amount)
        .filter(|v| *v <= GUILD_BALANCE_MAX && amount <= credit_headroom)
        .ok_or(Refusal::BalanceLimit)?;
    Ok(match direction {
        TransferDirection::Deposit => TransferPlan {
            bank_after: source,
            guild_after: target,
        },
        TransferDirection::Withdraw => TransferPlan {
            bank_after: target,
            guild_after: source,
        },
    })
}

/// Headroom already includes every currently held escrow, under the Account balance lock.
pub fn split_payout(amount: u64, bank: u64, headroom: u64) -> Option<(u64, u64)> {
    if amount > GUILD_BALANCE_MAX
        || bank > ACCOUNT_HARD_MAX
        || headroom > ACCOUNT_HARD_MAX.checked_sub(bank)?
    {
        return None;
    }
    let credit = amount.min(headroom);
    Some((credit, amount - credit))
}

#[cfg(test)]
#[path = "guild_bank_tests.rs"]
mod tests;
