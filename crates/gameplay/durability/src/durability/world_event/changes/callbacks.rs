//! Native quest callbacks observe the existing World owner; no quest state is invented.
use super::*;
use crate::durability::character_progression::{
    CurrentCharacterGameplayFence, assert_gameplay_fence,
};
use crate::durability::quest_world_callback::{QuestWorldContext, context_facts_are_current};
use crate::durability::runtime_scope_assignment::NodeIncarnationProof;
use crate::durability::spell_owner_commit::SpellLanePermit;
use oteryn_gameplay_quests::quest::world_callbacks::world_change_requirement;

/// Joins an already-fenced quest transaction, before consumption or rewards.
/// A retained World snapshot never grants permission; load resolves the current save.
pub async fn require(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    epoch: u64,
    callback: &str,
) -> Result<bool, DurabilityError> {
    let Some((key, stage)) = world_change_requirement(callback) else {
        return Ok(true);
    };
    let Some(day) = super::load(tx, world, epoch).await? else {
        return Ok(false);
    };
    if !oteryn_gameplay_world_change::owner::is_active(&day, key) {
        return Ok(false);
    }
    Ok(match stage {
        None => true,
        Some(stage) => oteryn_gameplay_world_change::counter::CounterKind::from_key(key)
            .is_some_and(|kind| day.world_counters && day.counter_stages[kind.index()] == stage),
    })
}

impl DurabilityRoot {
    /// Stateless entry callbacks use the same current Character/node/map/reset proof
    /// as a value-bearing quest callback. This reads no personal quest progress and
    /// creates no quest receipt. The caller retains its permit and rechecks the local
    /// actor/object CAS before applying the prepared movement.
    #[allow(
        clippy::too_many_arguments,
        reason = "the independent current fence and native map context remain explicit"
    )]
    pub async fn observe_world_callback_gate(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        permit: &SpellLanePermit,
        fence: CurrentCharacterGameplayFence,
        context: QuestWorldContext,
        callback: String,
    ) -> Result<bool, DurabilityError> {
        let crate::foundation::RuntimeScopeRefV1::Channel {
            world_id,
            channel_id,
        } = fence.runtime_scope
        else {
            return Ok(false);
        };
        permit.check_channel(world_id, channel_id)?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    if assert_gameplay_fence(&mut tx, &fence, &node)
                        .await?
                        .is_err()
                        || !context_facts_are_current(&mut tx, fence, &context).await?
                        || !require(&mut tx, world_id, context.reset_epoch, &callback).await?
                    {
                        return Ok(false);
                    }
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(true)
                })
            })
            .await
    }
}
