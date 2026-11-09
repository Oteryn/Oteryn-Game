//! Fixed Soul War STEP scheduling in the existing World raid journal.
use super::*;
use crate::durability::DurabilityRoot;
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::db::{begin_semantic_transaction, commit_semantic_transaction};
use crate::durability::runtime_scope_assignment::{
    NodeIncarnationProof, prove_current_incarnation,
};
use crate::foundation::{ChannelId, ScopeOwnershipGeneration};
use oteryn_gameplay_encounter_rules::raid::source_claustrophobic::{Controller, Position, Raid};

#[derive(Debug, Clone, Copy)]
pub struct ClaustrophobicRaidStart {
    pub world: WorldId,
    pub channel: ChannelId,
    pub generation: ScopeOwnershipGeneration,
    pub reset_epoch: u64,
    pub bundle_digest: [u8; 32],
    pub firing_id: [u8; 16],
    pub raid: Raid,
    pub from: Position,
    pub to: Position,
    pub now_ms: i64,
}

impl DurabilityRoot {
    /// The caller proves the actual committed STEP and published source frame.
    /// This writer independently proves the current node/reset/Channel and
    /// serializes the source active/cooldown predicate with the World raid lock.
    pub async fn start_claustrophobic_raid(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        request: ClaustrophobicRaidStart,
    ) -> Result<bool, DurabilityError> {
        // Epoch zero is the canonical initial World epoch (0148); the
        // independently current reset-map proof below still fences it.
        if request.now_ms < 0
            || request.bundle_digest == [0; 32]
            || crate::foundation::WorldId::decode(&request.firing_id).is_err()
        {
            return Err(DurabilityError::InvalidStoredState);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            if !prove_current_incarnation(&mut tx, &node).await? { return Err(DurabilityError::Unavailable); }
            let fact=node.fact();
            let reset_current:bool=sqlx::query_scalar("SELECT game_world_reset_map_epoch_current(encode($1,'hex')::uuid,$2::text::numeric)")
                .bind(request.world.as_bytes().as_slice()).bind(request.reset_epoch.to_string()).fetch_one(&mut *tx).await?;
            if !reset_current{return Err(DurabilityError::Unavailable)}
            let current: Option<String>=sqlx::query_scalar("SELECT a.ownership_generation::text FROM game_runtime_scope_assignments a JOIN game_durability_admission_runtime_guards g USING(scope_key) JOIN game_world_reset_active r USING(world_id) JOIN game_world_reset_epochs e ON e.world_id=r.world_id AND e.reset_epoch=r.reset_epoch WHERE a.world_id=encode($1,'hex')::uuid AND a.channel_id=encode($2,'hex')::uuid AND a.scope_kind=1 AND a.state=1 AND a.ownership_generation=$3::text::numeric AND a.holder_node_id=encode($4,'hex')::uuid AND a.holder_registration_revision=$5::text::numeric AND g.ready AND g.ownership_generation=a.ownership_generation AND r.reset_epoch=$6::text::numeric AND e.target_digest=$7 FOR SHARE OF a,g")
                .bind(request.world.as_bytes().as_slice()).bind(request.channel.as_bytes().as_slice())
                .bind(request.generation.get().to_string()).bind(fact.node_id().as_bytes().as_slice())
                .bind(fact.registration_revision().to_string()).bind(request.reset_epoch.to_string())
                .bind(request.bundle_digest.as_slice()).fetch_optional(&mut *tx).await?;
            if current.as_deref()!=Some(&request.generation.get().to_string()) { return Err(DurabilityError::Unavailable); }
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:raid-world:' || encode($1,'hex'),0))")
                .bind(request.world.as_bytes().as_slice()).execute(&mut *tx).await?;
            // Match the existing scheduler's reset retirement before this
            // source can encounter the per-World/key one-running constraint.
            sqlx::query("UPDATE game_world_raid_firings SET state=3 WHERE world_id=encode($1,'hex')::uuid AND reset_epoch<>$2::text::numeric AND state=1")
                .bind(request.world.as_bytes().as_slice()).bind(request.reset_epoch.to_string()).execute(&mut *tx).await?;
            let key=ProductionKey::new(request.raid.key()).map_err(|_|DurabilityError::InvalidStoredState)?;
            super::lock(&mut tx,&recovery,request.world,&key).await.map_err(|_|DurabilityError::Unavailable)?;
            let replay: bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_world_raid_firings WHERE firing_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND raid_key=$3 AND reset_epoch=$4::text::numeric AND bundle_digest=$5)")
                .bind(request.firing_id.as_slice()).bind(request.world.as_bytes().as_slice()).bind(key.as_str()).bind(request.reset_epoch.to_string()).bind(request.bundle_digest.as_slice()).fetch_one(&mut *tx).await?;
            if replay { commit_semantic_transaction(tx,deadline).await?; return Ok(true); }
            let active: bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_world_raid_firings WHERE world_id=encode($1,'hex')::uuid AND raid_key=$2 AND reset_epoch=$3::text::numeric AND state=1)")
                .bind(request.world.as_bytes().as_slice()).bind(key.as_str()).bind(request.reset_epoch.to_string()).fetch_one(&mut *tx).await?;
            if active { commit_semantic_transaction(tx,deadline).await?; return Ok(false); }
            let seconds: i64=sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint").fetch_one(&mut *tx).await?;
            // Source onReset records os.time; it is not the nominal 120s deadline.
            let previous: Option<String>=sqlx::query_scalar("SELECT r.source_state::text FROM game_world_raid_channel_runs r JOIN game_world_raid_firings f USING(firing_id) WHERE f.world_id=encode($1,'hex')::uuid AND f.raid_key=$2 AND f.reset_epoch=$3::text::numeric AND r.source_state IS NOT NULL AND r.state=2 ORDER BY f.ordinal DESC,(r.source_state->>'last_end_seconds')::numeric DESC NULLS LAST,r.channel_id LIMIT 1")
                .bind(request.world.as_bytes().as_slice()).bind(key.as_str()).bind(request.reset_epoch.to_string()).fetch_optional(&mut *tx).await?;
            let mut controller=match previous {Some(state)=>Controller::from_snapshot(request.raid,state.as_bytes()).map_err(|_|DurabilityError::InvalidStoredState)?,None=>Controller::new(request.raid)};
            if !controller.start(request.from,request.to,request.now_ms as u64,seconds as u64).map_err(|_|DurabilityError::InvalidStoredState)? { commit_semantic_transaction(tx,deadline).await?;return Ok(false); }
            let state=String::from_utf8(controller.snapshot().map_err(|_|DurabilityError::InvalidStoredState)?).map_err(|_|DurabilityError::InvalidStoredState)?;
            if storage::load(&mut tx,request.world,&key).await.map_err(|_|DurabilityError::InvalidStoredState)?.is_none(){
                let initial=RaidSchedule::initial(request.reset_epoch,request.now_ms).map_err(|_|DurabilityError::InvalidStoredState)?;
                storage::insert_schedule(&mut tx,request.world,&key,&initial).await.map_err(|_|DurabilityError::Unavailable)?;
            }
            sqlx::query("INSERT INTO game_world_raid_schedule_sources(world_id,raid_key,source_kind) VALUES(encode($1,'hex')::uuid,$2,4) ON CONFLICT DO NOTHING")
                .bind(request.world.as_bytes().as_slice()).bind(key.as_str()).execute(&mut *tx).await?;
            let source:i16=sqlx::query_scalar("SELECT source_kind FROM game_world_raid_schedule_sources WHERE world_id=encode($1,'hex')::uuid AND raid_key=$2").bind(request.world.as_bytes().as_slice()).bind(key.as_str()).fetch_one(&mut *tx).await?;
            if source!=4 {return Err(DurabilityError::InvalidStoredState);}
            let ordinal:String=sqlx::query_scalar("SELECT next_firing_ordinal::text FROM game_world_raid_schedules WHERE world_id=encode($1,'hex')::uuid AND raid_key=$2 FOR UPDATE").bind(request.world.as_bytes().as_slice()).bind(key.as_str()).fetch_one(&mut *tx).await?;
            sqlx::query("INSERT INTO game_world_raid_firings(world_id,raid_key,ordinal,firing_id,reset_epoch,fired_at_ms,deadline_ms,bundle_digest,state) VALUES(encode($1,'hex')::uuid,$2,$3::text::numeric,encode($4,'hex')::uuid,$5::text::numeric,$6,$7,$8,1)")
                .bind(request.world.as_bytes().as_slice()).bind(key.as_str()).bind(&ordinal).bind(request.firing_id.as_slice()).bind(request.reset_epoch.to_string()).bind(request.now_ms).bind(request.now_ms.checked_add(120000).ok_or(DurabilityError::InvalidStoredState)?).bind(request.bundle_digest.as_slice()).execute(&mut *tx).await?;
            sqlx::query("UPDATE game_world_raid_schedules SET revision=revision+1,reset_epoch=$3::text::numeric,next_firing_ordinal=next_firing_ordinal+1,next_check_at_ms=GREATEST(next_check_at_ms,$4+60000),last_fired_at_ms=$4 WHERE world_id=encode($1,'hex')::uuid AND raid_key=$2")
                .bind(request.world.as_bytes().as_slice()).bind(key.as_str()).bind(request.reset_epoch.to_string()).bind(request.now_ms).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO game_world_raid_targets(firing_id,channel_id,owner_generation) SELECT encode($1,'hex')::uuid,a.channel_id,a.ownership_generation FROM game_runtime_scope_assignments a JOIN game_durability_admission_runtime_guards g USING(scope_key) WHERE a.world_id=encode($2,'hex')::uuid AND a.scope_kind=1 AND a.state=1 AND g.ready AND g.ownership_generation=a.ownership_generation AND game_node_lock_current_registration(a.holder_node_id,a.holder_registration_revision) ORDER BY a.channel_id FOR SHARE OF a,g")
                .bind(request.firing_id.as_slice()).bind(request.world.as_bytes().as_slice()).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO game_world_raid_channel_runs(firing_id,channel_id,owner_generation,updated_at_ms,source_state) SELECT firing_id,channel_id,owner_generation,$2,$3::jsonb FROM game_world_raid_targets WHERE firing_id=encode($1,'hex')::uuid")
                .bind(request.firing_id.as_slice()).bind(request.now_ms).bind(state).execute(&mut *tx).await?;
            commit_semantic_transaction(tx,deadline).await?;Ok(true)
        })).await
    }
}
