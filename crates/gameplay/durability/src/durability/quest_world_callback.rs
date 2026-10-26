//! QUESTCB2: altar inputs are consumed in the existing quest transition transaction.
use super::{DurabilityError, character_progression::CurrentCharacterGameplayFence};
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
use serde::{Deserialize, Serialize};
use sqlx::Row;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuestWorldContext {
    pub map_revision: String,
    pub content_revision: String,
    pub placement_context: Vec<u8>,
    pub reset_epoch: u64,
    pub actor_placement: [u8; 16],
    pub movement_revision: u64,
}

pub(super) async fn consume(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterGameplayFence,
    request: &super::quest_state::QuestTransitionRequest,
    callback: &Callback,
    context: &QuestWorldContext,
) -> Result<bool, DurabilityError> {
    let (super::quest_state::QuestCause::Use(command)
    | super::quest_state::QuestCause::Step(command)) = request.cause
    else {
        return Ok(false);
    };
    let crate::foundation::RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = fence.runtime_scope
    else {
        return Ok(false);
    };
    let epoch: String = sqlx::query_scalar("SELECT coalesce((SELECT reset_epoch FROM game_world_reset_active WHERE world_id=encode($1,'hex')::uuid),0)::text")
        .bind(world_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
    if epoch != context.reset_epoch.to_string() {
        return Ok(false);
    }
    let pinned: bool = sqlx::query_scalar("SELECT coalesce((SELECT ('sha256:'||encode(server_artifact_digest,'hex')=$3 AND frame_binding_digest=$4) FROM game_content_activations WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid ORDER BY activation_sequence DESC LIMIT 1),false)")
        .bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice()).bind(&context.content_revision).bind(&context.placement_context).fetch_one(&mut **tx).await?;
    if !pinned {
        return Ok(false);
    }
    if callback.offerings.is_empty() {
        return Ok(true);
    }
    // Stable tile order and then instance order. A single item cannot satisfy two altars.
    let mut offerings = callback.offerings.iter().collect::<Vec<_>>();
    offerings.sort_by_key(|o| o.position);
    let mut inputs = Vec::new();
    for offering in offerings {
        let Position { x, y, floor } = offering.position;
        let mut spatial = i32::from(x).to_be_bytes().to_vec();
        spatial.extend_from_slice(&i32::from(y).to_be_bytes());
        spatial.extend_from_slice(&i16::from(floor).to_be_bytes());
        let row = sqlx::query("SELECT i.item_instance_id::text AS item,to_jsonb(i) AS before_item,to_jsonb(g) AS before_ground,i.quantity FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id) WHERE g.world_id=encode($1,'hex')::uuid AND g.channel_id=encode($2,'hex')::uuid AND g.spatial_position=$3 AND g.map_revision=$4 AND g.content_revision=$5 AND g.native_room_placement_context=$6 AND g.runtime_scope_ownership_generation=$7::text::numeric AND i.lifecycle=1 AND i.definition_family='Item' AND i.definition_production_key=$8 AND i.quantity >= $9 AND game_item_loop_drop_ref(g.corpse_ref) ORDER BY i.item_instance_id LIMIT 1 FOR UPDATE OF i,g")
            .bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice()).bind(spatial)
            .bind(&context.map_revision).bind(&context.content_revision).bind(&context.placement_context)
            .bind(fence.scope_ownership_generation.get().to_string()).bind(&offering.item).bind(i64::from(offering.count))
            .fetch_optional(&mut **tx).await?;
        let Some(row) = row else {
            return Ok(false);
        };
        let item: String = row.try_get("item")?;
        if inputs.iter().any(|(id, _, _, _)| id == &item) {
            return Ok(false);
        }
        inputs.push((
            item,
            row.try_get::<serde_json::Value, _>("before_item")?,
            row.try_get::<serde_json::Value, _>("before_ground")?,
            offering.count,
        ));
    }
    let transaction: String = sqlx::query_scalar("SELECT game_character_uuid_v7()::text")
        .fetch_one(&mut **tx)
        .await?;
    for (item, before, ground, count) in inputs {
        let proof = serde_json::to_vec(&serde_json::json!({"callback": callback, "context": context,
            "character": fence.character_id.as_bytes(), "session": command.game_session_id().as_bytes(),
            "command": command.command_id().get(), "connection": fence.connection_generation.get(),
            "lease": fence.character_lease_generation, "scope": fence.scope_ownership_generation.get(),
            "before": before, "ground": ground, "count": count})).map_err(|_| DurabilityError::InvalidStoredState)?;
        sqlx::query("INSERT INTO game_quest_callback_consumptions(transaction_id,item_instance_id,character_id,cause_id,cause_ordinal,transition_key,before_item,before_ground,quantity,proof,proof_sha256) VALUES($1::uuid,$2::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5::text::numeric,$6,$7,$8,$9,$10,sha256($10))")
            .bind(&transaction).bind(&item).bind(fence.character_id.as_bytes().as_slice())
            .bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string())
            .bind(&request.transition_key).bind(before).bind(ground).bind(i32::from(count)).bind(proof).execute(&mut **tx).await?;
        let changed = sqlx::query("UPDATE game_item_instances SET quantity=quantity-$2,lifecycle=CASE WHEN quantity=$2 THEN 2 ELSE 1 END,last_transaction_id=$3::uuid WHERE item_instance_id=$1::uuid")
            .bind(&item).bind(i64::from(count)).bind(&transaction).execute(&mut **tx).await?;
        if changed.rows_affected() != 1 {
            return Err(DurabilityError::InvalidStoredState);
        }
        sqlx::query("DELETE FROM game_item_ground_locations g USING game_item_instances i WHERE g.item_instance_id=$1::uuid AND i.item_instance_id=g.item_instance_id AND i.quantity=0")
            .bind(&item).execute(&mut **tx).await?;
    }
    Ok(true)
}
