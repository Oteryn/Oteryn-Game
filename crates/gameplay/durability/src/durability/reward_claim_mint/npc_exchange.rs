//! A source-selected output shares the NPC recipe's physical transaction.
use super::*;
use crate::durability::item_loop_move::ItemLoopContent;
use crate::durability::item_mint_audit::Type2Transaction;
use crate::durability::quest_state::npc_program::NpcProgramRequest;
use oteryn_gameplay_quests::quest::npc_rules::{NpcRuleEffect, NpcRulePlan};

/// Match the mint owner's cause-before-Character lock order before any NPC
/// recipe takes its root lock. The output owner reuses the same transaction lock.
pub(in crate::durability) async fn lock_command(
    tx: &mut Type2Transaction<'_>, command: CommandRef,
) -> std::result::Result<(), DurabilityError> {
    lock_cause(tx, command).await
}

pub(in crate::durability) async fn claim<C: ItemLoopContent>(
    tx: &mut Type2Transaction<'_>, node: &NodeIncarnationProof,
    fence: CurrentCharacterItemFence, source: &NpcProgramRequest,
    plan: &NpcRulePlan, content: &C,
) -> std::result::Result<bool, DurabilityError> {
    let mut outputs = plan.effects.iter().filter_map(|effect| match effect {
        NpcRuleEffect::GiveItem { item, count, subtype } => Some((item, *count, *subtype)), _ => None,
    });
    let Some((key, quantity, subtype)) = outputs.next() else { return Ok(true); };
    if outputs.next().is_some() { return Err(DurabilityError::InvalidStoredState); }
    let Some(slot) = load_slot(tx, fence.character_id).await? else { return Ok(false); };
    let definition = TypedDefinitionRef { family: "Item".into(), production_key: key.clone(), revision_ref: "definition-r1".into() };
    let (Some(item), Some(backpack)) = (content.loop_facts(&definition), content.loop_facts(&slot.definition)) else { return Ok(false); };
    let (ruleset_revision, sim_revision): (String, String) = sqlx::query_as(
        "SELECT ruleset_revision,simulation_revision FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid",
    ).bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut ***tx).await?;
    // Quest progress controls repeat policy. This internal once claim identifies
    // only this exact command; it cannot prevent a later source-authorized recipe.
    let digest = Sha256::digest(source.binding(fence.character_id));
    let suffix = digest.iter().map(|byte| format!("{byte:02x}")).collect::<String>();
    let request = RewardClaimMintRequest {
        command: source.command,
        claim: TypedDefinitionRef { family: "RewardClaim".into(), production_key: format!("oteryn:reward-claim.npc-exchange/{suffix}"), revision_ref: "npc-exchange-r1".into() },
        source_placement: source.transition_key.clone(), item, quantity, contents: Vec::new(),
        root_text: None, root_fluid_subtype: subtype.map(u8::try_from).transpose().map_err(|_| DurabilityError::InvalidStoredState)?,
        root_charges_remaining: None, backpack,
        content_revision: format!("sha256:{}", content.loop_content_digest().iter().map(|byte| format!("{byte:02x}")).collect::<String>()),
        ruleset_revision, sim_revision, achievement: None, quest_transition: None, key_number: None,
    };
    if validate_request(&request).is_err() || !contents::weight_pin_matches(&request, content) { return Ok(false); }
    let binding = intent_binding(&request, fence.character_id).map_err(|_| DurabilityError::InvalidStoredState)?;
    lock_cause(tx, source.command).await?;
    if load_receipt(tx, source.command).await?.is_some() || load_reservation(tx, source.command).await?.is_some() { return Ok(false); }
    let (world, channel) = scope_of(&fence).map_err(|_| DurabilityError::InvalidStoredState)?;
    let row = sqlx::query("SELECT game_character_uuid_v7()::text AS transaction_id,game_character_uuid_v7()::text AS event_id,game_character_uuid_v7()::text AS item_instance_id,floor(extract(epoch FROM statement_timestamp())*1000)::bigint AS occurred_at").fetch_one(&mut ***tx).await?;
    let reservation = Reservation {
        character_id: fence.character_id, world_id: *world.as_bytes(), channel_id: *channel.as_bytes(),
        transaction_id: uuid_text(row.try_get("transaction_id")?)?, event_id: uuid_text(row.try_get("event_id")?)?,
        item_instance_id: uuid_text(row.try_get("item_instance_id")?)?, occurred_at_unix_ms: row.try_get("occurred_at")?, work_units_used: 0,
    };
    let (destination, _) = match admit(tx, node, &fence, &request, &reservation).await? { Ok(admitted) => admitted, Err(_) => return Ok(false) };
    let incoming = InventoryItem { item_instance_id: reservation.item_instance_id, definition: request.item.definition.clone(), quantity };
    if super::super::item_transfer::check_incoming_capacity(tx, fence.character_id, world.as_bytes(), &request.item, &incoming, content).await?.is_err() { return Ok(false); }
    insert_reservation(tx, &request, &binding, &reservation).await?;
    sqlx::query("UPDATE game_reward_claim_mint_reservations SET work_units_used=work_units_used+1 WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric")
        .bind(source.command.game_session_id().as_bytes().as_slice()).bind(source.command.command_id().get().to_string()).execute(&mut ***tx).await?;
    let frozen = FrozenClaim {
        request, character_id: fence.character_id, transaction_id: reservation.transaction_id, event_id: reservation.event_id,
        item_instance_id: reservation.item_instance_id, occurred_at_unix_ms: reservation.occurred_at_unix_ms, intent_binding: binding,
    };
    let tuple = tx.tuple();
    let envelope = audit::encode_reward_claim_mint_event(RewardClaimMintEventIdentity {
        event_id: frozen.event_id, transaction_id: frozen.transaction_id, occurred_at_unix_ms: frozen.occurred_at_unix_ms,
        channel_id: reservation.channel_id, server_build_id: SERVER_BUILD_ID,
    }, claim_message(&frozen, &reservation, &fence, destination), tuple.tuple()).map_err(|_| DurabilityError::InvalidStoredState)?;
    apply_claim(tx, &frozen, &reservation, destination, &envelope, tuple).await?;
    Ok(true)
}
