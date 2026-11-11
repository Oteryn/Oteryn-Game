//! Pemaret's trophy reuses the once-claim, capacity and audit owner in the fish exchange transaction.
use super::*;
use crate::durability::item_loop_move::ItemLoopContent;
use crate::durability::item_mint_audit::Type2Transaction;

pub(in crate::durability) async fn claim<C: ItemLoopContent>(
    tx: &mut Type2Transaction<'_>,
    node: &NodeIncarnationProof,
    fence: CurrentCharacterItemFence,
    command: CommandRef,
    content: &C,
) -> Pass<()> {
    let context: Option<(String, String)> = sqlx::query_as("SELECT ruleset_revision,simulation_revision FROM game_character_progression_state s WHERE s.character_id=encode($1,'hex')::uuid AND coalesce((SELECT value FROM game_character_quest_tracks WHERE character_id=s.character_id AND track_key='oteryn:quest-progress/quest/u7_8/marlin_trophy'),-1)<1")
        .bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut ***tx).await?;
    let Some((ruleset_revision, sim_revision)) = context else {
        return Ok(Err(RewardClaimMintError::InvalidInput));
    };
    let Some(slot) = load_slot(tx, fence.character_id).await? else {
        return Ok(Err(RewardClaimMintError::Refused(
            RewardClaimRefusal::NoMainBackpack,
        )));
    };
    let definition = TypedDefinitionRef {
        family: "Item".into(),
        production_key: "oteryn:item.tibia.i902".into(),
        revision_ref: "definition-r1".into(),
    };
    let (Some(item), Some(backpack)) = (
        content.loop_facts(&definition),
        content.loop_facts(&slot.definition),
    ) else {
        return Ok(Err(RewardClaimMintError::InvalidInput));
    };
    let request = RewardClaimMintRequest {
        root_fluid_subtype: None,
        root_charges_remaining: None,
        backpack_outputs: false,
        command,
        claim: TypedDefinitionRef {
            family: "RewardClaim".into(),
            production_key: "oteryn:reward-claim.npc.pemaret.marlin_trophy".into(),
            revision_ref: "marlin-trophy-r1".into(),
        },
        source_placement: "oteryn:npc.pemaret/deliver".into(),
        item,
        quantity: 1,
        contents: Vec::new(),
        root_text: None,
        backpack,
        content_revision: format!(
            "sha256:{}",
            content
                .loop_content_digest()
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        ),
        ruleset_revision,
        sim_revision,
        achievement: None,
        quest_transition: None,
        key_number: None,
    };
    if validate_request(&request).is_err() || !contents::weight_pin_matches(&request, content) {
        return Ok(Err(RewardClaimMintError::InvalidInput));
    }
    let binding = match intent_binding(&request, fence.character_id) {
        Ok(binding) => binding,
        Err(error) => return Ok(Err(error)),
    };
    lock_cause(tx, command).await?;
    if load_receipt(tx, command).await?.is_some() || load_reservation(tx, command).await?.is_some()
    {
        return Ok(Err(RewardClaimMintError::ConflictingCause));
    }
    let (world, channel) = scope_of(&fence).map_err(|_| DurabilityError::InvalidStoredState)?;
    let row = sqlx::query("SELECT game_character_uuid_v7()::text AS transaction_id,game_character_uuid_v7()::text AS event_id,game_character_uuid_v7()::text AS item_instance_id,floor(extract(epoch FROM statement_timestamp())*1000)::bigint AS occurred_at")
        .fetch_one(&mut ***tx).await?;
    let reservation = Reservation {
        character_id: fence.character_id,
        world_id: *world.as_bytes(),
        channel_id: *channel.as_bytes(),
        transaction_id: uuid_text(row.try_get("transaction_id")?)?,
        event_id: uuid_text(row.try_get("event_id")?)?,
        item_instance_id: uuid_text(row.try_get("item_instance_id")?)?,
        occurred_at_unix_ms: row.try_get("occurred_at")?,
        work_units_used: 0,
    };
    let (destination, _) = match admit(tx, node, &fence, &request, &reservation).await? {
        Ok(admitted) => admitted,
        Err(error) => return Ok(Err(error)),
    };
    let incoming = InventoryItem {
        item_instance_id: reservation.item_instance_id,
        definition: request.item.definition.clone(),
        quantity: 1,
    };
    if let Err(refusal) = super::super::item_transfer::check_incoming_capacity(
        tx,
        fence.character_id,
        world.as_bytes(),
        &request.item,
        &incoming,
        content,
    )
    .await?
    {
        return Ok(Err(if refusal == ItemTransferRefusal::Overweight {
            RewardClaimMintError::Refused(RewardClaimRefusal::CarryingCapacity)
        } else {
            RewardClaimMintError::InvalidInput
        }));
    }
    insert_reservation(tx, &request, &binding, &reservation).await?;
    sqlx::query("UPDATE game_reward_claim_mint_reservations SET work_units_used=work_units_used+1 WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric")
        .bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string()).execute(&mut ***tx).await?;
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
    let message = claim_message(&frozen, &reservation, &fence, destination);
    let envelope = match audit::encode_reward_claim_mint_event(
        RewardClaimMintEventIdentity {
            event_id: frozen.event_id,
            transaction_id: frozen.transaction_id,
            occurred_at_unix_ms: frozen.occurred_at_unix_ms,
            channel_id: reservation.channel_id,
            server_build_id: SERVER_BUILD_ID,
        },
        message,
        tuple.tuple(),
    ) {
        Ok(envelope) => envelope,
        Err(error) => return Ok(Err(error.into())),
    };
    apply_claim(tx, &frozen, &reservation, destination, &envelope, tuple).await?;
    Ok(Ok(()))
}
