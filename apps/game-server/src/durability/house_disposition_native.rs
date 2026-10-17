//! Native transaction envelope for the canonical HOUSE-1b modules.
//!
//! The actual Durability owner validates its retained recovery capability, starts the
//! semantic transaction and takes admission locks before invoking House operations.
//! The House SQL/body is in `house::{move_out,transfer}`. Existing private native
//! helpers remain private; this module exports no recovery record or node secret.

use super::DurabilityRoot;
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::runtime_scope_assignment::NodeIncarnationProof;
use crate::foundation::{ChannelId, WorldId};
use crate::house::move_out::{
    self as owner, HouseMoveOutError, HouseMoveOutOutcome, HouseMoveOutPass, Result,
};

/// A private owner pass. It is executable with the native Root and fences, but is not wired
/// or production-qualified until the owning registered event/projection/release seams land.
impl DurabilityRoot {
    pub async fn run_private_house_move_out(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        operation: HouseMoveOutPass,
    ) -> Result<HouseMoveOutOutcome> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| HouseMoveOutError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    // Before property/item locks. Together with the native admission relation fence
                    // this freezes both canonical session and PREPARED/COMMITTED entrant census.
                    sqlx::query("LOCK TABLE game_house_scope_handoffs IN SHARE MODE")
                        .execute(&mut *tx)
                        .await?;
                    let result = match operation {
                        HouseMoveOutPass::Notice {
                            fence,
                            command,
                            request,
                        } => owner::notice(&mut tx, &node, fence, command, request).await,
                        HouseMoveOutPass::Cancel {
                            fence,
                            command,
                            request,
                        } => owner::cancel(&mut tx, &node, fence, command, request).await,
                        HouseMoveOutPass::Freeze {
                            operation,
                            house,
                            execution_scope,
                            scope_generation,
                        } => {
                            owner::freeze(
                                &mut tx,
                                &node,
                                operation,
                                &house,
                                execution_scope,
                                scope_generation,
                            )
                            .await
                        }
                        HouseMoveOutPass::Step {
                            operation,
                            house,
                            execution_scope,
                            scope_generation,
                            step,
                        } => {
                            crate::house::transfer::step(
                                &mut tx,
                                &node,
                                operation,
                                &house,
                                execution_scope,
                                scope_generation,
                                step,
                            )
                            .await
                        }
                    };
                    match result {
                        Ok(value) => {
                            commit_semantic_transaction(tx, deadline).await?;
                            Ok(Ok(value))
                        }
                        Err(error) => Ok(Err(error)), // transaction drops/rolls back as a whole
                    }
                })
            })
            .await?
    }
}

/// The owning native encoder; these bytes are a scope key, never current authority.
impl DurabilityRoot {
    pub(crate) fn house_disposition_channel_scope_key(
        world: WorldId,
        channel: ChannelId,
    ) -> [u8; 33] {
        super::runtime_scope_assignment::scope_key(world, channel)
    }
}
