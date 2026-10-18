//! D1176 native purchase participant. Upper-layer operations consume this port;
//! this module never imports a Premium policy, NPC service or purchase receipt.
//! The revision sequencer builds the owner under a held slot; the Premium gate and the NPC
//! service live in the upper layer.

use std::future::Future;
use std::pin::Pin;

use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence, assert_gameplay_fence, numeric_u64,
    state_matches_root,
};
use super::db::{begin_type2_transaction, lock_admission_relations};
use super::item_mint_audit::Type2Transaction;
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use crate::character_recovery_fence::CharacterRecoveryFenceV1;
use crate::domain::CharacterId;
use sqlx::Row;

/// Boxed future an [`NpcPurchaseOperation`] returns from inside the held transaction.
pub(crate) type NpcPurchaseFuture<'a, T, E> =
    Pin<Box<dyn Future<Output = Result<T, E>> + Send + 'a>>;

/// A source operation runs in the already-open native Type2 transaction. Only
/// the outer owner can commit; a typed refusal rolls back all callback writes.
pub(crate) trait NpcPurchaseOperation: Send {
    type Output: Send;
    type Error: From<DurabilityError> + Send;

    fn apply<'a, 't>(
        self,
        context: &'a NpcPurchaseWriteContext,
        tx: &'a mut Type2Transaction<'t>,
    ) -> NpcPurchaseFuture<'a, Self::Output, Self::Error>
    where
        Self: 'a;
}

/// Constructed only inside durability from the reconciled native root. The XID
/// prevents a context admitted on one transaction being reused by another one.
/// It is not a Premium status, a reusable purchase permit or live NPC evidence.
pub(crate) struct NpcPurchaseWriteContext {
    recovery: CharacterRecoveryFenceV1,
    node: NodeIncarnationProof,
    character_id: CharacterId,
    xid: String,
}

impl NpcPurchaseWriteContext {
    async fn check_transaction(
        &self,
        tx: &mut Type2Transaction<'_>,
    ) -> Result<(), DurabilityError> {
        let xid: String = sqlx::query_scalar("SELECT pg_current_xact_id()::text")
            .fetch_one(&mut ***tx)
            .await?;
        if xid != self.xid {
            return Err(DurabilityError::InvalidStoredState);
        }
        Ok(())
    }

    /// Historical receipt replay checks recovery without reacquiring gameplay
    /// authority. Fresh writes call `admit` only after the receipt lookup misses.
    pub(crate) async fn check_recovery(
        &self,
        tx: &mut Type2Transaction<'_>,
    ) -> Result<(), DurabilityError> {
        self.check_transaction(tx).await?;
        assert_recovery_fence(tx, &self.recovery).await
    }

    pub(crate) async fn admit(
        &self,
        tx: &mut Type2Transaction<'_>,
        fence: CurrentCharacterGameplayFence,
        account_id: [u8; 16],
        content_revision: &str,
    ) -> Result<(), CharacterProgressionError> {
        self.check_recovery(tx).await?;
        if fence.character_id != self.character_id {
            return Err(CharacterProgressionError::AuthorityRejected);
        }
        lock_admission_relations(tx).await?;
        let root = assert_gameplay_fence(tx, &fence, &self.node).await??;
        // The root is locked by the real gameplay fence. The request's account
        // is compared to that current root; it is never authority by itself.
        let account_matches: bool = sqlx::query_scalar(
            "SELECT account_id=encode($2,'hex')::uuid FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
        )
        .bind(fence.character_id.as_bytes().as_slice())
        .bind(account_id.as_slice())
        .fetch_one(&mut ***tx)
        .await
        .map_err(DurabilityError::from)?;
        if !account_matches {
            return Err(CharacterProgressionError::AuthorityRejected);
        }
        let state = sqlx::query(
            "SELECT character_revision::text,profile_revision,ruleset_revision,content_revision FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid FOR UPDATE",
        )
        .bind(fence.character_id.as_bytes().as_slice())
        .fetch_optional(&mut ***tx)
        .await
        .map_err(DurabilityError::from)?
        .ok_or(CharacterProgressionError::MissingProgressionState)?;
        if numeric_u64(&state, "character_revision")? != root.revision {
            return Err(DurabilityError::InvalidStoredState.into());
        }
        if !state_matches_root(&state, &root)
            || state
                .try_get::<String, _>("content_revision")
                .map_err(DurabilityError::from)?
                != content_revision
        {
            return Err(CharacterProgressionError::ProgressionContextMismatch);
        }
        Ok(())
    }
}

/// The native purchase owner of one Character, built only by the revision sequencer under that
/// Character's held slot (`RevisionSlot::commit_npc_purchase`). It is not a permit: every write
/// still re-checks recovery, the exact gameplay fence and the account inside its own Type2
/// transaction, and the XID binds a write context to the one transaction it was admitted on.
#[derive(Clone)]
pub(crate) struct NpcPurchaseOwner {
    root: DurabilityRoot,
    recovery: CharacterRecoveryFenceV1,
    node: NodeIncarnationProof,
    character_id: CharacterId,
}

impl DurabilityRoot {
    /// Only the revision sequencer calls this, under a held slot (structural gate in
    /// `character_revision_sequencer`).
    pub(crate) fn npc_purchase_owner(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        character_id: CharacterId,
    ) -> Result<NpcPurchaseOwner, DurabilityError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        Ok(NpcPurchaseOwner {
            root: self.clone(),
            recovery,
            node: node.clone(),
            character_id,
        })
    }
}

impl NpcPurchaseOwner {
    /// Binds a write context to `tx`, the original Type2 transaction of a pass (not a
    /// savepoint), after checking the recovery fence on it.
    pub(crate) async fn bind(
        &self,
        tx: &mut Type2Transaction<'_>,
    ) -> Result<NpcPurchaseWriteContext, DurabilityError> {
        let xid = sqlx::query_scalar("SELECT pg_current_xact_id()::text")
            .fetch_one(&mut ***tx)
            .await?;
        let context = NpcPurchaseWriteContext {
            recovery: self.recovery.clone(),
            node: self.node.clone(),
            character_id: self.character_id,
            xid,
        };
        context.check_recovery(tx).await?;
        Ok(context)
    }

    /// Runs `operation` in one fresh semantic pass and Type2 transaction of the native root.
    /// A success commits; a typed refusal rolls back every callback write. Pending source
    /// results leave this function only after the physical COMMIT. An unknown commit outcome
    /// closes the pass holder and surfaces as `DurabilityError`.
    pub(crate) async fn commit<O: NpcPurchaseOperation>(
        &self,
        operation: O,
    ) -> Result<O::Output, O::Error> {
        let mut context = (Some(operation), self.clone());
        self.root
            .try_issue_semantic_pass()?
            .run_with_context(&mut context, |holder, deadline, context| {
                Box::pin(async move {
                    let mut tx = begin_type2_transaction(holder, deadline).await?;
                    let operation = context
                        .0
                        .take()
                        .ok_or(DurabilityError::InvalidStoredState)?;
                    let write = context.1.bind(&mut tx).await?;
                    match operation.apply(&write, &mut tx).await {
                        Ok(output) => {
                            tx.commit(deadline).await?;
                            Ok(Ok(output))
                        }
                        Err(error) => {
                            tx.into_inner().rollback().await?;
                            Ok(Err(error))
                        }
                    }
                })
            })
            .await?
    }
}
