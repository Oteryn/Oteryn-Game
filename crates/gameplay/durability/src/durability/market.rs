//! MARKET0 §§4/6/7: native private storage reads and transaction-bound book selection.
//!
//! Migration 0155 installs the dormant offer book. The missing Market ledger owner,
//! reserved Inbox delivery and conservation/audit arms must commit with the value writer. A locked book or
//! historical outcome is not actor authority. The eventual owner must acquire real recovery,
//! admission and (for commands) current session/root fences BEFORE this book boundary.
#![allow(
    dead_code,
    reason = "Market atomic writer awaits content, fenced custody and audit integration"
)]

use oteryn_gameplay_economy::market as rules;
pub use oteryn_gameplay_economy::market::{
    EndReason, EndState, HARD_CREDIT_CEILING, LIFETIME_US, MAX_TOTAL, MAX_TOUCHED_ITEMS, MAX_OPEN_OFFERS, OfferState, PlanError, Side,
    has_nonproduction_marker, offer_total, placement_fee, valid_key_bytes,
};

use super::DurabilityError;
use super::character_progression::{numeric_u64, uuid_text};
use crate::domain::{AccountId, CharacterId, ItemInstanceId, WorldId};
use crate::foundation::CommandRef;
use sqlx::{PgConnection, Postgres, Row, Transaction};

type Result<T, E = DurabilityError> = std::result::Result<T, E>;
fn invalid() -> DurabilityError {
    DurabilityError::InvalidStoredState
}
/// Storage key with the exact existing ProductionKey lexical restrictions. This validates
/// persisted identity only; neither its syntax nor its constructor admits marketability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarketDefinitionKey(String);
impl MarketDefinitionKey {
    pub fn new(value: &str) -> Result<Self> {
        super::item_mint_audit::check_content_key(value).map_err(|_| invalid())?;
        let namespaced = value
            .split_once(':')
            .is_some_and(|(namespace, local)| !namespace.is_empty() && !local.is_empty());
        if !namespaced || !valid_key_bytes(value) || has_nonproduction_marker(value) {
            return Err(invalid());
        }
        Ok(Self(value.to_owned()))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Accepted durable definition ABI, privately checked before use as a Market ITEM ware.
/// No unchecked native string DTO or Content admission fact escapes through this wrapper.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredItemDefinition(super::item_mint::TypedDefinitionRef);
impl StoredItemDefinition {
    pub fn new(value: super::item_mint::TypedDefinitionRef) -> Result<Self> {
        if value.family != "Item" {
            return Err(invalid());
        }
        MarketDefinitionKey::new(&value.production_key)?;
        super::item_mint_audit::check_content_key(&value.revision_ref).map_err(|_| invalid())?;
        if !value.revision_ref.is_ascii()
            || !value
                .revision_ref
                .bytes()
                .all(|byte| byte.is_ascii_graphic())
            || has_nonproduction_marker(&value.revision_ref)
        {
            return Err(invalid());
        }
        Ok(Self(value))
    }
    pub fn family(&self) -> &str {
        &self.0.family
    }
    pub fn key(&self) -> &str {
        &self.0.production_key
    }
    pub fn revision(&self) -> &str {
        &self.0.revision_ref
    }
}

/// The accepted Game MarketOfferId, private until its owning registration slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MarketOfferId([u8; 16]);
impl MarketOfferId {
    pub fn new(bytes: [u8; 16]) -> Result<Self> {
        super::item_mint_audit::check_uuid_v7(&bytes).map_err(|_| invalid())?;
        Ok(Self(bytes))
    }
    pub const fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BookKey {
    pub world: WorldId,
    pub definition: MarketDefinitionKey,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Offer {
    pub id: MarketOfferId,
    pub world: WorldId,
    pub definition: StoredItemDefinition,
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
impl Offer {
    pub fn book(&self) -> BookKey {
        BookKey {
            world: self.world,
            definition: MarketDefinitionKey(self.definition.key().to_owned()),
        }
    }
    fn rules(&self) -> rules::Offer<'_> {
        rules::Offer {
            id: *self.id.as_bytes(),
            world: self.world,
            definition: &self.definition.0,
            side: self.side,
            account: self.account,
            character: self.character,
            piece_price: self.piece_price,
            amount: self.amount,
            remaining: self.remaining,
            anonymous: self.anonymous,
            created_at_us: self.created_at_us,
            expires_at_us: self.expires_at_us,
            state: self.state,
            escrow_gold: self.escrow_gold,
            held_credit_gold: self.held_credit_gold,
            matching_pending: self.matching_pending,
        }
    }
    pub fn validate(&self) -> Result<(), PlanError> {
        self.rules().validate()
    }
}

/// Economic terms only. Current admitted definition compatibility, custody selection (at
/// most 100 touched items), Inbox reservations and all bank effects remain the atomic writer's
/// job. This plan neither transfers value nor authorizes an execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchTerms {
    pub maker: MarketOfferId,
    pub buyer: MarketOfferId,
    pub seller: MarketOfferId,
    pub amount: u32,
    pub piece_price: u64,
    pub sale_gold: u64,
    pub buy_escrow_debit: u64,
    pub buyer_refund_gold: u64,
}
pub fn match_terms(
    a: &Offer,
    b: &Offer,
    amount: u32,
    now_us: i64,
) -> Result<MatchTerms, PlanError> {
    let terms = rules::match_terms(&a.rules(), &b.rules(), amount, now_us)?;
    Ok(MatchTerms {
        maker: MarketOfferId(terms.maker),
        buyer: MarketOfferId(terms.buyer),
        seller: MarketOfferId(terms.seller),
        amount: terms.amount,
        piece_price: terms.piece_price,
        sale_gold: terms.sale_gold,
        buy_escrow_debit: terms.buy_escrow_debit,
        buyer_refund_gold: terms.buyer_refund_gold,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndPlan {
    pub offer: MarketOfferId,
    pub ended_as: EndState,
    pub return_gold: u64,
    pub return_item_units: u32,
}
pub fn end_plan(offer: &Offer, reason: EndReason, now_us: i64) -> Result<EndPlan, PlanError> {
    let plan = rules::end_plan(&offer.rules(), reason, now_us)?;
    Ok(EndPlan {
        offer: MarketOfferId(plan.offer),
        ended_as: plan.ended_as,
        return_gold: plan.return_gold,
        return_item_units: plan.return_item_units,
    })
}

/// An owner's credit that does not fit remains explicit custody, never a refused/lost credit.
/// The caller supplies the independently locked current balance; no balance write occurs here.
pub fn held_credit_release(offer: &Offer, locked_balance: u64) -> Result<Option<u64>, PlanError> {
    rules::held_credit_release(&offer.rules(), locked_balance)
}

fn state(code: i16, ended: Option<i16>) -> Result<OfferState> {
    let end = |v| match v {
        2 => Ok(EndState::Filled),
        3 => Ok(EndState::Cancelled),
        4 => Ok(EndState::Expired),
        _ => Err(invalid()),
    };
    match (code, ended) {
        (1, None) => Ok(OfferState::Open),
        (2..=4, None) => Ok(OfferState::Ended(end(code)?)),
        (5, Some(e)) => Ok(OfferState::CreditHeld(end(e)?)),
        _ => Err(invalid()),
    }
}
fn decode(row: &sqlx::postgres::PgRow) -> Result<Offer> {
    let offer = Offer {
        id: MarketOfferId::new(uuid_text(row.try_get("offer_id")?)?).map_err(|_| invalid())?,
        world: WorldId::from_bytes(uuid_text(row.try_get("world_id")?)?).map_err(|_| invalid())?,
        definition: StoredItemDefinition::new(super::item_mint::TypedDefinitionRef {
            family: "Item".to_owned(),
            production_key: row.try_get("definition_key")?,
            revision_ref: row.try_get("definition_revision")?,
        })?,
        side: match row.try_get::<i16, _>("side")? {
            1 => Side::Buy,
            2 => Side::Sell,
            _ => return Err(invalid()),
        },
        account: AccountId::from_bytes(uuid_text(row.try_get("account_id")?)?)
            .map_err(|_| invalid())?,
        character: CharacterId::from_bytes(uuid_text(row.try_get("character_id")?)?)
            .map_err(|_| invalid())?,
        piece_price: numeric_u64(row, "piece_price")?,
        amount: u32::try_from(row.try_get::<i32, _>("amount")?).map_err(|_| invalid())?,
        remaining: u32::try_from(row.try_get::<i32, _>("remaining")?).map_err(|_| invalid())?,
        anonymous: row.try_get("anonymous")?,
        created_at_us: row.try_get("created_at_us")?,
        expires_at_us: row.try_get("expires_at_us")?,
        state: state(row.try_get("state")?, row.try_get("ended_as")?)?,
        escrow_gold: numeric_u64(row, "escrow_gold")?,
        held_credit_gold: numeric_u64(row, "held_credit_gold")?,
        matching_pending: row.try_get("matching_pending")?,
    };
    offer.validate().map_err(|_| invalid())?;
    Ok(offer)
}


mod book;
mod operations;
mod reads;

pub use book::*;
pub use operations::*;
pub use reads::*;
