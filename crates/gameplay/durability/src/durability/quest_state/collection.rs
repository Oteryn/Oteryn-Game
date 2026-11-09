//! COLLECT reads the fenced custody owner after a committed native Item move or reward MINT.
use super::*;
use oteryn_gameplay_quests::quest::authored_bindings::collection::Collection;

pub(super) async fn admit(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterGameplayFence,
    request: &QuestTransitionRequest,
    collection: &Collection,
) -> StoredResult<bool> {
    let QuestCause::Command(command) = request.cause else {
        return Ok(false);
    };
    if command.game_session_id() != fence.game_session_id {
        return Ok(false);
    }
    let character = fence.character_id.as_bytes().as_slice();
    let session = command.game_session_id();
    let changed: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM game_item_loop_move_receipts r WHERE r.character_id=encode($1,'hex')::uuid AND r.game_session_id=encode($2,'hex')::uuid AND r.command_id=$3::text::numeric AND r.operation<>4 AND r.connection_generation=$4::text::numeric AND r.lease_generation=$5::text::numeric UNION ALL SELECT 1 FROM game_item_transfer_receipts r WHERE r.character_id=encode($1,'hex')::uuid AND r.game_session_id=encode($2,'hex')::uuid AND r.command_id=$3::text::numeric UNION ALL SELECT 1 FROM game_reward_claim_mint_receipts r WHERE r.character_id=encode($1,'hex')::uuid AND r.game_session_id=encode($2,'hex')::uuid AND r.command_id=$3::text::numeric)")
        .bind(character).bind(session.as_bytes().as_slice()).bind(command.command_id().get().to_string())
        .bind(fence.connection_generation.get().to_string()).bind(fence.character_lease_generation.to_string())
        .fetch_one(&mut **tx).await?;
    if !changed {
        return Ok(false);
    }
    let keys: Vec<_> = collection
        .items
        .iter()
        .map(|item| item.item.as_str())
        .collect();
    let world = fence.runtime_scope.world_id();
    let rows = sqlx::query("SELECT i.definition_production_key,i.quantity,coalesce(k.key_number,0)::bigint AS key_number FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid) c JOIN game_item_instances i ON i.item_instance_id=c.item LEFT JOIN game_item_keys k ON k.item_instance_id=i.item_instance_id WHERE i.lifecycle=1 AND i.definition_family='Item' AND i.definition_production_key=ANY($3::text[]) ORDER BY i.item_instance_id LIMIT 501 FOR UPDATE OF i")
        .bind(character).bind(world.as_bytes().as_slice()).bind(keys).fetch_all(&mut **tx).await?;
    if rows.len() > 500 {
        return Ok(false);
    }
    for required in &collection.items {
        let mut needed = i64::from(required.count);
        for row in &rows {
            if row.try_get::<&str, _>("definition_production_key")? == required.item
                && required.key_number.is_none_or(|number| {
                    row.try_get::<i64, _>("key_number").ok() == Some(i64::from(number))
                })
            {
                let quantity: i64 = row.try_get("quantity")?;
                needed -= needed.min(quantity);
            }
        }
        if needed != 0 {
            return Ok(false);
        }
    }
    Ok(true)
}
