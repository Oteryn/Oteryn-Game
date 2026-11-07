use super::*;

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
        if rows.len() > MAX_TOUCHED_ITEMS {
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
