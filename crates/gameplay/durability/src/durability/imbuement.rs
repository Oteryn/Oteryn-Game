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
use super::character_progression::{CurrentCharacterGameplayFence, assert_item_only_fence};
use super::item_mint::TypedDefinitionRef;
use super::item_transfer::load_slot;
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use crate::domain::CharacterId;
use sqlx::Row;

/// A stored equipped lifetime. An occurrence survives transfers and identifies exactly
/// one application, even if a cleared slot is immediately reused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EquippedImbuement {
    pub item: [u8; 16],
    pub slot: u8,
    pub category: u8,
    pub tier: u8,
    pub remaining_ms: u32,
    pub occurrence: [u8; 16],
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImbuementElapsed {
    pub item: String,
    pub occurrence: String,
    pub before_ms: u32,
    pub after_ms: u32,
}

#[derive(Debug, Clone)]
pub struct ImbuementCheckpoint {
    pub transaction_id: [u8; 16],
    pub changes: Vec<ImbuementElapsed>,
}

fn decode_uuid(text: String) -> Result<[u8; 16], DurabilityError> {
    let hex = text.replace('-', "");
    let mut bytes = [0; 16];
    if hex.len() != 32 {
        return Err(DurabilityError::InvalidStoredState);
    }
    for (i, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
    }
    Ok(bytes)
}

fn uuid_text(bytes: &[u8; 16]) -> String {
    let h: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &h[..8],
        &h[8..12],
        &h[12..16],
        &h[16..20],
        &h[20..]
    )
}

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
    /// Read the ten worn positions, including the main backpack (which has a separate
    /// custody owner), under current item-only session and root fences.
    pub async fn read_worn_imbuements(
        &self,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
    ) -> Result<Vec<EquippedImbuement>, DurabilityError> {
        let record = recovery
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = super::db::begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &record).await?;
            if assert_item_only_fence(&mut tx, &fence.item_fence(), &node).await?.is_err() {
                return Err(DurabilityError::Unavailable);
            }
            let rows = sqlx::query(
                "WITH worn AS (SELECT item_instance_id,world_id FROM game_character_equipment_slots WHERE character_id=encode($1,'hex')::uuid UNION ALL SELECT item_instance_id,world_id FROM game_item_container_slots WHERE character_id=encode($1,'hex')::uuid) SELECT b.item_instance_id::text,b.slot::bigint,b.category::bigint,b.tier::bigint,b.remaining_ms,b.applying_occurrence_id::text FROM worn e JOIN game_item_instances i USING(item_instance_id) JOIN game_item_imbuements b USING(item_instance_id) WHERE i.lifecycle=1 AND i.world_id=encode($2,'hex')::uuid AND e.world_id=i.world_id ORDER BY b.item_instance_id,b.slot LIMIT 31"
            ).bind(fence.character_id.as_bytes().as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice())
                .fetch_all(&mut *tx).await?;
            if rows.len()>30 { return Err(DurabilityError::InvalidStoredState); }
            let rows = rows.iter().map(|b| Ok(EquippedImbuement {
                item: decode_uuid(b.try_get(0)?)?, slot: stored(b.try_get(1)?)?,
                category: stored(b.try_get(2)?)?, tier: stored(b.try_get(3)?)?,
                remaining_ms: stored(b.try_get(4)?)?, occurrence: decode_uuid(b.try_get(5)?)?,
            })).collect::<Result<Vec<_>, DurabilityError>>()?;
            super::db::commit_semantic_transaction(tx, deadline).await?;
            Ok(rows)
        })).await
    }

    /// Commit one bounded elapsed checkpoint. Its immutable receipt binds every lifetime
    /// and before/after budget; an ambiguous retry never debits a replacement occurrence.
    pub async fn checkpoint_imbuements(
        &self,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: ImbuementCheckpoint,
    ) -> Result<(), DurabilityError> {
        if request.changes.is_empty() || request.changes.len() > 30 {
            return Err(DurabilityError::InvalidStoredState);
        }
        let record = recovery
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = super::db::begin_type2_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &record).await?;
            if assert_item_only_fence(&mut tx, &fence.item_fence(), &node).await?.is_err() {
                return Err(DurabilityError::Unavailable);
            }
            let changes = serde_json::to_value(&request.changes).map_err(|_| DurabilityError::InvalidStoredState)?;
            let existing: Option<(String, String, serde_json::Value)> = sqlx::query_as(
                "SELECT game_session_id::text, character_id::text, changes FROM game_item_imbuement_checkpoints WHERE transaction_id=encode($1,'hex')::uuid"
            ).bind(request.transaction_id.as_slice()).fetch_optional(&mut **tx).await?;
            if let Some((session, character, original)) = existing {
                if session != uuid_text(fence.game_session_id.as_bytes()) || character != uuid_text(fence.character_id.as_bytes()) || original != changes {
                    return Err(DurabilityError::InvalidStoredState);
                }
            } else {
                sqlx::query("INSERT INTO game_item_imbuement_checkpoints(transaction_id,game_session_id,character_id,world_id,changes) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5)")
                    .bind(request.transaction_id.as_slice()).bind(fence.game_session_id.as_bytes().as_slice())
                    .bind(fence.character_id.as_bytes().as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice())
                    .bind(changes).execute(&mut **tx).await?;
            }
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
