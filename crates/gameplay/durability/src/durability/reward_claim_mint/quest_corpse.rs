//! Read-only corpse sampling joins immutable native race provenance to the tool MINT.
use super::QuestCarriedUse;
use super::quest_creature::QuestCreatureUseProof;
use crate::durability::{
    DurabilityError,
    character_progression::CurrentCharacterGameplayFence,
    quest_state::{QuestCause, QuestTransitionRequest},
};
use oteryn_gameplay_content::content::native_gameplay::NativeGameplayState;
use oteryn_gameplay_quests::quest::reward_callbacks::RewardCallback;
use sqlx::Row;

pub(super) async fn record(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterGameplayFence,
    request: &QuestTransitionRequest,
    callback: &RewardCallback,
    carried: &QuestCarriedUse,
    proof: &QuestCreatureUseProof<'_>,
    native: &NativeGameplayState,
) -> Result<bool, DurabilityError> {
    let Some(spec) = &callback.corpse_target else {
        return Ok(true);
    };
    let Some((corpse, position)) = proof.corpse() else {
        return Ok(false);
    };
    let Some(species) = &callback.creature_target else {
        return Ok(false);
    };
    let QuestCause::Command(command) = request.cause else {
        return Ok(false);
    };
    let crate::foundation::RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = fence.runtime_scope
    else {
        return Ok(false);
    };
    let key = format!("oteryn:item.tibia.i{}", spec.appearance);
    let Some(revision) = native.item_index().and_then(|index| index.revision(&key)) else {
        return Ok(false);
    };
    let row=sqlx::query("SELECT to_jsonb(i) AS item,to_jsonb(g) AS ground,g.spatial_position FROM game_item_instances i JOIN game_item_ground_locations g USING(item_instance_id,world_id) JOIN game_corpse_source_races p USING(item_instance_id) JOIN game_item_mint_receipts r ON r.transaction_id=p.mint_transaction_id AND r.item_instance_id=i.item_instance_id WHERE i.item_instance_id=encode($1,'hex')::uuid AND i.world_id=encode($2,'hex')::uuid AND g.channel_id=encode($3,'hex')::uuid AND g.runtime_scope_ownership_generation=$4::text::numeric AND i.lifecycle=1 AND i.quantity=1 AND i.definition_family='Item' AND i.definition_production_key=$5 AND i.definition_revision_ref=$6 AND p.race_key=$7 AND i.minted_transaction_id=p.mint_transaction_id AND floor(extract(epoch FROM transaction_timestamp())*1000)::bigint-r.occurred_at BETWEEN 0 AND $8 FOR SHARE OF i,g")
        .bind(corpse.as_slice()).bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice())
        .bind(fence.scope_ownership_generation.get().to_string()).bind(&key).bind(revision).bind(species)
        .bind(i64::from(spec.maximum_age_ms)-1).fetch_optional(&mut **tx).await?;
    let Some(row) = row else { return Ok(false) };
    let cell = crate::durability::spell_items_abi::decode_ground_cell(
        &row.try_get::<Vec<u8>, _>("spatial_position")?,
    )
    .map_err(|_| DurabilityError::InvalidStoredState)?;
    if position[0].abs_diff(cell.x) > 1
        || position[1].abs_diff(cell.y) > 1
        || position[2] != -cell.z
    {
        return Ok(false);
    }
    let before: serde_json::Value = row.try_get("item")?;
    let ground: serde_json::Value = row.try_get("ground")?;
    Ok(sqlx::query("INSERT INTO game_quest_corpse_samples(character_id,cause_id,cause_ordinal,transition_key,world_id,channel_id,scope_generation,corpse_item,source_item,appearance,maximum_age_ms,actor_position,before_item,before_ground,ingress) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,$4,encode($5,'hex')::uuid,encode($6,'hex')::uuid,$7::text::numeric,encode($8,'hex')::uuid,encode($9,'hex')::uuid,$10,$11,$12,$13,$14,$15) ON CONFLICT DO NOTHING")
        .bind(fence.character_id.as_bytes().as_slice()).bind(command.game_session_id().as_bytes().as_slice())
        .bind(command.command_id().get().to_string()).bind(&callback.key).bind(world_id.as_bytes().as_slice())
        .bind(channel_id.as_bytes().as_slice()).bind(fence.scope_ownership_generation.get().to_string())
        .bind(corpse.as_slice()).bind(carried.source.as_slice()).bind(i64::from(spec.appearance))
        .bind(i32::try_from(spec.maximum_age_ms).map_err(|_|DurabilityError::InvalidStoredState)?)
        .bind(serde_json::json!(position)).bind(before).bind(ground).bind(&carried.intent)
        .execute(&mut **tx).await?.rows_affected()==1)
}
