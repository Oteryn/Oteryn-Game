//! Seasonal NPCs compose eligibility with the mature weighted fresh-container owner.
//! As with Grizzly rewards, reservation, complete mint and source flags share one commit.
use super::super::item_mint_audit::Type2Transaction;
use super::super::item_transfer::ItemWeightLookup;
use super::*;

/// Historical receipt resolution grants no fresh session authority.
pub(crate) async fn replay(
    tx: &mut Type2Transaction<'_>,
    character: CharacterId,
    request: &RewardClaimMintRequest,
) -> Result<Option<CommittedRewardClaimMint>> {
    validate_request(request)?;
    let binding = intent_binding(request, character)?;
    lock_cause(tx, request.command).await?;
    let Some(row) = load_receipt(tx, request.command).await? else {
        return Ok(None);
    };
    if row
        .try_get::<Vec<u8>, _>("intent_binding")
        .map_err(DurabilityError::from)?
        != binding
    {
        return Err(RewardClaimMintError::ConflictingCause);
    }
    Ok(Some(decode_receipt(&row)?))
}

/// Caller has locked and proved source eligibility under its current Character fence.
/// Reuses native reservation, capacity, weight, placement and complete child-audit proofs.
pub(crate) async fn mint(
    tx: &mut Type2Transaction<'_>,
    node: &NodeIncarnationProof,
    fence: &CurrentCharacterItemFence,
    request: RewardClaimMintRequest,
    weights: &dyn ItemWeightLookup,
) -> Result<CommittedRewardClaimMint> {
    validate_request(&request)?;
    if request.achievement.is_some()
        || request.quest_transition.is_some()
        || !contents::weight_pin_matches(&request, weights)
    {
        return Err(RewardClaimMintError::InvalidInput);
    }
    let binding = intent_binding(&request, fence.character_id)?;
    lock_cause(tx, request.command).await?;
    if load_reservation(tx, request.command).await?.is_some() {
        return Err(RewardClaimMintError::ConflictingCause);
    }
    let (world, channel) = scope_of(fence).map_err(from_transfer_input)?;
    let row = sqlx::query("SELECT game_character_uuid_v7()::text AS transaction_id,game_character_uuid_v7()::text AS event_id,game_character_uuid_v7()::text AS item_instance_id,floor(extract(epoch FROM statement_timestamp())*1000)::bigint AS occurred_at")
        .fetch_one(&mut ***tx).await.map_err(DurabilityError::from)?;
    let reservation = Reservation {
        character_id: fence.character_id,
        world_id: *world.as_bytes(),
        channel_id: *channel.as_bytes(),
        transaction_id: uuid_text(
            row.try_get("transaction_id")
                .map_err(DurabilityError::from)?,
        )?,
        event_id: uuid_text(row.try_get("event_id").map_err(DurabilityError::from)?)?,
        item_instance_id: uuid_text(
            row.try_get("item_instance_id")
                .map_err(DurabilityError::from)?,
        )?,
        occurred_at_unix_ms: row.try_get("occurred_at").map_err(DurabilityError::from)?,
        work_units_used: 0,
    };
    let (destination, _) = admit(tx, node, fence, &request, &reservation).await??;
    contents::check_capacity(
        tx,
        fence.character_id,
        reservation.world_id,
        &request,
        weights,
    )
    .await??;
    insert_reservation(tx, &request, &binding, &reservation).await?;
    sqlx::query("UPDATE game_reward_claim_mint_reservations SET work_units_used=work_units_used+1 WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric(20,0)")
        .bind(request.command.game_session_id().as_bytes().as_slice())
        .bind(request.command.command_id().get().to_string()).execute(&mut ***tx).await.map_err(DurabilityError::from)?;
    let frozen = FrozenClaim {
        request,
        character_id: fence.character_id,
        transaction_id: reservation.transaction_id,
        event_id: reservation.event_id,
        item_instance_id: reservation.item_instance_id,
        occurred_at_unix_ms: reservation.occurred_at_unix_ms,
        intent_binding: binding,
    };
    let tuple = tx.tuple();
    let envelope = audit::encode_reward_claim_mint_event(
        RewardClaimMintEventIdentity {
            event_id: frozen.event_id,
            transaction_id: frozen.transaction_id,
            occurred_at_unix_ms: frozen.occurred_at_unix_ms,
            channel_id: reservation.channel_id,
            server_build_id: SERVER_BUILD_ID,
        },
        claim_message(&frozen, &reservation, fence, destination),
        tuple.tuple(),
    )?;
    Ok(apply_claim(tx, &frozen, &reservation, destination, &envelope, tuple).await?)
}
