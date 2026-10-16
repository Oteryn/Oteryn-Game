//! IMBUE-FORGE-0 §§4.1,4.3,15: checkpoint planning and PRIVATE in-transaction storage.
//! Remaining-time inputs are DATA, never proof that time elapsed. No Root entrypoint, ticking,
//! effect activation, applying/clearing/expiry or production schema/grant is installed here.
//! An admitted elapsed owner must eventually compose this primitive with its own provenance.

use crate::domain::ItemInstanceId;
use crate::domain::imbuement::{MAX_SLOTS, SlotRecord};
use std::collections::{BTreeMap, BTreeSet};

/// IMBFORGE0-RL-13 and DUR03-RL-06-IMBUE-CHECKPOINT-{PARTICIPANTS,EFFECT-WORK-UNITS}.
pub(crate) const MAX_CHECKPOINT_ITEMS: usize = 10;
pub(crate) const MAX_CHECKPOINT_WORK: usize = 40;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CheckpointError {
    BindingChanged,
    TimeIncreased,
    DuplicateSlot,
    DuplicateCategory,
    BudgetExceeded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SlotCheckpoint {
    pub(crate) before: SlotRecord,
    pub(crate) after: SlotRecord,
}

/// Numeric ceilings only; passing them is not audit-shape/current-authority validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CheckpointUsage {
    pub(crate) items: usize,
    pub(crate) participants: usize,
    pub(crate) work: usize,
}

impl CheckpointUsage {
    pub(crate) fn check(self) -> Result<(), CheckpointError> {
        if self.items > MAX_CHECKPOINT_ITEMS
            || self.participants > MAX_CHECKPOINT_ITEMS
            || self.work > MAX_CHECKPOINT_WORK
        {
            Err(CheckpointError::BudgetExceeded)
        } else {
            Ok(())
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CheckpointItem {
    pub(crate) item: ItemInstanceId,
    pub(crate) changes: Vec<SlotCheckpoint>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CheckpointBatch {
    pub(crate) items: Vec<CheckpointItem>,
    pub(crate) usage: CheckpointUsage,
}

impl CheckpointBatch {
    /// Groups/sorts one bounded transaction's input; it does not split or commit a character's
    /// entire interval. Equal snapshots are omitted; zero remaining is an update, never deletion.
    pub(crate) fn plan(changes: Vec<SlotCheckpoint>) -> Result<Self, CheckpointError> {
        if changes.len() > MAX_CHECKPOINT_ITEMS * MAX_SLOTS {
            return Err(CheckpointError::BudgetExceeded);
        }
        let mut grouped: BTreeMap<ItemInstanceId, Vec<SlotCheckpoint>> = BTreeMap::new();
        let mut slots = BTreeSet::new();
        let mut categories = BTreeSet::new();
        for change in changes {
            let binding = change.before.binding();
            if binding != change.after.binding() {
                return Err(CheckpointError::BindingChanged);
            }
            if change.after.remaining_ms() > change.before.remaining_ms() {
                return Err(CheckpointError::TimeIncreased);
            }
            if !slots.insert((binding.item, binding.slot)) {
                return Err(CheckpointError::DuplicateSlot);
            }
            if !categories.insert((binding.item, binding.category.clone())) {
                return Err(CheckpointError::DuplicateCategory);
            }
            grouped.entry(binding.item).or_default().push(change);
        }
        if grouped.len() > MAX_CHECKPOINT_ITEMS {
            return Err(CheckpointError::BudgetExceeded);
        }
        let mut items = Vec::new();
        let mut work = 0;
        for (item, mut changes) in grouped {
            changes.retain(|change| change.before != change.after);
            if changes.is_empty() {
                continue;
            }
            changes.sort_by_key(|change| change.before.binding().slot);
            work += 1 + changes.len();
            items.push(CheckpointItem { item, changes });
        }
        let usage = CheckpointUsage {
            items: items.len(),
            participants: items.len(),
            work,
        };
        usage.check()?;
        Ok(Self { items, usage })
    }
}

use super::character_equipment::{
    EquipmentAuthority, EquipmentError, EquipmentSnapshot, read_equipment_in_transaction,
};
use crate::domain::imbuement::{SlotBinding, Tier};
use sqlx::{Acquire, Postgres, Row, Transaction};

#[derive(Debug)]
pub(super) enum CheckpointStorageError {
    InvalidBatch,
    NotEquipped,
    Stale,
    Equipment(EquipmentError),
    Database(sqlx::Error),
}
impl From<sqlx::Error> for CheckpointStorageError {
    fn from(error: sqlx::Error) -> Self {
        Self::Database(error)
    }
}
impl From<EquipmentError> for CheckpointStorageError {
    fn from(error: EquipmentError) -> Self {
        Self::Equipment(error)
    }
}
type StorageResult<T> = Result<T, CheckpointStorageError>;

/// Complete before DATA, not elapsed-time or current-authority evidence. The native opaque
/// equipment authority must be independently obtained in EACH physical transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CheckpointSnapshot {
    equipment: EquipmentSnapshot,
    items: Vec<ItemInstanceId>,
    rows: Vec<SlotRecord>,
}
impl CheckpointSnapshot {
    pub(super) fn rows(&self) -> &[SlotRecord] {
        &self.rows
    }
}

/// Same-transaction native equipment read, then deterministic item/slot row locks. Includes
/// every slot of each selected item so a new/rebound otherwise-unchanged row also invalidates
/// a later before snapshot. No row means empty; zero remaining is still a retained row.
pub(super) async fn load_checkpoint_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &EquipmentAuthority,
    items: &[ItemInstanceId],
) -> StorageResult<CheckpointSnapshot> {
    if items.len() > MAX_CHECKPOINT_ITEMS {
        return Err(CheckpointStorageError::InvalidBatch);
    }
    let mut ordered = items.to_vec();
    ordered.sort();
    if ordered.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(CheckpointStorageError::InvalidBatch);
    }
    // This real native reader checks the authority's physical transaction and reads/locks
    // current equipment state/items. It does not accept item IDs as custody grants.
    let equipment = read_equipment_in_transaction(tx, authority).await?;
    let mut rows = Vec::new();
    for item in &ordered {
        if !equipment
            .items
            .iter()
            .any(|entry| entry.item_instance_id == *item.as_bytes())
        {
            return Err(CheckpointStorageError::NotEquipped);
        }
        let stored = sqlx::query("SELECT slot,imbuement_key,content_revision,category,tier,remaining_ms,uuid_send(applying_occurrence) AS applying_occurrence,uuid_send(transaction_id) AS transaction_id FROM game_item_imbuements WHERE item_instance_id=encode($1,'hex')::uuid ORDER BY slot FOR UPDATE")
            .bind(item.as_bytes().as_slice()).fetch_all(&mut **tx).await?;
        if stored.len() > MAX_SLOTS {
            return Err(CheckpointStorageError::Stale);
        }
        for record in stored {
            let identifier = |name| -> StorageResult<[u8; 16]> {
                record
                    .try_get::<Vec<u8>, _>(name)?
                    .try_into()
                    .map_err(|_| CheckpointStorageError::Stale)
            };
            let tier = match record.try_get::<i16, _>("tier")? {
                1 => Tier::Basic,
                2 => Tier::Intricate,
                3 => Tier::Powerful,
                _ => return Err(CheckpointStorageError::Stale),
            };
            let binding = SlotBinding {
                item: *item,
                slot: u8::try_from(record.try_get::<i16, _>("slot")?)
                    .map_err(|_| CheckpointStorageError::Stale)?,
                key: record.try_get("imbuement_key")?,
                content_revision: record.try_get("content_revision")?,
                category: record.try_get("category")?,
                tier,
                applying_occurrence: identifier("applying_occurrence")?,
                transaction_id: identifier("transaction_id")?,
            };
            let remaining = u64::try_from(record.try_get::<i64, _>("remaining_ms")?)
                .map_err(|_| CheckpointStorageError::Stale)?;
            rows.push(
                SlotRecord::new(binding, remaining).map_err(|_| CheckpointStorageError::Stale)?,
            );
        }
    }
    Ok(CheckpointSnapshot {
        equipment,
        items: ordered,
        rows,
    })
}

fn checked_storage_batch(batch: &CheckpointBatch) -> StorageResult<()> {
    // Batch fields are intentionally DATA and can be assembled by another crate module.
    // Re-plan rather than treating the supplied usage/sorting as a trusted proof.
    if batch.items.len() > MAX_CHECKPOINT_ITEMS
        || batch
            .items
            .iter()
            .any(|item| item.changes.len() > MAX_SLOTS)
    {
        return Err(CheckpointStorageError::InvalidBatch);
    }
    let changes = batch
        .items
        .iter()
        .flat_map(|item| item.changes.iter().cloned())
        .collect();
    let planned =
        CheckpointBatch::plan(changes).map_err(|_| CheckpointStorageError::InvalidBatch)?;
    if &planned != batch {
        return Err(CheckpointStorageError::InvalidBatch);
    }
    Ok(())
}

/// PRIVATE downward storage CAS over explicitly supplied remaining-time DATA. This function
/// is not an admission boundary: there is no public Root commit/tick path. It must not be
/// called by production until the actual elapsed owner is admitted and composed at the CP slot.
/// Native current equipment authority is necessary, but cannot justify caller-chosen elapsed
/// time. A savepoint makes any refusal/error atomic even when a compositor retains its outer TX.
/// The planner omits unchanged items, so the write set may be a strict subset of the loaded
/// items (including empty). The complete before snapshot is still re-read and compared:
/// unchanged/zero-time rows cannot hide a stale application or equipment binding.
pub(super) async fn store_checkpoint_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &EquipmentAuthority,
    before: &CheckpointSnapshot,
    batch: &CheckpointBatch,
) -> StorageResult<()> {
    checked_storage_batch(batch)?;
    if batch
        .items
        .iter()
        .any(|item| !before.items.contains(&item.item))
    {
        return Err(CheckpointStorageError::InvalidBatch);
    }
    let mut savepoint = tx.begin().await?;
    let result = store_checkpoint_rows(&mut savepoint, authority, before, batch).await;
    match result {
        Ok(()) => {
            savepoint.commit().await?;
            Ok(())
        }
        Err(error) => {
            savepoint.rollback().await?;
            Err(error)
        }
    }
}

async fn store_checkpoint_rows(
    tx: &mut Transaction<'_, Postgres>,
    authority: &EquipmentAuthority,
    before: &CheckpointSnapshot,
    batch: &CheckpointBatch,
) -> StorageResult<()> {
    let current = load_checkpoint_in_transaction(tx, authority, &before.items).await?;
    if &current != before {
        return Err(CheckpointStorageError::Stale);
    }
    // Check every row before the first UPDATE. Source locks stay held until the enclosing
    // transaction finishes; complete equality includes the original applying IDs/revision.
    for item in &batch.items {
        for change in &item.changes {
            if !current.rows.contains(&change.before) {
                return Err(CheckpointStorageError::Stale);
            }
        }
    }
    for item in &batch.items {
        for change in &item.changes {
            let binding = change.before.binding();
            let tier = match binding.tier {
                Tier::Basic => 1_i16,
                Tier::Intricate => 2,
                Tier::Powerful => 3,
            };
            let count = sqlx::query("UPDATE game_item_imbuements SET remaining_ms=$10 WHERE item_instance_id=encode($1,'hex')::uuid AND slot=$2 AND imbuement_key=$3 AND content_revision=$4 AND category=$5 AND tier=$6 AND applying_occurrence=encode($7,'hex')::uuid AND transaction_id=encode($8,'hex')::uuid AND remaining_ms=$9 AND $10<=remaining_ms")
                .bind(binding.item.as_bytes().as_slice()).bind(i16::from(binding.slot))
                .bind(&binding.key).bind(&binding.content_revision).bind(&binding.category).bind(tier)
                .bind(binding.applying_occurrence.as_slice()).bind(binding.transaction_id.as_slice())
                .bind(i64::try_from(change.before.remaining_ms()).map_err(|_| CheckpointStorageError::InvalidBatch)?)
                .bind(i64::try_from(change.after.remaining_ms()).map_err(|_| CheckpointStorageError::InvalidBatch)?)
                .execute(&mut **tx).await?.rows_affected();
            if count != 1 {
                return Err(CheckpointStorageError::Stale);
            }
        }
    }
    Ok(())
}

/// UNINSTALLED private owner proposal/isolated fixture only. No runtime role is granted
/// anything here. Initial rows are privileged fixture DATA, not an apply or timing proof.
/// The future audited apply/clear/expiry owners must compose their closed proof guards with
/// this monotone binding guard; this component cannot insert or delete through any API.
pub(super) const IMBUEMENT_CHECKPOINT_SCHEMA_PROPOSAL: &str = r#"
CREATE TABLE game_item_imbuements(
 item_instance_id UUID NOT NULL REFERENCES game_item_instances,
 slot SMALLINT NOT NULL CHECK(slot BETWEEN 0 AND 2),
 imbuement_key TEXT NOT NULL CHECK(length(btrim(imbuement_key))>0),
 content_revision TEXT NOT NULL CHECK(length(btrim(content_revision))>0),
 category TEXT NOT NULL CHECK(length(btrim(category))>0),
 tier SMALLINT NOT NULL CHECK(tier BETWEEN 1 AND 3),
 remaining_ms BIGINT NOT NULL CHECK(remaining_ms BETWEEN 0 AND 72000000),
 applying_occurrence UUID NOT NULL CHECK(game_character_is_uuid_v7(applying_occurrence)),
 transaction_id UUID NOT NULL CHECK(game_character_is_uuid_v7(transaction_id)),
 PRIMARY KEY(item_instance_id,slot),UNIQUE(item_instance_id,category));
CREATE FUNCTION game_imbuement_checkpoint_guard() RETURNS trigger LANGUAGE plpgsql AS $$BEGIN
 IF TG_OP<>'UPDATE' THEN
  RAISE EXCEPTION 'checkpoint cannot delete retained imbuement rows' USING ERRCODE='23514';
 END IF;
 IF (to_jsonb(NEW)-'remaining_ms')<>(to_jsonb(OLD)-'remaining_ms') OR NEW.remaining_ms>OLD.remaining_ms THEN
  RAISE EXCEPTION 'checkpoint preserves binding and only decreases remaining' USING ERRCODE='23514';
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_imbuement_checkpoint_guard BEFORE UPDATE OR DELETE ON game_item_imbuements
 FOR EACH ROW EXECUTE FUNCTION game_imbuement_checkpoint_guard();
CREATE FUNCTION game_imbuement_no_truncate() RETURNS trigger LANGUAGE plpgsql AS $$BEGIN
 RAISE EXCEPTION 'retained imbuement rows cannot be truncated' USING ERRCODE='23514';END $$;
CREATE TRIGGER game_imbuement_no_truncate BEFORE TRUNCATE ON game_item_imbuements
 FOR EACH STATEMENT EXECUTE FUNCTION game_imbuement_no_truncate();
REVOKE ALL ON game_item_imbuements FROM PUBLIC;
REVOKE ALL ON FUNCTION game_imbuement_checkpoint_guard(),game_imbuement_no_truncate() FROM PUBLIC;
"#;

#[cfg(test)]
#[path = "imbuement_tests.rs"]
mod tests;
