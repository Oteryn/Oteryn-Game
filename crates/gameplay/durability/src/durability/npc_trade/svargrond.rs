//! One ring sold for zero gold inside Buddel's durable departure transaction.
use super::*;
use crate::durability::npc_travel::svargrond::{BUDDEL, RING};

pub(in crate::durability) async fn exchange_ring(
    tx: &mut Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    request: &NpcTradeRequest,
    arrival_binding: &[u8],
) -> Result<bool> {
    if request.cause.npc_key != BUDDEL
        || request.cause.side != NpcTradeSide::Sell
        || request.item.definition.production_key != RING
        || validate(request, false)? != Some((0, 1))
    {
        return Err(NpcTradeError::InvalidInput);
    }
    let (world, channel) = scope_of(&fence).map_err(|_| NpcTradeError::AuthorityRejected)?;
    let backpack: Option<Vec<u8>> = sqlx::query_scalar(
        "SELECT uuid_send(i.item_instance_id) FROM game_item_container_slots s
         JOIN game_item_instances i ON i.item_instance_id=s.item_instance_id
         WHERE s.character_id=encode($1,'hex')::uuid AND s.world_id=encode($2,'hex')::uuid
         AND i.definition_family=$3 AND i.definition_production_key=$4
         AND i.definition_revision_ref=$5 AND i.lifecycle=1 FOR UPDATE OF s",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(world.as_bytes().as_slice())
    .bind(&request.backpack.definition.family)
    .bind(&request.backpack.definition.production_key)
    .bind(&request.backpack.definition.revision_ref)
    .fetch_optional(&mut ***tx)
    .await?;
    let Some(backpack) = backpack else {
        return Ok(false);
    };
    let backpack = backpack
        .try_into()
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let capacity = request
        .backpack
        .container_capacity
        .and_then(|capacity| usize::try_from(capacity).ok())
        .ok_or(NpcTradeError::InvalidInput)?;
    let entries = lock_entries(tx, &backpack, world, 0, RING).await?;
    let plan = match plan_sell(request, 0, 1, capacity, &entries) {
        Ok(plan) => plan,
        Err(NpcTradeError::ItemNotFound) => return Ok(false),
        Err(error) => return Err(error),
    };
    let binding = Sha256::new()
        .chain_update(request_binding(&fence, world, request, false))
        .chain_update(arrival_binding)
        .finalize()
        .to_vec();
    let tuple = tx.tuple().tuple();
    let message = trade_message(
        &fence,
        world,
        channel,
        &backpack,
        request,
        (1, 0, false),
        &plan,
    );
    let envelope = encode_npc_trade_event(
        NpcTradeEventIdentity {
            event_id: request.event_id,
            transaction_id: request.transaction_id,
            occurred_at_unix_ms: request.occurred_at_unix_ms,
            server_build_id: SERVER_BUILD_ID,
        },
        message,
        tuple,
    )?;
    apply_trade(
        tx,
        &fence,
        (world, channel),
        &backpack,
        request,
        (1, 0, 1, false),
        &binding,
        &plan,
        &envelope,
        tuple,
        None,
    )
    .await?;
    Ok(true)
}
