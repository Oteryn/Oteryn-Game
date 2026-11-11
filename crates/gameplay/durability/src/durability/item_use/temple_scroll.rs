//! WU26-06 Temple Teleport Scroll (`docs/contracts/TEMPLE_TELEPORT_SCROLL_V1.md`): one scroll
//! burnt by the item-use writer (cause 481, shape burn) with one `game_temple_scroll_uses` row
//! in the same fenced transaction. The row's database stamp reads the scroll's seven-day
//! deadline from its daily delivery line and the daily effect clock, and records whether this
//! burn is a use or the scroll's `Expire {Deadline}` (nothing else happens). A use moves the
//! holder through the caller's [`TempleLanding`] before COMMIT: no landing rolls the burn back,
//! so the scroll is never consumed without the move. Migration 2762 pairs the burn and the row.
//! English TibiaWiki "Temple Teleport Scroll"; Winter Update 2026 thread 565.
use super::*;
use std::future::Future;
use std::pin::Pin;

/// Item 25718, the only definition this cause burns.
pub const TEMPLE_SCROLL_KEY: &str = "oteryn:item.tibia.i25718";
/// `ItemUseCause` 481 `TempleScroll` (Winter Update 2026 number range).
pub const TEMPLE_SCROLL_CAUSE: u32 = 481;
/// The deadline after delivery: 168 hours, the `remaining_ms` ceiling of the TIMED rows.
pub const TEMPLE_SCROLL_LIFETIME_MS: i64 = 7 * 24 * 60 * 60 * 1_000;

/// The Channel owner's move of the holder to its temple landing, run inside the burn's
/// transaction after the use row is stamped unexpired and before COMMIT.
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
    /// This command burnt the scroll at or after its deadline: nothing else happens.
    Expired,
    /// The command committed earlier: its effect, if any, was applied then.
    AlreadyCommitted { expired: bool },
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
                            let expired = match recorded(&mut tx, &request.transaction_id).await
                            {
                                Ok(expired) => expired,
                                Err(ItemUseError::Unavailable(error)) => return Err(error),
                                Err(error) => return Ok(Err(error)),
                            };
                            tx.commit(deadline).await?;
                            return Ok(Ok(TempleScrollOutcome::AlreadyCommitted { expired }));
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
                    let expired = match pair(&mut tx, &fence, &request).await {
                        Ok(expired) => expired,
                        Err(ItemUseError::Unavailable(error)) => return Err(error),
                        Err(error) => return Ok(Err(error)),
                    };
                    // Dropping the transaction rolls the burn and its row back.
                    if !expired && !landing.land().await {
                        drop(tx);
                        return Ok(Ok(TempleScrollOutcome::NoLanding));
                    }
                    tx.commit(deadline).await?;
                    Ok(Ok(if expired {
                        TempleScrollOutcome::Expired
                    } else {
                        TempleScrollOutcome::Used
                    }))
                })
            })
            .await?
    }
}

/// Insert the burn's use row; its stamp decides `expired` from the delivery deadline.
async fn pair(
    connection: &mut PgConnection,
    fence: &CurrentCharacterItemFence,
    request: &ItemUseRequest,
) -> Result<bool> {
    Ok(sqlx::query_scalar::<_, bool>(
        "INSERT INTO game_temple_scroll_uses(transaction_id,item_instance_id,character_id) \
         VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid) \
         RETURNING expired",
    )
    .bind(request.transaction_id.as_slice())
    .bind(request.source_item_instance_id.as_slice())
    .bind(fence.character_id.as_bytes().as_slice())
    .fetch_one(&mut *connection)
    .await?)
}

/// The recorded `expired` of a committed scroll command.
async fn recorded(connection: &mut PgConnection, transaction_id: &[u8; 16]) -> Result<bool> {
    sqlx::query_scalar::<_, bool>(
        "SELECT expired FROM game_temple_scroll_uses WHERE transaction_id=encode($1,'hex')::uuid",
    )
    .bind(transaction_id.as_slice())
    .fetch_optional(&mut *connection)
    .await?
    .ok_or(ItemUseError::Unavailable(DurabilityError::InvalidStoredState))
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
    }
}
