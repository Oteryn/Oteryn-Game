//! Native transaction envelope for the canonical HOUSE-1b modules.
//!
//! The actual Durability owner validates its retained recovery capability, starts the
//! semantic transaction and takes admission locks before invoking House operations.
//! The House SQL/body is in `house::{move_out,transfer}`. Existing private native
//! helpers remain private; this module exports no recovery record or node secret.

use super::DurabilityRoot;
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_progression::assert_item_only_fence;
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::house_scope_handoff::HouseId;
use super::item_transfer::CurrentCharacterItemFence;
use super::runtime_scope_assignment::NodeIncarnationProof;
use crate::foundation::{ChannelId, CommandRef, RuntimeScopeRefV1, WorldId};
use crate::house::move_out::{
    self as owner, HouseMoveOutCommand, HouseMoveOutCommandReceipt, HouseMoveOutError,
    HouseMoveOutOutcome, HouseMoveOutOwnerView, HouseMoveOutPass, Result,
};

impl DurabilityRoot {
    /// D1384's command owner resolves the operation and ownership revision in this TX.
    /// The request contains only the exact absolute due time or a cancellation intent.
    pub async fn apply_house_move_out_command(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        command: CommandRef,
        house: HouseId,
        request: HouseMoveOutCommand,
    ) -> Result<HouseMoveOutCommandReceipt> {
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
                    sqlx::query("LOCK TABLE game_house_scope_handoffs IN SHARE MODE")
                        .execute(&mut *tx)
                        .await?;
                    let result =
                        owner::apply_command(&mut tx, &node, fence, command, house, request).await;
                    match result {
                        Ok(value) => {
                            commit_semantic_transaction(tx, deadline).await?;
                            Ok(Ok(value))
                        }
                        Err(error) => Ok(Err(error)),
                    }
                })
            })
            .await?
    }

    /// Current-owner-only projection. The native fence is checked without minting a
    /// CommandRef, changing a revision or reserving a future mutation capability.
    pub async fn read_house_move_out_owner(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        house: HouseId,
    ) -> Result<HouseMoveOutOwnerView> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| HouseMoveOutError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let result = async {
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    if !matches!(fence.runtime_scope, RuntimeScopeRefV1::Channel { world_id, .. }
                        if world_id == house.world_id()) {
                        return Err(HouseMoveOutError::AuthorityRejected);
                    }
                    assert_item_only_fence(&mut tx, &fence, &node).await?
                        .map_err(|_| HouseMoveOutError::AuthorityRejected)?;
                    owner::read_owner_projection(&mut tx, fence.character_id, house).await
                }.await;
                    match result {
                        Ok(value) => {
                            commit_semantic_transaction(tx, deadline).await?;
                            Ok(Ok(value))
                        }
                        Err(error) => Ok(Err(error)),
                    }
                })
            })
            .await?
    }
}

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
