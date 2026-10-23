use super::*;

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
    if rows.len() > MAX_OPEN_OFFERS {
        return Err(invalid());
    }
    rows.iter().map(decode).collect()
}

/// An ended offer plus its stored ending order, separate from creation/expiry metadata.
/// Migration 0155 stores these fields; the atomic ending writer remains pending.
/// This reader does not infer ending order from creation timestamps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndedOffer {
    pub offer: Offer,
    pub ended_at_us: i64,
    pub end_sequence: i64,
}

/// Private My Offers history: the last 600 endings, including unreleased CREDIT_HELD.
/// Native current recovery/session/depot admission and owner resolution remain caller gates.
/// CP D1159 requires a DB-clock ending timestamp and monotonic bigint sequence from the
/// atomic ending writer. The migration constrains metadata but does not supply that writer.
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
