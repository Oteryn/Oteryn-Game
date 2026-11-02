//! Source failure termination through the existing fenced raid effect owner.
use super::{
    CommittedRaidAction, CommittedRaidStep, DurabilityError, DurabilityRoot, NodeIncarnationProof,
    RaidCreatureIdentity, RaidStepIdentity, begin_semantic_transaction,
    commit_semantic_transaction, effects,
};
use oteryn_gameplay_encounter_rules::raid::MAX_CREATURES;
use sqlx::Row;
use std::collections::BTreeSet;

impl DurabilityRoot {
    /// Crystal 201189f raids.cpp:391-400 returns false after a failed single
    /// placement; executeRaidEvent:244-258 then stops later events. Earlier
    /// creatures survive. The caller supplies only identities whose selected
    /// catalogue placement mode is Single; this boundary independently proves
    /// their acknowledged failures and the current World/Channel authority.
    pub async fn finish_raid_after_failed_single(
        &self,
        node: &NodeIncarnationProof,
        mut identity: RaidStepIdentity,
        failed: Vec<RaidCreatureIdentity>,
        now_ms: i64,
    ) -> Result<Option<CommittedRaidStep>, DurabilityError> {
        if now_ms < 0
            || failed.is_empty()
            || failed.len() > usize::from(MAX_CREATURES)
            || failed.iter().copied().collect::<BTreeSet<_>>().len() != failed.len()
        {
            return Err(DurabilityError::InvalidStoredState);
        }
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            if !effects::world_current(&mut tx, identity.firing_id).await?
                || !effects::current(&mut tx, &node, identity.firing_id, identity.channel, identity.generation).await?
            {
                commit_semantic_transaction(tx, deadline).await?;
                return Ok(None);
            }
            let firing_state: i16 = sqlx::query_scalar("SELECT state FROM game_world_raid_firings WHERE firing_id=encode($1,'hex')::uuid FOR UPDATE")
                .bind(identity.firing_id.as_slice()).fetch_one(&mut *tx).await?;
            if !matches!(firing_state, 1 | 2) {
                commit_semantic_transaction(tx, deadline).await?;
                return Ok(None);
            }
            let owner = identity.generation.get().to_string();
            let run = sqlx::query("SELECT state,committed_steps,updated_at_ms FROM game_world_raid_channel_runs WHERE firing_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND owner_generation=$3::text::numeric AND state IN(1,2) FOR UPDATE")
                .bind(identity.firing_id.as_slice()).bind(identity.channel.as_bytes().as_slice())
                .bind(&owner).fetch_optional(&mut *tx).await?;
            let Some(run) = run else {
                commit_semantic_transaction(tx, deadline).await?;
                return Ok(None);
            };
            if now_ms < run.try_get::<i64, _>("updated_at_ms")? {
                return Err(DurabilityError::InvalidStoredState);
            }
            // The live report alone is insufficient. Each identity must have
            // one false placement belonging to an acknowledged wave of this
            // generation, already recorded by the requested termination time.
            for creature in failed {
                let acknowledged_failure: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_world_raid_creatures c JOIN game_world_raid_creature_placements p USING(firing_id,channel_id,wave,entry,creature_index) JOIN game_world_raid_effects e ON e.firing_id=c.firing_id AND e.channel_id=c.channel_id AND e.wave=c.wave WHERE c.firing_id=encode($1,'hex')::uuid AND c.channel_id=encode($2,'hex')::uuid AND c.wave=$3 AND c.entry=$4 AND c.creature_index=$5 AND c.state=2 AND NOT p.placed AND p.recorded_at_ms<=$6 AND e.action_kind=2 AND e.state=2 AND e.settled_at_ms<=$6 AND e.owner_generation=$7::text::numeric)")
                    .bind(identity.firing_id.as_slice()).bind(identity.channel.as_bytes().as_slice())
                    .bind(effects::small(creature.wave)?).bind(effects::small(creature.entry)?).bind(effects::small(creature.index)?)
                    .bind(now_ms).bind(&owner).fetch_one(&mut *tx).await?;
                if !acknowledged_failure {
                    commit_semantic_transaction(tx, deadline).await?;
                    return Ok(None);
                }
            }
            identity.ordinal = u32::try_from(run.try_get::<i32, _>("committed_steps")?)
                .map_err(|_| DurabilityError::InvalidStoredState)?;
            if let Some(pending) = effects::pending(&mut tx, identity).await? {
                commit_semantic_transaction(tx, deadline).await?;
                return Ok(matches!(pending.action, CommittedRaidAction::Complete { cleanup: false }).then_some(pending));
            }
            if run.try_get::<i16, _>("state")? == 2 {
                // An uncertain terminal ACK must still recover the source
                // abort before a later boss observer considers cleanup.
                let retained = sqlx::query("SELECT * FROM game_world_raid_effects WHERE firing_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND owner_generation=$3::text::numeric AND action_kind=3 AND NOT cleanup AND state=2 ORDER BY step_ordinal DESC LIMIT 1")
                    .bind(identity.firing_id.as_slice()).bind(identity.channel.as_bytes().as_slice())
                    .bind(&owner).fetch_optional(&mut *tx).await?;
                let replay = match retained {
                    Some(row) => Some(effects::decode(&mut tx, identity, &row).await?),
                    None => None,
                };
                commit_semantic_transaction(tx, deadline).await?;
                return Ok(replay);
            }
            if firing_state != 1 {
                commit_semantic_transaction(tx, deadline).await?;
                return Ok(None);
            }
            let next = identity.ordinal.checked_add(1).and_then(|value| i32::try_from(value).ok())
                .ok_or(DurabilityError::InvalidStoredState)?;
            let action = CommittedRaidAction::Complete { cleanup: false };
            effects::insert(&mut tx, identity, &owner, &action, now_ms).await?;
            sqlx::query("UPDATE game_world_raid_channel_runs SET committed_steps=$3,updated_at_ms=$4 WHERE firing_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND owner_generation=$5::text::numeric AND state=1")
                .bind(identity.firing_id.as_slice()).bind(identity.channel.as_bytes().as_slice())
                .bind(next).bind(now_ms).bind(&owner).execute(&mut *tx).await?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Some(CommittedRaidStep { identity, action, replayed: false }))
        })).await
    }
}
