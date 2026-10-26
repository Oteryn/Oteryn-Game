//! ITEM-IMBUE-1 storage preparation, owned here under ADR-0022.
//!
//! The installed migration is the single schema definition; no private parallel
//! table or alternate item owner. Shrine Apply/Clear writes through
//! [`super::item_imbuement`] with the shared item-only fee owner (migration 0196);
//! checkpoint/expiry are written by the current Channel owner's fenced clock.
//! A retained zero row is occupied state, never permission for bare deletion.

/// Installed by the normal GAME_MIGRATOR; this module does not execute DDL.
pub const IMBUEMENT_MIGRATION_VERSION: i64 = 150;

use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::item_mint::TypedDefinitionRef;
use super::item_transfer::load_slot;
use super::{DurabilityError, DurabilityRoot};
use crate::domain::CharacterId;
use sqlx::Row;
use super::character_progression::{CurrentCharacterGameplayFence, assert_gameplay_fence};
use super::runtime_scope_assignment::NodeIncarnationProof;

/// One stored imbuement slot; `category` is the stored 0-19 category.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoredImbuement {
    pub slot: u8,
    pub category: u8,
    pub tier: u8,
    pub remaining_ms: u32,
}

/// A live main backpack direct entry with its stored imbuement rows, ascending by slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImbuementTarget {
    pub definition: TypedDefinitionRef,
    pub applied: Vec<StoredImbuement>,
}

fn stored<T: TryFrom<i64>>(value: i64) -> Result<T, DurabilityError> {
    T::try_from(value).map_err(|_| DurabilityError::InvalidStoredState)
}

impl DurabilityRoot {
    /// Debit eligible equipped imbuements and atomically remove exhausted slots. The database
    /// record is the idempotency receipt, so an ambiguous retry cannot debit twice.
    pub async fn checkpoint_imbuements(
        &self,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        transaction_id: [u8; 16],
        elapsed_ms: u64,
        in_combat: bool,
        occurred_at_unix_ms: i64,
    ) -> Result<(), DurabilityError> {
        if elapsed_ms == 0 || elapsed_ms > 604_800_000 || occurred_at_unix_ms <= 0 {
            return Err(DurabilityError::InvalidStoredState);
        }
        let record = recovery.record_for(self).map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = super::db::begin_type2_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &record).await?;
            if assert_gameplay_fence(&mut tx, &fence, &node).await?.is_err() {
                return Err(DurabilityError::Unavailable);
            }
            sqlx::query("INSERT INTO game_item_imbuement_checkpoints(transaction_id,game_session_id,character_id,world_id,elapsed_ms,in_combat,occurred_at) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5,$6,$7) ON CONFLICT(transaction_id) DO NOTHING")
                .bind(transaction_id.as_slice()).bind(fence.game_session_id.as_bytes().as_slice())
                .bind(fence.character_id.as_bytes().as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice())
                .bind(i64::try_from(elapsed_ms).map_err(|_| DurabilityError::InvalidStoredState)?)
                .bind(in_combat).bind(occurred_at_unix_ms).execute(&mut **tx).await?;
            tx.commit(deadline).await
        })).await
    }
    /// `item` with its imbuement rows when it is a live direct entry of `character_id`'s
    /// main backpack (IMBUE-FORGE0 §3); `None` otherwise. Read-only.
    pub async fn read_imbuement_target(
        &self,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        character_id: CharacterId,
        item: [u8; 16],
    ) -> Result<Option<ImbuementTarget>, DurabilityError> {
        let record = recovery
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = super::db::begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &record).await?;
                    let target = match load_slot(&mut tx, character_id).await? {
                        None => None,
                        Some(backpack) => {
                            let row = sqlx::query(
                                "SELECT i.definition_family, i.definition_production_key, \
                                        i.definition_revision_ref \
                                   FROM game_item_container_entries e \
                                   JOIN game_item_instances i USING (item_instance_id, world_id) \
                                  WHERE e.item_instance_id = encode($1,'hex')::uuid \
                                    AND e.parent_item_instance_id = encode($2,'hex')::uuid \
                                    AND e.character_id = encode($3,'hex')::uuid AND i.lifecycle = 1",
                            )
                            .bind(item.as_slice())
                            .bind(backpack.item_instance_id.as_slice())
                            .bind(character_id.as_bytes().as_slice())
                            .fetch_optional(&mut *tx)
                            .await?;
                            match row {
                                None => None,
                                Some(row) => {
                                    let definition = TypedDefinitionRef {
                                        family: row.try_get("definition_family")?,
                                        production_key: row.try_get("definition_production_key")?,
                                        revision_ref: row.try_get("definition_revision_ref")?,
                                    };
                                    let applied = sqlx::query(
                                        "SELECT slot::bigint, category::bigint, tier::bigint, \
                                                remaining_ms \
                                           FROM game_item_imbuements \
                                          WHERE item_instance_id = encode($1,'hex')::uuid \
                                          ORDER BY slot",
                                    )
                                    .bind(item.as_slice())
                                    .fetch_all(&mut *tx)
                                    .await?
                                    .iter()
                                    .map(|row| {
                                        Ok(StoredImbuement {
                                            slot: stored(row.try_get(0)?)?,
                                            category: stored(row.try_get(1)?)?,
                                            tier: stored(row.try_get(2)?)?,
                                            remaining_ms: stored(row.try_get(3)?)?,
                                        })
                                    })
                                    .collect::<Result<Vec<_>, DurabilityError>>()?;
                                    Some(ImbuementTarget {
                                        definition,
                                        applied,
                                    })
                                }
                            }
                        }
                    };
                    super::db::commit_semantic_transaction(tx, deadline).await?;
                    Ok(target)
                })
            })
            .await
    }
}
