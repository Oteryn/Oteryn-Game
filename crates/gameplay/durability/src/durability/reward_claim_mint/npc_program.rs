//! A bounded source NPC reward reuses the existing once-claim, capacity and MINT audit owner.
use super::*;
use crate::durability::item_loop_move::ItemLoopContent;
use crate::durability::item_mint_audit::Type2Transaction;

#[allow(
    clippy::too_many_arguments,
    reason = "the source plan shares one existing fenced MINT transaction"
)]
pub(in crate::durability) async fn claim<C: ItemLoopContent>(
    tx: &mut Type2Transaction<'_>,
    node: &NodeIncarnationProof,
    fence: CurrentCharacterItemFence,
    command: CommandRef,
    content: &C,
    plan: &oteryn_gameplay_quests::quest::npc_rules::NpcRulePlan,
) -> Pass<[u8; 16]> {
    use oteryn_gameplay_quests::quest::npc_rules::NpcRuleEffect;
    let gifts = plan
        .effects
        .iter()
        .filter_map(|effect| match effect {
            NpcRuleEffect::GiveItem {
                item,
                count,
                subtype: None,
            } => Some((item, *count)),
            _ => None,
        })
        .collect::<Vec<_>>();
    let Some((reward_item, quantity @ 1..=100)) = gifts.first() else {
        return Ok(Err(RewardClaimMintError::InvalidInput));
    };
    if gifts.len() > 20 || gifts.iter().any(|(_, count)| !(1..=100).contains(count)) {
        return Ok(Err(RewardClaimMintError::InvalidInput));
    }
    let quantity = *quantity;
    let Some(rule) = plan.selected_rule_keys.first() else {
        return Ok(Err(RewardClaimMintError::InvalidInput));
    };
    let claim_key = crate::durability::quest_state::npc_program::hot_cuisine_m42::claim_key(plan)?;
    let context: Option<(String, String)> = sqlx::query_as("SELECT ruleset_revision,simulation_revision FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid")
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
        production_key: (*reward_item).clone(),
        revision_ref: "definition-r1".into(),
    };
    let (Some(item), Some(backpack)) = (
        content.loop_facts(&definition),
        content.loop_facts(&slot.definition),
    ) else {
        return Ok(Err(RewardClaimMintError::InvalidInput));
    };
    if item.container_capacity.is_some() {
        return Ok(Err(RewardClaimMintError::InvalidInput));
    }
    let mut additional_items = Vec::new();
    for (key, quantity) in gifts.iter().skip(1) {
        let definition = TypedDefinitionRef {
            family: "Item".into(),
            production_key: (*key).clone(),
            revision_ref: "definition-r1".into(),
        };
        let Some(item) = content.loop_facts(&definition) else {
            return Ok(Err(RewardClaimMintError::InvalidInput));
        };
        if item.container_capacity.is_some() {
            return Ok(Err(RewardClaimMintError::InvalidInput));
        }
        additional_items.push(RewardClaimMintContent {
            charges_remaining: None,
            item,
            quantity: *quantity,
            key_number: None,
            text: None,
            fluid_subtype: None,
        });
    }
    let request = RewardClaimMintRequest {
        root_fluid_subtype: None,
        root_charges_remaining: None,
        backpack_outputs: false,
        additional_items,
        command,
        claim: TypedDefinitionRef {
            family: "RewardClaim".into(),
            production_key: claim_key,
            revision_ref: "npc-program-r1".into(),
        },
        source_placement: rule.clone(),
        item,
        quantity,
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
        // M41 grants the source achievement in the same fenced NPC transaction.
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
    if let Err(error) =
        contents::check_capacity(tx, fence.character_id, *world.as_bytes(), &request, content)
            .await?
    {
        return Ok(Err(error));
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
    Ok(Ok(frozen.transaction_id))
}
