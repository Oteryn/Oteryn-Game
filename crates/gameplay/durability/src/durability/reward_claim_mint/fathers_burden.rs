//! Tereban's final source reward reuses the once-claim reservation, admission,
//! item application and one-item audit owner in the NPC's physical transaction.
use super::*;
use crate::durability::item_loop_move::ItemLoopContent;
use crate::durability::item_mint_audit::Type2Transaction;

pub(in crate::durability) async fn claim<C: ItemLoopContent>(
    tx: &mut Type2Transaction<'_>,
    node: &NodeIncarnationProof,
    fence: CurrentCharacterItemFence,
    command: CommandRef,
    content: &C,
    platinum: u32,
) -> Pass<()> {
    let context: Option<(String,String)> = sqlx::query_as("SELECT ruleset_revision,simulation_revision FROM game_character_progression_state s WHERE s.character_id=encode($1,'hex')::uuid AND EXISTS(SELECT 1 FROM game_character_quest_tracks WHERE character_id=s.character_id AND track_key='oteryn:quest-progress/quest/u8_6/afathers_burden/status' AND value=1) AND EXISTS(SELECT 1 FROM game_character_quest_tracks WHERE character_id=s.character_id AND track_key='oteryn:quest-progress/quest/u8_6/afathers_burden/progress' AND value=8)")
        .bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut ***tx).await?;
    let Some((ruleset_revision, sim_revision)) = context else {
        return Ok(Err(RewardClaimMintError::InvalidInput));
    };
    mint(
        tx,
        node,
        fence,
        command,
        content,
        (
            "oteryn:reward-claim.npc.tereban.old_cape",
            "oteryn:npc.tereban/mission-complete",
        ),
        "oteryn:item.tibia.i11701",
        Some(platinum),
        (ruleset_revision, sim_revision),
    )
    .await
    .map(|result| result.map(|_| ()))
}

#[allow(
    clippy::too_many_arguments,
    reason = "the two Father rewards share the fenced mint"
)]
async fn mint<C: ItemLoopContent>(
    tx: &mut Type2Transaction<'_>,
    node: &NodeIncarnationProof,
    fence: CurrentCharacterItemFence,
    command: CommandRef,
    content: &C,
    (claim, source): (&str, &str),
    reward: &str,
    platinum: Option<u32>,
    (ruleset_revision, sim_revision): (String, String),
) -> Pass<bool> {
    let Some(slot) = load_slot(tx, fence.character_id).await? else {
        return Ok(Err(RewardClaimMintError::Refused(
            RewardClaimRefusal::NoMainBackpack,
        )));
    };
    let definition = TypedDefinitionRef {
        family: "Item".into(),
        production_key: reward.into(),
        revision_ref: "definition-r1".into(),
    };
    let (Some(item), Some(backpack)) = (
        content.loop_facts(&definition),
        content.loop_facts(&slot.definition),
    ) else {
        return Ok(Err(RewardClaimMintError::InvalidInput));
    };
    let mut outputs = Vec::new();
    if let Some(quantity) = platinum {
        let definition = TypedDefinitionRef {
            family: "Item".into(),
            production_key: "oteryn:item.tibia.i3035".into(),
            revision_ref: "definition-r1".into(),
        };
        let Some(item) = content.loop_facts(&definition) else {
            return Ok(Err(RewardClaimMintError::InvalidInput));
        };
        outputs.push(RewardClaimMintContent {
            item,
            quantity,
            fluid_subtype: None,
            charges_remaining: None,
            key_number: None,
            text: None,
        });
    }
    let request = RewardClaimMintRequest {
        additional_items: Vec::new(),
        root_fluid_subtype: None,
        root_charges_remaining: None,
        backpack_outputs: !outputs.is_empty(),
        contents: outputs,
        root_text: None,
        backpack,
        command,
        claim: TypedDefinitionRef {
            family: "RewardClaim".into(),
            production_key: claim.into(),
            revision_ref: "fathers-burden-r1".into(),
        },
        source_placement: source.into(),
        item,
        quantity: 1,
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
    if let Some(row) = load_receipt(tx, command).await? {
        return Ok(if row.try_get::<Vec<u8>, _>("intent_binding")? == binding {
            Ok(false)
        } else {
            Err(RewardClaimMintError::ConflictingCause)
        });
    }
    if load_reservation(tx, command).await?.is_some() {
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
    Ok(Ok(true))
}

/// Donor actions_corpse.lua:2–19. The claim key is character-scoped,
/// while the physical corpse remains available to everyone in reach.
#[derive(Clone, Copy)]
pub struct FatherCorpseReward {
    creature: &'static str,
    corpse: &'static str,
    item: &'static str,
    claim: &'static str,
    pub feedback: &'static str,
}

pub fn father_corpse_reward(corpse: &str) -> Option<FatherCorpseReward> {
    let (corpse, creature, item, claim, feedback) = match corpse {
        "oteryn:item.tibia.i11589" => (
            "oteryn:item.tibia.i11589",
            "oteryn:creature.glitterscale",
            "oteryn:item.tibia.i11550",
            "oteryn:reward-claim.quest.fathers-burden.scale",
            "You acquired Glitterscale's scale.",
        ),
        "oteryn:item.tibia.i11590" => (
            "oteryn:item.tibia.i11590",
            "oteryn:creature.heoni",
            "oteryn:item.tibia.i11548",
            "oteryn:reward-claim.quest.fathers-burden.sinew",
            "You acquired Heoni's sinew",
        ),
        _ => return None,
    };
    Some(FatherCorpseReward {
        creature,
        corpse,
        item,
        claim,
        feedback,
    })
}

impl FatherCorpseReward {
    #[allow(clippy::too_many_arguments, reason = "the exact actor fence and Ground operand remain explicit")]
    pub async fn claim<C: ItemLoopContent>(
        self,
        root: &DurabilityRoot,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        command: CommandRef,
        corpse: ([u8; 16], Vec<u8>),
        content: &C,
    ) -> Result<bool> {
        let recovery = authority
            .record_for(root)
            .map_err(|_| RewardClaimMintError::AuthorityRejected)?;
        let node = node.clone();
        let mut context = content;
        root.try_issue_semantic_pass()?.run_with_context(&mut context, move |holder, deadline, content| {
            Box::pin(async move {
                let mut tx = begin_type2_transaction(holder, deadline).await?;
                assert_recovery_fence(&mut tx, &recovery).await?;
                lock_admission_relations(&mut tx).await?;
                let (world, channel) = scope_of(&fence).map_err(|_| DurabilityError::InvalidStoredState)?;
                if !character_item_fence_is_current(&mut tx, &node, &fence, command,
                    fence.character_id, *world.as_bytes(), *channel.as_bytes()).await? {
                    return Ok(Err(RewardClaimMintError::AuthorityRejected));
                }
                let ground: Option<Vec<u8>> = sqlx::query_scalar(
                    "SELECT g.spatial_position FROM game_item_instances i
                     JOIN game_item_ground_locations g USING(item_instance_id,world_id)
                     LEFT JOIN game_corpse_source_races p USING(item_instance_id)
                     WHERE i.item_instance_id=encode($1,'hex')::uuid
                       AND i.world_id=encode($2,'hex')::uuid AND g.channel_id=encode($3,'hex')::uuid
                       AND g.runtime_scope_ownership_generation=$4::text::numeric
                       AND i.lifecycle=1 AND i.definition_family='Item' AND i.definition_production_key=$5
                       AND i.definition_revision_ref='definition-r1'
                       AND (p.race_key IS NULL OR p.race_key=$6)
                       AND game_item_plain_corpse_source(i.item_instance_id)
                     FOR SHARE OF i,g",
                )
                    .bind(corpse.0.as_slice()).bind(world.as_bytes().as_slice())
                    .bind(channel.as_bytes().as_slice()).bind(fence.scope_ownership_generation.get().to_string())
                    .bind(self.corpse).bind(self.creature)
                    .fetch_optional(&mut **tx).await?;
                if ground.as_deref() != Some(corpse.1.as_slice()) {
                    return Ok(Err(RewardClaimMintError::InvalidInput));
                }
                let revisions: (String, String) = sqlx::query_as(
                    "SELECT ruleset_revision,simulation_revision FROM game_character_progression_state
                     WHERE character_id=encode($1,'hex')::uuid",
                ).bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
                let source = format!("{}/{:032x}", self.creature, u128::from_be_bytes(corpse.0));
                let result = mint(&mut tx, &node, fence, command, *content,
                    (self.claim, &source), self.item, None, revisions).await?;
                if result.is_ok() {
                    tx.commit(deadline).await?;
                }
                Ok(result)
            })
        }).await?
    }
}

#[cfg(test)]
mod tests {
    use super::father_corpse_reward;

    #[test]
    fn qfather_exact_corpse_definitions_grant_distinct_materials() {
        let scale = father_corpse_reward("oteryn:item.tibia.i11589")
            .map(|rule| (rule.creature, rule.item, rule.claim));
        let sinew = father_corpse_reward("oteryn:item.tibia.i11590")
            .map(|rule| (rule.creature, rule.item, rule.claim));
        assert_eq!(
            scale,
            Some((
                "oteryn:creature.glitterscale",
                "oteryn:item.tibia.i11550",
                "oteryn:reward-claim.quest.fathers-burden.scale"
            ))
        );
        assert_eq!(
            sinew,
            Some((
                "oteryn:creature.heoni",
                "oteryn:item.tibia.i11548",
                "oteryn:reward-claim.quest.fathers-burden.sinew"
            ))
        );
        assert!(father_corpse_reward("oteryn:item.tibia.i11548").is_none());
    }
}
