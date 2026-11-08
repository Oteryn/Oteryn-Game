//! Quest exchanges borrow the existing complete backpack MINT and custody owners.
use super::*;
use crate::domain::appearance::{AppearanceContent, EarnedCosmetic};
use crate::durability::account_achievement::{
    AchievementCatalogueLookup, AchievementGrantRequest, AchievementSourceEvent,
    FencedGrantingCharacter, record_achievement_grant,
};
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::durability::quest_state::QuestTransitionRequest;
use oteryn_gameplay_content::content::native_gameplay::NativeGameplayState;
use oteryn_gameplay_quests::quest::reward_callbacks::RewardCallback;

mod claim;
pub(crate) use claim::QuestMintClaim;

pub(crate) fn prepare_achievement(
    callback: Option<&RewardCallback>,
) -> std::result::Result<Option<AchievementGrantRequest>, DurabilityError> {
    let Some(callback) = callback else {
        return Ok(None);
    };
    if callback.achievement.is_none() && callback.achievement_progress.is_empty() {
        return Ok(None);
    }

    let catalogue = oteryn_gameplay_progression::achievement_catalogue::parse_shards(
        &oteryn_gameplay_progression::achievement_catalogue::SHARDS,
    )
    .map_err(|_| DurabilityError::InvalidStoredState)?;
    for progress in &callback.achievement_progress {
        if catalogue
            .get(&progress.key)
            .is_none_or(|entry| entry.retired)
        {
            return Err(DurabilityError::InvalidStoredState);
        }
    }
    let Some(key) = callback.achievement.as_ref() else {
        return Ok(None);
    };
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

pub(crate) use super::super::character_appearance::quest_grants::outfit_compatible;

/// Retained native coin/change identities; the source price remains in the callback.
#[derive(Clone, Debug)]
pub struct QuestRewardFeePayment {
    pub command: crate::foundation::CommandRef,
    pub occurrence: [u8; 16],
    pub transaction_id: [u8; 16],
    pub event_id: [u8; 16],
    pub coins: crate::durability::item_fee_burn::FeeChangeFacts,
    pub expected_backpack: Option<[u8; 16]>,
    pub occurred_at_unix_ms: i64,
}

/// Exact selected carried instance and ingress bytes, bound to its consumption proof.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct QuestCarriedUse {
    pub source: [u8; 16],
    pub intent: Vec<u8>,
}

/// The runtime supplies its admitted generation and the normal wheel-aware capacity lookup.
#[derive(Clone, Copy)]
pub struct QuestRewardAdmission<'a> {
    pub native: &'a NativeGameplayState,
    pub weights: &'a dyn super::super::item_transfer::ItemWeightLookup,
    pub fee_payment: Option<&'a QuestRewardFeePayment>,
    pub actor_conditions: Option<&'a super::QuestActorConditionProof>,
    pub carried_use: Option<&'a QuestCarriedUse>,
    pub creature_use: Option<&'a super::quest_creature::QuestCreatureUseProof>,
    pub peer_use: Option<&'a super::quest_peer::QuestPeerUseProof>,
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
    catalogue: &oteryn_gameplay_quests::quest::QuestStateCatalogue,
    reserve_primary_state: bool,
) -> std::result::Result<bool, DurabilityError> {
    let super::super::quest_state::QuestCause::Command(command) = request.cause else {
        return Ok(false);
    };
    if callback.carried_item.is_some() != admission.carried_use.is_some()
        || admission.carried_use.is_some_and(|use_| {
            use_.source == [0; 16] || use_.intent.is_empty() || use_.intent.len() > 4096
        })
    {
        return Ok(false);
    }
    match (callback.creature_target.is_some() || callback.self_target.is_some(), admission.creature_use, admission.carried_use) {
        (false, None, _) => {}
        (true, Some(proof), Some(carried)) if proof.permits(fence, request, callback, carried) => {}
        _ => return Ok(false),
    }
    if admission.weights.generation_digest() != admission.native.source_digest() {
        return Ok(false);
    }
    match (&callback.peer_requires, admission.peer_use, admission.carried_use) {
        (None, None, _) => {}
        (Some(requirement), Some(peer), Some(carried)) => {
            if !super::quest_peer::holds_in_transaction(tx, node, fence, peer, requirement, catalogue, carried).await? {
                return Ok(false);
            }
        }
        _ => return Ok(false),
    }
    if callback
        .cosmetic_targets()
        .any(|reward| !outfit_compatible(admission.native, reward))
    {
        return Ok(false);
    }
    if !callback.requires_actor_conditions.is_empty()
        && admission.actor_conditions.is_none_or(|proof| {
            !proof.permits(
                fence,
                request,
                admission.native.source_digest(),
                &callback.requires_actor_conditions,
            )
        })
    {
        return Ok(false);
    }
    if let Some(minimum) = callback.minimum_level {
        let level:Option<i64>=sqlx::query_scalar("SELECT level::bigint FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid")
            .bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
        if level.is_none_or(|level| level < i64::from(minimum)) {
            return Ok(false);
        }
    }
    if !callback.forbids_addons.is_empty() || !callback.forbids_outfits.is_empty() {
        // Keep a negative ownership predicate stable through costs and grant.
        // Reuse the account grant owner's lock; another character cannot win
        // the same addon after this read and still let this transaction charge.
        let account: sqlx::types::Uuid = sqlx::query_scalar(
            "SELECT account_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
        )
        .bind(fence.character_id.as_bytes().as_slice())
        .fetch_one(&mut **tx)
        .await?;
        super::super::character_appearance::lock_account_grants(tx, account).await?;
        let Some(cosmetics) = admission.native.cosmetics() else {
            return Ok(false);
        };
        for outfit in &callback.forbids_outfits {
            if super::super::character_appearance::podium_cosmetic_owned(
                tx,
                fence.character_id,
                cosmetics,
                EarnedCosmetic::Outfit(outfit),
            )
            .await?
            {
                return Ok(false);
            }
        }
        for reward in &callback.forbids_addons {
            let Some(count) = cosmetics.outfit_addon_count(&reward.outfit) else {
                return Ok(false);
            };
            for addon in 1..=2 {
                if reward.addons & (1 << (addon - 1)) != 0 && addon > count {
                    return Ok(false);
                }
                if reward.addons & (1 << (addon-1)) != 0
                    && super::super::character_appearance::cosmetic_owned_for_sex(
                        tx,fence.character_id,cosmetics,EarnedCosmetic::Addon(&reward.outfit,addon),reward.sex.map(|sex|match sex {
                        oteryn_gameplay_quests::quest::reward_callbacks::CreationSex::Female=>crate::domain::appearance::Sex::Female,
                        oteryn_gameplay_quests::quest::reward_callbacks::CreationSex::Male=>crate::domain::appearance::Sex::Male,
                    })
                    ).await? { return Ok(false); }
            }
        }
    }
    if !callback.requires_outfits.is_empty() {
        let Some(cosmetics) = admission.native.cosmetics() else {
            return Ok(false);
        };
        for outfit in &callback.requires_outfits {
            if !super::super::character_appearance::podium_cosmetic_owned(
                tx,
                fence.character_id,
                cosmetics,
                EarnedCosmetic::Outfit(outfit),
            )
            .await?
            {
                return Ok(false);
            }
        }
    }
    if !callback.requires_addons.is_empty() {
        let Some(cosmetics) = admission.native.cosmetics() else {
            return Ok(false);
        };
        for reward in &callback.requires_addons {
            for addon in 1..=2 {
                if reward.addons & (1 << (addon-1)) != 0
                    && !super::super::character_appearance::cosmetic_owned_for_sex(
                        tx,fence.character_id,cosmetics,EarnedCosmetic::Addon(&reward.outfit,addon),reward.sex.map(|sex|match sex {
                        oteryn_gameplay_quests::quest::reward_callbacks::CreationSex::Female=>crate::domain::appearance::Sex::Female,
                        oteryn_gameplay_quests::quest::reward_callbacks::CreationSex::Male=>crate::domain::appearance::Sex::Male,
                    })).await? {return Ok(false);}
            }
        }
    }
    if let Some(required) = callback.creation_sex {
        use oteryn_gameplay_quests::quest::reward_callbacks::CreationSex;
        let sex =
            super::super::character_appearance::item_action_sex(tx, fence.character_id).await?;
        let expected = match required {
            CreationSex::Female => crate::domain::appearance::Sex::Female,
            CreationSex::Male => crate::domain::appearance::Sex::Male,
        };
        if sex != expected {
            return Ok(false);
        }
    }
    if !super::quest_participants::apply(
        tx,
        fence,
        request,
        callback,
        catalogue,
        reserve_primary_state,
    )
    .await?
    {
        return Ok(false);
    }
    if let Some(fee) = &callback.fee {
        use crate::durability::item_fee_burn::{
            FeeBurnCause, FeeBurnError, FeeBurnRequest, burn_quest_reward_fee_in_transaction,
        };
        let Some(payment) = admission.fee_payment else {
            return Ok(false);
        };
        if payment.command != command {
            return Ok(false);
        }
        let current: Option<Vec<u8>> = sqlx::query_scalar("SELECT uuid_send(item_instance_id) FROM game_item_container_slots WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
            .bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
        if current.as_deref() != payment.expected_backpack.as_ref().map(|id| id.as_slice()) {
            return Ok(false);
        }
        let request = FeeBurnRequest {
            cause: FeeBurnCause::QuestRewardPayment {
                callback: callback.key.clone(),
                occurrence: payment.occurrence,
                gold: fee.gold,
                include_bank: fee.include_bank,
            },
            fee_gold_units: fee.gold,
            transaction_id: payment.transaction_id,
            event_id: payment.event_id,
            occurred_at_unix_ms: payment.occurred_at_unix_ms,
            server_build_id: "oteryn-game-server".into(),
            change: payment.coins.clone(),
        };
        match burn_quest_reward_fee_in_transaction(tx, &fence, &request, tuple).await {
            Ok(_) => {}
            Err(
                FeeBurnError::InsufficientFunds
                | FeeBurnError::ChangeDoesNotFit
                | FeeBurnError::CapacityExceeded,
            ) => return Ok(false),
            Err(FeeBurnError::Unavailable(error)) => return Err(error),
            Err(_) => return Err(DurabilityError::InvalidStoredState),
        }
    }
    if let (Some(fee), Some(payment)) = (&callback.fee, admission.fee_payment) {
        let committed = fence
            .expected_character_revision
            .get()
            .checked_add(1)
            .ok_or(DurabilityError::InvalidStoredState)?;
        sqlx::query("INSERT INTO game_quest_reward_fees(character_id,fee_occurrence,cause_id,cause_ordinal,transition_key,fee_gold_units,include_bank,committed_character_revision) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4::text::numeric,$5,$6,$7,$8::text::numeric)")
            .bind(fence.character_id.as_bytes().as_slice()).bind(payment.occurrence.as_slice())
            .bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string())
            .bind(&callback.key).bind(i64::try_from(fee.gold).map_err(|_|DurabilityError::InvalidStoredState)?)
            .bind(fee.include_bank).bind(committed.to_string()).execute(&mut **tx).await?;
    }
    if !super::super::quest_world_callback::consume_reward_inputs(
        tx,
        fence,
        request,
        callback,
        admission.carried_use,
    )
    .await?
    {
        return Ok(false);
    }
    let mut grant_keys = Vec::new();
    for reward in callback.cosmetic_targets() {
        let occurrence = serde_json::to_vec(&(
            command.game_session_id().as_bytes(),
            command.command_id().get(),
            &callback.key,
        ))
        .map_err(|_| DurabilityError::InvalidStoredState)?;
        grant_keys.extend(
            super::super::character_appearance::grant_content_outfit(
                tx,
                fence.character_id,
                &occurrence,
                reward,
                admission.native,
            )
            .await?,
        );
    }
    let minted = if let Some(reward) = &callback.give {
        let Some(transaction) = mint(
            tx,
            tuple,
            node,
            fence,
            command,
            &callback.key,
            if callback.is_paid_exchange() { QuestMintClaim::PaidExchange } else { QuestMintClaim::Once },
            reward,
            admission,
        )
        .await?
        else {
            return Ok(false);
        };
        Some(transaction)
    } else {
        None
    };
    let progress_event = Sha256::new()
        .chain_update(fence.character_id.as_bytes())
        .chain_update(command.game_session_id().as_bytes())
        .chain_update(command.command_id().get().to_be_bytes())
        .chain_update(callback.key.as_bytes())
        .finalize();
    for progress in &callback.achievement_progress {
        super::super::account_achievement::add_progress(
            tx,
            fence.character_id,
            &progress.key,
            progress.target,
            "oteryn:quest-reward",
            &progress_event,
        )
        .await?;
    }
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
pub(crate) async fn mint(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tuple: SelectedType2Tuple,
    node: &NodeIncarnationProof,
    fence: CurrentCharacterGameplayFence,
    command: CommandRef,
    callback_key: &str,
    claim: QuestMintClaim<'_>,
    reward: &oteryn_gameplay_quests::quest::reward_callbacks::ItemReward,
    admission: QuestRewardAdmission<'_>,
) -> std::result::Result<Option<[u8; 16]>, DurabilityError> {
    if reward.premium {
        let account: Vec<u8> = sqlx::query_scalar("SELECT uuid_send(account_id) FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid AND lifecycle=1 FOR UPDATE")
            .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
        let account = account
            .try_into()
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        if !admission.weights.wheel_premium_current(account) {
            return Ok(None);
        }
    }
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
    // Every occurrence still uses the existing MINT once ledger and reservations.
    let claim_key = claim::key(fence.character_id, command, callback_key, claim)?;
    let request = RewardClaimMintRequest {
        additional_items: reward
            .additional
            .iter()
            .map(|item| {
                if item.fluid_subtype.is_some() {
                    return Err(DurabilityError::InvalidStoredState);
                }
                Ok(RewardClaimMintContent {
                    fluid_subtype: None,
                    item: resolve(&item.item)?,
                    quantity: u32::from(item.count),
                    key_number: None,
                    text: None,
                })
            })
            .collect::<std::result::Result<Vec<_>, DurabilityError>>()?,
        command,
        claim: TypedDefinitionRef {
            family: "RewardClaim".into(),
            production_key: claim_key,
            revision_ref: content_revision.clone(),
        },
        source_placement: callback_key.into(),
        item: resolve(&reward.item)?,
        quantity: u32::from(reward.count),
        contents: reward
            .contents
            .iter()
            .map(|i| {
                if i.fluid_subtype.is_some() {
                    use oteryn_gameplay_content::content::{
                        ReferenceItemField, ReferenceItemFluid,
                    };
                    let policy = admission
                        .native
                        .item_policy(&i.item, "definition-r1")
                        .ok_or(DurabilityError::InvalidStoredState)?;
                    if !matches!(
                        &policy.record().semantics.fluid,
                        ReferenceItemField::Known(ReferenceItemFluid {
                            fluid_type: ReferenceItemField::NotApplicable
                        })
                    ) {
                        return Err(DurabilityError::InvalidStoredState);
                    }
                }
                Ok(RewardClaimMintContent {
                    fluid_subtype: i.fluid_subtype,
                    item: resolve(&i.item)?,
                    quantity: u32::from(i.count),
                    key_number: None,
                    text: None,
                })
            })
            .collect::<std::result::Result<Vec<_>, DurabilityError>>()?,
        root_text: reward.text.clone(),
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
