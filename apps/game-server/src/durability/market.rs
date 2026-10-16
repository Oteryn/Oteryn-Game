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
use crate::content::{DefinitionFamily, DefinitionRevisionRef, ProductionKey, TypedDefinitionRef};
use crate::domain::market::*;
use crate::domain::{AccountId, CharacterId, ItemInstanceId, WorldId};
use crate::foundation::CommandRef;
use sqlx::{PgConnection, Postgres, Row, Transaction};

type Result<T> = std::result::Result<T, DurabilityError>;
fn invalid() -> DurabilityError {
    DurabilityError::InvalidStoredState
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
        definition: TypedDefinitionRef::new(
            DefinitionFamily::Item,
            ProductionKey::new(row.try_get::<&str, _>("definition_key")?).map_err(|_| invalid())?,
            DefinitionRevisionRef::new(row.try_get("definition_revision")?)
                .map_err(|_| invalid())?,
        ),
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
pub(crate) async fn open_character_offers(
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

/// Stable price/time/id pagination data, never a reservation or current offer proof.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BookCursor {
    pub(crate) book: BookKey,
    pub(crate) side: Side,
    pub(crate) piece_price: u64,
    pub(crate) created_at_us: i64,
    pub(crate) offer: MarketOfferId,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct BookPage {
    pub(crate) offers: Vec<Offer>,
    pub(crate) next: Option<BookCursor>,
}

/// MARKET0 §10 private stored book data, 32 offers per page. The adapter must enforce
/// the native depot-view gates, admit the requested ware and redact anonymous sellers.
/// A page is not a matching decision: offers may change between pages or before execution.
pub(crate) async fn book_page(
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
            "SELECT offer_id::text,world_id::text,definition_key,definition_revision,side,account_id::text,character_id::text,piece_price::text,amount,remaining,anonymous,created_at_us,expires_at_us,state,ended_as,escrow_gold::text,held_credit_gold::text,matching_pending FROM game_market_offers WHERE world_id=encode($1,'hex')::uuid AND definition_key=$2 AND side=2 AND state=1 AND created_at_us<=floor(extract(epoch FROM statement_timestamp())*1000000)::bigint AND expires_at_us>floor(extract(epoch FROM statement_timestamp())*1000000)::bigint AND ($3::text IS NULL OR (piece_price,created_at_us,offer_id)>($3::text::numeric,$4,encode($5,'hex')::uuid)) ORDER BY piece_price ASC,created_at_us,offer_id LIMIT 33"
        }
        Side::Buy => {
            "SELECT offer_id::text,world_id::text,definition_key,definition_revision,side,account_id::text,character_id::text,piece_price::text,amount,remaining,anonymous,created_at_us,expires_at_us,state,ended_as,escrow_gold::text,held_credit_gold::text,matching_pending FROM game_market_offers WHERE world_id=encode($1,'hex')::uuid AND definition_key=$2 AND side=1 AND state=1 AND created_at_us<=floor(extract(epoch FROM statement_timestamp())*1000000)::bigint AND expires_at_us>floor(extract(epoch FROM statement_timestamp())*1000000)::bigint AND ($3::text IS NULL OR piece_price<$3::text::numeric OR (piece_price=$3::text::numeric AND (created_at_us,offer_id)>($4,encode($5,'hex')::uuid))) ORDER BY piece_price DESC,created_at_us,offer_id LIMIT 33"
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
pub(crate) struct LockedBook<'a, 'c> {
    tx: &'a mut Transaction<'c, Postgres>,
    key: BookKey,
}
pub(crate) async fn lock_book<'a, 'c>(
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
pub(crate) async fn try_lock_job_book<'a, 'c>(
    tx: &'a mut Transaction<'c, Postgres>,
    key: BookKey,
) -> Result<Option<LockedBook<'a, 'c>>> {
    let row=sqlx::query("SELECT world_id FROM game_market_books WHERE world_id=encode($1,'hex')::uuid AND definition_key=$2 FOR UPDATE SKIP LOCKED")
        .bind(key.world.as_bytes().as_slice()).bind(key.definition.as_str()).fetch_optional(&mut **tx).await?;
    Ok(row.map(|_| LockedBook { tx, key }))
}
pub(crate) struct PendingMatch {
    pub(crate) pending: Offer,
    pub(crate) counter: Option<Offer>,
}
#[derive(Debug)]
pub(crate) struct EscrowEntry {
    pub(crate) item: ItemInstanceId,
    pub(crate) ordinal: u64,
    pub(crate) quantity: u32,
}

impl LockedBook<'_, '_> {
    async fn now(&mut self) -> Result<i64> {
        Ok(sqlx::query_scalar(
            "SELECT floor(extract(epoch FROM clock_timestamp())*1000000)::bigint",
        )
        .fetch_one(&mut **self.tx)
        .await?)
    }
    pub(crate) async fn offer(&mut self, id: MarketOfferId) -> Result<Option<Offer>> {
        let row=sqlx::query("SELECT offer_id::text,world_id::text,definition_key,definition_revision,side,account_id::text,character_id::text,piece_price::text,amount,remaining,anonymous,created_at_us,expires_at_us,state,ended_as,escrow_gold::text,held_credit_gold::text,matching_pending FROM game_market_offers WHERE offer_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND definition_key=$3 FOR UPDATE")
            .bind(id.as_bytes().as_slice()).bind(self.key.world.as_bytes().as_slice()).bind(self.key.definition.as_str()).fetch_optional(&mut **self.tx).await?;
        row.as_ref().map(decode).transpose()
    }
    pub(crate) async fn accepts_busy(&mut self) -> Result<bool> {
        Ok(sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_market_offers WHERE world_id=encode($1,'hex')::uuid AND definition_key=$2 AND state=1 AND matching_pending)")
            .bind(self.key.world.as_bytes().as_slice()).bind(self.key.definition.as_str()).fetch_one(&mut **self.tx).await?)
    }
    /// Oldest pending first, then best crossing price/oldest counter, skipping same Account.
    /// Different stored revisions remain explicit: no compatibility or marketability is inferred.
    /// Pair rows lock in offer-id order, independently of maker/taker order.
    pub(crate) async fn next_match(&mut self) -> Result<Option<PendingMatch>> {
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
    pub(crate) async fn plan_end(
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
    pub(crate) async fn escrow(&mut self, offer: &Offer) -> Result<Vec<EscrowEntry>> {
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
                || row.try_get::<&str, _>("definition_production_key")?
                    != offer.definition.key().as_str()
                || row.try_get::<&str, _>("definition_revision_ref")?
                    != offer.definition.revision().as_str()
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
pub(crate) async fn expired_candidates(
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
                    definition: ProductionKey::new(r.try_get::<&str, _>("definition_key")?)
                        .map_err(|_| invalid())?,
                },
                MarketOfferId::new(uuid_text(r.try_get("offer_id")?)?).map_err(|_| invalid())?,
            ))
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum JobStep {
    Expire,
    Match,
    HeldCredit,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i16)]
pub(crate) enum OperationKind {
    Place = 1,
    Accept = 2,
    Cancel = 3,
    Expire = 4,
    Match = 5,
    HeldCredit = 6,
}
#[derive(Debug, Clone, Copy)]
pub(crate) enum Occurrence {
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
pub(crate) struct HistoricalOutcome {
    pub(crate) transaction_id: [u8; 16],
    pub(crate) outcome: Vec<u8>,
}
#[derive(Debug)]
pub(crate) enum OperationReadError {
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
pub(crate) async fn read_operation(
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

#[cfg(test)]
#[path = "market_tests.rs"]
mod tests;
