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
use crate::foundation::{ChannelId, WorldId};
use sqlx::{Postgres, Row, Transaction};
use std::any::Any;
use std::sync::Arc;
use std::time::Instant;

/// The spell lane (ARCH-SPELL-LOCK-2 §1.2): one async mutex per Channel owner, the in-memory
/// mirror of the Channel item advisory lock (key 33). Every in-process key-33 caller takes it
/// first and holds it from before `begin` to after its install or release. It is never acquired
/// while a Channel guard is held.
#[derive(Clone)]
pub(crate) struct SpellLane {
    world_id: WorldId,
    channel_id: ChannelId,
    state: Arc<tokio::sync::Mutex<SpellLaneState>>,
}

/// The lane's own state, inside the lane mutex (§1.6). `unresolved` holds the complete attempt
/// of a committing writer whose `COMMIT` succeeded, or may have, without an install. It is
/// type-erased here because the durability layer is built without the gameplay owners; the
/// gameplay owner parks and resolves its `UnresolvedSpellCommit`. The inner mutex is never
/// contended: it is reached only through the lane guard, and it makes a permit `Sync`.
#[derive(Default)]
struct SpellLaneState {
    unresolved: std::sync::Mutex<Option<Box<dyn Any + Send>>>,
}

impl SpellLaneState {
    fn unresolved(&mut self) -> &mut Option<Box<dyn Any + Send>> {
        self.unresolved
            .get_mut()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

/// Proof that the holder owns the spell lane of one World and Channel. Its only constructor
/// acquires the lane, and it is never handed out while an attempt is parked in `unresolved`.
pub struct SpellLanePermit {
    world_id: WorldId,
    channel_id: ChannelId,
    guard: tokio::sync::OwnedMutexGuard<SpellLaneState>,
}

/// The lane acquired while an attempt is parked in `unresolved`. It yields a permit only through
/// the resolution of that attempt.
pub(crate) struct UnresolvedLane {
    permit: SpellLanePermit,
}

impl SpellLane {
    pub(crate) fn new(world_id: WorldId, channel_id: ChannelId) -> Self {
        Self {
            world_id,
            channel_id,
            state: Arc::default(),
        }
    }

    pub(crate) fn world_id(&self) -> WorldId {
        self.world_id
    }

    pub(crate) fn channel_id(&self) -> ChannelId {
        self.channel_id
    }

    /// Waits for the lane. Returns the permit, or the [`UnresolvedLane`] while a committed or
    /// possibly committed attempt still waits for its install.
    pub(crate) async fn acquire(&self) -> Result<SpellLanePermit, UnresolvedLane> {
        let guard = Arc::clone(&self.state).lock_owned().await;
        let permit = SpellLanePermit {
            world_id: self.world_id,
            channel_id: self.channel_id,
            guard,
        };
        if permit.has_unresolved() {
            Err(UnresolvedLane { permit })
        } else {
            Ok(permit)
        }
    }

    /// A Channel reload from durable truth (§1.6): the in-memory attempt is dropped with the
    /// runtime it belonged to.
    pub(crate) async fn clear_after_reload(&self) {
        *self.state.lock().await.unresolved() = None;
    }
}

/// A held attempt: an unresolved lane's from the park until its resolution takes it, an open
/// commit window's until the consuming call takes it. Both take it only while consuming their
/// owner, so it is never observed absent.
fn held<T>(attempt: Option<T>) -> T {
    match attempt {
        Some(attempt) => attempt,
        None => unreachable!("a held spell attempt is present until its owner is consumed"),
    }
}

impl UnresolvedLane {
    /// The only path from an [`UnresolvedLane`] to a permit: the caller takes the parked attempt
    /// and must install it, release it as proven uncommitted, or park it again.
    pub(crate) fn into_resolution(mut self) -> (SpellLanePermit, Box<dyn Any + Send>) {
        let attempt = self.permit.guard.unresolved().take();
        let attempt = held(attempt);
        (self.permit, attempt)
    }
}

impl SpellLanePermit {
    /// The permit of a fresh lane, which never holds a parked attempt: for the PostgreSQL test
    /// support of the DB-only item writers, which owns no Channel runtime.
    #[cfg(test)]
    #[allow(
        dead_code,
        reason = "used only by the PostgreSQL item-writer test support"
    )]
    pub(crate) async fn of_fresh_scope(
        scope: crate::foundation::RuntimeScopeRefV1,
    ) -> Result<Self, DurabilityError> {
        match scope {
            crate::foundation::RuntimeScopeRefV1::Channel {
                world_id,
                channel_id,
            } => Ok(Self::of_fresh_lane(world_id, channel_id).await),
            _ => Err(DurabilityError::InvalidStoredState),
        }
    }

    #[cfg(test)]
    #[allow(
        dead_code,
        reason = "used only by the PostgreSQL item-writer test support"
    )]
    pub(crate) async fn of_fresh_lane(world_id: WorldId, channel_id: ChannelId) -> Self {
        let lane = SpellLane::new(world_id, channel_id);
        Self {
            world_id,
            channel_id,
            guard: Arc::clone(&lane.state).lock_owned().await,
        }
    }

    pub(crate) fn world_id(&self) -> WorldId {
        self.world_id
    }

    pub(crate) fn channel_id(&self) -> ChannelId {
        self.channel_id
    }

    /// Whether an attempt is parked: the holder's own work must then be refused retryably.
    pub(crate) fn has_unresolved(&self) -> bool {
        self.guard
            .unresolved
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .is_some()
    }

    /// The permit's scope must match the Channel of every row its holder locks or writes. A
    /// permit whose lane holds a parked attempt is refused too: the holder's own work waits for
    /// the resolution.
    pub(crate) fn check_channel(
        &self,
        world_id: WorldId,
        channel_id: ChannelId,
    ) -> Result<(), DurabilityError> {
        if self.has_unresolved() {
            Err(DurabilityError::Unavailable)
        } else if self.world_id == world_id && self.channel_id == channel_id {
            Ok(())
        } else {
            Err(DurabilityError::InvalidStoredState)
        }
    }

    /// [`Self::check_channel`] against the stored identifiers of a Ground row (or of a corpse
    /// entry's Ground root).
    pub(crate) fn check_stored_channel(
        &self,
        world_id: &[u8],
        channel_id: &[u8],
    ) -> Result<(), DurabilityError> {
        if self.has_unresolved() {
            Err(DurabilityError::Unavailable)
        } else if self.world_id.as_bytes().as_slice() == world_id
            && self.channel_id.as_bytes().as_slice() == channel_id
        {
            Ok(())
        } else {
            Err(DurabilityError::InvalidStoredState)
        }
    }

    /// [`Self::check_channel`] against a runtime scope; any other scope is refused.
    pub(crate) fn check_scope(
        &self,
        scope: crate::foundation::RuntimeScopeRefV1,
    ) -> Result<(), DurabilityError> {
        match scope {
            crate::foundation::RuntimeScopeRefV1::Channel {
                world_id,
                channel_id,
            } => self.check_channel(world_id, channel_id),
            _ => Err(DurabilityError::InvalidStoredState),
        }
    }

    /// Opens the commit window of a committing writer around its retained attempt (§1.6).
    pub(crate) fn open_commit_window<T: Send + 'static>(
        &mut self,
        attempt: T,
        park: fn(T) -> Box<dyn Any + Send>,
    ) -> SpellCommitWindow<'_, T> {
        SpellCommitWindow {
            permit: self,
            attempt: Some(attempt),
            park,
            commit_called: false,
        }
    }
}

/// The commit window (§1.6): from the `COMMIT` call on, the permit owns the attempt. It is
/// consumed only by the install, the release of a batch proven uncommitted, or the move into
/// `unresolved`. Dropping it unconsumed parks the attempt, so no early return after the
/// `COMMIT` skips the fence.
pub(crate) struct SpellCommitWindow<'p, T: Send + 'static> {
    permit: &'p mut SpellLanePermit,
    attempt: Option<T>,
    park: fn(T) -> Box<dyn Any + Send>,
    commit_called: bool,
}

impl<T: Send + 'static> SpellCommitWindow<'_, T> {
    /// The permit that owns this window, for the scope checks of the writer's transaction.
    pub(crate) fn permit(&self) -> &SpellLanePermit {
        self.permit
    }

    pub(crate) fn attempt(&self) -> &T {
        held(self.attempt.as_ref())
    }

    pub(crate) fn attempt_mut(&mut self) -> &mut T {
        held(self.attempt.as_mut())
    }

    /// Consumes the window for the infallible install phase.
    pub(crate) fn install(mut self) -> T {
        held(self.attempt.take())
    }

    /// Consumes the window for the release of an attempt proven uncommitted.
    pub(crate) fn release(mut self) -> T {
        held(self.attempt.take())
    }

    /// Whether `commit_spell_owner_transaction` reached the `COMMIT` call through this window.
    pub(crate) fn commit_called(&self) -> bool {
        self.commit_called
    }

    /// Takes the attempt back when no `COMMIT` was called through this window, so the writer's
    /// transaction rolled back and the attempt is proven uncommitted. Otherwise the window is
    /// returned unchanged, to install or park.
    pub(crate) fn reclaim_uncommitted(mut self) -> Result<T, Self> {
        if self.commit_called {
            Err(self)
        } else {
            Ok(held(self.attempt.take()))
        }
    }

    /// A historical receipt proves the attempt's decision is already durable: the window then
    /// owns the attempt exactly as after its own `COMMIT` call.
    pub(crate) fn mark_already_committed(&mut self) {
        self.commit_called = true;
    }

    /// Parks the attempt in the lane's `unresolved` record.
    pub(crate) fn park(self) {}
}

impl<T: Send + 'static> Drop for SpellCommitWindow<'_, T> {
    fn drop(&mut self) {
        if let Some(attempt) = self.attempt.take() {
            *self.permit.guard.unresolved() = Some((self.park)(attempt));
        }
    }
}

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

/// Commits inside the caller's commit window: no committing writer can `COMMIT` outside one.
pub(crate) async fn commit_spell_owner_transaction<T: Send + 'static>(
    mut tx: Transaction<'_, Postgres>,
    pending: PendingSpellOwnerTransaction,
    window: &mut SpellCommitWindow<'_, T>,
) -> Result<CommittedSpellOwnerTransaction, DurabilityError> {
    if window.attempt.is_none() {
        return Err(DurabilityError::InvalidStoredState);
    }
    let current: String = sqlx::query_scalar("SELECT pg_current_xact_id()::text")
        .fetch_one(&mut *tx)
        .await?;
    if current != pending.physical_transaction {
        return Err(DurabilityError::InvalidStoredState);
    }
    window.commit_called = true;
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
