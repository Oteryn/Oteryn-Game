//! A quest tool borrows a live corpse under the same transaction as its selected input.
use super::super::reward_claim_mint::QuestCarriedUse;
use super::*;
use oteryn_gameplay_quests::quest::reward_callbacks::RewardCallback;

pub(super) async fn validate(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterGameplayFence,
    callback: &RewardCallback,
    carried_use: Option<&QuestCarriedUse>,
) -> Result<bool, DurabilityError> {
    let observed = carried_use.and_then(|use_| use_.corpse.as_ref());
    let (Some(target), Some(observed)) = (&callback.corpse_target, observed) else {
        return Ok(callback.corpse_target.is_none() && observed.is_none());
    };
    let crate::foundation::RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = fence.runtime_scope
    else {
        return Ok(false);
    };
    // Lock the exact corpse and its custody before evaluating expiry.
    // Decay, pickup, skinning and a second writer cannot change either row until
    // the enclosing quest/input receipt commits or rolls back.
    let row = sqlx::query(
        "WITH target AS MATERIALIZED ( \
         SELECT g.spatial_position,i.item_instance_id FROM game_item_instances i \
         JOIN game_item_ground_locations g USING(item_instance_id,world_id) \
         JOIN game_corpse_source_races s USING(item_instance_id) \
         WHERE i.item_instance_id=encode($1,'hex')::uuid AND i.world_id=encode($2,'hex')::uuid \
         AND g.channel_id=encode($3,'hex')::uuid \
         AND g.runtime_scope_ownership_generation=$4::text::numeric \
         AND g.map_revision=$5 AND g.content_revision=$6 AND g.native_room_placement_context=$7 \
         AND i.lifecycle=1 AND i.quantity=1 AND i.definition_family='Item' \
         AND i.definition_production_key=$8 AND i.definition_revision_ref=$9 \
         AND s.mint_transaction_id=i.minted_transaction_id \
         AND s.race_key=$10 AND s.race_definition_revision=$11 \
         FOR UPDATE OF i,g) \
         SELECT spatial_position FROM target \
         WHERE game_corpse_retirement_deadline(item_instance_id)>floor(extract(epoch FROM clock_timestamp())*1000)::bigint",
    )
    .bind(observed.item.as_slice())
    .bind(world_id.as_bytes().as_slice())
    .bind(channel_id.as_bytes().as_slice())
    .bind(fence.scope_ownership_generation.get().to_string())
    .bind(&observed.map_revision)
    .bind(&observed.content_revision)
    .bind(&observed.placement_context)
    .bind(&target.item)
    .bind(&target.revision)
    .bind(&target.creature)
    .bind(&target.creature_revision)
    .fetch_optional(&mut **tx)
    .await?;
    let Some(row) = row else { return Ok(false) };
    let position: Vec<u8> = row.try_get("spatial_position")?;
    let cell = super::super::spell_items_abi::decode_ground_cell(&position)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    Ok(observed.actor_position[0].abs_diff(cell.x) <= 1
        && observed.actor_position[1].abs_diff(cell.y) <= 1
        && observed.actor_position[2] == -cell.z)
}
