//! Crystal 201189f8 movements_all_teleports_tombs_coal_basin.lua:30-55.
use std::sync::Arc;

use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue,
    world_callbacks::coal_basins::{COIN, CoalBasin},
};

use super::*;
use crate::durability::{
    character_progression::CharacterProgressionError,
    npc_travel::{PendingArrival, TravelOccurrence},
};

// Crystal 201189f8 src/utils/utils_definitions.hpp:71, CONST_ME_HITBYFIRE.
pub const COIN_FIRE_EFFECT: u16 = 16;

#[derive(Debug, Clone)]
pub struct CoalBasinTarget {
    pub binding: CoalBasin,
    pub placement: Vec<u8>,
    pub reset_epoch: i64,
    pub recipient: Option<(CurrentCharacterItemFence, Arc<QuestStateCatalogue>)>,
}

#[derive(Debug, Clone, Copy)]
pub struct CoalBasinLanding {
    pub recipient: [u8; 16],
    pub arrival: PendingArrival,
}

impl DurabilityRoot {
    #[allow(clippy::too_many_arguments, reason = "the original move keeps its established fence")]
    pub async fn commit_item_loop_coal_basin<C: ItemLoopContent>(
        &self,
        permit: &SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        command: CommandRef,
        content: &C,
        request: LoopMoveRequest,
        target: CoalBasinTarget,
    ) -> Result<Result<LoopMoveReceipt, LoopMoveRefusal>, DurabilityError> {
        permit.check_scope(fence.runtime_scope)?;
        let record = recovery.record_for(self).map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        let mut context = (permit, content);
        self.try_issue_semantic_pass()?.run_with_context(&mut context, move |holder, deadline, context| Box::pin(async move {
            let (permit, content) = *context;
            let mut tx = crate::durability::db::begin_type2_transaction(holder, deadline).await?;
            let attempt: LoopResult<LoopMoveReceipt> = async {
                assert_recovery_fence(&mut tx, &record).await?;
                let intent = intent_bytes(fence.character_id, &request)?;
                lock_loop_command(&mut tx, command).await?;
                if let Some(replay) = committed_loop_move(&mut tx, command, fence.character_id, &intent).await? {
                    return replay.map_err(LoopError::Refused);
                }
                let LoopDestination::Ground(ground) = &request.destination else {
                    return refuse(LoopMoveRefusal::Rejected);
                };
                let (world, _) = scope_of(&fence).map_err(|_| LoopError::Refused(LoopMoveRefusal::Rejected))?;
                if target.recipient.as_ref().is_some_and(|(guest, _)| {
                    guest.runtime_scope != fence.runtime_scope
                        || guest.scope_ownership_generation != fence.scope_ownership_generation
                }) {
                    return refuse(LoopMoveRefusal::Stale);
                }
                let mut moved = apply_loop_move(
                    &mut tx, permit, &record, &node, &fence, command, content, &request,
                    &intent, None, None, None, None,
                ).await?;
                let row = sqlx::query(
                    "SELECT l.definition_production_key,i.item_instance_id::text,
                            to_jsonb(i) AS before_item,to_jsonb(g) AS before_ground
                       FROM game_item_loop_move_lines l JOIN game_item_instances i USING(item_instance_id)
                       JOIN game_item_ground_locations g USING(item_instance_id)
                      WHERE l.transaction_id=encode($1,'hex')::uuid AND l.to_kind=3
                        AND g.spatial_position=$2 FOR UPDATE OF i",
                ).bind(moved.transaction_id.as_slice()).bind(&ground.spatial_position)
                 .fetch_one(&mut **tx).await?;
                if row.try_get::<String, _>("definition_production_key")? != COIN {
                    moved.floor_effect = Some(3); // Crystal CONST_ME_POFF.
                    return Ok(moved);
                }
                let item: String = row.try_get("item_instance_id")?;
                let before_item: serde_json::Value = row.try_get("before_item")?;
                let before_ground: serde_json::Value = row.try_get("before_ground")?;
                let current: bool = sqlx::query_scalar(
                    "SELECT game_world_reset_map_epoch_current(encode($1,'hex')::uuid,$2::text::numeric)",
                ).bind(world.as_bytes().as_slice()).bind(target.reset_epoch.to_string()).fetch_one(&mut **tx).await?;
                if !current {
                    return refuse(LoopMoveRefusal::Stale);
                }

                // The original move has already proved its custody. The exact
                // resulting line, including a new split stack, now retires.
                sqlx::query("SET CONSTRAINTS ALL DEFERRED").execute(&mut **tx).await?;
                let recipient = target.recipient.as_ref();
                let quest_changed = match recipient {
                    Some((guest, catalogue)) => super::super::quest_state::coal_basin::apply(
                        &mut tx, &node, *guest, catalogue, &target.binding, moved.transaction_id,
                    ).await.map_err(|error| match error {
                        CharacterProgressionError::Unavailable(error) => LoopError::Durability(error),
                        _ => LoopError::Refused(LoopMoveRefusal::Stale),
                    })?,
                    None => false,
                };
                let (transaction, event, occurred): (String, String, i64) = sqlx::query_as(
                    "SELECT game_character_uuid_v7()::text,game_character_uuid_v7()::text,
                            floor(extract(epoch FROM clock_timestamp())*1000)::bigint",
                ).fetch_one(&mut **tx).await?;
                let tuple = tx.tuple();
                let destination = recipient.map(|_| target.binding.destination);
                let recipient_id = recipient.map(|(guest, _)| *guest.character_id.as_bytes());
                let envelope = crate::durability::item_craft::audit::encode_map_harvest(
                    fence, command,
                    json!({"source_unique":target.binding.source_uid, "placement":target.placement,
                           "before":before_item, "ground":before_ground, "outputs":[],
                           "move_transaction":moved.transaction_id, "recipient":recipient_id,
                           "destination":destination, "quest_changed":quest_changed}),
                    &transaction, &event, occurred, tuple.tuple(),
                )?;
                sqlx::query(
                    "INSERT INTO game_coal_basin_arrivals(
                        transaction_id,event_id,move_transaction_id,item_instance_id,source_uid,
                        before_item,before_ground,placement,reset_epoch,recipient,destination_x,
                        destination_y,destination_floor,quest_changed,occurred_at,envelope_sha256)
                     VALUES($1::uuid,$2::uuid,encode($3,'hex')::uuid,$4::uuid,$5,$6,$7,$8,$9,
                            encode($10,'hex')::uuid,$11,$12,$13,$14,$15,sha256($16))",
                ).bind(&transaction).bind(&event).bind(moved.transaction_id.as_slice()).bind(&item)
                 .bind(i32::from(target.binding.source_uid)).bind(&before_item).bind(&before_ground)
                 .bind(&target.placement).bind(target.reset_epoch).bind(recipient_id.map(|id| id.to_vec()))
                 .bind(destination.map(|pos| i32::from(pos.x))).bind(destination.map(|pos| i32::from(pos.y)))
                 .bind(destination.map(|pos| -i16::from(pos.floor))).bind(quest_changed).bind(occurred)
                 .bind(&envelope).execute(&mut **tx).await?;
                sqlx::query("UPDATE game_item_instances SET quantity=0,lifecycle=2,last_transaction_id=$2::uuid WHERE item_instance_id=$1::uuid")
                    .bind(&item).bind(&transaction).execute(&mut **tx).await?;
                sqlx::query("DELETE FROM game_item_ground_locations WHERE item_instance_id=$1::uuid")
                    .bind(&item).execute(&mut **tx).await?;
                sqlx::query(
                    "INSERT INTO game_item_audit_outbox(event_id,transaction_id,transaction_ordinal,
                        transaction_count,event_type_id,schema_revision,retention_profile_id,item_instance_id,
                        occurred_at,expires_at,envelope,envelope_sha256,publication_state)
                     VALUES($1::uuid,$2::uuid,1,1,2,$3,$4,$5::uuid,$6,$6+$7,$8,sha256($8),1)",
                ).bind(&event).bind(&transaction).bind(i64::from(tuple.schema_revision()))
                 .bind(tuple.retention_profile_id()).bind(&item).bind(occurred)
                 .bind(crate::durability::item_mint_audit::AUDIT_RETENTION_P90D_MS).bind(&envelope)
                 .execute(&mut **tx).await?;
                sqlx::query("SET CONSTRAINTS ALL IMMEDIATE").execute(&mut **tx).await?;
                moved.floor_effect = Some(COIN_FIRE_EFFECT);
                Ok(moved)
            }.await;
            match attempt {
                Ok(receipt) => { tx.commit(deadline).await?; Ok(Ok(receipt)) }
                Err(error) => loop_error(error).map(Err),
            }
        })).await
    }

    /// A lost drop acknowledgement can realize only the outstanding landing
    /// named by that depositor's unchanged move receipt.
    pub async fn read_coal_basin_landing(
        &self,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        depositor: CharacterId,
        command: CommandRef,
    ) -> Result<Option<CoalBasinLanding>, DurabilityError> {
        let record = recovery.record_for(self).map_err(|_| DurabilityError::Unavailable)?;
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = crate::durability::db::begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &record).await?;
            let row = sqlx::query(
                "SELECT p.character_id::text,p.occurrence_id::text,p.world_id::text,p.tile_x,p.tile_y,p.tile_floor
                   FROM game_item_loop_move_receipts m JOIN game_coal_basin_arrivals c
                     ON c.move_transaction_id=m.transaction_id
                   JOIN game_character_pending_arrivals p ON p.character_id=c.recipient
                     AND p.occurrence_id=m.transaction_id AND p.source_coal_basin
                  WHERE m.game_session_id=encode($1,'hex')::uuid AND m.command_id=$2::text::numeric
                    AND m.character_id=encode($3,'hex')::uuid",
            ).bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string())
             .bind(depositor.as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
            let landing = row.map(|row| -> Result<CoalBasinLanding, DurabilityError> {
                let occurrence = TravelOccurrence::from_bytes(uuid_text(row.try_get("occurrence_id")?)?)
                    .map_err(|_| DurabilityError::InvalidStoredState)?;
                let world_id = crate::foundation::WorldId::decode(&uuid_text(row.try_get("world_id")?)?)
                    .map_err(|_| DurabilityError::InvalidStoredState)?;
                Ok(CoalBasinLanding {
                    recipient: uuid_text(row.try_get("character_id")?)?,
                    arrival: PendingArrival {
                        occurrence, world_id,
                        destination: crate::durability::character_last_position::LastPositionCell {
                            x: row.try_get("tile_x")?, y: row.try_get("tile_y")?, floor: row.try_get("tile_floor")?,
                        },
                    },
                })
            }).transpose()?;
            crate::durability::db::commit_semantic_transaction(tx, deadline).await?;
            Ok(landing)
        })).await
    }
}
