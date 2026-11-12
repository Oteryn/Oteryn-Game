//! Manual combat.md:257: health/mana potions retain their source and produce no flask.
//! A semantic use receipt provides replay without inventing a durable Item mutation.
use super::*;
pub(super) async fn replay(
    connection: &mut sqlx::PgConnection,
    fence: &CurrentCharacterItemFence,
    request: &ItemUseRequest,
) -> Result<Option<ItemUseOutcome>> {
    let row=sqlx::query("SELECT binding,shape FROM game_tibiadrome_supply_receipts WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric AND character_id=encode($3,'hex')::uuid")
        .bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command_id.to_string()).bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut *connection).await?;
    if let Some(row) = row {
        if row.try_get::<Vec<u8>, _>("binding")? != request.binding() {
            return Err(ItemUseError::ConflictingCommand);
        }
        return Ok(Some(ItemUseOutcome::AlreadyCommitted {
            shape: shape_from_code(row.try_get("shape")?)?,
        }));
    }
    Ok(None)
}
pub(super) async fn preserve(
    connection: &mut sqlx::PgConnection,
    fence: &CurrentCharacterItemFence,
    request: &ItemUseRequest,
) -> Result<Option<ItemUseOutcome>> {
    if request.kind != ItemUseKind::Potion {
        return Ok(None);
    }
    let active:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_tibiadrome WHERE character_id=encode($1,'hex')::uuid AND body->>'active'='true' AND body->'session'=$2 AND (body->>'generation')::numeric=$3::text::numeric)")
        .bind(fence.character_id.as_bytes().as_slice()).bind(serde_json::json!(fence.game_session_id.as_bytes())).bind(fence.scope_ownership_generation.get().to_string()).fetch_one(&mut *connection).await?;
    if !active {
        return Ok(None);
    }
    lock_stack(
        connection,
        fence.character_id.as_bytes(),
        fence.runtime_scope.world_id().as_bytes(),
        &request.source_item_instance_id,
        &request.source_definition,
        request.source_quantity_before,
    )
    .await?;
    sqlx::query("INSERT INTO game_tibiadrome_supply_receipts(character_id,game_session_id,command_id,binding,shape) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,$4,$5)")
        .bind(fence.character_id.as_bytes().as_slice()).bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command_id.to_string()).bind(request.binding().as_slice()).bind(shape_code(request.shape())).execute(&mut *connection).await?;
    Ok(Some(ItemUseOutcome::Written {
        shape: request.shape(),
    }))
}
