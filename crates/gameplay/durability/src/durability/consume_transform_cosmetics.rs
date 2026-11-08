//! Source cosmetic effects inside the existing fenced consume/transform transaction.
//! The active canonical catalogue and explicit grant mappings are required before any spend.
use super::DurabilityError;
use super::consume_transform::{ActionPlan, ConsumeTransformRequest, ExtraInput};
use super::item_mint::TypedDefinitionRef;
use super::item_mint_audit::Type2Transaction;
use super::item_transfer::CurrentCharacterItemFence;
use crate::domain::appearance::{AppearanceContent, EarnedCosmetics};
use oteryn_gameplay_content::content::native_gameplay::NativeGameplayState;
use oteryn_gameplay_items::item_cosmetics::{self as rules, Progress, ResolvedGrant};
use oteryn_protocol_oteryn::world_object::UseDisposition;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::Row;
use std::collections::BTreeMap;

/// A live view of the existing Premium owner, not a request bit or durable snapshot.
/// The caller refreshes the view from current session owners; this method rechecks its
/// trusted clock and account binding while the item transaction is held.
pub trait ItemCosmeticPremium: Sync {
    fn current(&self, account: [u8; 16]) -> bool;
}

pub(super) enum Preparation {
    Unrelated,
    Refused(UseDisposition),
    Ready(ActionPlan),
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CosmeticEffect {
    source_item_id: u32,
    provenance: String,
    consumed: bool,
    grants: Vec<ResolvedGrant>,
    progress: Option<Progress>,
    achievements: Vec<String>,
    achievement_progress: Vec<oteryn_gameplay_items::novelty_items::AchievementProgress>,
    feedback: Option<String>,
}

#[allow(
    clippy::too_many_arguments,
    reason = "receipt request, current content and independent premium owner are distinct inputs"
)]
pub(super) async fn prepare(
    tx: &mut Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    request: &ConsumeTransformRequest,
    definition: &TypedDefinitionRef,
    native: &NativeGameplayState,
    premium: Option<&dyn ItemCosmeticPremium>,
) -> Result<Preparation, DurabilityError> {
    prepare_with_custody(tx, fence, request, definition, native, premium, true).await
}

pub(super) async fn prepare_on_ground(
    tx: &mut Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    request: &ConsumeTransformRequest,
    definition: &TypedDefinitionRef,
    native: &NativeGameplayState,
    premium: Option<&dyn ItemCosmeticPremium>,
) -> Result<Preparation, DurabilityError> {
    prepare_with_custody(tx, fence, request, definition, native, premium, false).await
}

async fn prepare_with_custody(
    tx: &mut Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    request: &ConsumeTransformRequest,
    definition: &TypedDefinitionRef,
    native: &NativeGameplayState,
    premium: Option<&dyn ItemCosmeticPremium>,
    source_is_carried: bool,
) -> Result<Preparation, DurabilityError> {
    let Some(rule) = rules::rule(&definition.production_key) else {
        return Ok(Preparation::Unrelated);
    };
    // Keep the same source-policy gate for carried and Ground writers.
    if !rules::source_policy_allows(rule, native.cosmetics(), &definition.production_key) {
        return Ok(Preparation::Refused(UseDisposition::NotPossible));
    }
    if request.target.is_some() {
        return Ok(Preparation::Refused(UseDisposition::NothingToUse));
    }
    // W19_CANONICAL_COSMETICS_SOURCE_MISSING. The source table does not promote a
    // canonical catalogue; this is the current READY_UNPOPULATED production case.
    let Some(content) = native
        .cosmetics()
        .filter(|c| c.source_digest() == native.source_digest())
    else {
        return Ok(Preparation::Refused(UseDisposition::NotPossible));
    };
    if !native
        .item_policy(&definition.production_key, &definition.revision_ref)
        .is_some_and(|policy| {
            policy.record().admission.materializable
                && policy.record().production_definition.family == definition.family
        })
    {
        return Ok(Preparation::Refused(UseDisposition::NotPossible));
    }
    let account: sqlx::types::Uuid = sqlx::query_scalar(
        "SELECT account_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid AND lifecycle=1",
    ).bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut ***tx).await?;
    // Source items and mount taming share this account lock; selecting different grant
    // keys for the same canonical cosmetic cannot burn two acquisition items concurrently.
    super::character_appearance::lock_account_grants(tx, account).await?;
    let sex = super::character_appearance::item_action_sex(tx, fence.character_id).await?;
    let account_id = crate::domain::AccountId::from_bytes(*account.as_bytes())
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let facts = super::character_appearance::read_account_appearance_unlocks(
        tx,
        account_id,
        &content.earned_grant_keys(),
    )
    .await
    .map_err(|_| DurabilityError::InvalidStoredState)?;
    let facts: Vec<_> = facts
        .into_iter()
        .filter(|fact| {
            matches!(
                fact.source_kind.as_str(),
                "quest" | "npc" | "mount_taming" | "task_board" | "item_action"
            )
        })
        .map(|fact| (fact.unlock_key, fact.content_revision))
        .collect();
    let owned = EarnedCosmetics {
        content,
        facts: &facts,
    }.with_sex(sex);
    let component_keys: Vec<_> = rule.components.iter().map(|p| p.key.as_str()).collect();
    let rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT component_key,contributions FROM game_character_item_cosmetic_progress WHERE character_id=encode($1,'hex')::uuid AND component_key=ANY($2::text[]) FOR UPDATE",
    ).bind(fence.character_id.as_bytes().as_slice()).bind(&component_keys).fetch_all(&mut ***tx).await?;
    let progress = rows
        .into_iter()
        .map(|(key, count)| {
            Ok((
                key,
                u32::try_from(count).map_err(|_| DurabilityError::InvalidStoredState)?,
            ))
        })
        .collect::<Result<BTreeMap<_, _>, DurabilityError>>()?;
    let Some(catalogue) = rules::catalogue() else {
        return Ok(Preparation::Refused(UseDisposition::NotPossible));
    };
    if rules::qualify_rule(catalogue, rule, content).is_err() {
        return Ok(Preparation::Refused(UseDisposition::NotPossible));
    }
    let Some(choice) = rules::select_for_command(
        rule,
        *fence.game_session_id.as_bytes(),
        request.command.command_id().get(),
        request.source,
    ) else {
        return Ok(Preparation::Refused(UseDisposition::NotPossible));
    };
    let decision = match rules::decide(
        catalogue,
        rule,
        choice,
        content,
        &owned,
        sex,
        premium.is_some_and(|owner| owner.current(*account.as_bytes())),
        &progress,
    ) {
        Ok(decision) => decision,
        Err(rules::Refusal::PremiumRequired | rules::Refusal::OutfitRequired) => {
            return Ok(Preparation::Refused(UseDisposition::RequirementNotMet));
        }
        Err(rules::Refusal::ContentUnavailable | rules::Refusal::InvalidProgress) => {
            return Ok(Preparation::Refused(UseDisposition::NotPossible));
        }
    };
    let inputs = rules::inputs(rule, &decision, source_is_carried);
    let mut extra_inputs = Vec::new();
    for item in &inputs.carried_items {
        let key = format!("oteryn:item.tibia.i{item}");
        let Some(policy) = native.item_policy_for_source_id(*item) else {
            return Ok(Preparation::Refused(UseDisposition::NotPossible));
        };
        if !policy.record().admission.materializable {
            return Ok(Preparation::Refused(UseDisposition::NotPossible));
        }
        // Crystal getItemCount/removeItem include nested carried bags. Use the same
        // bounded root proof as ordinary item USE; neither depot nor Ground is eligible.
        let row = sqlx::query(
            "SELECT uuid_send(i.item_instance_id) AS item,to_jsonb(i) AS before_item,uuid_send(coalesce(e.parent_item_instance_id,n.parent_item_instance_id)) AS parent,coalesce(e.placement_ordinal,n.placement_ordinal)::text AS ordinal,CASE WHEN n.item_instance_id IS NULL THEN 1 ELSE 4 END AS source_kind FROM game_item_instances i LEFT JOIN game_item_container_entries e ON e.item_instance_id=i.item_instance_id AND e.character_id=encode($1,'hex')::uuid LEFT JOIN game_item_container_slots b ON b.character_id=e.character_id AND b.item_instance_id=e.parent_item_instance_id LEFT JOIN game_item_nested_entries n ON n.item_instance_id=i.item_instance_id LEFT JOIN LATERAL game_item_loop_owner(n.parent_item_instance_id) owner ON n.item_instance_id IS NOT NULL WHERE i.world_id=encode($4,'hex')::uuid AND ((e.world_id=i.world_id AND b.world_id=i.world_id) OR (n.world_id=i.world_id AND owner.owner_character=encode($1,'hex')::uuid AND owner.depth BETWEEN 0 AND 7)) AND i.definition_family='Item' AND i.definition_production_key=$2 AND i.definition_revision_ref=$3 AND i.lifecycle=1 AND i.quantity>0 ORDER BY i.item_instance_id LIMIT 1 FOR UPDATE OF i",
        ).bind(fence.character_id.as_bytes().as_slice()).bind(&key)
            .bind(&policy.record().production_definition.revision_ref)
            .bind(fence.runtime_scope.world_id().as_bytes().as_slice())
            .fetch_optional(&mut ***tx).await?;
        let Some(row) = row else {
            return Ok(Preparation::Refused(UseDisposition::RequirementNotMet));
        };
        extra_inputs.push(ExtraInput {
            item: row
                .try_get::<Vec<u8>, _>("item")?
                .try_into()
                .map_err(|_| DurabilityError::InvalidStoredState)?,
            before: row.try_get("before_item")?,
            parent: row
                .try_get::<Vec<u8>, _>("parent")?
                .try_into()
                .map_err(|_| DurabilityError::InvalidStoredState)?,
            ordinal: row.try_get("ordinal")?,
            count: 1,
            source_kind: row.try_get("source_kind")?,
        });
    }
    let effect = CosmeticEffect {
        source_item_id: rule.item_id,
        provenance: rules::PROVENANCE.into(),
        consumed: decision.consume,
        grants: decision.grants,
        progress: decision.progress,
        achievements: decision.achievements,
        achievement_progress: decision.achievement_progress,
        feedback: decision.feedback,
    };
    Ok(Preparation::Ready(ActionPlan {
        recipe: format!("oteryn:item-cosmetic.i{}.v1", rule.item_id),
        outputs: Vec::new(),
        consume_source: inputs.consume_source,
        source_count: u32::from(inputs.consume_source),
        extra_inputs,
        effects: json!({"cosmetic": effect, "kind": "cosmetic", "private_speech": choice.private_speech,
            "require_head": false, "require_player_target": false, "achievement_progress": [],
            "cues": if decision.consume { choice.cues.clone() } else { Vec::new() },
            "speech": if decision.consume { choice.speech.clone() } else { None },
            "source_item_id": rule.item_id, "donor": rule.donor}),
    }))
}

pub(super) async fn apply(
    tx: &mut Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    _request: &ConsumeTransformRequest,
    plan: &ActionPlan,
    transaction: &str,
) -> Result<(), DurabilityError> {
    let Some(value) = plan.effects.get("cosmetic") else {
        return Ok(());
    };
    let effect: CosmeticEffect =
        serde_json::from_value(value.clone()).map_err(|_| DurabilityError::InvalidStoredState)?;
    if effect.provenance != rules::PROVENANCE
        || effect.grants.len() > 2
        || effect.consumed != (plan.consume_source || !plan.extra_inputs.is_empty())
        || plan.recipe != format!("oteryn:item-cosmetic.i{}.v1", effect.source_item_id)
        || (!effect.consumed && (!effect.grants.is_empty() || effect.progress.is_some()))
    {
        return Err(DurabilityError::InvalidStoredState);
    }
    let account: sqlx::types::Uuid = sqlx::query_scalar(
        "SELECT account_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .fetch_one(&mut ***tx)
    .await?;
    for grant in &effect.grants {
        let occurrence: sqlx::types::Uuid = sqlx::query_scalar("SELECT game_character_uuid_v7()")
            .fetch_one(&mut ***tx)
            .await?;
        sqlx::query("INSERT INTO game_account_appearance_unlock_receipts \
            (grant_occurrence_id,account_id,unlock_key,content_revision,granting_character_id,source_kind,source_event_id) \
            VALUES($1,$2,$3,$4,encode($5,'hex')::uuid,'item_action',uuid_send($6::uuid))")
            .bind(occurrence).bind(account).bind(&grant.grant_key).bind(rules::PROVENANCE)
            .bind(fence.character_id.as_bytes().as_slice()).bind(transaction).execute(&mut ***tx).await?;
        sqlx::query("INSERT INTO game_account_appearance_unlocks(account_id,unlock_key,grant_occurrence_id) VALUES($1,$2,$3)")
            .bind(account).bind(&grant.grant_key).bind(occurrence).execute(&mut ***tx).await?;
    }
    if let Some(progress) = &effect.progress {
        if progress.after
            != progress
                .before
                .checked_add(1)
                .ok_or(DurabilityError::InvalidStoredState)?
        {
            return Err(DurabilityError::InvalidStoredState);
        }
        let changed = sqlx::query("INSERT INTO game_character_item_cosmetic_progress AS p \
            (character_id,component_key,contributions,last_transaction_id) \
            SELECT encode($1,'hex')::uuid,$2,$3,$4::uuid WHERE $5=0 \
            ON CONFLICT(character_id,component_key) DO UPDATE SET contributions=$3,last_transaction_id=$4::uuid WHERE p.contributions=$5")
            .bind(fence.character_id.as_bytes().as_slice()).bind(&progress.key).bind(i64::from(progress.after))
            .bind(transaction).bind(i64::from(progress.before)).execute(&mut ***tx).await?.rows_affected();
        if changed != 1 {
            // The INSERT's initial predicate must not suppress an existing-row CAS.
            let changed = sqlx::query("UPDATE game_character_item_cosmetic_progress SET contributions=$3,last_transaction_id=$4::uuid \
                WHERE character_id=encode($1,'hex')::uuid AND component_key=$2 AND contributions=$5")
                .bind(fence.character_id.as_bytes().as_slice()).bind(&progress.key).bind(i64::from(progress.after))
                .bind(transaction).bind(i64::from(progress.before)).execute(&mut ***tx).await?.rows_affected();
            if changed != 1 {
                return Err(DurabilityError::InvalidStoredState);
            }
        }
    }
    let event = sqlx::types::Uuid::parse_str(transaction)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let achievements: Vec<_> = effect.achievements.iter().map(String::as_str).collect();
    super::account_achievement::grant_missing(
        tx,
        fence.character_id,
        &achievements,
        "item_action",
        event.as_bytes(),
    )
    .await?;
    for progress in &effect.achievement_progress {
        super::account_achievement::add_progress(
            tx,
            fence.character_id,
            &progress.key,
            progress.target,
            "item_action",
            event.as_bytes(),
        )
        .await?;
    }
    Ok(())
}
