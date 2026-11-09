//! Cake's annual world phase. Existing world/reset owners call these helpers
//! inside their already fenced physical transaction; no new identity registry.
use super::DurabilityError;
use crate::foundation::WorldId;
use sqlx::{Postgres, Transaction};

/// Crystal globalevents_cake_eating_window.lua:29-80. The normal current world
/// cadence settles at most the single current annual eating window. The phase
/// lock serializes concurrent Channel owners, so one layer cannot settle twice.
pub(super) async fn settle_current_world(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    epoch: u64,
) -> Result<(), DurabilityError> {
    sqlx::query("SELECT game_cake_settle_window(p.world_id,p.event_year,$2::text::numeric) FROM game_cake_phases p WHERE p.world_id=encode($1,'hex')::uuid AND p.stage=2 AND p.window_opened_at IS NOT NULL ORDER BY p.event_year DESC LIMIT 1")
        .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).execute(&mut **tx).await?;
    Ok(())
}

/// Settle the old occurrence BEFORE reset activation advances the active epoch.
/// This retains a completed round's next-save debt when nobody observes the isle.
pub(super) async fn before_reset(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
) -> Result<(), DurabilityError> {
    let epoch: Option<String> = sqlx::query_scalar("SELECT reset_epoch::text FROM game_world_reset_active WHERE world_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(world.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    if let Some(epoch) = epoch {
        let epoch = epoch
            .parse()
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        settle_current_world(tx, world, epoch).await?;
    }
    Ok(())
}

/// Crystal globalevents_cake_server_save.lua:8-10 and config.lua:456-470.
/// This runs only after the reset owner activates its immutable successor epoch.
pub(super) async fn after_reset(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    epoch: u64,
) -> Result<(), DurabilityError> {
    let _ = super::raid_runner::cake::phase_one_in_transaction(tx, world, epoch).await?;
    sqlx::query("SELECT game_cake_reset_continuation(encode($1,'hex')::uuid,$2::text::numeric)")
        .bind(world.as_bytes().as_slice())
        .bind(epoch.to_string())
        .execute(&mut **tx)
        .await?;
    Ok(())
}

use super::DurabilityRoot;
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_equipment::assert_current_equipment_in_transaction;
use super::db::{
    begin_semantic_transaction, begin_type2_transaction, commit_semantic_transaction,
    lock_admission_relations,
};
use super::item_decay_retire::{CorpseDecayFence, fence_is_live};
use super::item_transfer::{CurrentCharacterItemFence, scope_of};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::spell_owner_commit::SpellLanePermit;
use super::terrain_tool::{MapMutation, TerrainScope, TerrainState};
use crate::foundation::CommandRef;
use oteryn_protocol_oteryn::world_object::UseDisposition;
use serde::Serialize;
use sqlx::Row;

#[derive(Clone, Debug, Serialize)]
pub struct CakeMapSource {
    pub placement: String,
    pub spatial: String,
    pub source_item: u16,
    pub x: u16,
    pub y: u16,
    pub z: u8,
}
#[derive(Clone, Debug)]
pub struct CakeBite {
    pub command: CommandRef,
    pub intent: Vec<u8>,
    pub scope: TerrainScope,
    pub mutation: MapMutation,
    pub floor: u8,
}

async fn latest_year(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
) -> Result<Option<i32>, DurabilityError> {
    Ok(sqlx::query_scalar("SELECT event_year FROM game_cake_world_progress WHERE world_id=encode($1,'hex')::uuid ORDER BY event_year DESC LIMIT 1")
       .bind(world.as_bytes().as_slice()).fetch_optional(&mut **tx).await?)
}

impl DurabilityRoot {
    /// Fixed native source, world contribution and existing map CAS share one
    /// current Character/Channel Type2 transaction. A refused CAS rolls all back.
    #[allow(clippy::too_many_arguments)]
    pub async fn bite_world_cake(
        &self,
        permit: &SpellLanePermit,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: CakeBite,
    ) -> Result<UseDisposition, DurabilityError> {
        if request.command.game_session_id() != fence.game_session_id
            || request.intent.is_empty()
            || request.intent.len() > 529
            || !matches!(request.mutation.before, 12085 | 12086)
            || request.mutation.after != 0
            || !request.mutation.decay.is_empty()
            || request.mutation.placement.len() != 40
        {
            return Ok(UseDisposition::Rejected);
        }
        permit.check_scope(fence.runtime_scope)?;
        let (world, channel) = scope_of(&fence).map_err(|_| DurabilityError::InvalidStoredState)?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move {
            let mut tx=begin_type2_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;
            if assert_current_equipment_in_transaction(&mut tx,&recovery,&node,&fence,Some(request.command),request.scope.content_generation).await.is_err(){
                return Ok(UseDisposition::StaleState)
            }
            let current:bool=sqlx::query_scalar("SELECT game_world_reset_map_epoch_current(encode($1,'hex')::uuid,$2::text::numeric) AND EXISTS(SELECT 1 FROM game_world_reset_active a JOIN game_world_reset_epochs e USING(world_id,reset_epoch) WHERE a.world_id=encode($1,'hex')::uuid AND a.reset_epoch=$2::text::numeric AND e.target_digest=$3)")
                .bind(world.as_bytes().as_slice()).bind(request.scope.reset_epoch.to_string()).bind(&request.mutation.placement[..32]).fetch_one(&mut **tx).await?;
            if !current{return Ok(UseDisposition::StaleState)}
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:cake-command:'||encode($1,'hex')||':'||$2,0))")
                .bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string()).execute(&mut **tx).await?;
            let year:Option<i32>=sqlx::query_scalar("SELECT event_year FROM game_cake_bite_receipts WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric")
                .bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string()).fetch_optional(&mut **tx).await?;
            let year=match year {Some(year)=>Some(year),None=>latest_year(&mut tx,world).await?};
            let Some(year)=year else{return Ok(UseDisposition::NotPossible)};
            let prepared:String=sqlx::query_scalar("SELECT game_cake_bite_prepare(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,$4::text::numeric,encode($5,'hex')::uuid,$6,$7,$8,$9,$10)")
                .bind(world.as_bytes().as_slice()).bind(year).bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string())
                .bind(fence.character_id.as_bytes().as_slice()).bind(&request.mutation.placement).bind(request.mutation.expected_revision)
                .bind(i32::try_from(request.mutation.before).map_err(|_|DurabilityError::InvalidStoredState)?).bind(i16::from(request.floor)).bind(&request.intent)
                .fetch_one(&mut **tx).await?;
            if prepared=="replay" {tx.commit(deadline).await?;return Ok(UseDisposition::Committed)}
            if prepared!="prepared" {
                // Opening a window precedes donor eligibility checks and is
                // intentionally retained even for an ineligible first bite.
                tx.commit(deadline).await?;
                return Ok(match prepared.as_str(){"cooldown"=>UseDisposition::NotPossible,"stale"=>UseDisposition::StaleState,"exhausted"=>UseDisposition::Exhausted,_=>UseDisposition::NotPossible})
            }
            if !super::terrain_tool::apply_map_in_transaction(&mut tx,fence,request.command,&request.intent,&request.scope,&request.mutation).await?{
                return Ok(UseDisposition::StaleState)
            }
            sqlx::query("UPDATE game_terrain_tool_transforms SET cake_event_year=$5,cake_source_item=$6 WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND placement=$3 AND reset_epoch=$4")
                .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).bind(request.mutation.placement).bind(request.scope.reset_epoch).bind(year)
                .bind(i32::try_from(request.mutation.before).map_err(|_|DurabilityError::InvalidStoredState)?).execute(&mut **tx).await?;
            tx.commit(deadline).await?;Ok(UseDisposition::Committed)
        })).await
    }

    /// Recover the shared world phase into the already owned fixed-map rows of
    /// one exact current Channel. Another Channel gets no mutation authority.
    #[allow(clippy::too_many_arguments)]
    pub async fn project_world_cake(
        &self,
        node: &NodeIncarnationProof,
        fence: CorpseDecayFence,
        bundle: [u8; 32],
        scope: TerrainScope,
        sources: Vec<CakeMapSource>,
    ) -> Result<Vec<TerrainState>, DurabilityError> {
        if sources.len() > 8192 {
            return Err(DurabilityError::InvalidStoredState);
        }
        let crate::foundation::RuntimeScopeRefV1::Channel {
            world_id,
            channel_id,
        } = fence.runtime_scope
        else {
            return Err(DurabilityError::Unavailable);
        };
        let sources =
            serde_json::to_value(sources).map_err(|_| DurabilityError::InvalidStoredState)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move {
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            lock_admission_relations(&mut tx).await?;
            if !fence_is_live(&mut tx,&fence,&node).await?{return Err(DurabilityError::Unavailable)}
            let ready:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_runtime_scope_assignments a JOIN game_durability_admission_runtime_guards g USING(scope_key) WHERE a.world_id=encode($1,'hex')::uuid AND a.channel_id=encode($2,'hex')::uuid AND a.scope_kind=1 AND a.state=1 AND a.ownership_generation=$3::text::numeric AND g.ready AND g.ownership_generation=a.ownership_generation)")
                .bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice()).bind(fence.scope_ownership_generation.get().to_string()).fetch_one(&mut *tx).await?;
            if !ready{return Err(DurabilityError::Unavailable)}
            let Some(year)=latest_year(&mut tx,world_id).await? else{commit_semantic_transaction(tx,deadline).await?;return Ok(Vec::new())};
            let projected:bool=sqlx::query_scalar("SELECT game_cake_project_channel(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,$5,$6,$7,$8)")
                .bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice()).bind(bundle.as_slice()).bind(&scope.map_revision)
                .bind(scope.content_generation.as_slice()).bind(scope.reset_epoch).bind(year).bind(sources).fetch_one(&mut *tx).await?;
            if !projected{return Err(DurabilityError::InvalidStoredState)}
            let rows=sqlx::query("SELECT placement,object_revision,state_item_id FROM game_terrain_tool_transforms WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND reset_epoch=$3 AND map_revision=$4 AND content_generation=$5 AND cake_event_year=$6 ORDER BY placement LIMIT 8193")
                .bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice()).bind(scope.reset_epoch).bind(scope.map_revision).bind(scope.content_generation.as_slice()).bind(year).fetch_all(&mut *tx).await?;
            if rows.len()>8192{return Err(DurabilityError::InvalidStoredState)}
            let result=rows.into_iter().map(|row|Ok(TerrainState{placement:row.try_get("placement")?,revision:row.try_get("object_revision")?,item:u32::try_from(row.try_get::<i64,_>("state_item_id")?).map_err(|_|DurabilityError::InvalidStoredState)?})).collect::<Result<Vec<_>,DurabilityError>>()?;
            commit_semantic_transaction(tx,deadline).await?;Ok(result)
        })).await
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CakeWorldBonus {
    pub remaining_ms: u64,
}
impl DurabilityRoot {
    /// NPC spawning reads the latest annual occurrence, including the period
    /// after the seasonal catalogue key ends. Only the reset owner changes it.
    pub async fn cake_brutus_present(&self, world: WorldId) -> Result<bool, DurabilityError> {
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move {
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            let present:bool=sqlx::query_scalar("SELECT coalesce((SELECT p.brutus_present FROM game_cake_world_progress w LEFT JOIN game_cake_phases p USING(world_id,event_year) WHERE w.world_id=encode($1,'hex')::uuid ORDER BY w.event_year DESC LIMIT 1),false)")
                .bind(world.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
            commit_semantic_transaction(tx,deadline).await?;Ok(present)
        })).await
    }
    /// Expiry uses the same database clock that opens the seven-day bonus. The
    /// caller installs the remaining duration on the normal condition clock.
    pub async fn cake_world_bonus(
        &self,
        world: WorldId,
    ) -> Result<Option<CakeWorldBonus>, DurabilityError> {
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move {
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            let remaining:Option<i64>=sqlx::query_scalar("SELECT floor(extract(epoch FROM(bonus_until-clock_timestamp()))*1000)::bigint FROM game_cake_phases WHERE world_id=encode($1,'hex')::uuid AND bonus_activated_at<=clock_timestamp() AND clock_timestamp()<bonus_until ORDER BY event_year DESC LIMIT 1")
                .bind(world.as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
            let bonus=remaining.filter(|ms|*ms>0).map(|ms|u64::try_from(ms).map(|remaining_ms|CakeWorldBonus{remaining_ms}).map_err(|_|DurabilityError::InvalidStoredState)).transpose()?;
            commit_semantic_transaction(tx,deadline).await?;Ok(bonus)
        })).await
    }
}

impl DurabilityRoot {
    /// Probe immutable command history before the current map can hide the eaten
    /// placement. Current session/node authority is re-proved; no old actor is restored.
    #[allow(clippy::too_many_arguments)]
    pub async fn replay_world_cake(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        command: CommandRef,
        intent: Vec<u8>,
        content: [u8; 32],
        candidate: bool,
    ) -> Result<Option<UseDisposition>, DurabilityError> {
        if command.game_session_id() != fence.game_session_id {
            return Ok(Some(UseDisposition::StaleState));
        }
        let record = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&record).await?;
            if !candidate {
                let historical:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_cake_bite_receipts WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric)")
                    .bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string()).fetch_one(&mut *tx).await?;
                if !historical{commit_semantic_transaction(tx,deadline).await?;return Ok(None)}
            }
            if assert_current_equipment_in_transaction(&mut tx,&record,&node,&fence,Some(command),content).await.is_err(){return Ok(Some(UseDisposition::StaleState))}
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:cake-command:'||encode($1,'hex')||':'||$2,0))")
                .bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string()).execute(&mut *tx).await?;
            let row:Option<(Vec<u8>,Vec<u8>)>=sqlx::query_as("SELECT uuid_send(character_id),intent FROM game_cake_bite_receipts WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric")
                .bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string()).fetch_optional(&mut *tx).await?;
            let Some((character,stored))=row else{commit_semantic_transaction(tx,deadline).await?;return Ok(None)};
            let result=if character==fence.character_id.as_bytes()&&stored==intent{UseDisposition::Committed}else{UseDisposition::StaleState};
            commit_semantic_transaction(tx,deadline).await?;Ok(Some(result))
        })).await
    }
}

impl DurabilityRoot {
    /// A bounded read by the same live Channel owner that will project the map.
    /// Presence survives stage-three completion and ends only at the donor cleanup.
    pub async fn cake_bridge_present(
        &self,
        node: &NodeIncarnationProof,
        fence: CorpseDecayFence,
        bundle: [u8; 32],
        scope: TerrainScope,
    ) -> Result<bool, DurabilityError> {
        let crate::foundation::RuntimeScopeRefV1::Channel {
            world_id,
            channel_id,
        } = fence.runtime_scope
        else {
            return Err(DurabilityError::Unavailable);
        };
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            lock_admission_relations(&mut tx).await?;
            if !fence_is_live(&mut tx, &fence, &node).await? {
                return Err(DurabilityError::Unavailable);
            }
            let present: Option<bool> = sqlx::query_scalar("SELECT coalesce((SELECT p.stage IN (2,3) AND NOT p.event_end_cleaned_up FROM game_cake_world_progress w LEFT JOIN game_cake_phases p USING(world_id,event_year) WHERE w.world_id=a.world_id ORDER BY w.event_year DESC LIMIT 1),false) FROM game_world_reset_active a JOIN game_world_reset_epochs r USING(world_id,reset_epoch) WHERE a.world_id=encode($1,'hex')::uuid AND a.reset_epoch=$2::text::numeric AND game_world_reset_map_epoch_current(a.world_id,a.reset_epoch) AND r.target_digest=$5 AND r.map_revision=$6 AND EXISTS(SELECT 1 FROM game_content_activations c WHERE c.world_id=a.world_id AND c.channel_id=encode($3,'hex')::uuid AND c.activation_sequence=(SELECT max(z.activation_sequence) FROM game_content_activations z WHERE z.world_id=c.world_id AND z.channel_id=c.channel_id) AND c.server_artifact_digest=$7) AND EXISTS(SELECT 1 FROM game_runtime_scope_assignments s JOIN game_durability_admission_runtime_guards g USING(scope_key) WHERE s.world_id=a.world_id AND s.channel_id=encode($3,'hex')::uuid AND s.scope_kind=1 AND s.state=1 AND s.ownership_generation=$4::text::numeric AND g.ready AND g.ownership_generation=s.ownership_generation)")
                .bind(world_id.as_bytes().as_slice()).bind(scope.reset_epoch.to_string())
                .bind(channel_id.as_bytes().as_slice()).bind(fence.scope_ownership_generation.get().to_string())
                .bind(bundle.as_slice()).bind(scope.map_revision).bind(scope.content_generation.as_slice())
                .fetch_optional(&mut *tx).await?;
            let present = present.ok_or(DurabilityError::Unavailable)?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(present)
        })).await
    }
}
