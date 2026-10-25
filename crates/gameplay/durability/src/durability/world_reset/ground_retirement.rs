//! Resumable one-item passes on the existing frozen retirement owner.
use super::*;
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::item_decay_retire::DecayRetireError;
use crate::durability::runtime_scope_assignment::NodeIncarnationProof;
use crate::durability::spell_owner_commit::SpellLane;

impl DurabilityRoot {
    /// Retire the current job's Ground only after a complete fresh-generation
    /// drain. False is retryable NotCurrent; partial progress remains durable.
    pub async fn retire_world_reset_ground(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        reset: &RetiringWorldReset,
    ) -> std::result::Result<bool, DecayRetireError> {
        crate::durability::fresh_admission::release_retired_holder_sessions(self, node, *reset)
            .await?;
        let mut lanes = std::collections::BTreeMap::new();
        loop {
            let root = self.clone();
            let node_for_pass = node.clone();
            let reset = *reset;
            let next = self
                .try_issue_semantic_pass()?
                .run(move |holder, deadline| {
                    Box::pin(async move {
                        let mut tx = begin_semantic_transaction(holder, deadline).await?;
                        let channels =
                            match census::drained_channels(&root, &mut tx, &node_for_pass, reset)
                                .await?
                            {
                                census::WorldResetDrain::Drained(channels) => channels,
                                census::WorldResetDrain::NotCurrent => return Ok(Ok(None)),
                                census::WorldResetDrain::CapacityExceeded => {
                                    return Ok(Err(DecayRetireError::CapacityExceeded));
                                }
                            };
                        let next = ground_step::next(&mut tx, reset).await?;
                        let next = next
                            .map(|(channel, step)| {
                                channels
                                    .into_iter()
                                    .find(|fence| *fence.channel_id.as_bytes() == channel)
                                    .map(|fence| (fence, step))
                                    .ok_or(DurabilityError::InvalidStoredState)
                            })
                            .transpose()?;
                        commit_semantic_transaction(tx, deadline).await?;
                        Ok(Ok(Some(next)))
                    })
                })
                .await??;
            let Some(next) = next else {
                return Ok(false);
            };
            let Some((fence, step)) = next else {
                return Ok(true);
            };
            let lane = lanes
                .entry(*fence.channel_id.as_bytes())
                .or_insert_with(|| SpellLane::new(fence.world_id, fence.channel_id));
            let permit = lane
                .acquire()
                .await
                .map_err(|_| DurabilityError::InvalidStoredState)?;
            let mut candidate = self
                .freeze_decay_retire(authority, node, fence, step)
                .await?;
            // A crash or unknown commit is resumed with the same durable
            // reservation/receipt on the next boot pass, never a new identity.
            self.commit_decay_retire(&permit, authority, node, fence, &mut candidate)
                .await?;
        }
    }
}
