//! INVCAP-1: the own character's capacity and carried weight for the inventory panel's `Cap`,
//! read with the same derivation item pickups and moves into the character check against.
use super::character_authority::{ReconciledCharacterAuthority, observe_recovery_fence};
use super::character_progression::observe_session_fence;
use super::item_loop_move::{LoopError, observe_carried_weight};
use super::item_transfer::{
    CurrentCharacterItemFence, ItemWeightLookup, observe_transfer_capacity_centi_oz,
};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};

impl DurabilityRoot {
    /// `(capacity, carried)` in centi-oz under the session's current item fence. `None` when the
    /// fence is no longer current or a carried weight cannot be resolved. Writes nothing.
    pub async fn read_inventory_capacity(
        &self,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        weights: &dyn ItemWeightLookup,
    ) -> Result<Option<(u64, u64)>, DurabilityError> {
        let record = recovery
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        let mut context = weights;
        self.try_issue_semantic_pass()?
            .run_with_context(&mut context, move |holder, deadline, weights| {
                Box::pin(async move {
                    let weights: &dyn ItemWeightLookup = *weights;
                    let mut tx = super::db::begin_observation_transaction(holder, deadline).await?;
                    observe_recovery_fence(&mut tx, &record).await?;
                    if observe_session_fence(&mut tx, &fence, &node)
                        .await?
                        .is_err()
                    {
                        tx.rollback().await?;
                        return Ok(None);
                    }
                    let capacity =
                        observe_transfer_capacity_centi_oz(&mut tx, fence.character_id, weights)
                            .await?;
                    let world = *fence.runtime_scope.world_id().as_bytes();
                    let carried =
                        match observe_carried_weight(&mut tx, fence.character_id, world, weights)
                            .await
                        {
                            Ok(carried) => carried,
                            Err(LoopError::Durability(error)) => return Err(error),
                            Err(LoopError::Database(error)) => return Err(error.into()),
                            Err(_) => {
                                tx.rollback().await?;
                                return Ok(None);
                            }
                        };
                    tx.rollback().await?;
                    Ok(Some((capacity, carried)))
                })
            })
            .await
    }
}
