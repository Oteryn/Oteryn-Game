//! The real common PostgreSQL commit boundary for locally proposed spell item
//! and companion source receipts. Its token is historical durable evidence,
//! never a replacement for current physical actor/session/scope authority.
#![allow(
    dead_code,
    reason = "spell import candidate; awaits its production owner caller"
)]
use super::spell_item_transaction::{CommittedCompanionAcquisition, PendingCompanionAcquisition};
use super::spell_item_transaction::{
    CommittedDirectCompanionAcquisition, PendingDirectCompanionAcquisition,
};
use super::spell_items_abi::CommittedSpellItems;
use super::{DurabilityError, db};
use sqlx::{Postgres, Row, Transaction};
use std::time::Instant;

pub(crate) struct PendingSpellOwnerTransaction {
    items: Option<CommittedSpellItems>,
    companion: Option<PendingCompanionAcquisition>,
    direct_companion: Option<PendingDirectCompanionAcquisition>,
    familiar: Option<super::character_familiar::PendingCharacterFamiliar>,
    physical_transaction: String,
    deadline: Instant,
}
pub(crate) struct CommittedSpellOwnerTransaction {
    items: Option<CommittedSpellItems>,
    companion: Option<CommittedCompanionAcquisition>,
    direct_companion: Option<CommittedDirectCompanionAcquisition>,
    familiar: Option<super::character_familiar::CommittedCharacterFamiliar>,
    physical_transaction: String,
}
impl CommittedSpellOwnerTransaction {
    pub(crate) fn items(&self) -> Option<&CommittedSpellItems> {
        self.items.as_ref()
    }
    pub(crate) fn companion(&self) -> Option<&CommittedCompanionAcquisition> {
        self.companion.as_ref()
    }
    pub(crate) fn direct_companion(&self) -> Option<&CommittedDirectCompanionAcquisition> {
        self.direct_companion.as_ref()
    }
    pub(crate) fn physical_transaction(&self) -> &str {
        &self.physical_transaction
    }
    pub(crate) fn familiar(
        &self,
    ) -> Option<&super::character_familiar::CommittedCharacterFamiliar> {
        self.familiar.as_ref()
    }
}

/// Reconciliation observes the same immutable command/event/cost bytes and a
/// committed physical XID. It does not stage a second source or upgrade a row
/// written in the reader's transaction. Current Item authority is checked by
/// the writer before calling this historical-evidence reader.
pub(super) async fn reconcile_committed_spell_owner_transaction(
    tx: &mut Transaction<'_, Postgres>,
    items: CommittedSpellItems,
    companion: Option<CommittedCompanionAcquisition>,
) -> Result<CommittedSpellOwnerTransaction, DurabilityError> {
    reconcile_committed_spell_owner_transaction_with_direct(tx, items, companion, None).await
}
pub(super) async fn reconcile_committed_spell_owner_transaction_with_direct(
    tx: &mut Transaction<'_, Postgres>,
    items: CommittedSpellItems,
    companion: Option<CommittedCompanionAcquisition>,
    direct_companion: Option<CommittedDirectCompanionAcquisition>,
) -> Result<CommittedSpellOwnerTransaction, DurabilityError> {
    let row = sqlx::query("SELECT event_id::text,binding,cost_binding,created_xact_id::text AS physical FROM game_spell_item_receipts WHERE transaction_id=encode($1,'hex')::uuid AND game_session_id=encode($2,'hex')::uuid AND command_id=$3::text::numeric(20,0) AND created_xact_id<>pg_current_xact_id() AND pg_xact_status(created_xact_id)='committed'")
        .bind(items.transaction_id.as_slice()).bind(items.command.game_session_id().as_bytes().as_slice()).bind(items.command.command_id().get().to_string()).fetch_optional(&mut **tx).await?.ok_or(DurabilityError::InvalidStoredState)?;
    let expected_event: String = items.event_id.iter().map(|b| format!("{b:02x}")).collect();
    if row.try_get::<String, _>("event_id")?.replace('-', "") != expected_event
        || row.try_get::<Vec<u8>, _>("binding")?.as_slice() != items.binding
        || row.try_get::<Vec<u8>, _>("cost_binding")?.as_slice() != items.cost_binding
    {
        return Err(DurabilityError::InvalidStoredState);
    }
    let physical_transaction: String = row.try_get("physical")?;
    if let Some(value) = &direct_companion {
        if companion.is_some() {
            return Err(DurabilityError::InvalidStoredState);
        }
        value
            .verify_committed_row(tx, &items.transaction_id, &physical_transaction)
            .await?;
    }
    Ok(CommittedSpellOwnerTransaction {
        physical_transaction,
        items: Some(items),
        companion,
        direct_companion,
        familiar: None,
    })
}

/// Only the actual writer can stage this descriptor. Independently verify its
/// real rows in this physical transaction, including all event/cost bindings.
pub(super) async fn prepare_pending_spell_owner_transaction(
    tx: &mut Transaction<'_, Postgres>,
    items: CommittedSpellItems,
    companion: Option<PendingCompanionAcquisition>,
    familiar: Option<super::character_familiar::PendingCharacterFamiliar>,
    deadline: Instant,
) -> Result<PendingSpellOwnerTransaction, DurabilityError> {
    prepare_pending_spell_owner_transaction_with_direct(
        tx, items, companion, None, familiar, deadline,
    )
    .await
}
pub(super) async fn prepare_pending_spell_owner_transaction_with_direct(
    tx: &mut Transaction<'_, Postgres>,
    items: CommittedSpellItems,
    companion: Option<PendingCompanionAcquisition>,
    direct_companion: Option<PendingDirectCompanionAcquisition>,
    familiar: Option<super::character_familiar::PendingCharacterFamiliar>,
    deadline: Instant,
) -> Result<PendingSpellOwnerTransaction, DurabilityError> {
    if deadline <= Instant::now() || deadline > Instant::now() + db::DB_PASS_DEADLINE {
        return Err(DurabilityError::RootPassDeadlineExceeded);
    }
    let row=sqlx::query("SELECT event_id::text,binding,cost_binding,created_xact_id::text AS physical,pg_current_xact_id()::text AS current FROM game_spell_item_receipts WHERE transaction_id=encode($1,'hex')::uuid AND game_session_id=encode($2,'hex')::uuid AND command_id=$3::text::numeric(20,0)")
        .bind(items.transaction_id.as_slice()).bind(items.command.game_session_id().as_bytes().as_slice()).bind(items.command.command_id().get().to_string()).fetch_optional(&mut **tx).await?.ok_or(DurabilityError::InvalidStoredState)?;
    let physical: String = row.try_get("physical")?;
    let event: String = row.try_get("event_id")?;
    let expected_event: String = items.event_id.iter().map(|b| format!("{b:02x}")).collect();
    if physical != row.try_get::<String, _>("current")?
        || event.replace('-', "") != expected_event
        || row.try_get::<Vec<u8>, _>("binding")?.as_slice() != items.binding
        || row.try_get::<Vec<u8>, _>("cost_binding")?.as_slice() != items.cost_binding
    {
        return Err(DurabilityError::InvalidStoredState);
    }
    if let Some(value) = &companion {
        if !value.matches_transaction(&items.transaction_id) {
            return Err(DurabilityError::InvalidStoredState);
        }
        value.verify_staged_row(tx).await?;
    }
    if let Some(value) = &familiar {
        value.verify_staged_row(tx).await?;
    }
    if let Some(value) = &direct_companion {
        if companion.is_some()
            || familiar.is_some()
            || !value.matches_transaction(&items.transaction_id)
        {
            return Err(DurabilityError::InvalidStoredState);
        }
        value.verify_staged_row(tx).await?;
    }
    // Execute every deferred source/custody check before the final commit. The
    // commit also rechecks constraints; any error rolls back the whole source.
    sqlx::query("SET CONSTRAINTS ALL IMMEDIATE")
        .execute(&mut **tx)
        .await?;
    Ok(PendingSpellOwnerTransaction {
        items: Some(items),
        companion,
        direct_companion,
        familiar,
        physical_transaction: physical,
        deadline,
    })
}

/// Familiar lifecycle transitions have their own genuine source receipt and
/// cannot invent a cast command or a mana/cooldown debit merely to use COMMIT.
pub(crate) async fn stage_familiar_owner_commit(
    tx: &mut Transaction<'_, Postgres>,
    familiar: super::character_familiar::PendingCharacterFamiliar,
    deadline: Instant,
) -> Result<PendingSpellOwnerTransaction, DurabilityError> {
    if deadline <= Instant::now() || deadline > Instant::now() + db::DB_PASS_DEADLINE {
        return Err(DurabilityError::RootPassDeadlineExceeded);
    }
    familiar.verify_staged_row(tx).await?;
    let physical_transaction = sqlx::query_scalar("SELECT pg_current_xact_id()::text")
        .fetch_one(&mut **tx)
        .await?;
    sqlx::query("SET CONSTRAINTS ALL IMMEDIATE")
        .execute(&mut **tx)
        .await?;
    Ok(PendingSpellOwnerTransaction {
        items: None,
        companion: None,
        direct_companion: None,
        familiar: Some(familiar),
        physical_transaction,
        deadline,
    })
}

/// A rejected source callback has genuine immutable result history and no
/// resource debit. Its descriptor is minted only from this physical outbox row.
pub(crate) async fn stage_parameter_result_commit(
    tx: &mut Transaction<'_, Postgres>,
    result: &super::spell_parameter_result::ParameterResultRecord,
    deadline: Instant,
) -> Result<PendingSpellOwnerTransaction, DurabilityError> {
    if deadline <= Instant::now() || deadline > Instant::now() + db::DB_PASS_DEADLINE {
        return Err(DurabilityError::RootPassDeadlineExceeded);
    }
    if result.cost_transaction().is_some() {
        return Err(DurabilityError::InvalidStoredState);
    }
    result.verify_staged_row(tx).await?;
    sqlx::query("SET CONSTRAINTS ALL IMMEDIATE")
        .execute(&mut **tx)
        .await?;
    Ok(PendingSpellOwnerTransaction {
        items: None,
        companion: None,
        direct_companion: None,
        familiar: None,
        physical_transaction: result.physical_transaction().to_owned(),
        deadline,
    })
}

pub(crate) async fn commit_spell_owner_transaction(
    mut tx: Transaction<'_, Postgres>,
    pending: PendingSpellOwnerTransaction,
) -> Result<CommittedSpellOwnerTransaction, DurabilityError> {
    let current: String = sqlx::query_scalar("SELECT pg_current_xact_id()::text")
        .fetch_one(&mut *tx)
        .await?;
    if current != pending.physical_transaction {
        return Err(DurabilityError::InvalidStoredState);
    }
    db::commit_semantic_transaction(tx, pending.deadline).await?;
    // The private token constructor is reached only after an observed
    // successful real COMMIT. Unknown outcome produces no install capability.
    Ok(CommittedSpellOwnerTransaction {
        items: pending.items,
        companion: pending
            .companion
            .map(CommittedCompanionAcquisition::after_successful_commit),
        direct_companion: pending
            .direct_companion
            .map(CommittedDirectCompanionAcquisition::after_successful_commit),
        familiar: pending
            .familiar
            .map(super::character_familiar::PendingCharacterFamiliar::after_successful_commit),
        physical_transaction: pending.physical_transaction,
    })
}
