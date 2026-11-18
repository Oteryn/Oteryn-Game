//! Crystal 201189f8 movements_herald_red_floors.lua:42–56.
use super::*;
use crate::durability::quest_state::{QuestCause, QuestTransitionRequest};

const MAX_SWEEP_FIELDS: usize = 1024;

pub(super) async fn consume(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    permit: &SpellLanePermit,
    fence: CurrentCharacterGameplayFence,
    request: &QuestTransitionRequest,
    callback: &Callback,
    context: &QuestWorldToolContext,
) -> Result<bool, DurabilityError> {
    let (QuestCause::Step(command), Some(sweep)) = (request.cause, &callback.effects.sweep_fields)
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
    let keys = sweep
        .appearances
        .iter()
        .map(|id| format!("oteryn:item.tibia.i{id}"))
        .collect::<Vec<_>>();
    let rows = sqlx::query(
        "SELECT i.item_instance_id::text AS item,to_jsonb(i) AS before_item,to_jsonb(g) AS before_ground,\
         g.spatial_position,i.definition_production_key FROM game_item_ground_locations g \
         JOIN game_item_instances i USING(item_instance_id,world_id) \
         WHERE g.world_id=encode($1,'hex')::uuid AND g.channel_id=encode($2,'hex')::uuid \
          AND g.runtime_scope_ownership_generation=$3::text::numeric \
          AND g.map_revision=$4 AND g.content_revision=$5 AND g.native_room_placement_context=$6 \
          AND octet_length(g.spatial_position)=10 AND substring(g.spatial_position from 9 for 2)=$7 \
          AND substring(g.spatial_position from 1 for 4) BETWEEN $8 AND $9 \
          AND substring(g.spatial_position from 5 for 4) BETWEEN $10 AND $11 \
          AND i.lifecycle=1 AND i.quantity=1 AND i.definition_family='Item' \
          AND i.definition_production_key=ANY($12::text[]) \
          AND game_quest_dynamic_field_placement(i,g) \
         ORDER BY g.spatial_position,i.definition_production_key,g.stack_ordinal,i.item_instance_id \
         LIMIT $13 FOR UPDATE OF i,g",
    )
    .bind(world_id.as_bytes().as_slice())
    .bind(channel_id.as_bytes().as_slice())
    .bind(fence.scope_ownership_generation.get().to_string())
    .bind(&context.map_revision)
    .bind(&context.content_revision)
    .bind(&context.placement_context)
    .bind((-i16::from(sweep.minimum.floor)).to_be_bytes().as_slice())
    .bind(i32::from(sweep.minimum.x).to_be_bytes().as_slice())
    .bind(i32::from(sweep.maximum.x).to_be_bytes().as_slice())
    .bind(i32::from(sweep.minimum.y).to_be_bytes().as_slice())
    .bind(i32::from(sweep.maximum.y).to_be_bytes().as_slice())
    .bind(keys)
    .bind((MAX_SWEEP_FIELDS + 1) as i64)
    .fetch_all(&mut **tx)
    .await?;
    if rows.len() > MAX_SWEEP_FIELDS {
        return Ok(false);
    }
    let proof = serde_json::to_vec(&serde_json::json!({
        "schema": "OTERYN_QUEST_FIELD_SWEEP/v1",
        "callback":callback,
        "context":context,
    }))
    .map_err(|_| DurabilityError::InvalidStoredState)?;
    let mut observed = std::collections::BTreeSet::new();
    for row in rows {
        // Tile:getItemById removes one current field for each appearance.
        if !observed.insert((
            row.try_get::<Vec<u8>, _>("spatial_position")?,
            row.try_get::<String, _>("definition_production_key")?,
        )) {
            continue;
        }
        let input = Input {
            item: row.try_get("item")?,
            before: row.try_get("before_item")?,
            location: row.try_get("before_ground")?,
            kind: "ground".into(),
            quantity: 1,
            count: 1,
            replacement: None,
        };
        let transaction: String = sqlx::query_scalar(
            "INSERT INTO game_quest_callback_consumptions(transaction_id,item_instance_id,character_id,cause_id,\
              cause_ordinal,transition_key,before_item,before_ground,quantity,proof,proof_sha256,location_kind) \
             VALUES(game_character_uuid_v7(),$1::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,\
               $4::text::numeric,$5,$6,$7,1,$8,sha256($8),'ground') RETURNING transaction_id::text",
        ).bind(&input.item)
            .bind(fence.character_id.as_bytes().as_slice())
            .bind(command.game_session_id().as_bytes().as_slice())
            .bind(command.command_id().get().to_string())
            .bind(&request.transition_key)
            .bind(&input.before)
            .bind(&input.location)
            .bind(&proof)
            .fetch_one(&mut **tx)
            .await?;
        apply_input(tx, permit, fence, &input, &transaction).await?;
    }
    Ok(true)
}
