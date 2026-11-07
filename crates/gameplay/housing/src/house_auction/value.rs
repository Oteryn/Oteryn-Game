//! Account balance and already debited bid custody; no SQL or runtime authority.

use super::AuctionError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i16)]
pub enum LedgerKind {
    Reserve = 7,
    Release = 8,
    Price = 9,
    Rent = 10,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValueLine {
    pub kind: LedgerKind,
    pub amount: u64,
    pub balance_before: u64,
    pub balance_after: u64,
    pub escrow_before: u64,
    pub escrow_after: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValuePlan {
    pub balance: u64,
    pub escrow: u64,
    pub lines: Vec<ValueLine>,
}

fn initial(balance: u64, escrow: u64, cap: u64) -> Result<ValuePlan, AuctionError> {
    if u128::from(balance) + u128::from(escrow) > u128::from(cap) {
        return Err(AuctionError::Overflow);
    }
    Ok(ValuePlan {
        balance,
        escrow,
        lines: Vec::new(),
    })
}

fn movement(plan: &mut ValuePlan, kind: LedgerKind, amount: u64) -> Result<(), AuctionError> {
    if amount == 0 {
        return Ok(());
    }
    let before = (plan.balance, plan.escrow);
    match kind {
        LedgerKind::Reserve => {
            plan.balance = plan
                .balance
                .checked_sub(amount)
                .ok_or(AuctionError::Unfunded)?;
            plan.escrow = plan
                .escrow
                .checked_add(amount)
                .ok_or(AuctionError::Overflow)?;
        }
        LedgerKind::Release => {
            plan.escrow = plan
                .escrow
                .checked_sub(amount)
                .ok_or(AuctionError::Unfunded)?;
            plan.balance = plan
                .balance
                .checked_add(amount)
                .ok_or(AuctionError::Overflow)?;
        }
        LedgerKind::Price | LedgerKind::Rent => {
            plan.escrow = plan
                .escrow
                .checked_sub(amount)
                .ok_or(AuctionError::Unfunded)?;
        }
    }
    plan.lines.push(ValueLine {
        kind,
        amount,
        balance_before: before.0,
        balance_after: plan.balance,
        escrow_before: before.1,
        escrow_after: plan.escrow,
    });
    Ok(())
}

/// A bid edit moves only the difference. Repeating a maximum produces no ledger line.
pub fn plan_bid_value(
    balance: u64,
    escrow: u64,
    maximum: u64,
    first_rent: u64,
    cap: u64,
) -> Result<ValuePlan, AuctionError> {
    let mut plan = initial(balance, escrow, cap)?;
    let wanted = maximum
        .checked_add(first_rent)
        .filter(|n| *n <= cap)
        .ok_or(AuctionError::Overflow)?;
    if wanted > escrow {
        movement(&mut plan, LedgerKind::Reserve, wanted - escrow)?;
    } else {
        movement(&mut plan, LedgerKind::Release, escrow - wanted)?;
    }
    Ok(plan)
}

pub fn plan_release_value(balance: u64, escrow: u64, cap: u64) -> Result<ValuePlan, AuctionError> {
    let mut plan = initial(balance, escrow, cap)?;
    movement(&mut plan, LedgerKind::Release, escrow)?;
    Ok(plan)
}

/// Price and first rent are burned from custody, without charging the balance again.
pub fn plan_winner_value(
    balance: u64,
    escrow: u64,
    maximum: u64,
    price: u64,
    first_rent: u64,
    cap: u64,
) -> Result<ValuePlan, AuctionError> {
    let mut plan = initial(balance, escrow, cap)?;
    if maximum < price || maximum.checked_add(first_rent) != Some(escrow) {
        return Err(AuctionError::InvalidInput);
    }
    movement(&mut plan, LedgerKind::Release, maximum - price)?;
    movement(&mut plan, LedgerKind::Price, price)?;
    movement(&mut plan, LedgerKind::Rent, first_rent)?;
    Ok(plan)
}
