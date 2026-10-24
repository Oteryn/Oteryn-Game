//! MAIL-owned same-transaction Inbox sink (CP D1152, D1158).
//!
//! This is the transaction boundary consumed by a reward owner, not a second
//! mint/claim implementation. The owner freezes its TransactionId, complete item
//! output list and canonical source binding before the first attempt, and writes
//! its claim, fresh items, item bindings and `reward_claim_mint` audit in the
//! transaction borrowed below. MAIL commits that transaction only after every
//! planned Inbox placement succeeds. A source refusal or omitted output rolls
//! back the entire transaction.
//!
//! The native Root opens the type-2 transaction, checks its own recovery proof,
//! locks admission and checks the real session/node/Character fence. Neither a
//! plan, callback nor caller-supplied boolean is authority. The borrowed context
//! cannot be constructed outside this module or retained beyond the native pass.
//!
//! Migration 0101 supplies the narrow privileged lookup/delivery functions and
//! the actual fresh-mint Inbox receipt/audit guards. Production activation stays
//! after #2197 under CP D1356. Migration 0076 alone deliberately cannot execute
//! this path. The existing CHEST backpack commit must never be followed by a
//! separate Inbox commit and described as an atomic reward delivery.

use super::super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::super::db::{
    begin_semantic_transaction, begin_type2_transaction, commit_semantic_transaction,
    lock_admission_relations,
};
use super::super::item_mint_audit::{SelectedType2Tuple, Type2Transaction};
use super::super::item_transfer::{CurrentCharacterItemFence, character_item_fence_is_current};
use super::super::reward_claim_mint_audit::REWARD_CLAIM_MINT_TYPED_CAUSE;
use super::super::runtime_scope_assignment::NodeIncarnationProof;
use super::super::{DurabilityError, DurabilityRoot};
use super::{CharacterInboxDeliveryError, numeric_u64};
use crate::domain::CharacterId;
use crate::foundation::{ChannelId, CommandRef, WorldId};
use sha2::{Digest, Sha256};
use sqlx::{Postgres, Row, Transaction};
use std::future::Future;
use std::pin::Pin;

mod mint;
pub use mint::{
    RewardClaimInboxMintCandidate, RewardClaimInboxMintOutput, RewardClaimInboxMintRequest,
};

/// DAILY-REWARD-0 §4.1: the source's output bound, not an Inbox capacity limit.
pub const REWARD_CLAIM_INBOX_ITEMS_MAX: usize = 20;

/// A frozen source intent, never a native authority capability. The source owner
/// must retain this exact plan across retries and bind its own claim receipt to
/// `source_binding`; MAIL additionally hashes the complete tuple and item order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RewardClaimInboxPlan {
    pub command: CommandRef,
    pub character_id: CharacterId,
    pub world_id: WorldId,
    pub channel_id: ChannelId,
    pub transaction_id: [u8; 16],
    pub source_binding: [u8; 32],
    pub item_instance_ids: Vec<[u8; 16]>,
}

impl RewardClaimInboxPlan {
    fn validate(&self) -> Result<(), RewardClaimInboxError> {
        if self.item_instance_ids.is_empty()
            || self.item_instance_ids.len() > REWARD_CLAIM_INBOX_ITEMS_MAX
            || !uuid_v7(&self.transaction_id)
        {
            return Err(RewardClaimInboxError::InvalidPlan);
        }
        for (index, item) in self.item_instance_ids.iter().enumerate() {
            if !uuid_v7(item) || self.item_instance_ids[..index].contains(item) {
                return Err(RewardClaimInboxError::InvalidPlan);
            }
        }
        Ok(())
    }

    fn binding(&self) -> [u8; 32] {
        let mut digest = Sha256::new();
        digest.update(b"oteryn:reward_claim_mint:inbox-plan:v1\0");
        digest.update(self.command.game_session_id().as_bytes());
        digest.update(self.command.command_id().get().to_be_bytes());
        digest.update(self.character_id.as_bytes());
        digest.update(self.world_id.as_bytes());
        digest.update(self.channel_id.as_bytes());
        digest.update(self.transaction_id);
        digest.update(self.source_binding);
        // The length is validated to 1..=20 before this function is used.
        digest.update((self.item_instance_ids.len() as u32).to_be_bytes());
        for item in &self.item_instance_ids {
            digest.update(item);
        }
        digest.finalize().into()
    }
}

fn uuid_v7(value: &[u8; 16]) -> bool {
    value[6] >> 4 == 7 && value[8] & 0xc0 == 0x80
}

/// The exact permanent location returned by the Inbox owner for audit encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RewardClaimInboxPosition {
    pub item_instance_id: [u8; 16],
    pub character_id: CharacterId,
    pub world_id: WorldId,
    pub ordinal: u64,
}

#[derive(Debug)]
pub enum RewardClaimInboxError {
    InvalidPlan,
    CapacityExceeded,
    /// The source owner refused its claim or item binding in the borrowed
    /// transaction. Fresh mint, Inbox placement and audit must all roll back.
    SourceRefused,
    AuthorityRejected,
    /// An item already has a different delivery binding, or only a proper
    /// subset of this supposedly atomic plan has committed.
    ConflictingDelivery,
    /// The owner omitted an output, delivered an unplanned item, or continued
    /// after a delivery refusal. No source write may commit in those cases.
    IncompleteDelivery,
    /// The delivery does not name a fresh live item of this TransactionId.
    MintNotProven,
    Delivery(CharacterInboxDeliveryError),
    Unavailable(DurabilityError),
}

impl From<DurabilityError> for RewardClaimInboxError {
    fn from(error: DurabilityError) -> Self {
        Self::Unavailable(error)
    }
}

impl From<sqlx::Error> for RewardClaimInboxError {
    fn from(error: sqlx::Error) -> Self {
        Self::Delivery(error.into())
    }
}

impl std::fmt::Display for RewardClaimInboxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPlan => f.write_str("invalid reward Inbox output plan"),
            Self::CapacityExceeded => {
                f.write_str("reward Inbox registered work or size bound exceeded")
            }
            Self::SourceRefused => f.write_str("reward Inbox source refused its claim or binding"),
            Self::AuthorityRejected => f.write_str("reward Inbox authority rejected"),
            Self::ConflictingDelivery => f.write_str("reward Inbox delivery binding conflicts"),
            Self::IncompleteDelivery => f.write_str("reward Inbox output plan is incomplete"),
            Self::MintNotProven => f.write_str("reward Inbox fresh mint is not proven"),
            Self::Delivery(error) => std::fmt::Display::fmt(error, f),
            Self::Unavailable(error) => write!(f, "reward Inbox storage failed: {error:?}"),
        }
    }
}

impl std::error::Error for RewardClaimInboxError {}

#[derive(Debug)]
pub enum RewardClaimInboxOutcome<T> {
    Committed {
        source: T,
        positions: Vec<RewardClaimInboxPosition>,
    },
    /// The original immutable delivery outcome. The source callback is not
    /// called again, and current session authority is not reacquired on replay.
    AlreadyCommitted(Vec<RewardClaimInboxPosition>),
}

/// A borrowed, physically fenced native transaction. Only the Root entrypoint
/// below constructs it, after recovery/admission/session/node/Character checks.
pub struct RewardClaimInboxTransaction<'connection> {
    transaction: Type2Transaction<'connection>,
    plan: RewardClaimInboxPlan,
    binding: [u8; 32],
    physical_xact_id: String,
    backend_pid: i32,
    connection_generation: u64,
    character_lease_generation: u64,
    positions: Vec<Option<RewardClaimInboxPosition>>,
    refused: bool,
}

impl<'connection> RewardClaimInboxTransaction<'connection> {
    /// The source's claim, fresh mint, binding and audit writes use this same
    /// transaction. The source must return an error for every refused value
    /// operation; it must not COMMIT, ROLLBACK or replace the borrowed transaction.
    pub fn transaction(&mut self) -> &mut Transaction<'connection, Postgres> {
        &mut self.transaction
    }

    pub const fn selected_type2_tuple(&self) -> SelectedType2Tuple {
        self.transaction.tuple()
    }

    /// Reuse the existing typed cause. There is no caller-selected Cause variant.
    pub const fn typed_cause(&self) -> &'static str {
        REWARD_CLAIM_MINT_TYPED_CAUSE
    }

    pub const fn plan(&self) -> &RewardClaimInboxPlan {
        &self.plan
    }

    async fn assert_same_transaction(&mut self) -> Result<(), RewardClaimInboxError> {
        let row = sqlx::query(
            "SELECT pg_current_xact_id()::text AS xact_id, pg_backend_pid() AS backend_pid",
        )
        .fetch_one(&mut **self.transaction)
        .await?;
        if row.try_get::<String, _>("xact_id")? != self.physical_xact_id
            || row.try_get::<i32, _>("backend_pid")? != self.backend_pid
        {
            self.refused = true;
            return Err(RewardClaimInboxError::AuthorityRejected);
        }
        Ok(())
    }

    /// Place one planned fresh item. Repeating this call in the same transaction
    /// returns the first ordinal without another counter increment. A refusal
    /// poisons the context even if the source catches the returned error.
    pub async fn deliver(
        &mut self,
        item_instance_id: [u8; 16],
    ) -> Result<RewardClaimInboxPosition, RewardClaimInboxError> {
        let result = self.deliver_inner(item_instance_id).await;
        if result.is_err() {
            self.refused = true;
        }
        result
    }

    async fn deliver_inner(
        &mut self,
        item_instance_id: [u8; 16],
    ) -> Result<RewardClaimInboxPosition, RewardClaimInboxError> {
        if self.refused {
            return Err(RewardClaimInboxError::IncompleteDelivery);
        }
        self.assert_same_transaction().await?;
        let Some(index) = self
            .plan
            .item_instance_ids
            .iter()
            .position(|item| *item == item_instance_id)
        else {
            return Err(RewardClaimInboxError::IncompleteDelivery);
        };
        if let Some(position) = self.positions[index] {
            return Ok(position);
        }
        let row = sqlx::query(
            "SELECT character_id::text AS character_id, ordinal::text AS ordinal \
             FROM game_character_inbox_reward_claim_mint_deliver(\
               encode($1,'hex')::uuid, encode($2,'hex')::uuid, encode($3,'hex')::uuid, \
               encode($4,'hex')::uuid, $5, encode($6,'hex')::uuid, $7::text::numeric, $8)",
        )
        .bind(item_instance_id.as_slice())
        .bind(self.plan.character_id.as_bytes().as_slice())
        .bind(self.plan.world_id.as_bytes().as_slice())
        .bind(self.plan.transaction_id.as_slice())
        .bind(self.binding.as_slice())
        .bind(self.plan.command.game_session_id().as_bytes().as_slice())
        .bind(self.plan.command.command_id().get().to_string())
        .bind(self.plan.item_instance_ids.len() as i16)
        .fetch_optional(&mut **self.transaction)
        .await?
        .ok_or(RewardClaimInboxError::MintNotProven)?;
        if super::uuid_text(&row.try_get::<String, _>("character_id")?)?
            != *self.plan.character_id.as_bytes()
        {
            return Err(DurabilityError::InvalidStoredState.into());
        }
        let ordinal = numeric_u64(&row, "ordinal")?;
        if ordinal == 0 {
            return Err(DurabilityError::InvalidStoredState.into());
        }
        let position = RewardClaimInboxPosition {
            item_instance_id,
            character_id: self.plan.character_id,
            world_id: self.plan.world_id,
            ordinal,
        };
        self.positions[index] = Some(position);
        Ok(position)
    }
}

impl DurabilityRoot {
    /// Compose a source reward and its Inbox outputs inside one native Root
    /// pass and one PostgreSQL transaction. `owner` is borrowed real owner
    /// context, following the native `run_with_context` boundary.
    ///
    /// The source must have reserved the exact plan and retry budget before
    /// entering. This function neither manufactures a reservation nor mints an
    /// item independently. Only a complete, matching immutable delivery journal
    /// permits replay; a changed plan or partial journal is refused.
    pub async fn with_reward_claim_inbox_transaction<T, C, F>(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        plan: &RewardClaimInboxPlan,
        owner: &mut C,
        operation: F,
    ) -> Result<RewardClaimInboxOutcome<T>, RewardClaimInboxError>
    where
        T: Send + 'static,
        C: Send,
        F: for<'a, 'connection> FnOnce(
                &'a mut RewardClaimInboxTransaction<'connection>,
                &'a mut C,
            ) -> Pin<
                Box<dyn Future<Output = Result<T, RewardClaimInboxError>> + Send + 'a>,
            > + Send
            + 'static,
    {
        plan.validate()?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| RewardClaimInboxError::AuthorityRejected)?;
        let plan = plan.clone();
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run_with_context(owner, move |holder, deadline, owner| {
                Box::pin(async move {
                    let mut tx = begin_type2_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    lock_reward_claim_cause(&mut tx, plan.command).await?;
                    let binding = plan.binding();
                    match lookup_plan(&mut tx, &plan, &binding).await? {
                        Ok(Some(positions)) => {
                            tx.commit(deadline).await?;
                            return Ok(Ok(RewardClaimInboxOutcome::AlreadyCommitted(positions)));
                        }
                        Ok(None) => {}
                        Err(error) => return Ok(Err(error)),
                    }
                    if !character_item_fence_is_current(
                        &mut tx,
                        &node,
                        &fence,
                        plan.command,
                        plan.character_id,
                        *plan.world_id.as_bytes(),
                        *plan.channel_id.as_bytes(),
                    )
                    .await?
                    {
                        return Ok(Err(RewardClaimInboxError::AuthorityRejected));
                    }
                    let row = sqlx::query(
                        "SELECT pg_current_xact_id()::text AS xact_id, pg_backend_pid() AS backend_pid",
                    )
                    .fetch_one(&mut **tx)
                    .await?;
                    let positions = vec![None; plan.item_instance_ids.len()];
                    let mut context = RewardClaimInboxTransaction {
                        transaction: tx,
                        plan,
                        binding,
                        physical_xact_id: row.try_get("xact_id")?,
                        backend_pid: row.try_get("backend_pid")?,
                        connection_generation: fence.connection_generation.get(),
                        character_lease_generation: fence.character_lease_generation,
                        positions,
                        refused: false,
                    };
                    let source = match operation(&mut context, owner).await {
                        Ok(source) => source,
                        Err(error) => return Ok(Err(error)),
                    };
                    if context.refused || context.positions.iter().any(Option::is_none) {
                        return Ok(Err(RewardClaimInboxError::IncompleteDelivery));
                    }
                    if let Err(error) = context.assert_same_transaction().await {
                        return Ok(Err(error));
                    }
                    // Evaluate the owner's and Inbox's deferred atomicity guards
                    // before COMMIT, preserving typed Inbox SQLSTATE refusals.
                    if let Err(error) = sqlx::query("SET CONSTRAINTS ALL IMMEDIATE")
                        .execute(&mut **context.transaction)
                        .await
                    {
                        return Ok(Err(error.into()));
                    }
                    let positions = context.positions.into_iter().flatten().collect();
                    context.transaction.commit(deadline).await?;
                    Ok(Ok(RewardClaimInboxOutcome::Committed { source, positions }))
                })
            })
            .await?
    }

    /// Resolve an unknown commit using this Root's current recovery proof and
    /// the complete frozen plan. No value is written and no session fence is
    /// reacquired. A partial or changed delivery journal is never success.
    pub async fn reconcile_reward_claim_inbox(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        plan: &RewardClaimInboxPlan,
    ) -> Result<Option<Vec<RewardClaimInboxPosition>>, RewardClaimInboxError> {
        plan.validate()?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| RewardClaimInboxError::AuthorityRejected)?;
        let plan = plan.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    lock_reward_claim_cause(&mut tx, plan.command).await?;
                    let result = lookup_plan(&mut tx, &plan, &plan.binding()).await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(result)
                })
            })
            .await?
    }
}

async fn lookup_plan(
    tx: &mut Transaction<'_, Postgres>,
    plan: &RewardClaimInboxPlan,
    binding: &[u8; 32],
) -> Result<Result<Option<Vec<RewardClaimInboxPosition>>, RewardClaimInboxError>, DurabilityError> {
    // The indexed CommandRef lookup is bounded to source maximum + one. It
    // finds a conflicting earlier plan even when *all* requested item IDs have
    // changed. Looking up only the new IDs would incorrectly treat that as new.
    let rows = sqlx::query(
        "SELECT disposition, item_instance_id::text AS item_instance_id, ordinal::text AS ordinal \
         FROM game_character_inbox_reward_claim_mint_lookup(\
           encode($1,'hex')::uuid, encode($2,'hex')::uuid, encode($3,'hex')::uuid, \
           $4, encode($5,'hex')::uuid, $6::text::numeric, $7)",
    )
    .bind(plan.character_id.as_bytes().as_slice())
    .bind(plan.world_id.as_bytes().as_slice())
    .bind(plan.transaction_id.as_slice())
    .bind(binding.as_slice())
    .bind(plan.command.game_session_id().as_bytes().as_slice())
    .bind(plan.command.command_id().get().to_string())
    .bind(plan.item_instance_ids.len() as i16)
    .fetch_all(&mut **tx)
    .await?;
    if rows.is_empty() {
        return Ok(Ok(None));
    }
    if rows.len() != plan.item_instance_ids.len() {
        return Ok(Err(RewardClaimInboxError::ConflictingDelivery));
    }
    let mut positions = vec![None; plan.item_instance_ids.len()];
    for row in rows {
        match row.try_get::<i16, _>("disposition")? {
            1 => {
                let item = super::uuid_text(&row.try_get::<String, _>("item_instance_id")?)?;
                let Some(index) = plan
                    .item_instance_ids
                    .iter()
                    .position(|expected| *expected == item)
                else {
                    return Ok(Err(RewardClaimInboxError::ConflictingDelivery));
                };
                let ordinal = numeric_u64(&row, "ordinal")?;
                if ordinal == 0 || positions[index].is_some() {
                    return Err(DurabilityError::InvalidStoredState);
                }
                positions[index] = Some(RewardClaimInboxPosition {
                    item_instance_id: item,
                    character_id: plan.character_id,
                    world_id: plan.world_id,
                    ordinal,
                });
            }
            2 => return Ok(Err(RewardClaimInboxError::ConflictingDelivery)),
            _ => return Err(DurabilityError::InvalidStoredState),
        }
    }
    if positions.iter().all(Option::is_some) {
        Ok(Ok(Some(positions.into_iter().flatten().collect())))
    } else {
        Ok(Err(RewardClaimInboxError::ConflictingDelivery))
    }
}

/// The exact existing reward_claim_mint CommandRef lock, before the Character
/// root lock. This is the CHEST writer's key, not a new advisory-lock namespace.
async fn lock_reward_claim_cause(
    tx: &mut Transaction<'_, Postgres>,
    command: CommandRef,
) -> Result<(), DurabilityError> {
    let mut key = Vec::with_capacity(24);
    key.extend_from_slice(command.game_session_id().as_bytes());
    key.extend_from_slice(&command.command_id().get().to_be_bytes());
    sqlx::query(
        "SELECT pg_advisory_xact_lock(hashtextextended(\
         'oteryn:reward-claim-mint:' || encode($1, 'hex'), 0))",
    )
    .bind(key.as_slice())
    .execute(&mut **tx)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests;
