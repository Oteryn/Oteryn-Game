//! A real Cake Golem occurrence counts at most once, under the current Channel owner.
use super::{advance_world, finish_world_change, lock_world};
use crate::durability::DurabilityRoot;
use crate::durability::boss_encounter::{BossClockError, assert_channel_owner};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::db::{begin_semantic_transaction, commit_semantic_transaction};
use crate::durability::runtime_scope_assignment::NodeIncarnationProof;
use crate::durability::spell_owner_commit::SpellLanePermit;
use crate::foundation::{ChannelId, RuntimeScopeRefV1, ScopeOwnershipGeneration, WorldId};

impl DurabilityRoot {
    /// The dispatch owner resolves the source creature and machine under the live runtime
    /// lock while holding this physical permit. The receipt is an observation, not actor authority.
    #[allow(
        clippy::too_many_arguments,
        reason = "complete current owner and exact source creature occurrence"
    )]
    pub async fn extract_cake_golem(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        permit: &SpellLanePermit,
        world: WorldId,
        channel: ChannelId,
        generation: ScopeOwnershipGeneration,
        source_occurrence: [u8; 32],
        now_seconds: i64,
    ) -> Result<bool, BossClockError> {
        if source_occurrence == [0; 32] || now_seconds < 0 {
            return Err(BossClockError::InvalidInput);
        }
        permit.check_scope(RuntimeScopeRefV1::channel(world, channel))?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| BossClockError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            let result = async {
                assert_channel_owner(&mut tx, &recovery, &node, world, channel, generation).await?;
                super::authority::arm(&mut tx,&node,world,channel,generation,now_seconds).await?;
                let Some(mut snapshot) = lock_world(&mut tx, world, now_seconds).await? else { return Ok(false); };
                if let Some(counted) = sqlx::query_scalar::<_,bool>("SELECT counted FROM game_seasonal_cake_extractions WHERE world_id=encode($1,'hex')::uuid AND event_start=$2 AND source_occurrence=$3")
                    .bind(world.as_bytes().as_slice()).bind(snapshot.event_start).bind(source_occurrence.as_slice()).fetch_optional(&mut *tx).await? {
                    return Ok(counted);
                }
                advance_world(&mut tx, &mut snapshot, now_seconds).await?;
                if now_seconds >= snapshot.event_end { return Ok(false); }
                let counted = snapshot.state.stage == 1 && snapshot.state.golems < 200 && now_seconds < snapshot.event_end;
                if counted {
                    let previous = snapshot.state.clone();
                    snapshot.state.golems += 1;
                    finish_world_change(&mut tx, &mut snapshot, now_seconds, previous).await?;
                }
                sqlx::query("INSERT INTO game_seasonal_cake_extractions(world_id,event_start,source_occurrence,extracted_at,counted) VALUES(encode($1,'hex')::uuid,$2,$3,$4,$5)")
                    .bind(world.as_bytes().as_slice()).bind(snapshot.event_start).bind(source_occurrence.as_slice()).bind(now_seconds).bind(counted)
                    .execute(&mut *tx).await?;
                Ok(counted)
            }.await;
            if result.is_ok() { commit_semantic_transaction(tx, deadline).await?; }
            Ok(result)
        })).await?
    }
}
