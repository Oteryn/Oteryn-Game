//! SRVGAP-C1: one fenced cleanup, immutable receipt and saved destination in one transaction.
//! Crystal 201189f8 `data-global/scripts/actions/dawnport/vocation_door.lua:26-36` and
//! `data-global/lib/others/dawnport.lua:145-171`. Login kits remain a separate once grant.
use super::character_authority::ReconciledCharacterAuthority;
use super::character_equipment::assert_current_equipment_in_transaction;
use super::character_last_position::{
    FinalLastPosition, LastPositionKey, LastPositionWrite, upsert_last_position,
};
use super::item_transfer::{CurrentCharacterItemFence, scope_of};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::starter_grant::{grant_error, new_uuid};
use super::{DurabilityError, DurabilityRoot};
use crate::foundation::CommandRef;
use oteryn_gameplay_items::starter_grant::starting_loadout;
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DepartureOutcome {
    Committed,
    AlreadyDeparted,
    NotEligible,
}

impl DurabilityRoot {
    /// Read historical success before checking live reach: a committed teleport changed reach.
    pub async fn replay_dawnport_departure(
        &self,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        command: CommandRef,
        unique: u16,
        digest: [u8; 32],
    ) -> Result<bool, DurabilityError> {
        let Some((vocation, _)) = starting_loadout::departure(unique) else {
            return Ok(false);
        };
        let recovery = recovery
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = super::db::begin_semantic_transaction(holder, deadline).await?;
            assert_current_equipment_in_transaction(&mut tx, &recovery, &node, &fence, Some(command), digest).await.map_err(grant_error)?;
            let matched: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_dawnport_departures WHERE character_id=encode($1,'hex')::uuid AND game_session_id=encode($2,'hex')::uuid AND command_id=$3::text::numeric AND vocation=$4)")
                .bind(fence.character_id.as_bytes().as_slice()).bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string()).bind(vocation).fetch_one(&mut *tx).await?;
            super::db::commit_semantic_transaction(tx, deadline).await?;
            Ok(matched)
        })).await
    }

    /// Current authority is proved even for replay; the receipt alone never grants control.
    #[allow(
        clippy::too_many_arguments,
        reason = "the departure binds its current item fence, command, content and saved position"
    )]
    pub async fn depart_dawnport(
        &self,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        command: CommandRef,
        unique: u16,
        digest: [u8; 32],
        placed: FinalLastPosition,
    ) -> Result<DepartureOutcome, DurabilityError> {
        let Some((vocation, (x, y, floor))) = starting_loadout::departure(unique) else {
            return Ok(DepartureOutcome::NotEligible);
        };
        let (world, _) = scope_of(&fence).map_err(|_| DurabilityError::Unavailable)?;
        if placed.record.world_id != world
            || placed.record.cell.x != x
            || placed.record.cell.y != y
            || placed.record.cell.floor != floor
        {
            return Ok(DepartureOutcome::NotEligible);
        }
        let recovery = recovery
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = super::db::begin_semantic_transaction(holder, deadline).await?;
            assert_current_equipment_in_transaction(&mut tx, &recovery, &node, &fence, Some(command), digest)
                .await.map_err(grant_error)?;
            let character = fence.character_id;
            let prior: Option<(Vec<u8>, String, String)> = sqlx::query_as(
                "SELECT uuid_send(game_session_id),command_id::text,vocation FROM game_dawnport_departures WHERE character_id=encode($1,'hex')::uuid")
                .bind(character.as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
            if let Some((session, id, prior_vocation)) = prior {
                // oteryn-choice: the packet requires one departure per Character, across commands and reconnects.
                let outcome = if prior_vocation != vocation { DepartureOutcome::NotEligible } else if session == command.game_session_id().as_bytes().as_slice()
                    && id == command.command_id().get().to_string() {
                    DepartureOutcome::Committed
                } else { DepartureOutcome::AlreadyDeparted };
                super::db::commit_semantic_transaction(tx, deadline).await?;
                return Ok(outcome);
            }
            // The first-choice receipt is Oteryn's durable equivalent of DoorVocation storage.
            // A trial build alone is deliberately insufficient (vocation_door.lua:28-29).
            let eligible: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM game_character_build_receipts WHERE character_id=encode($1,'hex')::uuid AND cause='vocation_choice' AND vocation_after=$2) \
                 AND EXISTS(SELECT 1 FROM game_character_build_state WHERE character_id=encode($1,'hex')::uuid AND vocation=$2)")
                .bind(character.as_bytes().as_slice()).bind(vocation).fetch_one(&mut *tx).await?;
            if !eligible {
                super::db::commit_semantic_transaction(tx, deadline).await?;
                return Ok(DepartureOutcome::NotEligible);
            }
            // The owner graph is bounded. Read in item-id order so all item locks have one order.
            let removed: Vec<Value> = sqlx::query_scalar(
                "SELECT jsonb_build_object('before',to_jsonb(i),'kind',CASE WHEN e.item_instance_id IS NOT NULL THEN 'equipment' WHEN b.item_instance_id IS NOT NULL THEN 'entry' ELSE 'nested' END, \
                    'location',COALESCE(to_jsonb(e),to_jsonb(b),to_jsonb(n))) \
                 FROM game_dawnport_carried_items(encode($1,'hex')::uuid) c JOIN game_item_instances i ON i.item_instance_id=c.item \
                 LEFT JOIN game_character_equipment_slots e ON e.item_instance_id=i.item_instance_id AND e.character_id=encode($1,'hex')::uuid \
                 LEFT JOIN game_item_container_entries b ON b.item_instance_id=i.item_instance_id AND b.character_id=encode($1,'hex')::uuid \
                 LEFT JOIN game_item_nested_entries n ON n.item_instance_id=i.item_instance_id \
                 WHERE i.lifecycle=1 AND i.world_id=encode($2,'hex')::uuid AND c.cleanup \
                 ORDER BY i.item_instance_id LIMIT 2049 FOR UPDATE OF i")
                .bind(character.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).fetch_all(&mut *tx).await?;
            if removed.len() > 2048 { return Err(DurabilityError::InvalidStoredState); }
            let equipment_revision: Option<String> = if removed.iter().any(|item| item["kind"] == "equipment") {
                Some(sqlx::query_scalar("SELECT revision::text FROM game_character_equipment_state WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
                    .bind(character.as_bytes().as_slice()).fetch_one(&mut *tx).await?)
            } else { None };
            let transaction = new_uuid(&mut tx).await?;
            let event = new_uuid(&mut tx).await?;
            sqlx::query("INSERT INTO game_dawnport_departures(character_id,transaction_id,event_id,world_id,game_session_id,command_id,vocation,destination,character_lease_generation,write_sequence,equipment_revision_before,removed,content_digest) \
                VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,encode($5,'hex')::uuid,$6::text::numeric,$7,$8,$9::text::numeric,$10::text::numeric,$11::text::numeric,$12,$13)")
                .bind(character.as_bytes().as_slice()).bind(transaction.as_slice()).bind(event.as_slice()).bind(world.as_bytes().as_slice())
                .bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string()).bind(vocation)
                .bind(vec![x, y, i32::from(floor)]).bind(fence.character_lease_generation.to_string()).bind(placed.write_sequence.to_string())
                .bind(&equipment_revision).bind(Value::Array(removed.clone())).bind(digest.as_slice()).execute(&mut *tx).await?;
            for item in &removed {
                let id = item["before"]["item_instance_id"].as_str().ok_or(DurabilityError::InvalidStoredState)?;
                let query = match item["kind"].as_str() {
                    Some("equipment") => "DELETE FROM game_character_equipment_slots WHERE item_instance_id=$1::text::uuid",
                    Some("entry") => "DELETE FROM game_item_container_entries WHERE item_instance_id=$1::text::uuid",
                    Some("nested") => "DELETE FROM game_item_nested_entries WHERE item_instance_id=$1::text::uuid",
                    _ => return Err(DurabilityError::InvalidStoredState),
                };
                sqlx::query(query).bind(id).execute(&mut *tx).await?;
                sqlx::query("UPDATE game_item_instances SET quantity=0,lifecycle=2,state_revision=state_revision+1,last_transaction_id=encode($2,'hex')::uuid WHERE item_instance_id=$1::text::uuid")
                    .bind(id).bind(transaction.as_slice()).execute(&mut *tx).await?;
            }
            if equipment_revision.is_some() {
                sqlx::query("UPDATE game_character_equipment_state SET revision=revision+1,last_transaction_id=encode($2,'hex')::uuid WHERE character_id=encode($1,'hex')::uuid")
                    .bind(character.as_bytes().as_slice()).bind(transaction.as_slice()).execute(&mut *tx).await?;
            }
            let key = LastPositionKey { character_lease_generation: fence.character_lease_generation, write_sequence: placed.write_sequence };
            if upsert_last_position(&mut tx, character.as_bytes(), key, &placed.record).await? != LastPositionWrite::Written {
                return Err(DurabilityError::Unavailable);
            }
            sqlx::query("SET CONSTRAINTS ALL IMMEDIATE").execute(&mut *tx).await?;
            super::db::commit_semantic_transaction(tx, deadline).await?;
            Ok(DepartureOutcome::Committed)
        })).await
    }
}
