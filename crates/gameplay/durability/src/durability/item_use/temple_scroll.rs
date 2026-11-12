//! WU26-06 Temple Teleport Scroll (`docs/contracts/TEMPLE_TELEPORT_SCROLL_V1.md`): one scroll
//! burnt by the item-use writer (cause 481, shape burn) with one `game_temple_scroll_uses` row
//! in the same fenced transaction. A use moves the holder through the caller's
//! [`TempleLanding`] before COMMIT: no landing rolls the burn back, so the scroll is never
//! consumed without the move. The daily claim gives each delivered scroll its seven-day
//! absolute deadline on the existing Item deadline owner (`game_spell_item_temporal_schedules`,
//! target none), whose Channel pass removes it from the Inbox or the carried tree once due; a
//! use that races that deadline is refused and writes nothing. Migration 2762 pairs the burn and
//! the row and admits the scroll's schedule. English TibiaWiki "Temple Teleport Scroll"; Winter
//! Update 2026 thread 565.
use super::*;
use std::future::Future;
use std::pin::Pin;

/// Item 25718, the only definition this cause burns.
pub const TEMPLE_SCROLL_KEY: &str = "oteryn:item.tibia.i25718";
/// `ItemUseCause` 481 `TempleScroll` (Winter Update 2026 number range).
pub const TEMPLE_SCROLL_CAUSE: u32 = 481;
/// The deadline after delivery: 168 hours, the scroll's Item deadline schedule duration.
pub const TEMPLE_SCROLL_LIFETIME_MS: i64 = 7 * 24 * 60 * 60 * 1_000;
/// The definition revision the daily claim delivers and schedules.
pub const TEMPLE_SCROLL_REVISION: &str = "definition-r1";

/// The Channel owner's move of the holder to its temple landing, run inside the burn's
/// transaction after the use row is stamped before the deadline and before COMMIT.
pub trait TempleLanding: Send + Sync {
    /// Move the holder to the free landing closest to its temple; `false` when none exists,
    /// which rolls the burn back.
    fn land(&self) -> Pin<Box<dyn Future<Output = bool> + Send + '_>>;
}

/// The result of one scroll command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TempleScrollOutcome {
    /// This command burnt the scroll as a use and moved the holder to its temple landing.
    Used,
    /// No temple landing existed at commit: the burn rolled back and nothing was written.
    NoLanding,
    /// The scroll's deadline passed before the Item deadline owner removed it: the burn rolled
    /// back and nothing was written (a defensive refusal; the owner's pass removes it).
    Expired,
    /// The command committed earlier: its effect was applied then.
    AlreadyCommitted,
}

impl DurabilityRoot {
    /// One scroll use under its holder's current item fence. An exact replay of a committed
    /// CommandRef answers from its record and row; every refusal writes nothing.
    pub async fn commit_temple_scroll_use(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: ItemUseRequest,
        landing: &dyn TempleLanding,
    ) -> Result<TempleScrollOutcome> {
        if request.kind != ItemUseKind::TempleScroll
            || request.source_definition.production_key != TEMPLE_SCROLL_KEY
            || request.flask != ItemUseFlaskPlan::None
            || !matches!(fence.runtime_scope, RuntimeScopeRefV1::Channel { .. })
        {
            return Err(ItemUseError::InvalidInput);
        }
        request.validate()?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| ItemUseError::AuthorityRejected)?;
        let node = node.clone();
        let mut context = landing;
        self.try_issue_semantic_pass()?
            .run_with_context(&mut context, move |holder, deadline, landing| {
                Box::pin(async move {
                    let mut tx = begin_type2_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    match replay(&mut tx, &fence, &request).await {
                        Ok(Some(_)) => {
                            match recorded(&mut tx, &request.transaction_id).await {
                                Ok(()) => {}
                                Err(ItemUseError::Unavailable(error)) => return Err(error),
                                Err(error) => return Ok(Err(error)),
                            }
                            tx.commit(deadline).await?;
                            return Ok(Ok(TempleScrollOutcome::AlreadyCommitted));
                        }
                        Ok(None) => {}
                        Err(ItemUseError::Unavailable(error)) => return Err(error),
                        Err(error) => return Ok(Err(error)),
                    }
                    match assert_item_only_fence(&mut tx, &fence, &node).await? {
                        Ok(root)
                            if root.content_revision == request.content_revision
                                && root.ruleset_revision == request.ruleset_revision => {}
                        Ok(_) => return Ok(Err(ItemUseError::StaleState)),
                        Err(CharacterProgressionError::Unavailable(error)) => return Err(error),
                        Err(_) => return Ok(Err(ItemUseError::AuthorityRejected)),
                    }
                    let tuple = tx.tuple();
                    match write(&mut tx, &fence, &request, tuple).await {
                        Ok(ItemUseOutcome::Written { .. }) => {}
                        Ok(ItemUseOutcome::AlreadyCommitted { .. }) => {
                            return Ok(Err(ItemUseError::ConflictingCommand));
                        }
                        Err(ItemUseError::Unavailable(error)) => return Err(error),
                        Err(error) => return Ok(Err(error)),
                    }
                    // The burn holds the scroll's row: the deadline pass cannot retire it now.
                    // Dropping the transaction rolls the burn back.
                    match due(&mut tx, &request.source_item_instance_id).await {
                        Ok(false) => {}
                        Ok(true) => {
                            drop(tx);
                            return Ok(Ok(TempleScrollOutcome::Expired));
                        }
                        Err(ItemUseError::Unavailable(error)) => return Err(error),
                        Err(error) => return Ok(Err(error)),
                    }
                    match pair(&mut tx, &fence, &request).await {
                        Ok(true) => {}
                        // The deadline passed between the read above and the use row's stamp.
                        Ok(false) => {
                            drop(tx);
                            return Ok(Ok(TempleScrollOutcome::Expired));
                        }
                        Err(ItemUseError::Unavailable(error)) => return Err(error),
                        Err(error) => return Ok(Err(error)),
                    }
                    if !landing.land().await {
                        drop(tx);
                        return Ok(Ok(TempleScrollOutcome::NoLanding));
                    }
                    tx.commit(deadline).await?;
                    Ok(Ok(TempleScrollOutcome::Used))
                })
            })
            .await?
    }
}

/// Whether the scroll's Item deadline has passed on the deadline owner's database clock.
async fn due(connection: &mut PgConnection, item: &[u8; 16]) -> Result<bool> {
    Ok(sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM game_spell_item_temporal_schedules \
          WHERE item_instance_id=encode($1,'hex')::uuid AND target_family IS NULL \
            AND expires_at_unix_ms<=floor(extract(epoch FROM statement_timestamp())*1000)::bigint)",
    )
    .bind(item.as_slice())
    .fetch_one(&mut *connection)
    .await?)
}

/// The use row's CHECK that refuses a scroll stamped at or after its deadline (migration 2762).
const USE_BEFORE_DEADLINE: &str = "game_temple_scroll_uses_before_deadline";

/// Insert the burn's use row; its stamp copies the deadline and refuses a due scroll: `false`
/// when the scroll fell due after [`due`] read it (the caller drops the transaction).
async fn pair(
    connection: &mut PgConnection,
    fence: &CurrentCharacterItemFence,
    request: &ItemUseRequest,
) -> Result<bool> {
    match sqlx::query(
        "INSERT INTO game_temple_scroll_uses(transaction_id,item_instance_id,character_id) \
         VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid)",
    )
    .bind(request.transaction_id.as_slice())
    .bind(request.source_item_instance_id.as_slice())
    .bind(fence.character_id.as_bytes().as_slice())
    .execute(&mut *connection)
    .await
    {
        Ok(_) => Ok(true),
        Err(sqlx::Error::Database(error)) if error.constraint() == Some(USE_BEFORE_DEADLINE) => {
            Ok(false)
        }
        Err(error) => Err(error.into()),
    }
}

/// A committed scroll command has its use row.
async fn recorded(connection: &mut PgConnection, transaction_id: &[u8; 16]) -> Result<()> {
    sqlx::query_scalar::<_, i32>(
        "SELECT 1 FROM game_temple_scroll_uses WHERE transaction_id=encode($1,'hex')::uuid",
    )
    .bind(transaction_id.as_slice())
    .fetch_optional(&mut *connection)
    .await?
    .map(|_| ())
    .ok_or(ItemUseError::Unavailable(DurabilityError::InvalidStoredState))
}

/// The daily claim's seven-day deadline for one delivered scroll, in the claim transaction: one
/// schedule on the existing Item deadline owner with no decay target, so its Channel pass
/// removes the scroll once due. Migration 2762 proves it against the same-transaction delivery.
pub(crate) async fn schedule_delivered_scroll(
    connection: &mut PgConnection,
    item: &str,
    claim: &[u8; 16],
) -> std::result::Result<(), DurabilityError> {
    sqlx::query(
        "INSERT INTO game_spell_item_temporal_schedules(item_instance_id,source_transaction_id,\
         source_ordinal,duration_millis,expires_at_unix_ms,definition_family,\
         definition_production_key,definition_revision,content_digest) \
         VALUES($1::text::uuid,encode($2,'hex')::uuid,1,$3::bigint,\
         floor(extract(epoch FROM transaction_timestamp())*1000)::bigint+$3::bigint,'Item',\
         $4::text,$5::text,sha256(convert_to($4::text||'|'||$5::text,'UTF8')))",
    )
    .bind(item)
    .bind(claim.as_slice())
    .bind(TEMPLE_SCROLL_LIFETIME_MS)
    .bind(TEMPLE_SCROLL_KEY)
    .bind(TEMPLE_SCROLL_REVISION)
    .execute(&mut *connection)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn temple_scroll_cause_and_lifetime_are_pinned() {
        assert_eq!(TEMPLE_SCROLL_CAUSE, 481);
        assert_eq!(ItemUseKind::TempleScroll.code(), 481);
        assert_eq!(TEMPLE_SCROLL_LIFETIME_MS, 604_800_000);
        assert_eq!(TEMPLE_SCROLL_KEY, "oteryn:item.tibia.i25718");
        assert_eq!(TEMPLE_SCROLL_REVISION, "definition-r1");
    }

    #[test]
    fn temple_scroll_lifetime_fits_the_item_deadline_owner() {
        // `game_spell_item_temporal_schedules.duration_millis` is 1..=u32::MAX, and the
        // scroll vanishes at its deadline: no decay target.
        assert_eq!(u32::try_from(TEMPLE_SCROLL_LIFETIME_MS), Ok(604_800_000));
        assert_eq!(TEMPLE_SCROLL_LIFETIME_MS, 168 * 60 * 60 * 1_000);
    }
}
