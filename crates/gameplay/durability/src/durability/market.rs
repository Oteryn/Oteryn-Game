//! MARKET0 §§4/6/7: native private storage reads and transaction-bound book selection.
//!
//! No value/offer/operation writer is exposed: the missing Market ledger owner, reserved Inbox
//! delivery and conservation/audit arms must commit together in the CP slot. A locked book or
//! historical outcome is not actor authority. The eventual owner must acquire real recovery,
//! admission and (for commands) current session/root fences BEFORE this book boundary.
#![allow(
    dead_code,
    reason = "Market atomic writer and module registration await CP integration"
)]

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

fn has_nonproduction_marker(value: &str) -> bool {
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
fn valid_key_bytes(value: &str) -> bool {
    !value.is_empty()
        && value.is_ascii()
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b':' | b'.' | b'_' | b'-' | b'/')
        })
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

pub const MAX_TOTAL: u64 = 999_999_999_999;
pub const HARD_CREDIT_CEILING: u64 = 9_000_000_000_000_000;
pub const LIFETIME_US: i64 = 30 * 24 * 60 * 60 * 1_000_000;

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
impl Offer {
    pub fn book(&self) -> BookKey {
        BookKey {
            world: self.world,
            definition: MarketDefinitionKey(self.definition.key().to_owned()),
        }
    }
    pub fn validate(&self) -> Result<(), PlanError> {
        offer_total(self.amount, self.piece_price)?;
        if self.definition.family() != "Item"
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
    pub offer: MarketOfferId,
    pub ended_as: EndState,
    pub return_gold: u64,
    pub return_item_units: u32,
}
pub fn end_plan(
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
pub fn held_credit_release(
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

/// Private My Offers data, not depot admission or permission to see another character's
/// offers. The eventual adapter must independently resolve the requesting character and
/// World under the native recovery/session/depot-view gates before calling this reader.
/// Expired OPEN rows remain visible until the expiry writer actually ends them. Anonymous
/// offers also remain visible to their owner. No custody, reservation or write is performed.
pub async fn open_character_offers(
    connection: &mut PgConnection,
    world: WorldId,
    character: CharacterId,
) -> Result<Vec<Offer>> {
    // One statement gives one snapshot. Read one sentinel beyond MARKET0's 100-open-offer
    // ceiling so corrupt storage is rejected rather than silently hiding an owner's offer.
    let rows = sqlx::query(
        "SELECT offer_id::text,world_id::text,definition_key,definition_revision,side,account_id::text,character_id::text,piece_price::text,amount,remaining,anonymous,created_at_us,expires_at_us,state,ended_as,escrow_gold::text,held_credit_gold::text,matching_pending FROM game_market_offers WHERE world_id=encode($1,'hex')::uuid AND character_id=encode($2,'hex')::uuid AND state=1 ORDER BY created_at_us DESC,offer_id DESC LIMIT 101",
    )
    .bind(world.as_bytes().as_slice())
    .bind(character.as_bytes().as_slice())
    .fetch_all(connection)
    .await?;
    if rows.len() > 100 {
        return Err(invalid());
    }
    rows.iter().map(decode).collect()
}

/// An ended offer plus its stored ending order, separate from creation/expiry metadata.
/// These fields require the owning production migration and ending writer; this private
/// reader does not create them or infer them from the offer's creation timestamp.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndedOffer {
    pub offer: Offer,
    pub ended_at_us: i64,
    pub end_sequence: i64,
}

/// Private My Offers history: the last 600 endings, including unreleased CREDIT_HELD.
/// Native current recovery/session/depot admission and owner resolution remain caller gates.
/// CP D1159 requires a DB-clock ending timestamp and monotonic bigint sequence from the
/// atomic ending writer. Proposed fixture columns are not a deployed production schema.
pub async fn ended_character_offers(
    connection: &mut PgConnection,
    world: WorldId,
    character: CharacterId,
) -> Result<Vec<EndedOffer>> {
    let rows = sqlx::query(
        "SELECT offer_id::text,world_id::text,definition_key,definition_revision,side,account_id::text,character_id::text,piece_price::text,amount,remaining,anonymous,created_at_us,expires_at_us,state,ended_as,escrow_gold::text,held_credit_gold::text,matching_pending,ended_at_us,end_sequence FROM game_market_offers WHERE world_id=encode($1,'hex')::uuid AND character_id=encode($2,'hex')::uuid AND state IN(2,3,4,5) ORDER BY ended_at_us DESC,end_sequence DESC LIMIT 600",
    )
    .bind(world.as_bytes().as_slice())
    .bind(character.as_bytes().as_slice())
    .fetch_all(connection)
    .await?;
    let mut sequences = std::collections::BTreeSet::new();
    rows.iter()
        .map(|row| {
            let offer = decode(row)?;
            let ended_at_us = row
                .try_get::<Option<i64>, _>("ended_at_us")?
                .ok_or_else(invalid)?;
            let end_sequence = row
                .try_get::<Option<i64>, _>("end_sequence")?
                .ok_or_else(invalid)?;
            if ended_at_us < offer.created_at_us
                || end_sequence <= 0
                || !sequences.insert(end_sequence)
            {
                return Err(invalid());
            }
            Ok(EndedOffer {
                offer,
                ended_at_us,
                end_sequence,
            })
        })
        .collect()
}

/// Stable price/time/id pagination data, never a reservation or current offer proof.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BookCursor {
    pub book: BookKey,
    pub side: Side,
    pub piece_price: u64,
    pub created_at_us: i64,
    pub offer: MarketOfferId,
}

#[derive(Debug, PartialEq, Eq)]
pub struct BookPage {
    pub offers: Vec<Offer>,
    pub next: Option<BookCursor>,
}

/// MARKET0 §10 private stored book data, 32 offers per page. The adapter must enforce
/// the native depot-view gates, admit the requested ware and redact anonymous sellers.
/// A page is not a matching decision: offers may change between pages or before execution.
pub async fn book_page(
    connection: &mut PgConnection,
    key: &BookKey,
    side: Side,
    after: Option<&BookCursor>,
) -> Result<BookPage> {
    if after.is_some_and(|cursor| {
        cursor.book != *key
            || cursor.side != side
            || !(1..=MAX_TOTAL).contains(&cursor.piece_price)
            || cursor.created_at_us < 0
    }) {
        return Err(invalid());
    }
    // Closed queries keep the accepted best-price ordering explicit. The 33rd row is
    // only a continuation sentinel. No OFFSET scan or caller-controlled page size.
    let query = match side {
        Side::Sell => {
            "SELECT offer_id::text,world_id::text,definition_key,definition_revision,side,account_id::text,character_id::text,piece_price::text,amount,remaining,anonymous,created_at_us,expires_at_us,state,ended_as,escrow_gold::text,held_credit_gold::text,matching_pending FROM game_market_offers WHERE world_id=encode($1,'hex')::uuid AND definition_key=$2 AND side=2 AND state=1 AND created_at_us<=floor(extract(epoch FROM statement_timestamp())*1000000)::bigint AND expires_at_us>floor(extract(epoch FROM statement_timestamp())*1000000)::bigint AND ($3::text IS NULL OR (piece_price,created_at_us,offer_id)>($3::text::numeric,$4,encode($5,'hex')::uuid)) ORDER BY game_market_offers.piece_price ASC,created_at_us,offer_id LIMIT 33"
        }
        Side::Buy => {
            "SELECT offer_id::text,world_id::text,definition_key,definition_revision,side,account_id::text,character_id::text,piece_price::text,amount,remaining,anonymous,created_at_us,expires_at_us,state,ended_as,escrow_gold::text,held_credit_gold::text,matching_pending FROM game_market_offers WHERE world_id=encode($1,'hex')::uuid AND definition_key=$2 AND side=1 AND state=1 AND created_at_us<=floor(extract(epoch FROM statement_timestamp())*1000000)::bigint AND expires_at_us>floor(extract(epoch FROM statement_timestamp())*1000000)::bigint AND ($3::text IS NULL OR piece_price<$3::text::numeric OR (piece_price=$3::text::numeric AND (created_at_us,offer_id)>($4,encode($5,'hex')::uuid))) ORDER BY game_market_offers.piece_price DESC,created_at_us,offer_id LIMIT 33"
        }
    };
    let rows = sqlx::query(query)
        .bind(key.world.as_bytes().as_slice())
        .bind(key.definition.as_str())
        .bind(after.map(|cursor| cursor.piece_price.to_string()))
        .bind(after.map(|cursor| cursor.created_at_us))
        .bind(after.map(|cursor| cursor.offer.as_bytes().as_slice()))
        .fetch_all(connection)
        .await?;
    let mut offers: Vec<Offer> = rows.iter().map(decode).collect::<Result<_>>()?;
    let next = if offers.len() > 32 {
        offers.truncate(32);
        let last = offers.last().ok_or_else(invalid)?;
        Some(BookCursor {
            book: key.clone(),
            side,
            piece_price: last.piece_price,
            created_at_us: last.created_at_us,
            offer: last.id,
        })
    } else {
        None
    };
    Ok(BookPage { offers, next })
}

/// The borrow prevents reuse of this transaction through the component while the book is held.
/// PostgreSQL holds the actual row lock to transaction end; callers may not commit a selection
/// as a Market mutation without all bank/item/Inbox/event effects in that same transaction.
pub struct LockedBook<'a, 'c> {
    tx: &'a mut Transaction<'c, Postgres>,
    key: BookKey,
}
pub async fn lock_book<'a, 'c>(
    tx: &'a mut Transaction<'c, Postgres>,
    key: BookKey,
) -> Result<LockedBook<'a, 'c>> {
    sqlx::query("INSERT INTO game_market_books(world_id,definition_key) VALUES(encode($1,'hex')::uuid,$2) ON CONFLICT DO NOTHING")
        .bind(key.world.as_bytes().as_slice()).bind(key.definition.as_str()).execute(&mut **tx).await?;
    sqlx::query("SELECT world_id FROM game_market_books WHERE world_id=encode($1,'hex')::uuid AND definition_key=$2 FOR UPDATE")
        .bind(key.world.as_bytes().as_slice()).bind(key.definition.as_str()).fetch_one(&mut **tx).await?;
    Ok(LockedBook { tx, key })
}
/// Job candidates already have an existing book (offer FK). Do not block on its upsert before
/// SKIP LOCKED: an absent book has no valid offer and is not a job lock acquisition.
pub async fn try_lock_job_book<'a, 'c>(
    tx: &'a mut Transaction<'c, Postgres>,
    key: BookKey,
) -> Result<Option<LockedBook<'a, 'c>>> {
    let row=sqlx::query("SELECT world_id FROM game_market_books WHERE world_id=encode($1,'hex')::uuid AND definition_key=$2 FOR UPDATE SKIP LOCKED")
        .bind(key.world.as_bytes().as_slice()).bind(key.definition.as_str()).fetch_optional(&mut **tx).await?;
    Ok(row.map(|_| LockedBook { tx, key }))
}
pub struct PendingMatch {
    pub pending: Offer,
    pub counter: Option<Offer>,
}
#[derive(Debug)]
pub struct EscrowEntry {
    pub item: ItemInstanceId,
    pub ordinal: u64,
    pub quantity: u32,
}

impl LockedBook<'_, '_> {
    async fn now(&mut self) -> Result<i64> {
        Ok(sqlx::query_scalar(
            "SELECT floor(extract(epoch FROM clock_timestamp())*1000000)::bigint",
        )
        .fetch_one(&mut **self.tx)
        .await?)
    }
    pub async fn offer(&mut self, id: MarketOfferId) -> Result<Option<Offer>> {
        let row=sqlx::query("SELECT offer_id::text,world_id::text,definition_key,definition_revision,side,account_id::text,character_id::text,piece_price::text,amount,remaining,anonymous,created_at_us,expires_at_us,state,ended_as,escrow_gold::text,held_credit_gold::text,matching_pending FROM game_market_offers WHERE offer_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND definition_key=$3 FOR UPDATE")
            .bind(id.as_bytes().as_slice()).bind(self.key.world.as_bytes().as_slice()).bind(self.key.definition.as_str()).fetch_optional(&mut **self.tx).await?;
        row.as_ref().map(decode).transpose()
    }
    pub async fn accepts_busy(&mut self) -> Result<bool> {
        Ok(sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_market_offers WHERE world_id=encode($1,'hex')::uuid AND definition_key=$2 AND state=1 AND matching_pending)")
            .bind(self.key.world.as_bytes().as_slice()).bind(self.key.definition.as_str()).fetch_one(&mut **self.tx).await?)
    }
    /// Oldest pending first, then best crossing price/oldest counter, skipping same Account.
    /// Different stored revisions remain explicit: no compatibility or marketability is inferred.
    /// Pair rows lock in offer-id order, independently of maker/taker order.
    pub async fn next_match(&mut self) -> Result<Option<PendingMatch>> {
        let now = self.now().await?;
        let pending=sqlx::query("SELECT offer_id::text,side,piece_price::text,account_id::text FROM game_market_offers WHERE world_id=encode($1,'hex')::uuid AND definition_key=$2 AND state=1 AND matching_pending AND expires_at_us>$3 AND created_at_us<=$3 ORDER BY created_at_us,offer_id LIMIT 1")
            .bind(self.key.world.as_bytes().as_slice()).bind(self.key.definition.as_str()).bind(now).fetch_optional(&mut **self.tx).await?;
        let Some(pending) = pending else {
            return Ok(None);
        };
        let id =
            MarketOfferId::new(uuid_text(pending.try_get("offer_id")?)?).map_err(|_| invalid())?;
        let side: i16 = pending.try_get("side")?;
        if !(1..=2).contains(&side) {
            return Err(invalid());
        }
        let price = numeric_u64(&pending, "piece_price")?;
        let account = uuid_text(pending.try_get("account_id")?)?;
        // Closed literal queries preserve price-order index use for prepared generic plans.
        let query = if side == 1 {
            "SELECT offer_id::text FROM game_market_offers WHERE world_id=encode($1,'hex')::uuid AND definition_key=$2 AND state=1 AND side=2 AND account_id<>encode($3,'hex')::uuid AND expires_at_us>$4 AND created_at_us<=$4 AND piece_price<=$5::text::numeric ORDER BY piece_price ASC,created_at_us,offer_id LIMIT 1"
        } else {
            "SELECT offer_id::text FROM game_market_offers WHERE world_id=encode($1,'hex')::uuid AND definition_key=$2 AND state=1 AND side=1 AND account_id<>encode($3,'hex')::uuid AND expires_at_us>$4 AND created_at_us<=$4 AND piece_price>=$5::text::numeric ORDER BY piece_price DESC,created_at_us,offer_id LIMIT 1"
        };
        let counter: Option<String> = sqlx::query_scalar(query)
            .bind(self.key.world.as_bytes().as_slice())
            .bind(self.key.definition.as_str())
            .bind(account.as_slice())
            .bind(now)
            .bind(price.to_string())
            .fetch_optional(&mut **self.tx)
            .await?;
        let counter = counter
            .map(|v| MarketOfferId::new(uuid_text(&v)?).map_err(|_| invalid()))
            .transpose()?;
        let mut ids = vec![id];
        if let Some(other) = counter {
            ids.push(other);
        }
        ids.sort();
        let mut a = None;
        let mut b = None;
        for locked in ids {
            let offer = self.offer(locked).await?.ok_or_else(invalid)?;
            if locked == id {
                a = Some(offer);
            } else {
                b = Some(offer);
            }
        }
        Ok(Some(PendingMatch {
            pending: a.ok_or_else(invalid)?,
            counter: b,
        }))
    }
    /// The eventual fenced cancel writer must compare the stored owner to the actual actor.
    /// This only derives exact stored escrow return requirements, never actor authority.
    pub async fn plan_end(
        &mut self,
        id: MarketOfferId,
        reason: EndReason,
    ) -> Result<Option<EndPlan>> {
        let Some(offer) = self.offer(id).await? else {
            return Ok(None);
        };
        let now = self.now().await?;
        end_plan(&offer, reason, now)
            .map(Some)
            .map_err(|_| invalid())
    }
    /// Locks actual Item rows in ItemInstanceId order, after offer rows. Does not establish
    /// exclusive custody: MARKET0 §8 placement/removal/exclusivity guards remain the CP hook.
    pub async fn escrow(&mut self, offer: &Offer) -> Result<Vec<EscrowEntry>> {
        if offer.book() != self.key {
            return Err(invalid());
        }
        if self.offer(offer.id).await?.as_ref() != Some(offer) {
            return Err(invalid());
        }
        let rows=sqlx::query("SELECT e.item_instance_id::text,e.ordinal::text,i.quantity,i.definition_family,i.definition_production_key,i.definition_revision_ref,i.lifecycle FROM game_item_market_escrow_locations e JOIN game_item_instances i USING(item_instance_id,world_id) WHERE e.offer_id=encode($1,'hex')::uuid AND e.world_id=encode($2,'hex')::uuid ORDER BY e.item_instance_id LIMIT 101 FOR UPDATE OF e,i")
            .bind(offer.id.as_bytes().as_slice()).bind(self.key.world.as_bytes().as_slice()).fetch_all(&mut **self.tx).await?;
        if rows.len() > 100 {
            return Err(invalid());
        }
        let mut entries = Vec::with_capacity(rows.len());
        let mut units = 0_u64;
        for row in rows {
            if row.try_get::<&str, _>("definition_family")? != "Item"
                || row.try_get::<&str, _>("definition_production_key")? != offer.definition.key()
                || row.try_get::<&str, _>("definition_revision_ref")? != offer.definition.revision()
                || row.try_get::<i16, _>("lifecycle")? != 1
            {
                return Err(invalid());
            }
            let quantity =
                u32::try_from(row.try_get::<i64, _>("quantity")?).map_err(|_| invalid())?;
            if quantity == 0 {
                return Err(invalid());
            }
            units = units.checked_add(u64::from(quantity)).ok_or_else(invalid)?;
            let ordinal = numeric_u64(&row, "ordinal")?;
            if ordinal == 0 {
                return Err(invalid());
            }
            entries.push(EscrowEntry {
                item: ItemInstanceId::from_bytes(uuid_text(row.try_get("item_instance_id")?)?)
                    .map_err(|_| invalid())?,
                ordinal,
                quantity,
            });
        }
        if offer.state == OfferState::Open
            && units
                != if offer.side == Side::Sell {
                    u64::from(offer.remaining)
                } else {
                    0
                }
        {
            return Err(invalid());
        }
        Ok(entries)
    }
}

/// Unlocked job discovery only; re-lock the book and re-read each offer before planning a step.
pub async fn expired_candidates(
    connection: &mut PgConnection,
    world: WorldId,
) -> Result<Vec<(BookKey, MarketOfferId)>> {
    let rows=sqlx::query("SELECT offer_id::text,definition_key FROM game_market_offers WHERE world_id=encode($1,'hex')::uuid AND state=1 AND expires_at_us<=floor(extract(epoch FROM clock_timestamp())*1000000)::bigint ORDER BY expires_at_us,offer_id LIMIT 100")
        .bind(world.as_bytes().as_slice()).fetch_all(connection).await?;
    rows.iter()
        .map(|r| {
            Ok((
                BookKey {
                    world,
                    definition: MarketDefinitionKey::new(r.try_get::<&str, _>("definition_key")?)
                        .map_err(|_| invalid())?,
                },
                MarketOfferId::new(uuid_text(r.try_get("offer_id")?)?).map_err(|_| invalid())?,
            ))
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobStep {
    Expire,
    Match,
    HeldCredit,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i16)]
pub enum OperationKind {
    Place = 1,
    Accept = 2,
    Cancel = 3,
    Expire = 4,
    Match = 5,
    HeldCredit = 6,
}
#[derive(Debug, Clone, Copy)]
pub enum Occurrence {
    Command(CommandRef),
    Job {
        offer: MarketOfferId,
        step: JobStep,
        number: u64,
    },
}
impl Occurrence {
    fn key(self) -> Result<Vec<u8>> {
        let mut out = vec![1];
        match self {
            Self::Command(command) => {
                out.push(1);
                out.extend_from_slice(command.game_session_id().as_bytes());
                out.extend_from_slice(&command.command_id().get().to_be_bytes());
            }
            Self::Job {
                offer,
                step,
                number,
            } => {
                if number == 0 {
                    return Err(invalid());
                }
                out.push(2);
                out.extend_from_slice(offer.as_bytes());
                out.push(match step {
                    JobStep::Expire => 1,
                    JobStep::Match => 2,
                    JobStep::HeldCredit => 3,
                });
                out.extend_from_slice(&number.to_be_bytes());
            }
        }
        Ok(out)
    }
}
#[derive(Debug, PartialEq, Eq)]
pub struct HistoricalOutcome {
    pub transaction_id: [u8; 16],
    pub outcome: Vec<u8>,
}
#[derive(Debug)]
pub enum OperationReadError {
    ConflictingBinding,
    Storage(DurabilityError),
}
impl From<sqlx::Error> for OperationReadError {
    fn from(e: sqlx::Error) -> Self {
        Self::Storage(e.into())
    }
}
impl From<DurabilityError> for OperationReadError {
    fn from(e: DurabilityError) -> Self {
        Self::Storage(e)
    }
}
/// Immutable historical read, not a recovery/session authority constructor. The owner must
/// validate native recovery BEFORE calling and return a match before fresh authority/state.
/// Payload is an opaque recorded outcome; this does not invent the future Market wire codec.
pub async fn read_operation(
    connection: &mut PgConnection,
    world: WorldId,
    occurrence: Occurrence,
    kind: OperationKind,
    actor: Option<CharacterId>,
    binding: &[u8; 32],
) -> std::result::Result<Option<HistoricalOutcome>, OperationReadError> {
    let valid = match occurrence {
        Occurrence::Command(_) => {
            actor.is_some()
                && matches!(
                    kind,
                    OperationKind::Place | OperationKind::Accept | OperationKind::Cancel
                )
        }
        Occurrence::Job { step, .. } => {
            actor.is_none()
                && kind
                    == match step {
                        JobStep::Expire => OperationKind::Expire,
                        JobStep::Match => OperationKind::Match,
                        JobStep::HeldCredit => OperationKind::HeldCredit,
                    }
        }
    };
    if !valid {
        return Err(invalid().into());
    }
    let row=sqlx::query("SELECT world_id::text,transaction_id::text,kind,actor::text,request_binding,outcome FROM game_market_operations WHERE occurrence=$1")
        .bind(occurrence.key()?).fetch_optional(connection).await?;
    let Some(row) = row else {
        return Ok(None);
    };
    let stored_actor: Option<String> = row.try_get("actor")?;
    let stored_actor = stored_actor.map(|v| uuid_text(&v)).transpose()?;
    let stored_binding: Vec<u8> = row.try_get("request_binding")?;
    if uuid_text(row.try_get("world_id")?)? != *world.as_bytes()
        || row.try_get::<i16, _>("kind")? != kind as i16
        || stored_actor != actor.map(|v| *v.as_bytes())
        || stored_binding.as_slice() != binding
    {
        return Err(OperationReadError::ConflictingBinding);
    }
    let transaction_id = uuid_text(row.try_get("transaction_id")?)?;
    super::item_mint_audit::check_uuid_v7(&transaction_id).map_err(|_| invalid())?;
    let outcome: Vec<u8> = row.try_get("outcome")?;
    if outcome.is_empty() || outcome.len() > 512 {
        return Err(invalid().into());
    }
    Ok(Some(HistoricalOutcome {
        transaction_id,
        outcome,
    }))
}

/// Administrative fixture access to the exact production encoder; never a command grant.
#[cfg(test)]
pub fn fixture_occurrence_key(occurrence: Occurrence) -> Result<Vec<u8>> {
    occurrence.key()
}
