//! The selected Ground target stays locked with the tool debit and Quest receipt.
use super::*;

pub(super) async fn require(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterGameplayFence,
    callback: &Callback,
    context: &QuestWorldToolContext,
) -> Result<bool, DurabilityError> {
    let Some(target) = context.selected_tool.as_ref().and_then(|tool| tool.target.as_ref()) else {
        return Ok(true);
    };
    let Some(definition) = &callback.selector.target_definition else { return Ok(false); };
    let crate::foundation::RuntimeScopeRefV1::Channel { world_id, channel_id } = fence.runtime_scope else { return Ok(false); };
    let position: Option<Vec<u8>> = sqlx::query_scalar(
        "SELECT g.spatial_position FROM game_item_instances i JOIN game_item_ground_locations g USING(item_instance_id,world_id) \
         WHERE i.item_instance_id=encode($1,'hex')::uuid AND i.world_id=encode($2,'hex')::uuid \
         AND g.channel_id=encode($3,'hex')::uuid AND g.runtime_scope_ownership_generation=$4::text::numeric \
         AND i.lifecycle=1 AND i.definition_family='Item' AND i.definition_production_key=$5 \
         FOR SHARE OF i,g"
    ).bind(target.item.as_slice()).bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice())
        .bind(fence.scope_ownership_generation.get().to_string()).bind(definition).fetch_optional(&mut **tx).await?;
    let Some(position) = position else { return Ok(false); };
    let cell = super::super::spell_items_abi::decode_ground_cell(&position).map_err(|_| DurabilityError::InvalidStoredState)?;
    let [x,y,floor] = target.actor_position;
    Ok(x.abs_diff(cell.x) <= u32::from(callback.use_range) && y.abs_diff(cell.y) <= u32::from(callback.use_range) && floor == -cell.z)
}
