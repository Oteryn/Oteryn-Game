//! Channel-scoped terrain state. The ordinary item owner fences custody and command receipts;
//! the server item clock recovers durable deadlines without retaining player sessions.
use super::character_authority::ReconciledCharacterAuthority;
use super::character_equipment::assert_current_equipment_in_transaction;
use super::character_progression::{
    CurrentCharacterGameplayFence, ExperienceAwardRequest, ExperienceRewardOccurrence,
};
use super::db::begin_type2_transaction;
use super::item_decay_retire::{CorpseDecayFence, fence_is_live};
use super::item_transfer::{CurrentCharacterItemFence, scope_of};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::spell_owner_commit::SpellLanePermit;
use super::{DurabilityError, DurabilityRoot};
use crate::foundation::CommandRef;
use oteryn_protocol_oteryn::world_object::UseDisposition;
use sha2::{Digest, Sha256};
use sqlx::Row;

/// The existing native progression binding and revision fence accompany only a closed
/// gobbler operation. Its amount and occurrence are re-proved by the durable owner.
pub struct TerrainExperience {
    pub fence: CurrentCharacterGameplayFence,
    pub request: ExperienceAwardRequest<
        { crate::content::character_progression_content::CHARACTER_EXPERIENCE_TABLE_LEVELS },
    >,
}

pub fn voice_experience_occurrence(
    command: CommandRef,
) -> Result<ExperienceRewardOccurrence, DurabilityError> {
    let mut hash = Sha256::new();
    hash.update(b"oteryn.world_change.voice.gobble.xp.v1\0");
    hash.update(command.game_session_id().as_bytes());
    hash.update(command.command_id().get().to_be_bytes());
    let digest = hash.finalize();
    let mut bytes = [0; 16];
    bytes.copy_from_slice(&digest[..16]);
    bytes[6] = (bytes[6] & 0x0f) | 0x70;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    ExperienceRewardOccurrence::from_bytes(bytes).map_err(|_| DurabilityError::InvalidStoredState)
}

#[derive(Debug, Clone)]
pub struct TerrainMutation {
    pub tool: [u8; 16],
    pub tool_key: String,
    pub tool_revision: String,
    pub placement: Vec<u8>,
    pub expected_revision: i64,
    pub before: u32,
    pub after: u32,
    /// Successive states and delays from the preceding durable deadline, milliseconds.
    pub decay: Vec<(u32, i64)>,
}

/// A real map object used without a carried tool. Zero is the existing removed-placement
/// tombstone; positive states and decay states name admitted source appearances.
#[derive(Debug, Clone)]
pub struct MapMutation {
    pub placement: Vec<u8>,
    pub expected_revision: i64,
    pub before: u32,
    pub after: u32,
    pub decay: Vec<(u32, i64)>,
}

#[derive(Debug, Clone)]
pub struct TerrainScope {
    pub map_revision: String,
    pub content_generation: [u8; 32],
    pub reset_epoch: i64,
}

#[derive(Debug)]
pub struct TerrainState {
    pub placement: Vec<u8>,
    pub revision: i64,
    pub item: u32,
}

impl DurabilityRoot {
    /// Exact replay is checked before handles; fresh writes recheck the current equipment fence
    /// and carried custody under the same transaction as the terrain CAS and command receipt.
    #[allow(clippy::too_many_arguments)]
    pub async fn apply_terrain_tool(
        &self,
        permit: &SpellLanePermit,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        command: CommandRef,
        intent: Vec<u8>,
        scope: TerrainScope,
        mutation: Option<TerrainMutation>,
    ) -> Result<Option<UseDisposition>, DurabilityError> {
        self.apply_terrain_tool_with_experience(
            permit, authority, node, fence, command, intent, scope, mutation, None,
        )
        .await
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "retain the native item proofs and optional typed progression input"
    )]
    pub async fn apply_terrain_tool_with_experience(
        &self,
        permit: &SpellLanePermit,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        command: CommandRef,
        intent: Vec<u8>,
        scope: TerrainScope,
        mutation: Option<TerrainMutation>,
        experience: Option<Box<TerrainExperience>>,
    ) -> Result<Option<UseDisposition>, DurabilityError> {
        if command.game_session_id() != fence.game_session_id
            || intent.is_empty()
            || intent.len() > oteryn_protocol_oteryn::world_object::MAX_USE_INTENT_BYTES
        {
            return Ok(Some(UseDisposition::Rejected));
        }
        permit.check_scope(fence.runtime_scope)?;
        let record = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        let (world, channel) = scope_of(&fence).map_err(|_| DurabilityError::InvalidStoredState)?;
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_type2_transaction(holder, deadline).await?;
            super::character_authority::assert_recovery_fence(&mut tx, &record).await?;
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:terrain-command:' || encode($1,'hex') || ':' || $2,0))")
                .bind(fence.game_session_id.as_bytes().as_slice()).bind(command.command_id().get().to_string()).execute(&mut **tx).await?;
            let previous: Option<(Vec<u8>, Vec<u8>)> = sqlx::query_as("SELECT intent,uuid_send(character_id) FROM game_terrain_tool_receipts WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric")
                .bind(fence.game_session_id.as_bytes().as_slice()).bind(command.command_id().get().to_string()).fetch_optional(&mut **tx).await?;
            if let Some((prior_intent, character)) = previous {
                let result = if prior_intent == intent && character == fence.character_id.as_bytes() { UseDisposition::Committed } else { UseDisposition::Rejected };
                tx.commit(deadline).await?;
                return Ok(Some(result));
            }
            let Some(mut mutation) = mutation else { tx.commit(deadline).await?; return Ok(None); };
            let voice = mutation.tool_key == "oteryn:item.tibia.i12077";
            if voice != experience.is_some() || experience.as_ref().is_some_and(|xp|
                xp.fence.item_fence() != fence || xp.request.amount.get() != 20)
            {
                return Ok(Some(UseDisposition::Rejected));
            }
            if let Some(xp) = experience.as_ref()
                && xp.request.occurrence != voice_experience_occurrence(command)?
            {
                return Ok(Some(UseDisposition::Rejected));
            }
            if assert_current_equipment_in_transaction(&mut tx, &record, &node, &fence, Some(command), scope.content_generation).await.is_err() {
                return Ok(Some(UseDisposition::Rejected));
            }
            // The equipment guard holds the Character root lock. Only a live leaf in the
            // main backpack tree is a tool; equipment, Ground and another Character are refused.
            let tool: Option<(String, String)> = sqlx::query_as("WITH RECURSIVE carried AS (SELECT e.item_instance_id,1 AS depth FROM game_item_container_entries e JOIN game_item_container_slots b ON b.character_id=e.character_id AND b.item_instance_id=e.parent_item_instance_id WHERE e.character_id=encode($1,'hex')::uuid AND e.world_id=encode($2,'hex')::uuid UNION ALL SELECT e.item_instance_id,c.depth+1 FROM carried c JOIN game_item_nested_entries e ON e.parent_item_instance_id=c.item_instance_id WHERE c.depth<8) SELECT i.definition_production_key,i.definition_revision_ref FROM game_item_instances i WHERE i.item_instance_id=encode($3,'hex')::uuid AND i.world_id=encode($2,'hex')::uuid AND i.lifecycle=1 AND i.quantity>0 AND i.definition_family='Item' AND NOT EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=i.item_instance_id) AND NOT EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=i.item_instance_id) AND i.item_instance_id IN (SELECT item_instance_id FROM carried LIMIT 501) FOR UPDATE OF i")
                .bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).bind(mutation.tool.as_slice()).fetch_optional(&mut **tx).await?;
            if tool.as_ref().map(|(key, revision)| (key.as_str(), revision.as_str())) != Some((mutation.tool_key.as_str(), mutation.tool_revision.as_str())) { return Ok(Some(UseDisposition::StaleState)); }
            let mut voice_period = None;
            if mutation.tool_key == "oteryn:item.tibia.i12077" {
                // Crystal their_masters_voice.lua:43,111-149. Prove the exact physical
                // operation again at the durable boundary, then consult this World's
                // committed epoch inside the same fenced resource transaction.
                let Some(key) = mutation.placement.get(32..).and_then(|bytes| <[u8;8]>::try_from(bytes).ok()).map(u64::from_be_bytes) else {
                    return Ok(Some(UseDisposition::Rejected));
                };
                let x = ((key >> 32) & 0xffff) as u16;
                let y = ((key >> 16) & 0xffff) as u16;
                if mutation.placement.len() != 40
                    || oteryn_world_bundle::bundle::placement_key(-9,x,y,0) != Some(key)
                    || !(33306..=33369).contains(&x)
                    || !(31847..=31919).contains(&y)
                    || !(12059..=12063).contains(&mutation.before)
                    || mutation.after != 12065 || !mutation.decay.is_empty()
                {
                    return Ok(Some(UseDisposition::Rejected));
                }
                let epoch = u64::try_from(scope.reset_epoch).map_err(|_| DurabilityError::InvalidStoredState)?;
                let Some(day) = super::world_event::changes::load(&mut tx,world,epoch).await? else {
                    return Ok(Some(UseDisposition::Rejected));
                };
                if !oteryn_gameplay_world_change::owner::is_active(&day,"their_masters_voice") {
                    return Ok(Some(UseDisposition::Rejected));
                }
                // The equipment proof already holds the Character root lock, serializing
                // different sessions/channels. The receipt index is bounded to gobbler
                // results. Preserve Lua's inclusive integer-second exhaustion comparison.
                let exhausted: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_terrain_tool_receipts WHERE character_id=encode($1,'hex')::uuid AND transformed_item_id=12065 AND committed_at >= to_timestamp(floor(extract(epoch FROM clock_timestamp())) - 5) LIMIT 1)")
                    .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
                if exhausted { return Ok(Some(UseDisposition::Rejected)); }
                use super::world_event::changes::counters::{self, CounterKind, Credit, CreditEvent, CreditOutcome};
                let Some(current) = counters::authorize(&mut tx,&record,&node,fence.runtime_scope,
                    fence.scope_ownership_generation,epoch).await? else {
                    return Ok(Some(UseDisposition::Rejected));
                };
                let now: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint")
                    .fetch_one(&mut **tx).await?;
                let mut occurrence = b"voice:gobble:".to_vec();
                occurrence.extend_from_slice(fence.game_session_id.as_bytes());
                occurrence.extend_from_slice(&command.command_id().get().to_be_bytes());
                let event = CreditEvent {
                    source_kind: 2, occurrence,
                    credits: vec![(CounterKind::TheirMastersVoice,Credit::Task(0))],
                    counter_generations: day.counter_generations,
                    occurred_at_ms: u64::try_from(now).map_err(|_|DurabilityError::InvalidStoredState)?,
                    character: Some(crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                        .map_err(|_|DurabilityError::InvalidStoredState)?),
                };
                if !matches!(counters::credit(&mut tx,&current,&event).await?,CreditOutcome::Applied{..}) {
                    return Ok(Some(UseDisposition::Rejected));
                }
                let shared_deadline: i64 = sqlx::query_scalar("SELECT (state->>'deadline_ms')::bigint FROM game_world_change_counters WHERE world_id=encode($1,'hex')::uuid AND change_key='their_masters_voice'")
                    .bind(world.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
                let remaining = shared_deadline.checked_sub(now).filter(|delay| *delay > 0)
                    .ok_or(DurabilityError::InvalidStoredState)?;
                mutation.decay.push((mutation.before,remaining));
                voice_period = Some(shared_deadline);
            }
            let applied = apply_in_transaction(&mut tx, fence, command, &intent, &scope, &mutation).await?;
            if applied {
                if let Some(shared_deadline) = voice_period {
                    // Preserve the original ground in the existing terrain deadline chain,
                    // but all extractions share the FIRST use's absolute one-hour timer.
                    let revision = mutation.expected_revision.checked_add(1).ok_or(DurabilityError::InvalidStoredState)?;
                    let stamped = sqlx::query("UPDATE game_terrain_tool_transforms SET decay_at_unix_ms=$6 WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND placement=$3 AND reset_epoch=$4 AND object_revision=$5 AND state_item_id=12065")
                        .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).bind(&mutation.placement)
                        .bind(scope.reset_epoch).bind(revision).bind(shared_deadline).execute(&mut **tx).await?;
                    if stamped.rows_affected() != 1 { return Err(DurabilityError::InvalidStoredState); }
                    // Twenty-five uses qualify; they do not cap extraction. An old period
                    // reads as zero without an unbounded reset over all World Characters.
                    sqlx::query("INSERT INTO game_world_change_personal_counters(world_id,change_key,character_id,counter,period_deadline_ms) VALUES(encode($1,'hex')::uuid,'their_masters_voice',encode($2,'hex')::uuid,1,$3) ON CONFLICT(world_id,change_key,character_id) DO UPDATE SET counter=CASE WHEN game_world_change_personal_counters.period_deadline_ms=EXCLUDED.period_deadline_ms THEN LEAST(game_world_change_personal_counters.counter+1,25) ELSE 1 END,period_deadline_ms=EXCLUDED.period_deadline_ms")
                        .bind(world.as_bytes().as_slice()).bind(fence.character_id.as_bytes().as_slice()).bind(shared_deadline).execute(&mut **tx).await?;
                }
                if let Some(xp) = experience.as_ref()
                    && super::character_experience_transaction::commit_in_transaction(
                        &mut tx,&node,xp.fence,&xp.request,false,None).await?.is_err()
                {
                    return Ok(Some(UseDisposition::Rejected));
                }
                tx.commit(deadline).await?;
            }
            Ok(Some(if applied { UseDisposition::Committed } else { UseDisposition::StaleState }))
        })).await
    }

    /// Shared one-second item cadence: advance at most 32 due rows, then recover the bounded
    /// complete projection. Both mutation and read require the current Channel assignment.
    #[allow(clippy::too_many_arguments)]
    pub async fn terrain_tool_snapshot(
        &self,
        permit: &SpellLanePermit,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CorpseDecayFence,
        scope: TerrainScope,
    ) -> Result<Vec<TerrainState>, DurabilityError> {
        // Terrain transforms retain their existing Channel custody namespace.
        let crate::foundation::RuntimeScopeRefV1::Channel {
            world_id,
            channel_id,
        } = fence.runtime_scope
        else {
            return Err(DurabilityError::Unavailable);
        };
        permit.check_scope(fence.runtime_scope)?;
        let record = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_type2_transaction(holder, deadline).await?;
            super::character_authority::assert_recovery_fence(&mut tx, &record).await?;
            if !fence_is_live(&mut tx, &fence, &node).await? { return Err(DurabilityError::Unavailable); }
            sqlx::query("WITH due AS (SELECT world_id,channel_id,placement,reset_epoch FROM game_terrain_tool_transforms WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND map_revision=$3 AND content_generation=$4 AND reset_epoch=$5 AND decay_at_unix_ms <= floor(extract(epoch FROM clock_timestamp())*1000)::bigint ORDER BY decay_at_unix_ms,placement LIMIT 32 FOR UPDATE) UPDATE game_terrain_tool_transforms t SET state_item_id=t.decay_states[t.decay_step+1],object_revision=t.object_revision+1,decay_step=t.decay_step+1,decay_at_unix_ms=CASE WHEN t.decay_step+1 < cardinality(t.decay_states) THEN t.decay_at_unix_ms+t.decay_delays[t.decay_step+2] ELSE NULL END FROM due WHERE (t.world_id,t.channel_id,t.placement,t.reset_epoch)=(due.world_id,due.channel_id,due.placement,due.reset_epoch)")
                .bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice()).bind(&scope.map_revision).bind(scope.content_generation.as_slice()).bind(scope.reset_epoch).execute(&mut **tx).await?;
            let rows = sqlx::query("SELECT placement,object_revision,state_item_id,cake_event_year FROM game_terrain_tool_transforms WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND map_revision=$3 AND content_generation=$4 AND reset_epoch=$5 ORDER BY placement LIMIT 9217")
                .bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice()).bind(scope.map_revision).bind(scope.content_generation.as_slice()).bind(scope.reset_epoch).fetch_all(&mut **tx).await?;
            if rows.len()>9216 { return Err(DurabilityError::InvalidStoredState); }
            let mut ordinary=0_usize;
            let mut cake=0_usize;
            for row in &rows {
                if row.try_get::<Option<i32>,_>("cake_event_year")?.is_some(){cake+=1;}else{ordinary+=1;}
            }
            if ordinary>1024||cake>8192{return Err(DurabilityError::InvalidStoredState);}

            let result = rows.into_iter().map(|row| Ok(TerrainState { placement: row.try_get("placement")?, revision: row.try_get("object_revision")?, item: u32::try_from(row.try_get::<i64,_>("state_item_id")?).map_err(|_| DurabilityError::InvalidStoredState)? })).collect::<Result<Vec<_>,DurabilityError>>()?;
            tx.commit(deadline).await?;
            Ok(result)
        })).await
    }
}

/// The item craft owner uses this CAS in its own inventory transaction.
pub(super) async fn apply_in_transaction(
    tx: &mut super::item_mint_audit::Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    command: CommandRef,
    intent: &[u8],
    scope: &TerrainScope,
    mutation: &TerrainMutation,
) -> Result<bool, DurabilityError> {
    apply_object_in_transaction(
        tx,
        fence,
        command,
        intent,
        scope,
        Some(mutation.tool),
        &MapMutation {
            placement: mutation.placement.clone(),
            expected_revision: mutation.expected_revision,
            before: mutation.before,
            after: mutation.after,
            decay: mutation.decay.clone(),
        },
    )
    .await
}

/// Shared with source map harvesting and world-event consumption. The caller holds the
/// complete current Character/Channel fence and commits its value/counter receipt in this
/// same transaction. A failed CAS must abort every promised consequence.
pub(super) async fn apply_map_in_transaction(
    tx: &mut super::item_mint_audit::Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    command: CommandRef,
    intent: &[u8],
    scope: &TerrainScope,
    mutation: &MapMutation,
) -> Result<bool, DurabilityError> {
    apply_object_in_transaction(tx, fence, command, intent, scope, None, mutation).await
}

async fn apply_object_in_transaction(
    tx: &mut super::item_mint_audit::Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    command: CommandRef,
    intent: &[u8],
    scope: &TerrainScope,
    tool: Option<[u8; 16]>,
    mutation: &MapMutation,
) -> Result<bool, DurabilityError> {
    let (world, channel) = scope_of(&fence).map_err(|_| DurabilityError::InvalidStoredState)?;
    let placement: &[u8; 40] = mutation
        .placement
        .as_slice()
        .try_into()
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let reset_epoch =
        u64::try_from(scope.reset_epoch).map_err(|_| DurabilityError::InvalidStoredState)?;
    super::map_item_mint::lock_placement(
        tx,
        world.as_bytes(),
        channel.as_bytes(),
        placement,
        reset_epoch,
    )
    .await?;
    let placement_key = u64::from_be_bytes(
        placement[32..]
            .try_into()
            .map_err(|_| DurabilityError::InvalidStoredState)?,
    );
    let taken: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_map_item_mint_receipts WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND base_bundle_digest=$3 AND placement_key=$4::text::numeric AND reset_epoch=$5::text::numeric)")
        .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).bind(&placement[..32]).bind(placement_key.to_string()).bind(reset_epoch.to_string()).fetch_one(&mut ***tx).await?;
    if taken {
        return Ok(false);
    }
    let states: Vec<i64> = mutation
        .decay
        .iter()
        .map(|(state, _)| i64::from(*state))
        .collect();
    let delays: Vec<i64> = mutation.decay.iter().map(|(_, delay)| *delay).collect();
    let applied: bool = sqlx::query_scalar("SELECT game_apply_terrain_tool_transform(encode($1,'hex')::uuid,$2::text::numeric,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5,encode($6,'hex')::uuid,encode($7,'hex')::uuid,$8,$9,$10,$11,$12,$13,$14,$15,$16)")
                .bind(fence.game_session_id.as_bytes().as_slice()).bind(command.command_id().get().to_string())
                .bind(fence.character_id.as_bytes().as_slice()).bind(tool.as_ref().map(|id| id.as_slice())).bind(intent)
                .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).bind(&mutation.placement)
                .bind(&scope.map_revision).bind(scope.content_generation.as_slice()).bind(scope.reset_epoch)
                .bind(mutation.expected_revision).bind(i64::from(mutation.before)).bind(i64::from(mutation.after))
                .bind(states).bind(delays).fetch_one(&mut ***tx).await?;
    Ok(applied)
}
