//! Read a retained effect before considering a new source event. No cursor or
//! effect mutation occurs here, so an acknowledged death can stop later waves.
use super::{
    CommittedRaidStep, DurabilityError, DurabilityRoot, NodeIncarnationProof, RaidStepIdentity,
    begin_semantic_transaction, commit_semantic_transaction, effects,
};
use crate::foundation::{ChannelId, ScopeOwnershipGeneration};

impl DurabilityRoot {
    pub async fn pending_raid_effect(
        &self,
        node: &NodeIncarnationProof,
        firing_id: [u8; 16],
        channel: ChannelId,
        generation: ScopeOwnershipGeneration,
    ) -> Result<Option<CommittedRaidStep>, DurabilityError> {
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            if !effects::world_current(&mut tx, firing_id).await?
                || !effects::current(&mut tx, &node, firing_id, channel, generation).await?
            {
                commit_semantic_transaction(tx, deadline).await?;
                return Ok(None);
            }
            let firing: i16 = sqlx::query_scalar("SELECT state FROM game_world_raid_firings WHERE firing_id=encode($1,'hex')::uuid FOR UPDATE")
                .bind(firing_id.as_slice()).fetch_one(&mut *tx).await?;
            if !matches!(firing, 1 | 2) {
                commit_semantic_transaction(tx, deadline).await?;
                return Ok(None);
            }
            let run: Option<i16> = sqlx::query_scalar("SELECT state FROM game_world_raid_channel_runs WHERE firing_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND owner_generation=$3::text::numeric AND state IN(1,2) FOR UPDATE")
                .bind(firing_id.as_slice()).bind(channel.as_bytes().as_slice())
                .bind(generation.get().to_string()).fetch_optional(&mut *tx).await?;
            if run.is_none() {
                commit_semantic_transaction(tx, deadline).await?;
                return Ok(None);
            }
            let row = sqlx::query("SELECT * FROM game_world_raid_effects WHERE firing_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND owner_generation=$3::text::numeric AND state=1 ORDER BY step_ordinal LIMIT 1 FOR UPDATE")
                .bind(firing_id.as_slice()).bind(channel.as_bytes().as_slice())
                .bind(generation.get().to_string()).fetch_optional(&mut *tx).await?;
            let pending = match row {
                Some(row) => Some(effects::decode(&mut tx, RaidStepIdentity {
                    firing_id, channel, generation, ordinal: 0,
                }, &row).await?),
                None => None,
            };
            commit_semantic_transaction(tx, deadline).await?;
            Ok(pending)
        })).await
    }
}
