//! M08: the squeezing gear of girlpower's jam, a durable one-minute deadline on the tool.
//! Crystal data/scripts/actions/tools/squeezing_gear_of_girlpower.lua:34-40 transforms the gear
//! to the jammed 9597 (items.xml:25842-25847, duration 60, decayTo 9596). The deadline is owned
//! here and started by the database clock under the same fences as a terrain tool use.
use super::character_authority::ReconciledCharacterAuthority;
use super::character_equipment::assert_current_equipment_in_transaction;
use super::db::begin_type2_transaction;
use super::item_transfer::{CurrentCharacterItemFence, scope_of};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::spell_owner_commit::SpellLanePermit;
use super::{DurabilityError, DurabilityRoot};
use crate::foundation::CommandRef;
use oteryn_protocol_oteryn::world_object::UseDisposition;

impl DurabilityRoot {
    /// Start (or restart) the jam of one carried tool. An exact replay returns `Committed`;
    /// the same `CommandRef` with another intent is `Rejected`.
    #[allow(clippy::too_many_arguments)]
    pub async fn jam_tool(
        &self,
        permit: &SpellLanePermit,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        command: CommandRef,
        intent: Vec<u8>,
        tool: [u8; 16],
        tool_key: String,
        tool_revision: String,
        jam_ms: i64,
        content_generation: [u8; 32],
    ) -> Result<UseDisposition, DurabilityError> {
        if command.game_session_id() != fence.game_session_id
            || intent.is_empty()
            || intent.len() > oteryn_protocol_oteryn::world_object::MAX_USE_INTENT_BYTES
            || jam_ms <= 0
        {
            return Ok(UseDisposition::Rejected);
        }
        permit.check_scope(fence.runtime_scope)?;
        let record = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        let (world, _) = scope_of(&fence).map_err(|_| DurabilityError::InvalidStoredState)?;
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_type2_transaction(holder, deadline).await?;
            super::character_authority::assert_recovery_fence(&mut tx, &record).await?;
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:tool-jam:' || encode($1,'hex') || ':' || $2,0))")
                .bind(fence.game_session_id.as_bytes().as_slice()).bind(command.command_id().get().to_string()).execute(&mut **tx).await?;
            let previous: Option<(Vec<u8>, Vec<u8>)> = sqlx::query_as("SELECT intent,uuid_send(character_id) FROM game_tool_jam_receipts WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric")
                .bind(fence.game_session_id.as_bytes().as_slice()).bind(command.command_id().get().to_string()).fetch_optional(&mut **tx).await?;
            if let Some((prior_intent, character)) = previous {
                let result = if prior_intent == intent && character == fence.character_id.as_bytes() { UseDisposition::Committed } else { UseDisposition::Rejected };
                tx.commit(deadline).await?;
                return Ok(result);
            }
            if assert_current_equipment_in_transaction(&mut tx, &record, &node, &fence, Some(command), content_generation).await.is_err() {
                return Ok(UseDisposition::Rejected);
            }
            // Only a live leaf in the main backpack tree is a tool (as for a terrain tool).
            let carried: Option<(String, String)> = sqlx::query_as("WITH RECURSIVE carried AS (SELECT e.item_instance_id,1 AS depth FROM game_item_container_entries e JOIN game_item_container_slots b ON b.character_id=e.character_id AND b.item_instance_id=e.parent_item_instance_id WHERE e.character_id=encode($1,'hex')::uuid AND e.world_id=encode($2,'hex')::uuid UNION ALL SELECT e.item_instance_id,c.depth+1 FROM carried c JOIN game_item_nested_entries e ON e.parent_item_instance_id=c.item_instance_id WHERE c.depth<8) SELECT i.definition_production_key,i.definition_revision_ref FROM game_item_instances i WHERE i.item_instance_id=encode($3,'hex')::uuid AND i.world_id=encode($2,'hex')::uuid AND i.lifecycle=1 AND i.quantity>0 AND i.definition_family='Item' AND i.item_instance_id IN (SELECT item_instance_id FROM carried LIMIT 501) FOR UPDATE OF i")
                .bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).bind(tool.as_slice()).fetch_optional(&mut **tx).await?;
            if carried.as_ref().map(|(key, revision)| (key.as_str(), revision.as_str())) != Some((tool_key.as_str(), tool_revision.as_str())) {
                return Ok(UseDisposition::StaleState);
            }
            sqlx::query("INSERT INTO game_tool_jams(tool_item_instance_id,character_id,jammed_until_unix_ms) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,floor(extract(epoch FROM clock_timestamp())*1000)::bigint+$3) ON CONFLICT (tool_item_instance_id) DO UPDATE SET character_id=EXCLUDED.character_id,jammed_until_unix_ms=EXCLUDED.jammed_until_unix_ms")
                .bind(tool.as_slice()).bind(fence.character_id.as_bytes().as_slice()).bind(jam_ms).execute(&mut **tx).await?;
            sqlx::query("INSERT INTO game_tool_jam_receipts(game_session_id,command_id,character_id,tool_item_instance_id,intent) VALUES(encode($1,'hex')::uuid,$2::text::numeric,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5)")
                .bind(fence.game_session_id.as_bytes().as_slice()).bind(command.command_id().get().to_string()).bind(fence.character_id.as_bytes().as_slice()).bind(tool.as_slice()).bind(&intent).execute(&mut **tx).await?;
            tx.commit(deadline).await?;
            Ok(UseDisposition::Committed)
        })).await
    }

    /// Whether the carried tool is jammed now, by the database clock.
    pub async fn tool_jam_active(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        character: [u8; 16],
        tool: [u8; 16],
    ) -> Result<bool, DurabilityError> {
        let record = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_type2_transaction(holder, deadline).await?;
            super::character_authority::assert_recovery_fence(&mut tx, &record).await?;
            let active: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_tool_jams WHERE tool_item_instance_id=encode($1,'hex')::uuid AND character_id=encode($2,'hex')::uuid AND jammed_until_unix_ms>floor(extract(epoch FROM clock_timestamp())*1000)::bigint)")
                .bind(tool.as_slice()).bind(character.as_slice()).fetch_one(&mut **tx).await?;
            tx.commit(deadline).await?;
            Ok(active)
        })).await
    }
}
