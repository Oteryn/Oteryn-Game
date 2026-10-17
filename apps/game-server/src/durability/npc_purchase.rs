//! D1176 native purchase participant. Upper-layer operations consume this port;
//! this module never imports a Premium policy, NPC service or purchase receipt.
//! Registration and the semantic-pass/RevisionSlot caller remain held integration.

use std::future::Future;
use std::pin::Pin;
use std::time::Instant;

use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence, assert_gameplay_fence, numeric_u64,
    state_matches_root,
};
use super::db::lock_admission_relations;
use super::item_mint_audit::Type2Transaction;
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use crate::character_recovery_fence::CharacterRecoveryFenceV1;
use crate::domain::CharacterId;
use sqlx::Row;

/// A source operation runs in the already-open native Type2 transaction. Only
/// the outer owner can commit; a typed refusal rolls back all callback writes.
pub(crate) trait NpcPurchaseOperation: Send {
    type Output: Send;
    type Error: From<DurabilityError> + Send;

    fn apply<'a, 't>(
        self,
        context: &'a NpcPurchaseWriteContext,
        tx: &'a mut Type2Transaction<'t>,
    ) -> Pin<Box<dyn Future<Output = Result<Self::Output, Self::Error>> + Send + 'a>>
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

/// Called only from the native root's semantic pass while holding the owning
/// Character's RevisionSlot and Premium gate. `tx` must be that pass's original
/// Type2 transaction, not a savepoint; no nested transaction is introduced here.
/// Pending source results leave this function only after its physical COMMIT.
pub(super) async fn commit_npc_purchase<O: NpcPurchaseOperation>(
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    node: &NodeIncarnationProof,
    character_id: CharacterId,
    mut tx: Type2Transaction<'_>,
    deadline: Instant,
    operation: O,
) -> Result<O::Output, O::Error> {
    let recovery = authority
        .record_for(root)
        .map_err(|_| DurabilityError::Unavailable)?;
    let xid = sqlx::query_scalar("SELECT pg_current_xact_id()::text")
        .fetch_one(&mut **tx)
        .await
        .map_err(DurabilityError::from)?;
    let context = NpcPurchaseWriteContext {
        recovery,
        node: node.clone(),
        character_id,
        xid,
    };
    context.check_recovery(&mut tx).await?;
    match operation.apply(&context, &mut tx).await {
        Ok(output) => {
            tx.commit(deadline).await?;
            Ok(output)
        }
        Err(error) => {
            tx.into_inner()
                .rollback()
                .await
                .map_err(DurabilityError::from)?;
            Err(error)
        }
    }
}
