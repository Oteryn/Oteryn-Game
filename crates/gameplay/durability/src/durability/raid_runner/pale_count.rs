use super::{DurabilityError, DurabilityRoot, NodeIncarnationProof, RaidStepIdentity, effects};
use crate::durability::db::{begin_semantic_transaction, commit_semantic_transaction};
use oteryn_gameplay_encounter_rules::raid::{
    PALE_COUNT_EXIT_DELAY_MS, PALE_COUNT_OPEN_DELAY_MS, PALE_COUNT_RAID,
};

impl DurabilityRoot {
    /// The admitted deadline is bound to the scheduled first announcement.
    /// Its delivery receipt arms access in this generation of this Channel.
    pub async fn pale_count_deadline(
        &self,
        node: &NodeIncarnationProof,
        identity: RaidStepIdentity,
        now_ms: i64,
        reset_epoch: u64,
        bundle_digest: [u8; 32],
    ) -> Result<Option<i64>, DurabilityError> {
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| Box::pin(async move {
                let mut tx = begin_semantic_transaction(holder, deadline).await?;
                if !effects::world_current(&mut tx, identity.firing_id).await?
                    || !effects::current(&mut tx, &node, identity.firing_id, identity.channel, identity.generation).await?
                {
                    return Err(DurabilityError::Unavailable);
                }
                let close = sqlx::query_scalar(
                    "SELECT f.deadline_ms FROM game_world_raid_firings f
                     JOIN game_world_raid_targets t USING(firing_id)
                     JOIN game_world_raid_channel_runs r USING(firing_id,channel_id)
                     JOIN game_world_raid_effects e USING(firing_id,channel_id)
                     WHERE f.firing_id=encode($1,'hex')::uuid AND f.raid_key=$2
                     AND t.channel_id=encode($3,'hex')::uuid
                     AND t.owner_generation=$4::text::numeric AND r.owner_generation=t.owner_generation
                     AND e.owner_generation=t.owner_generation AND e.step_ordinal=0
                     AND e.action_kind=1 AND e.state=2 AND f.state=1 AND r.state=1
                     AND f.deadline_ms - f.fired_at_ms >= $5
                     AND $6 >= f.deadline_ms - $7 AND $6 < f.deadline_ms
                     AND f.reset_epoch=$8::text::numeric AND f.bundle_digest=$9"
                ).bind(identity.firing_id.as_slice()).bind(PALE_COUNT_RAID)
                    .bind(identity.channel.as_bytes().as_slice()).bind(identity.generation.get().to_string())
                    .bind(PALE_COUNT_EXIT_DELAY_MS).bind(now_ms)
                    .bind(PALE_COUNT_EXIT_DELAY_MS - PALE_COUNT_OPEN_DELAY_MS).bind(reset_epoch.to_string())
                    .bind(bundle_digest.as_slice()).fetch_optional(&mut *tx).await?;
                commit_semantic_transaction(tx, deadline).await?;
                Ok(close)
            }))
            .await
    }
}
