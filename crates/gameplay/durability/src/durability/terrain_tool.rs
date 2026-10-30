//! Channel-scoped terrain state. The ordinary item owner fences custody and command receipts;
//! the server item clock recovers durable deadlines without retaining player sessions.
use super::character_authority::ReconciledCharacterAuthority;
use super::character_equipment::assert_current_equipment_in_transaction;
use super::db::begin_type2_transaction;
use super::item_decay_retire::{CorpseDecayFence, fence_is_live};
use super::item_mint::TypedDefinitionRef;
use super::item_transfer::{CurrentCharacterItemFence, scope_of};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::spell_owner_commit::SpellLanePermit;
use super::{DurabilityError, DurabilityRoot};
use crate::foundation::CommandRef;
use oteryn_protocol_oteryn::world_object::UseDisposition;
use sqlx::Row;

#[derive(Debug, Clone)]
pub struct TerrainMutation {
    pub tool: [u8; 16],
    pub tool_key: String,
    pub tool_revision: String,
    pub placement: Vec<u8>,
    pub expected_revision: i64,
    pub before: u32,
    pub after: u32,
    /// Optional harvest output. Harvests merge one item into a matching carried stack in the
    /// same transaction as the terrain compare-and-swap.
    pub yield_definition: Option<TypedDefinitionRef>,
    /// Successive states and delays from the preceding durable deadline, milliseconds.
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
            let Some(mutation) = mutation else { tx.commit(deadline).await?; return Ok(None); };
            if assert_current_equipment_in_transaction(&mut tx, &record, &node, &fence, Some(command), scope.content_generation).await.is_err() {
                return Ok(Some(UseDisposition::Rejected));
            }
            // The equipment guard holds the Character root lock. Only a live leaf in the
            // main backpack tree is a tool; equipment, Ground and another Character are refused.
            let tool: Option<(String, String)> = sqlx::query_as("WITH RECURSIVE carried AS (SELECT e.item_instance_id,1 AS depth FROM game_item_container_entries e JOIN game_item_container_slots b ON b.character_id=e.character_id AND b.item_instance_id=e.parent_item_instance_id WHERE e.character_id=encode($1,'hex')::uuid AND e.world_id=encode($2,'hex')::uuid UNION ALL SELECT e.item_instance_id,c.depth+1 FROM carried c JOIN game_item_nested_entries e ON e.parent_item_instance_id=c.item_instance_id WHERE c.depth<8) SELECT i.definition_production_key,i.definition_revision_ref FROM game_item_instances i WHERE i.item_instance_id=encode($3,'hex')::uuid AND i.world_id=encode($2,'hex')::uuid AND i.lifecycle=1 AND i.quantity>0 AND i.definition_family='Item' AND NOT EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=i.item_instance_id) AND NOT EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=i.item_instance_id) AND i.item_instance_id IN (SELECT item_instance_id FROM carried LIMIT 501) FOR UPDATE OF i")
                .bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).bind(mutation.tool.as_slice()).fetch_optional(&mut **tx).await?;
            if tool.as_ref().map(|(key, revision)| (key.as_str(), revision.as_str())) != Some((mutation.tool_key.as_str(), mutation.tool_revision.as_str())) { return Ok(Some(UseDisposition::StaleState)); }
            let states: Vec<i64> = mutation.decay.iter().map(|(state, _)| i64::from(*state)).collect();
            let delays: Vec<i64> = mutation.decay.iter().map(|(_, delay)| *delay).collect();
            let applied: bool = sqlx::query_scalar("SELECT game_apply_terrain_tool_transform(encode($1,'hex')::uuid,$2::text::numeric,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5,encode($6,'hex')::uuid,encode($7,'hex')::uuid,$8,$9,$10,$11,$12,$13,$14,$15,$16)")
                .bind(fence.game_session_id.as_bytes().as_slice()).bind(command.command_id().get().to_string())
                .bind(fence.character_id.as_bytes().as_slice()).bind(mutation.tool.as_slice()).bind(&intent)
                .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).bind(mutation.placement)
                .bind(scope.map_revision).bind(scope.content_generation.as_slice()).bind(scope.reset_epoch)
                .bind(mutation.expected_revision).bind(i64::from(mutation.before)).bind(i64::from(mutation.after))
                .bind(states).bind(delays).fetch_one(&mut **tx).await?;
            if applied && let Some(yield_definition) = mutation.yield_definition {
                let granted = sqlx::query("UPDATE game_item_instances i SET quantity=i.quantity+1,state_revision=i.state_revision+1,last_transaction_id=encode($1,'hex')::uuid FROM game_item_container_entries e WHERE e.item_instance_id=i.item_instance_id AND e.character_id=encode($2,'hex')::uuid AND i.world_id=encode($3,'hex')::uuid AND i.lifecycle=1 AND i.definition_family=$4 AND i.definition_production_key=$5 AND i.definition_revision_ref=$6 AND i.quantity<100")
                    .bind(fence.game_session_id.as_bytes().as_slice()).bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).bind(&yield_definition.family).bind(&yield_definition.production_key).bind(&yield_definition.revision_ref).execute(&mut **tx).await?;
                if granted.rows_affected() != 1 { return Ok(Some(UseDisposition::Rejected)); }
            }
            if applied { tx.commit(deadline).await?; }
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
        permit.check_scope(crate::foundation::RuntimeScopeRefV1::Channel {
            world_id: fence.world_id,
            channel_id: fence.channel_id,
        })?;
        let record = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_type2_transaction(holder, deadline).await?;
            super::character_authority::assert_recovery_fence(&mut tx, &record).await?;
            if !fence_is_live(&mut tx, &fence, &node).await? { return Err(DurabilityError::Unavailable); }
            sqlx::query("WITH due AS (SELECT world_id,channel_id,placement,reset_epoch FROM game_terrain_tool_transforms WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND map_revision=$3 AND content_generation=$4 AND reset_epoch=$5 AND decay_at_unix_ms <= floor(extract(epoch FROM clock_timestamp())*1000)::bigint ORDER BY decay_at_unix_ms,placement LIMIT 32 FOR UPDATE) UPDATE game_terrain_tool_transforms t SET state_item_id=t.decay_states[t.decay_step+1],object_revision=t.object_revision+1,decay_step=t.decay_step+1,decay_at_unix_ms=CASE WHEN t.decay_step+1 < cardinality(t.decay_states) THEN t.decay_at_unix_ms+t.decay_delays[t.decay_step+2] ELSE NULL END FROM due WHERE (t.world_id,t.channel_id,t.placement,t.reset_epoch)=(due.world_id,due.channel_id,due.placement,due.reset_epoch)")
                .bind(fence.world_id.as_bytes().as_slice()).bind(fence.channel_id.as_bytes().as_slice()).bind(&scope.map_revision).bind(scope.content_generation.as_slice()).bind(scope.reset_epoch).execute(&mut **tx).await?;
            let rows = sqlx::query("SELECT placement,object_revision,state_item_id FROM game_terrain_tool_transforms WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND map_revision=$3 AND content_generation=$4 AND reset_epoch=$5 ORDER BY placement LIMIT 1025")
                .bind(fence.world_id.as_bytes().as_slice()).bind(fence.channel_id.as_bytes().as_slice()).bind(scope.map_revision).bind(scope.content_generation.as_slice()).bind(scope.reset_epoch).fetch_all(&mut **tx).await?;
            if rows.len()>1024 { return Err(DurabilityError::InvalidStoredState); }
            let result = rows.into_iter().map(|row| Ok(TerrainState { placement: row.try_get("placement")?, revision: row.try_get("object_revision")?, item: u32::try_from(row.try_get::<i64,_>("state_item_id")?).map_err(|_| DurabilityError::InvalidStoredState)? })).collect::<Result<Vec<_>,DurabilityError>>()?;
            tx.commit(deadline).await?;
            Ok(result)
        })).await
    }
}
