//! Pure Market offer validation, fees, matching, ending and held-credit rules.

use oteryn_gameplay_domain::domain::item_definition::TypedDefinitionRef;
use oteryn_gameplay_domain::domain::{AccountId, CharacterId, WorldId};

pub fn has_nonproduction_marker(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    let has_test_segment = lower
        .split([':', '.', '/', '_', '-'])
        .any(|segment| segment == "test");
    lower.contains("fixture")
        || lower.contains("synthetic")
        || lower.contains("evidence")
        || lower.contains("test-only")
        || has_test_segment
}
pub fn valid_key_bytes(value: &str) -> bool {
    !value.is_empty()
        && value.is_ascii()
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b':' | b'.' | b'_' | b'-' | b'/')
        })
}

pub const MAX_TOTAL: u64 = 999_999_999_999;
pub const HARD_CREDIT_CEILING: u64 = 9_000_000_000_000_000;
pub const LIFETIME_US: i64 = 30 * 24 * 60 * 60 * 1_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Buy,
    Sell,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndState {
    Filled,
    Cancelled,
    Expired,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OfferState {
    Open,
    Ended(EndState),
    CreditHeld(EndState),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Offer<'a> {
    pub id: [u8; 16],
    pub world: WorldId,
    pub definition: &'a TypedDefinitionRef,
    pub side: Side,
    pub account: AccountId,
    pub character: CharacterId,
    pub piece_price: u64,
    pub amount: u32,
    pub remaining: u32,
    pub anonymous: bool,
    pub created_at_us: i64,
    pub expires_at_us: i64,
    pub state: OfferState,
    pub escrow_gold: u64,
    pub held_credit_gold: u64,
    pub matching_pending: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanError {
    InvalidState,
    NotOpen,
    NotExpired,
    SameAccount,
    NotCrossed,
    InvalidAmount,
}

pub fn offer_total(amount: u32, price: u64) -> Result<u64, PlanError> {
    let total = u128::from(amount)
        .checked_mul(u128::from(price))
        .ok_or(PlanError::InvalidAmount)?;
    if !(1..=64_000).contains(&amount) || price == 0 || total > u128::from(MAX_TOTAL) {
        return Err(PlanError::InvalidAmount);
    }
    u64::try_from(total).map_err(|_| PlanError::InvalidAmount)
}
pub fn placement_fee(amount: u32, price: u64) -> Result<u64, PlanError> {
    Ok((offer_total(amount, price)? / 50).clamp(20, 1_000_000))
}
impl Offer<'_> {
    fn book(&self) -> (WorldId, &str) {
        (self.world, &self.definition.production_key)
    }
    pub fn validate(&self) -> Result<(), PlanError> {
        offer_total(self.amount, self.piece_price)?;
        if self.definition.family.as_str() != "Item"
            || self.remaining > self.amount
            || self.created_at_us < 0
            || self.created_at_us.checked_add(LIFETIME_US) != Some(self.expires_at_us)
        {
            return Err(PlanError::InvalidState);
        }
        let expected_escrow = if self.side == Side::Buy && self.state == OfferState::Open {
            u64::from(self.remaining)
                .checked_mul(self.piece_price)
                .ok_or(PlanError::InvalidState)?
        } else {
            0
        };
        if self.escrow_gold != expected_escrow {
            return Err(PlanError::InvalidState);
        }
        match self.state {
            OfferState::Open if self.remaining == 0 || self.held_credit_gold != 0 => {
                return Err(PlanError::InvalidState);
            }
            OfferState::CreditHeld(end)
                if self.held_credit_gold == 0
                    || self.matching_pending
                    || (end == EndState::Filled && self.remaining != 0) =>
            {
                return Err(PlanError::InvalidState);
            }
            OfferState::Ended(end)
                if self.held_credit_gold != 0
                    || self.matching_pending
                    || (end == EndState::Filled && self.remaining != 0) =>
            {
                return Err(PlanError::InvalidState);
            }
            _ => {}
        }
        Ok(())
    }
    fn open_at(&self, now_us: i64) -> Result<(), PlanError> {
        self.validate()?;
        if self.state != OfferState::Open
            || now_us < self.created_at_us
            || now_us >= self.expires_at_us
        {
            return Err(PlanError::NotOpen);
        }
        Ok(())
    }
}

/// Economic terms only. Current admitted definition compatibility, custody selection (at
/// most 100 touched items), Inbox reservations and all bank effects remain the atomic writer's
/// job. This plan neither transfers value nor authorizes an execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchTerms {
    pub maker: [u8; 16],
    pub buyer: [u8; 16],
    pub seller: [u8; 16],
    pub amount: u32,
    pub piece_price: u64,
    pub sale_gold: u64,
    pub buy_escrow_debit: u64,
    pub buyer_refund_gold: u64,
}
pub fn match_terms(
    a: &Offer<'_>,
    b: &Offer<'_>,
    amount: u32,
    now_us: i64,
) -> Result<MatchTerms, PlanError> {
    a.open_at(now_us)?;
    b.open_at(now_us)?;
    if a.id == b.id || a.book() != b.book() || a.side == b.side {
        return Err(PlanError::InvalidState);
    }
    if a.account == b.account {
        return Err(PlanError::SameAccount);
    }
    let (buy, sell) = if a.side == Side::Buy { (a, b) } else { (b, a) };
    if buy.piece_price < sell.piece_price {
        return Err(PlanError::NotCrossed);
    }
    if amount == 0 || amount > buy.remaining || amount > sell.remaining {
        return Err(PlanError::InvalidAmount);
    }
    let maker = if (a.created_at_us, a.id) < (b.created_at_us, b.id) {
        a
    } else {
        b
    };
    let sale_gold = offer_total(amount, maker.piece_price)?;
    let buy_escrow_debit = offer_total(amount, buy.piece_price)?;
    Ok(MatchTerms {
        maker: maker.id,
        buyer: buy.id,
        seller: sell.id,
        amount,
        piece_price: maker.piece_price,
        sale_gold,
        buy_escrow_debit,
        buyer_refund_gold: buy_escrow_debit
            .checked_sub(sale_gold)
            .ok_or(PlanError::InvalidState)?,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndReason {
    Cancel,
    Expire,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndPlan {
    pub offer: [u8; 16],
    pub ended_as: EndState,
    pub return_gold: u64,
    pub return_item_units: u32,
}
pub fn end_plan(offer: &Offer<'_>, reason: EndReason, now_us: i64) -> Result<EndPlan, PlanError> {
    offer.validate()?;
    if offer.state != OfferState::Open || now_us < offer.created_at_us {
        return Err(PlanError::NotOpen);
    }
    if reason == EndReason::Expire && now_us < offer.expires_at_us {
        return Err(PlanError::NotExpired);
    }
    Ok(EndPlan {
        offer: offer.id,
        ended_as: if reason == EndReason::Cancel {
            EndState::Cancelled
        } else {
            EndState::Expired
        },
        return_gold: offer.escrow_gold,
        return_item_units: if offer.side == Side::Sell {
            offer.remaining
        } else {
            0
        },
    })
}

/// An owner's credit that does not fit remains explicit custody, never a refused/lost credit.
/// The caller supplies the independently locked current balance; no balance write occurs here.
pub fn held_credit_release(
    offer: &Offer<'_>,
    locked_balance: u64,
) -> Result<Option<u64>, PlanError> {
    offer.validate()?;
    if !matches!(offer.state, OfferState::CreditHeld(_)) || locked_balance > HARD_CREDIT_CEILING {
        return Err(PlanError::InvalidState);
    }
    Ok(locked_balance
        .checked_add(offer.held_credit_gold)
        .filter(|value| *value <= HARD_CREDIT_CEILING)
        .map(|_| offer.held_credit_gold))
}

#[cfg(test)]
#[path = "market_tests.rs"]
mod tests;
