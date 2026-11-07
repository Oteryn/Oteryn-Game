//! A real Cake Golem occurrence counts at most once, under the current Channel owner.
use super::lock_world;
use crate::durability::DurabilityRoot;
use crate::durability::boss_encounter::{BossClockError, assert_channel_owner};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::db::{begin_semantic_transaction, commit_semantic_transaction};
use crate::durability::runtime_scope_assignment::NodeIncarnationProof;
use crate::foundation::{ChannelId, ScopeOwnershipGeneration, WorldId};

impl DurabilityRoot {
    /// One native raid attempt per Channel/hour, retained across owner restarts.
    #[allow(clippy::too_many_arguments)]
    pub async fn claim_cake_raid_slot(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        world: WorldId,
        channel: ChannelId,
        generation: ScopeOwnershipGeneration,
        now_seconds: i64,
    ) -> Result<Option<(i64, i64)>, BossClockError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| BossClockError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            let result=async{
                assert_channel_owner(&mut tx,&recovery,&node,world,channel,generation).await?;
                super::authority::arm(&mut tx,&node,world,channel,generation,now_seconds).await?;
                let Some(snapshot)=lock_world(&mut tx,world,now_seconds).await? else{return Ok(None);};
                if snapshot.state.stage!=1 || now_seconds>=snapshot.event_end{return Ok(None);}
                let Some(elapsed)=now_seconds.checked_sub(snapshot.event_start+3600).filter(|elapsed|*elapsed>=0) else{return Ok(None);};
                let slot=elapsed/3600;
                let claimed=sqlx::query("INSERT INTO game_seasonal_cake_raid_slots(world_id,channel_id,event_start,slot,claimed_at) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,$5) ON CONFLICT DO NOTHING")
                    .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).bind(snapshot.event_start).bind(slot).bind(now_seconds).execute(&mut *tx).await?.rows_affected()==1;
                Ok(claimed.then_some((snapshot.event_start,slot)))
            }.await;
            if result.is_ok(){commit_semantic_transaction(tx,deadline).await?;}
            Ok(result)
        })).await?
    }
}
