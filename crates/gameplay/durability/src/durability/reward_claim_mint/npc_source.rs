//! Six finite NPC outputs compose the mature backpack MINT in the parent TX.
//! No synthetic child command, nested container, Ground or Inbox fallback.
use super::*;
use crate::durability::item_mint_audit::Type2Transaction;
use crate::durability::npc_cosmetics::NpcProgramContext;

pub(in crate::durability) struct PreparedNpcOutput {
    frozen: FrozenClaim,
    reservation: Reservation,
    destination: ContainerEntryPosition,
}

pub(in crate::durability) fn output_id(source: &str) -> Option<u32> {
    match source {
        "oteryn:npc-cosmetic/sandra/belt-ticket-283"
        | "oteryn:npc-cosmetic/sandra/belt-ticket-284"
        | "oteryn:npc-cosmetic/sandra/belt-ticket-285" => Some(5957),
        "oteryn:npc-cosmetic/bozo/jester-addon-1" => Some(895),
        "oteryn:npc-cosmetic/bozo/jester-addon-2" => Some(894),
        "oteryn:npc-cosmetic/gnomadness/ripptor-prize" => Some(39546),
        _ => None,
    }
}

/// Called after actual source item costs, before the immutable NPC parent.
/// A refusal must roll back the parent transaction, including those costs.
pub(in crate::durability) async fn prepare_in_transaction(
    tx: &mut Type2Transaction<'_>,
    node: &NodeIncarnationProof,
    ctx: &NpcProgramContext<'_>,
    request: RewardClaimMintRequest,
    weights: &dyn super::super::item_transfer::ItemWeightLookup,
) -> Pass<PreparedNpcOutput> {
    let Some(output) = output_id(ctx.source_key) else {
        return Ok(Err(RewardClaimMintError::InvalidInput));
    };
    let parent = sqlx::types::Uuid::from_bytes(ctx.npc_transaction);
    let claim_key = format!("oteryn:npc-source-output.{parent}");
    if request.command != ctx.command
        || request.quantity != 1
        || request.item.definition.production_key != format!("oteryn:item.tibia.i{output}")
        || request.item.definition.family != "Item"
        || request.item.definition.revision_ref != "definition-r1"
        || request.item.stack != ItemStackClass::NonStackable
        || request.item.container_capacity.is_some()
        || request.claim.family != "NpcCosmetic"
        || request.claim.production_key != claim_key
        || request.claim.revision_ref
            != format!(
                "npc-source-program.{}",
                ctx.program_sha256
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>()
            )
        || request.source_placement != ctx.source_key
        || !request.contents.is_empty()
        || request.root_text.is_some()
        || request.key_number.is_some()
        || request.quest_transition.is_some()
        || request.achievement.is_some()
        || !contents::weight_pin_matches(&request, weights)
    {
        return Ok(Err(RewardClaimMintError::InvalidInput));
    }
    validate_request(&request).map_err(|_| DurabilityError::InvalidStoredState)?;
    let admitted: bool =
        sqlx::query_scalar("SELECT game_npc_source_effect_current(encode($1,'hex')::uuid,$2,$3)")
            .bind(ctx.npc_transaction.as_slice())
            .bind(ctx.source_key)
            .bind(serde_json::json!({"kind":"mint_item","item":output,"count":1}))
            .fetch_one(&mut ***tx)
            .await?;
    if !admitted {
        return Err(DurabilityError::InvalidStoredState);
    }
    // Retain the real CommandRef: no fabricated MINT sub-command namespace.
    lock_cause(tx, request.command).await?;
    if load_reservation(tx, request.command).await?.is_some()
        || load_receipt(tx, request.command).await?.is_some()
    {
        return Ok(Err(RewardClaimMintError::ConflictingCause));
    }
    let (world, channel) =
        scope_of(&ctx.fence.item_fence()).map_err(|_| DurabilityError::InvalidStoredState)?;
    let row=sqlx::query("SELECT game_character_uuid_v7()::text AS transaction_id,game_character_uuid_v7()::text AS event_id,game_character_uuid_v7()::text AS item_instance_id,floor(extract(epoch FROM statement_timestamp())*1000)::bigint AS occurred_at")
        .fetch_one(&mut ***tx).await?;
    let reservation = Reservation {
        character_id: ctx.fence.character_id,
        world_id: *world.as_bytes(),
        channel_id: *channel.as_bytes(),
        transaction_id: uuid_text(row.try_get("transaction_id")?)?,
        event_id: uuid_text(row.try_get("event_id")?)?,
        item_instance_id: uuid_text(row.try_get("item_instance_id")?)?,
        occurred_at_unix_ms: row.try_get("occurred_at")?,
        work_units_used: 1,
    };
    let destination = match admit(tx, node, &ctx.fence.item_fence(), &request, &reservation).await?
    {
        Ok((destination, _granter)) => destination,
        Err(error) => return Ok(Err(error)),
    };
    if let Err(error) = contents::check_capacity(
        tx,
        ctx.fence.character_id,
        reservation.world_id,
        &request,
        weights,
    )
    .await?
    {
        return Ok(Err(error));
    }
    let binding = intent_binding(&request, ctx.fence.character_id)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    insert_reservation(tx, &request, &binding, &reservation).await?;
    // The bounded parent Source pass owns this single work unit atomically.
    // The ordinary reservation's existing 0->1 guard remains unchanged.
    if sqlx::query("UPDATE game_reward_claim_mint_reservations SET work_units_used=1 WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric AND work_units_used=0")
        .bind(ctx.command.game_session_id().as_bytes().as_slice())
        .bind(ctx.command.command_id().get().to_string()).execute(&mut ***tx).await?.rows_affected()!=1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    sqlx::query("SELECT game_npc_source_bind_output(encode($1,'hex')::uuid)")
        .bind(reservation.transaction_id.as_slice())
        .execute(&mut ***tx)
        .await?;
    let frozen = FrozenClaim {
        request,
        character_id: ctx.fence.character_id,
        transaction_id: reservation.transaction_id,
        event_id: reservation.event_id,
        item_instance_id: reservation.item_instance_id,
        occurred_at_unix_ms: reservation.occurred_at_unix_ms,
        intent_binding: binding,
    };
    Ok(Ok(PreparedNpcOutput {
        frozen,
        reservation,
        destination,
    }))
}

pub(in crate::durability) async fn persist_in_transaction(
    tx: &mut Type2Transaction<'_>,
    ctx: &NpcProgramContext<'_>,
    prepared: &PreparedNpcOutput,
) -> std::result::Result<CommittedRewardClaimMint, DurabilityError> {
    if prepared.frozen.character_id != ctx.fence.character_id
        || prepared.frozen.request.command != ctx.command
        || prepared.frozen.request.source_placement != ctx.source_key
    {
        return Err(DurabilityError::InvalidStoredState);
    }
    let message = claim_message(
        &prepared.frozen,
        &prepared.reservation,
        &ctx.fence.item_fence(),
        prepared.destination,
    );
    let tuple = tx.tuple();
    let envelope = audit::encode_reward_claim_mint_event(
        RewardClaimMintEventIdentity {
            event_id: prepared.frozen.event_id,
            transaction_id: prepared.frozen.transaction_id,
            occurred_at_unix_ms: prepared.frozen.occurred_at_unix_ms,
            channel_id: prepared.reservation.channel_id,
            server_build_id: SERVER_BUILD_ID,
        },
        message,
        tuple.tuple(),
    )
    .map_err(|_| DurabilityError::InvalidStoredState)?;
    let minted = apply_claim(
        tx,
        &prepared.frozen,
        &prepared.reservation,
        prepared.destination,
        &envelope,
        tuple,
    )
    .await?;
    // Exact private output binding and parent/claim/custody reciprocity are
    // independently required by the immutable child's deferred SQL guards.
    sqlx::query("SELECT game_npc_source_output_capture(encode($1,'hex')::uuid)")
        .bind(ctx.npc_transaction.as_slice())
        .execute(&mut ***tx)
        .await?;
    Ok(minted)
}
