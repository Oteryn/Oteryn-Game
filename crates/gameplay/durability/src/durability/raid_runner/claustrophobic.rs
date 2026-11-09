//! Fixed source cues share the existing independently fenced raid outbox.
use super::*;
use oteryn_gameplay_encounter_rules::raid::source_claustrophobic::{
    Controller, PreparedEvent, Raid,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaustrophobicRaidCue {
    pub identity: RaidStepIdentity,
    pub event: PreparedEvent,
}

async fn locked_state(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    node: &NodeIncarnationProof,
    firing: [u8; 16],
    channel: ChannelId,
    generation: ScopeOwnershipGeneration,
) -> Result<Option<(Raid, Controller, u32)>, DurabilityError> {
    if !effects::world_current(tx, firing).await?
        || !effects::current(tx, node, firing, channel, generation).await?
    {
        return Ok(None);
    }
    let row=sqlx::query("SELECT f.raid_key,r.source_state::text,r.committed_steps FROM game_world_raid_firings f JOIN game_world_raid_schedule_sources s USING(world_id,raid_key) JOIN game_world_raid_channel_runs r USING(firing_id) WHERE f.firing_id=encode($1,'hex')::uuid AND r.channel_id=encode($2,'hex')::uuid AND r.owner_generation=$3::text::numeric AND f.state=1 AND r.state=1 AND s.source_kind=4 FOR UPDATE OF f,r")
        .bind(firing.as_slice()).bind(channel.as_bytes().as_slice()).bind(generation.get().to_string()).fetch_optional(&mut **tx).await?;
    let Some(row) = row else { return Ok(None) };
    let key: String = row.try_get("raid_key")?;
    let raid = Raid::from_key(&key).ok_or(DurabilityError::InvalidStoredState)?;
    let state: String = row.try_get("source_state")?;
    let state = Controller::from_snapshot(raid, state.as_bytes())
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let steps = u32::try_from(row.try_get::<i32, _>("committed_steps")?)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    Ok(Some((raid, state, steps)))
}

/// Current physical owner guard; retain this transaction through the effect.
pub async fn claustrophobic_cue_pending_in_transaction(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    node: &NodeIncarnationProof,
    cue: &ClaustrophobicRaidCue,
    world: crate::foundation::WorldId,
    reset_epoch: u64,
    digest: [u8; 32],
) -> Result<bool, DurabilityError> {
    let id = cue.identity;
    let Some((raid, _, steps)) =
        locked_state(tx, node, id.firing_id, id.channel, id.generation).await?
    else {
        return Err(DurabilityError::Unavailable);
    };
    if raid != cue.event.raid() || steps != id.ordinal {
        return Err(DurabilityError::Unavailable);
    }
    let row=sqlx::query("SELECT e.source_cue::text,e.state FROM game_world_raid_effects e JOIN game_world_raid_firings f USING(firing_id) WHERE e.firing_id=encode($1,'hex')::uuid AND e.channel_id=encode($2,'hex')::uuid AND e.step_ordinal=$3 AND e.action_kind=5 AND f.world_id=encode($4,'hex')::uuid AND f.reset_epoch=$5::text::numeric AND f.bundle_digest=$6 FOR UPDATE OF e")
        .bind(id.firing_id.as_slice()).bind(id.channel.as_bytes().as_slice()).bind(i32::try_from(id.ordinal).map_err(|_|DurabilityError::InvalidStoredState)?).bind(world.as_bytes().as_slice()).bind(reset_epoch.to_string()).bind(digest.as_slice()).fetch_optional(&mut **tx).await?.ok_or(DurabilityError::Unavailable)?;
    let event: PreparedEvent = serde_json::from_str(&row.try_get::<String, _>("source_cue")?)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    if event != cue.event {
        return Err(DurabilityError::Unavailable);
    }
    Ok(row.try_get::<i16, _>("state")? == 1)
}

impl DurabilityRoot {
    pub async fn claustrophobic_raid_complete(
        &self,
        node: &NodeIncarnationProof,
        firing: [u8; 16],
        channel: ChannelId,
        generation: ScopeOwnershipGeneration,
    ) -> Result<bool, DurabilityError> {
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            if !effects::world_current(&mut tx,firing).await? || !effects::current(&mut tx,&node,firing,channel,generation).await?{return Err(DurabilityError::Unavailable)}
            let complete:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_world_raid_channel_runs r JOIN game_world_raid_firings f USING(firing_id) JOIN game_world_raid_schedule_sources s USING(world_id,raid_key) WHERE r.firing_id=encode($1,'hex')::uuid AND r.channel_id=encode($2,'hex')::uuid AND r.owner_generation=$3::text::numeric AND r.state=2 AND s.source_kind=4)").bind(firing.as_slice()).bind(channel.as_bytes().as_slice()).bind(generation.get().to_string()).fetch_one(&mut *tx).await?;
            commit_semantic_transaction(tx,deadline).await?;Ok(complete)
        })).await
    }

    pub async fn next_claustrophobic_raid_cue(
        &self,
        node: &NodeIncarnationProof,
        firing: [u8; 16],
        channel: ChannelId,
        generation: ScopeOwnershipGeneration,
        now_ms: i64,
    ) -> Result<Option<ClaustrophobicRaidCue>, DurabilityError> {
        let now = u64::try_from(now_ms).map_err(|_| DurabilityError::InvalidStoredState)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            let Some((raid,state,steps))=locked_state(&mut tx,&node,firing,channel,generation).await? else{commit_semantic_transaction(tx,deadline).await?;return Ok(None)};
            let retained:Option<String>=sqlx::query_scalar("SELECT source_cue::text FROM game_world_raid_effects WHERE firing_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND step_ordinal=$3 AND action_kind=5 AND state=1 FOR UPDATE")
                .bind(firing.as_slice()).bind(channel.as_bytes().as_slice()).bind(i32::try_from(steps).map_err(|_|DurabilityError::InvalidStoredState)?).fetch_optional(&mut *tx).await?;
            let event = if let Some(body)=retained {
                let event:PreparedEvent=serde_json::from_str(&body).map_err(|_|DurabilityError::InvalidStoredState)?;
                if event.raid()!=raid || !event.source_valid() || state.prepare(now)!=Some(event){return Err(DurabilityError::InvalidStoredState)}
                event
            }else{
                let Some(event)=state.prepare(now) else{commit_semantic_transaction(tx,deadline).await?;return Ok(None)};
                let body=serde_json::to_string(&event).map_err(|_|DurabilityError::InvalidStoredState)?;
                sqlx::query("INSERT INTO game_world_raid_step_receipts(firing_id,channel_id,step_ordinal,action_kind,committed_at_ms) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,5,$4)")
                    .bind(firing.as_slice()).bind(channel.as_bytes().as_slice()).bind(i32::try_from(steps).map_err(|_|DurabilityError::InvalidStoredState)?).bind(now_ms).execute(&mut *tx).await?;
                sqlx::query("INSERT INTO game_world_raid_effects(firing_id,channel_id,step_ordinal,owner_generation,action_kind,source_cue) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4::text::numeric,5,$5::jsonb)")
                    .bind(firing.as_slice()).bind(channel.as_bytes().as_slice()).bind(i32::try_from(steps).map_err(|_|DurabilityError::InvalidStoredState)?).bind(generation.get().to_string()).bind(body).execute(&mut *tx).await?;
                event
            };
            commit_semantic_transaction(tx,deadline).await?;
            Ok(Some(ClaustrophobicRaidCue{identity:RaidStepIdentity{firing_id:firing,channel,generation,ordinal:steps},event}))
        })).await
    }
    pub async fn acknowledge_claustrophobic_raid_cue(
        &self,
        node: &NodeIncarnationProof,
        cue: &ClaustrophobicRaidCue,
        now_ms: i64,
    ) -> Result<(), DurabilityError> {
        let now = u64::try_from(now_ms).map_err(|_| DurabilityError::InvalidStoredState)?;
        let node = node.clone();
        let cue = cue.clone();
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            let id=cue.identity;
            let Some((raid,mut state,steps))=locked_state(&mut tx,&node,id.firing_id,id.channel,id.generation).await? else{
                // A final ACK may have committed before its response was lost.
                // Reconcile only that identical settled cue under current owners.
                if !effects::world_current(&mut tx,id.firing_id).await? || !effects::current(&mut tx,&node,id.firing_id,id.channel,id.generation).await?{return Err(DurabilityError::Unavailable)}
                let prior:Option<String>=sqlx::query_scalar("SELECT source_cue::text FROM game_world_raid_effects WHERE firing_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND step_ordinal=$3 AND owner_generation=$4::text::numeric AND action_kind=5 AND state=2")
                    .bind(id.firing_id.as_slice()).bind(id.channel.as_bytes().as_slice()).bind(i32::try_from(id.ordinal).map_err(|_|DurabilityError::InvalidStoredState)?).bind(id.generation.get().to_string()).fetch_optional(&mut *tx).await?;
                if prior.and_then(|body|serde_json::from_str::<PreparedEvent>(&body).ok())!=Some(cue.event){return Err(DurabilityError::Unavailable)}
                commit_semantic_transaction(tx,deadline).await?;return Ok(())
            };
            let row=sqlx::query("SELECT state,source_cue::text FROM game_world_raid_effects WHERE firing_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND step_ordinal=$3 AND action_kind=5 FOR UPDATE")
                .bind(id.firing_id.as_slice()).bind(id.channel.as_bytes().as_slice()).bind(i32::try_from(id.ordinal).map_err(|_|DurabilityError::InvalidStoredState)?).fetch_one(&mut *tx).await?;
            let event:PreparedEvent=serde_json::from_str(&row.try_get::<String,_>("source_cue")?).map_err(|_|DurabilityError::InvalidStoredState)?;
            if event!=cue.event || event.raid()!=raid {return Err(DurabilityError::InvalidStoredState)}
            if row.try_get::<i16,_>("state")?==2 {commit_semantic_transaction(tx,deadline).await?;return Ok(())}
            if steps!=id.ordinal {return Err(DurabilityError::InvalidStoredState)}
            let seconds:i64=sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint").fetch_one(&mut *tx).await?;
            state.commit(event,now,seconds as u64).map_err(|_|DurabilityError::InvalidStoredState)?;
            let done=!state.active() && state.next_due().is_none();
            let body=String::from_utf8(state.snapshot().map_err(|_|DurabilityError::InvalidStoredState)?).map_err(|_|DurabilityError::InvalidStoredState)?;
            sqlx::query("UPDATE game_world_raid_effects SET state=2,settled_at_ms=$4 WHERE firing_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND step_ordinal=$3")
                .bind(id.firing_id.as_slice()).bind(id.channel.as_bytes().as_slice()).bind(i32::try_from(steps).map_err(|_|DurabilityError::InvalidStoredState)?).bind(now_ms).execute(&mut *tx).await?;
            sqlx::query("UPDATE game_world_raid_channel_runs SET committed_steps=committed_steps+1,source_state=$3::jsonb,state=$4,updated_at_ms=$5 WHERE firing_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid")
                .bind(id.firing_id.as_slice()).bind(id.channel.as_bytes().as_slice()).bind(body).bind(if done{2_i16}else{1}).bind(now_ms).execute(&mut *tx).await?;
            if done {sqlx::query("UPDATE game_world_raid_firings f SET state=2 WHERE firing_id=encode($1,'hex')::uuid AND state=1 AND NOT EXISTS(SELECT 1 FROM game_world_raid_channel_runs r WHERE r.firing_id=f.firing_id AND r.state=1)").bind(id.firing_id.as_slice()).execute(&mut *tx).await?;}
            commit_semantic_transaction(tx,deadline).await?;Ok(())
        })).await
    }
}
