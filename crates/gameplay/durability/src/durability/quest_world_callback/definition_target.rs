//! The selected Ground target stays locked with the tool debit and Quest receipt.
use super::*;

pub(super) async fn require(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterGameplayFence,
    callback: &Callback,
    context: &QuestWorldToolContext,
) -> Result<bool, DurabilityError> {
    let Some(target) = context
        .selected_tool
        .as_ref()
        .and_then(|tool| tool.target.as_ref())
    else {
        return Ok(true);
    };
    let Some(definition) = &callback.selector.target_definition else {
        return Ok(false);
    };
    let crate::foundation::RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = fence.runtime_scope
    else {
        return Ok(false);
    };
    let live: Option<bool> = sqlx::query_scalar(
        "SELECT true FROM game_item_instances \
         WHERE item_instance_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid \
         AND lifecycle=1 AND definition_family='Item' AND definition_production_key=$3 FOR SHARE",
    )
    .bind(target.item.as_slice())
    .bind(world_id.as_bytes().as_slice())
    .bind(definition)
    .fetch_optional(&mut **tx)
    .await?;
    if live.is_none() {
        return Ok(false);
    }
    // Item custody writers take the Item lock before the Ground owner lock.
    // Ground changes are serialized by game_item_ground_owner_lock.
    sqlx::query(
        "SELECT pg_advisory_xact_lock(hashtextextended(encode($1,'hex') || encode($2,'hex'),33))",
    )
    .bind(world_id.as_bytes().as_slice())
    .bind(channel_id.as_bytes().as_slice())
    .execute(&mut **tx)
    .await?;
    let position: Option<(Vec<u8>, bool)> = sqlx::query_as(
        "SELECT spatial_position,map_revision<>$5 FROM game_item_ground_locations \
         WHERE item_instance_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid \
         AND scope_kind=1 AND channel_id=encode($3,'hex')::uuid \
         AND runtime_scope_ownership_generation=$4::text::numeric \
         AND (map_revision=$5 OR map_revision='sha256:'||encode(sha256(convert_to($5,'UTF8')),'hex')) \
         AND content_revision=$6 AND native_room_placement_context=$7",
    )
    .bind(target.item.as_slice())
    .bind(world_id.as_bytes().as_slice())
    .bind(channel_id.as_bytes().as_slice())
    .bind(fence.scope_ownership_generation.get().to_string())
    .bind(&context.map_revision)
    .bind(&context.content_revision)
    .bind(&context.placement_context)
    .fetch_optional(&mut **tx)
    .await?;
    let Some((position, native_axes)) = position else {
        return Ok(false);
    };
    let cell = super::super::spell_items_abi::decode_ground_cell(&position)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let [x, y, floor] = target.actor_position;
    let target_floor = if native_axes { cell.z } else { -cell.z };
    Ok(target.target_position == [cell.x, cell.y, target_floor]
        && x.abs_diff(cell.x) <= u32::from(callback.use_range)
        && y.abs_diff(cell.y) <= u32::from(callback.use_range)
        && floor == target_floor)
}
