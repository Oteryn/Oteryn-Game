//! MARKET0 §§4/6/7: ITEM offer state and noncommitting economic plans.
//! Definition references identify stored wares; they do not admit marketability or compatibility.
#![allow(
    dead_code,
    reason = "private Market owner awaits atomic CP integration"
)]

use super::{AccountId, CharacterId, DomainError, WorldId, validate_uuid_v7};
use crate::content::{DefinitionFamily, ProductionKey, TypedDefinitionRef};

pub(crate) const MAX_TOTAL: u64 = 999_999_999_999;
pub(crate) const HARD_CREDIT_CEILING: u64 = 9_000_000_000_000_000;
pub(crate) const LIFETIME_US: i64 = 30 * 24 * 60 * 60 * 1_000_000;

/// The accepted Game MarketOfferId, private until its owning registration slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct MarketOfferId([u8; 16]);
impl MarketOfferId {
    pub(crate) fn new(bytes: [u8; 16]) -> Result<Self, DomainError> {
        validate_uuid_v7(&bytes)?;
        Ok(Self(bytes))
    }
    pub(crate) const fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BookKey {
    pub(crate) world: WorldId,
    pub(crate) definition: ProductionKey,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Side {
    Buy,
    Sell,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EndState {
    Filled,
    Cancelled,
    Expired,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OfferState {
    Open,
    Ended(EndState),
    CreditHeld(EndState),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Offer {
    pub(crate) id: MarketOfferId,
    pub(crate) world: WorldId,
    pub(crate) definition: TypedDefinitionRef,
    pub(crate) side: Side,
    pub(crate) account: AccountId,
    pub(crate) character: CharacterId,
    pub(crate) piece_price: u64,
    pub(crate) amount: u32,
    pub(crate) remaining: u32,
    pub(crate) anonymous: bool,
    pub(crate) created_at_us: i64,
    pub(crate) expires_at_us: i64,
    pub(crate) state: OfferState,
    pub(crate) escrow_gold: u64,
    pub(crate) held_credit_gold: u64,
    pub(crate) matching_pending: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlanError {
    InvalidState,
    NotOpen,
    NotExpired,
    SameAccount,
    NotCrossed,
    InvalidAmount,
}

pub(crate) fn offer_total(amount: u32, price: u64) -> Result<u64, PlanError> {
    let total = u128::from(amount)
        .checked_mul(u128::from(price))
        .ok_or(PlanError::InvalidAmount)?;
    if !(1..=64_000).contains(&amount) || price == 0 || total > u128::from(MAX_TOTAL) {
        return Err(PlanError::InvalidAmount);
    }
    u64::try_from(total).map_err(|_| PlanError::InvalidAmount)
}
pub(crate) fn placement_fee(amount: u32, price: u64) -> Result<u64, PlanError> {
    Ok((offer_total(amount, price)? / 50).clamp(20, 1_000_000))
}
impl Offer {
    pub(crate) fn book(&self) -> BookKey {
        BookKey {
            world: self.world,
            definition: self.definition.key().clone(),
        }
    }
    pub(crate) fn validate(&self) -> Result<(), PlanError> {
        offer_total(self.amount, self.piece_price)?;
        if self.definition.family() != DefinitionFamily::Item
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
            OfferState::CreditHeld(end) => {
                if self.held_credit_gold == 0
                    || self.matching_pending
                    || (end == EndState::Filled && self.remaining != 0)
                {
                    return Err(PlanError::InvalidState);
                }
            }
            OfferState::Ended(end) => {
                if self.held_credit_gold != 0
                    || self.matching_pending
                    || (end == EndState::Filled && self.remaining != 0)
                {
                    return Err(PlanError::InvalidState);
                }
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
pub(crate) struct MatchTerms {
    pub(crate) maker: MarketOfferId,
    pub(crate) buyer: MarketOfferId,
    pub(crate) seller: MarketOfferId,
    pub(crate) amount: u32,
    pub(crate) piece_price: u64,
    pub(crate) sale_gold: u64,
    pub(crate) buy_escrow_debit: u64,
    pub(crate) buyer_refund_gold: u64,
}
pub(crate) fn match_terms(
    a: &Offer,
    b: &Offer,
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
pub(crate) enum EndReason {
    Cancel,
    Expire,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EndPlan {
    pub(crate) offer: MarketOfferId,
    pub(crate) ended_as: EndState,
    pub(crate) return_gold: u64,
    pub(crate) return_item_units: u32,
}
pub(crate) fn end_plan(
    offer: &Offer,
    reason: EndReason,
    now_us: i64,
) -> Result<EndPlan, PlanError> {
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
pub(crate) fn held_credit_release(
    offer: &Offer,
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
