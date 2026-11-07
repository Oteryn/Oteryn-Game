//! Quest exchanges borrow the existing complete backpack MINT and custody owners.
use super::*;
use crate::domain::appearance::{AppearanceContent, EarnedCosmetic, UnlockFact};
use crate::durability::account_achievement::{
    AchievementCatalogueLookup, AchievementGrantRequest, AchievementSourceEvent,
    FencedGrantingCharacter, record_achievement_grant,
};
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::durability::quest_state::QuestTransitionRequest;
use oteryn_gameplay_content::content::native_gameplay::NativeGameplayState;
use oteryn_gameplay_quests::quest::reward_callbacks::{PROVENANCE, RewardCallback};

pub(crate) fn prepare_achievement(
    callback: Option<&RewardCallback>,
) -> std::result::Result<Option<AchievementGrantRequest>, DurabilityError> {
    let Some(key) = callback.and_then(|c| c.achievement.as_ref()) else {
        return Ok(None);
    };
    let catalogue = oteryn_gameplay_progression::achievement_catalogue::parse_shards(
        &oteryn_gameplay_progression::achievement_catalogue::SHARDS,
    )
    .map_err(|_| DurabilityError::InvalidStoredState)?;
    let entry = catalogue
        .get(key)
        .ok_or(DurabilityError::InvalidStoredState)?;
    if entry.retired {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(Some(AchievementGrantRequest {
        achievement_key: key.clone(),
        catalogue: AchievementCatalogueLookup::Earnable {
            revision: entry.revision.clone(),
        },
        source: AchievementSourceEvent {
            kind: "oteryn:quest-reward".into(),
            event_id: Vec::new(),
        },
    }))
}

/// Both NPC and world rewards use the admitted generation's explicit mappings.
pub(crate) fn outfit_compatible(
    native: &NativeGameplayState,
    reward: &oteryn_gameplay_quests::quest::reward_callbacks::OutfitReward,
) -> bool {
    let Some(cosmetics) = native.cosmetics() else { return false; };
    (0..=2).all(|addon| {
        if addon != 0 && reward.addons & (1 << (addon - 1)) == 0 { return true; }
        let key = format!("{}/quest-grant/{addon}", reward.outfit);
        let target = if addon == 0 { EarnedCosmetic::Outfit(&reward.outfit) }
            else { EarnedCosmetic::Addon(&reward.outfit, addon) };
        cosmetics.earned_unlock(&key, PROVENANCE, target) == UnlockFact::Compatible
    })
}

/// The runtime supplies its admitted generation and the normal wheel-aware capacity lookup.
#[derive(Clone, Copy)]
pub struct QuestRewardAdmission<'a> {
    pub native: &'a NativeGameplayState,
    pub weights: &'a dyn super::super::item_transfer::ItemWeightLookup,
}

#[allow(
    clippy::too_many_arguments,
    reason = "one fenced Quest transaction binds authority, receipt, admitted content and value participants"
)]
pub(crate) async fn apply(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tuple: SelectedType2Tuple,
    node: &NodeIncarnationProof,
    fence: CurrentCharacterGameplayFence,
    request: &QuestTransitionRequest,
    callback: &RewardCallback,
    admission: QuestRewardAdmission<'_>,
    achievement: Option<&AchievementGrantRequest>,
) -> std::result::Result<bool, DurabilityError> {
    let super::super::quest_state::QuestCause::Command(command) = request.cause else {
        return Ok(false);
    };
    if admission.weights.generation_digest() != admission.native.source_digest() {
        return Ok(false);
    }
    if callback.outfit.as_ref().is_some_and(|reward| !outfit_compatible(admission.native, reward)) {
        return Ok(false);
    }
    if !super::super::quest_world_callback::consume_reward_inputs(tx, fence, request, callback)
        .await?
    {
        return Ok(false);
    }
    let mut grant_keys = Vec::new();
    if let Some(reward) = &callback.outfit {
        let occurrence = serde_json::to_vec(&(
            command.game_session_id().as_bytes(),
            command.command_id().get(),
            &callback.key,
        ))
        .map_err(|_| DurabilityError::InvalidStoredState)?;
        grant_keys = super::super::character_appearance::grant_callback_outfit(
            tx,
            fence.character_id,
            &occurrence,
            reward,
        )
        .await?;
    }
    let minted = if let Some(reward) = &callback.give {
        let Some(transaction) =
            mint(tx, tuple, node, fence, command, callback, reward, admission).await?
        else {
            return Ok(false);
        };
        Some(transaction)
    } else {
        None
    };
    if let Some(achievement) = achievement {
        let mut grant = achievement.clone();
        grant.source.event_id = Sha256::new()
            .chain_update(fence.character_id.as_bytes())
            .chain_update(callback.key.as_bytes())
            .finalize()
            .to_vec();
        let granter = FencedGrantingCharacter::in_transaction(tx, fence.character_id).await?;
        record_achievement_grant(tx, granter, &grant)
            .await?
            .map_err(|_| DurabilityError::InvalidStoredState)?;
    }
    sqlx::query("INSERT INTO game_quest_reward_sources(character_id,cause_id,cause_ordinal,transition_key,plan,mint_transaction_id,grant_keys) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,$4,$5,CASE WHEN $6::bytea IS NULL THEN NULL ELSE encode($6,'hex')::uuid END,$7)")
        .bind(fence.character_id.as_bytes().as_slice()).bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string()).bind(&request.transition_key).bind(serde_json::to_value(callback).map_err(|_|DurabilityError::InvalidStoredState)?).bind(minted.map(|i|i.to_vec())).bind(grant_keys).execute(&mut **tx).await?;
    Ok(true)
}

#[allow(
    clippy::too_many_arguments,
    reason = "the existing MINT owner requires the same Quest, authority, content and request tuple"
)]
async fn mint(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tuple: SelectedType2Tuple,
    node: &NodeIncarnationProof,
    fence: CurrentCharacterGameplayFence,
    command: CommandRef,
    callback: &RewardCallback,
    reward: &oteryn_gameplay_quests::quest::reward_callbacks::ItemReward,
    admission: QuestRewardAdmission<'_>,
) -> std::result::Result<Option<[u8; 16]>, DurabilityError> {
    let resolve = |key: &str| {
        let id = key
            .strip_prefix("oteryn:item.tibia.i")
            .and_then(|s| s.parse::<u32>().ok())
            .ok_or(DurabilityError::InvalidStoredState)?;
        let policy = admission
            .native
            .item_policy_for_source_id(id)
            .ok_or(DurabilityError::InvalidStoredState)?
            .record();
        oteryn_gameplay_items::item_ref_admission::generation_item_facts(
            admission.native,
            &TypedDefinitionRef {
                family: policy.production_definition.family.clone(),
                production_key: key.into(),
                revision_ref: policy.production_definition.revision_ref.clone(),
            },
        )
        .map_err(|_| DurabilityError::InvalidStoredState)
    };
    let backpack:Option<String>=sqlx::query_scalar("SELECT i.definition_production_key FROM game_item_container_slots s JOIN game_item_instances i USING(item_instance_id) WHERE s.character_id=encode($1,'hex')::uuid AND i.lifecycle=1 FOR UPDATE OF s,i").bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    let Some(backpack) = backpack else {
        return Ok(None);
    };
    let (ruleset_revision,sim_revision):(String,String)=sqlx::query_as("SELECT ruleset_revision,simulation_revision FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid").bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
    let content_revision = format!(
        "sha256:{}",
        admission
            .native
            .source_digest()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    );
    let request = RewardClaimMintRequest {
        command,
        claim: TypedDefinitionRef {
            family: "RewardClaim".into(),
            production_key: callback.key.clone(),
            revision_ref: content_revision.clone(),
        },
        source_placement: callback.key.clone(),
        item: resolve(&reward.item)?,
        quantity: u32::from(reward.count),
        contents: reward
            .contents
            .iter()
            .map(|i| {
                Ok(RewardClaimMintContent {
                    item: resolve(&i.item)?,
                    quantity: u32::from(i.count),
                    key_number: None,
                    text: None,
                    fluid_subtype: None,
                    charges_remaining: None,
                })
            })
            .collect::<std::result::Result<Vec<_>, DurabilityError>>()?,
        root_text: None,
        root_fluid_subtype: None,
        root_charges_remaining: None,
        backpack: resolve(&backpack)?,
        content_revision,
        ruleset_revision,
        sim_revision,
        achievement: None,
        quest_transition: None,
        key_number: None,
    };
    validate_request(&request).map_err(|_| DurabilityError::InvalidStoredState)?;
    let binding = intent_binding(&request, fence.character_id)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let (world, channel) =
        scope_of(&fence.item_fence()).map_err(|_| DurabilityError::InvalidStoredState)?;
    let row=sqlx::query("SELECT game_character_uuid_v7()::text AS transaction_id,game_character_uuid_v7()::text AS event_id,game_character_uuid_v7()::text AS item_instance_id,floor(extract(epoch FROM statement_timestamp())*1000)::bigint AS occurred_at").fetch_one(&mut **tx).await?;
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
    let destination = match admit(tx, node, &fence.item_fence(), &request, &reservation).await? {
        Ok((destination, _)) => destination,
        Err(RewardClaimMintError::Refused(_)) => return Ok(None),
        Err(_) => return Err(DurabilityError::InvalidStoredState),
    };
    match contents::check_capacity(
        tx,
        fence.character_id,
        reservation.world_id,
        &request,
        admission.weights,
    )
    .await?
    {
        Ok(()) => {}
        Err(RewardClaimMintError::Refused(_)) => return Ok(None),
        Err(_) => return Err(DurabilityError::InvalidStoredState),
    }
    insert_reservation(tx, &request, &binding, &reservation).await?;
    sqlx::query("UPDATE game_reward_claim_mint_reservations SET work_units_used=1 WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric").bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string()).execute(&mut **tx).await?;
    let frozen = FrozenClaim {
        request,
        character_id: fence.character_id,
        transaction_id: reservation.transaction_id,
        event_id: reservation.event_id,
        item_instance_id: reservation.item_instance_id,
        occurred_at_unix_ms: reservation.occurred_at_unix_ms,
        intent_binding: binding,
    };
    let message = claim_message(&frozen, &reservation, &fence.item_fence(), destination);
    let envelope = audit::encode_reward_claim_mint_event(
        RewardClaimMintEventIdentity {
            event_id: frozen.event_id,
            transaction_id: frozen.transaction_id,
            occurred_at_unix_ms: frozen.occurred_at_unix_ms,
            channel_id: reservation.channel_id,
            server_build_id: SERVER_BUILD_ID,
        },
        message,
        tuple.tuple(),
    )
    .map_err(|_| DurabilityError::InvalidStoredState)?;
    apply_claim(tx, &frozen, &reservation, destination, &envelope, tuple).await?;
    Ok(Some(reservation.transaction_id))
}
