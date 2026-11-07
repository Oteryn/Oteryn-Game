//! S06 item-target mounts use the existing audited multi-input item transaction.
use super::super::character_appearance::{lock_account_grants, read_account_appearance_unlocks};
use super::super::consume_transform_cosmetics::Preparation;
use super::*;
use oteryn_gameplay_content::content::native_gameplay::mount_taming::{TamingRule, rules};
use oteryn_gameplay_domain::domain::appearance::{
    AccountUnlocks, AppearanceContent, EarnedCosmetic, EarnedCosmetics, UnlockFact,
};
use oteryn_gameplay_items::item_cosmetics::{GrantKind, PROVENANCE, ResolvedGrant};

/// A current, adjacent static machine resolved by the server's bundle map owner.
/// Private fields prevent a client source id from being used as target authority.
#[derive(Debug, Clone)]
pub struct MountKitMapTarget {
    source_id: u16,
    target: oteryn_protocol_oteryn::world_object::WorldObjectTarget,
    content_digest: [u8; 32],
    scope: crate::foundation::RuntimeScopeRefV1,
    generation: u64,
    session: crate::foundation::GameSessionId,
}

impl MountKitMapTarget {
    #[allow(
        clippy::too_many_arguments,
        reason = "live actor, physical placement and artifact are independent owner proofs"
    )]
    pub fn from_native_owner(
        runtime: &crate::foundation::ChannelRuntimeV1,
        actor: crate::foundation::ExactActorRef,
        session: crate::foundation::GameSessionId,
        position: crate::foundation::MovementLocalPosition,
        source_id: u16,
        target: oteryn_protocol_oteryn::world_object::WorldObjectTarget,
        content_digest: [u8; 32],
    ) -> Result<Self, &'static str> {
        if !matches!(source_id, 12319 | 12800) || target.placement.len() != 40 {
            return Err("unsupported mount machine");
        }
        let Some((current, at)) = runtime
            .positioned_player_for_session(session)
            .map_err(|_| "machine actor")?
        else {
            return Err("machine actor missing");
        };
        let at = at.position();
        if current != actor
            || at.floor != position.floor
            || at.x.abs_diff(position.x) > 1
            || at.y.abs_diff(position.y) > 1
        {
            return Err("machine is not adjacent");
        }
        Ok(Self {
            source_id,
            target,
            content_digest,
            session,
            scope: runtime.binding().runtime_scope(),
            generation: runtime.binding().scope_generation().get(),
        })
    }
}

pub(super) async fn prepare_carried(
    tx: &mut super::super::item_mint_audit::Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    request: &ConsumeTransformRequest,
    definition: &TypedDefinitionRef,
    native: &NativeGameplayState,
    map_target: Option<&MountKitMapTarget>,
) -> Result<Preparation, DurabilityError> {
    let Some(rule) = rules()
        .iter()
        .find(|r| r.item_key == definition.production_key && r.target_item_key.is_some())
    else {
        return Ok(Preparation::Unrelated);
    };
    let Some(content) = native
        .cosmetics()
        .filter(|c| c.source_digest() == native.source_digest())
    else {
        return Ok(Preparation::Refused(UseDisposition::NotPossible));
    };
    if content.earned_unlock(
        &rule.grant_key,
        PROVENANCE,
        EarnedCosmetic::Mount(&rule.mount_key),
    ) != UnlockFact::Compatible
    {
        return Ok(Preparation::Refused(UseDisposition::NotPossible));
    }
    if rule.self_use && (request.target.is_some() || map_target.is_some()) {
        return Ok(Preparation::Refused(UseDisposition::NothingToUse));
    }
    let mut extra_inputs = Vec::new();
    if !rule.self_use {
        if let Some(target) = map_target {
            let expected = format!("oteryn:item.tibia.i{}", target.source_id);
            let intent = oteryn_protocol_oteryn::world_object::decode_use_intent_target(
                &[4, 11, 15],
                &request.intent,
            )
            .map_err(|_| DurabilityError::InvalidStoredState)?;
            if request.target.is_some()
                || rule.consume_target
                || rule.target_item_key.as_deref() != Some(expected.as_str())
                || target.content_digest != native.source_digest()
                || target.scope != fence.runtime_scope
                || target.generation != fence.scope_ownership_generation.get()
                || target.session != fence.game_session_id
                || intent.use_with_map_item
                    != Some(oteryn_protocol_oteryn::world_object::MapUseTarget::Object(
                        target.target.clone(),
                    ))
            {
                return Ok(Preparation::Refused(UseDisposition::NothingToUse));
            }
        } else {
            let Some(target) = request.target.filter(|id| *id != request.source) else {
                return Ok(Preparation::Refused(UseDisposition::NothingToUse));
            };
            let Some(row) = carried_item(
                tx,
                fence,
                Some(target),
                rule.target_item_key
                    .as_deref()
                    .ok_or(DurabilityError::InvalidStoredState)?,
                native,
            )
            .await?
            else {
                return Ok(Preparation::Refused(UseDisposition::NothingToUse));
            };
            if rule.consume_target {
                extra_inputs.push(row);
            }
        }
    }
    for key in &rule.required_items {
        let Some(row) = carried_item(tx, fence, None, key, native).await? else {
            return Ok(Preparation::Refused(UseDisposition::RequirementNotMet));
        };
        extra_inputs.push(row);
    }
    let account: sqlx::types::Uuid = sqlx::query_scalar(
        "SELECT account_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid AND lifecycle=1",
    ).bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut ***tx).await?;
    lock_account_grants(tx, account).await?;
    let account_id = crate::domain::AccountId::from_bytes(*account.as_bytes())
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let facts = read_account_appearance_unlocks(tx, account_id, &content.earned_grant_keys())
        .await
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let facts: Vec<_> = facts
        .into_iter()
        .filter(|f| {
            matches!(
                f.source_kind.as_str(),
                "quest" | "npc" | "npc_wedding" | "mount_taming" | "item_action" | "task_board"
            )
        })
        .map(|f| (f.unlock_key, f.content_revision))
        .collect();
    if (EarnedCosmetics {
        content,
        facts: &facts,
    })
    .mount(&rule.mount_key)
        == UnlockFact::Compatible
    {
        return Ok(Preparation::Refused(UseDisposition::RequirementNotMet));
    }
    let draw = reward_bag_bounded_draw(
        *fence.game_session_id.as_bytes(),
        request.command.command_id().get(),
        request.source,
        41,
        100,
    )
    .ok_or(DurabilityError::InvalidStoredState)?;
    let outcome = if draw < u64::from(rule.chance) {
        0
    } else {
        let count =
            u64::try_from(rule.failures.len()).map_err(|_| DurabilityError::InvalidStoredState)?;
        let pick = reward_bag_bounded_draw(
            *fence.game_session_id.as_bytes(),
            request.command.command_id().get(),
            request.source,
            42,
            count,
        )
        .ok_or(DurabilityError::InvalidStoredState)?;
        u8::try_from(pick + 1).map_err(|_| DurabilityError::InvalidStoredState)?
    };
    let (consume, _, feedback) = rule
        .effects(outcome)
        .ok_or(DurabilityError::InvalidStoredState)?;
    if outcome != 0 {
        extra_inputs.clear();
    }
    #[cfg(debug_assertions)]
    if rule.item_key == "oteryn:item.tibia.i21897" {
        eprintln!(
            "srv7_glider stage=plan_ready extras={} consume={consume}",
            extra_inputs.len()
        );
    }
    let mut plan = plan(rule, outcome == 0, consume, feedback, extra_inputs)?;
    if let Some(target) = map_target {
        plan.effects["mount_machine"] = json!({"source_id":target.source_id,
            "placement":target.target.placement, "expected_revision":target.target.expected_revision});
    }
    Ok(Preparation::Ready(plan))
}

fn plan(
    rule: &TamingRule,
    success: bool,
    consume: bool,
    feedback: &str,
    extra_inputs: Vec<ExtraInput>,
) -> Result<ActionPlan, DurabilityError> {
    let id = rule
        .item_key
        .strip_prefix("oteryn:item.tibia.i")
        .and_then(|id| id.parse::<u32>().ok())
        .ok_or(DurabilityError::InvalidStoredState)?;
    let grants: Vec<_> = if success {
        vec![ResolvedGrant {
            grant_key: rule.grant_key.clone(),
            cosmetic_key: rule.mount_key.clone(),
            kind: GrantKind::Mount,
            addon: 0,
        }]
    } else {
        Vec::new()
    };
    let achievements: Vec<_> = if success {
        rule.achievement_key
            .iter()
            .map(String::as_str)
            .chain(std::iter::once("oteryn:achievement/natural_born_cowboy"))
            .collect()
    } else {
        Vec::new()
    };
    Ok(ActionPlan {
        recipe: format!("oteryn:item-cosmetic.i{id}.v1"),
        outputs: Vec::new(),
        consume_source: consume,
        source_count: u32::from(consume),
        extra_inputs,
        effects: json!({"kind":"cosmetic", "source_item_id":id, "donor":rule.source,
            "cues":[], "speech":null, "require_head":false, "require_player_target":false,
            "cosmetic":{"source_item_id":id, "provenance":PROVENANCE, "consumed":consume,
                "grants":grants, "progress":null, "achievements":achievements,
                "achievement_progress":[], "feedback":feedback}}),
    })
}

pub(super) async fn carried_item(
    tx: &mut super::super::item_mint_audit::Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    exact: Option<[u8; 16]>,
    key: &str,
    native: &NativeGameplayState,
) -> Result<Option<ExtraInput>, DurabilityError> {
    let Some(policy) = native.item_policy(key, "definition-r1") else {
        return Ok(None);
    };
    let row = sqlx::query(
        "SELECT uuid_send(i.item_instance_id) AS item,to_jsonb(i) AS before_item,uuid_send(coalesce(e.parent_item_instance_id,n.parent_item_instance_id)) AS parent,coalesce(e.placement_ordinal,n.placement_ordinal)::text AS ordinal,CASE WHEN n.item_instance_id IS NULL THEN 1 ELSE 4 END AS source_kind FROM game_item_instances i LEFT JOIN game_item_container_entries e ON e.item_instance_id=i.item_instance_id AND e.character_id=encode($1,'hex')::uuid LEFT JOIN game_item_container_slots b ON b.character_id=e.character_id AND b.item_instance_id=e.parent_item_instance_id LEFT JOIN game_item_nested_entries n ON n.item_instance_id=i.item_instance_id LEFT JOIN LATERAL game_item_loop_owner(n.parent_item_instance_id) owner ON n.item_instance_id IS NOT NULL WHERE i.definition_family='Item' AND i.definition_production_key=$2 AND i.definition_revision_ref=$3 AND i.world_id=encode($4,'hex')::uuid AND i.lifecycle=1 AND i.quantity>0 AND ($5::bytea IS NULL OR i.item_instance_id=encode($5,'hex')::uuid) AND ((e.world_id=i.world_id AND b.world_id=i.world_id) OR (n.world_id=i.world_id AND owner.owner_character=encode($1,'hex')::uuid AND owner.depth BETWEEN 0 AND 7)) ORDER BY i.item_instance_id LIMIT 1 FOR UPDATE OF i",
    ).bind(fence.character_id.as_bytes().as_slice()).bind(key)
        .bind(&policy.record().production_definition.revision_ref)
        .bind(fence.runtime_scope.world_id().as_bytes().as_slice()).bind(exact.map(|id| id.to_vec()))
        .fetch_optional(&mut ***tx).await?;
    row.map(|row| {
        Ok(ExtraInput {
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
        })
    })
    .transpose()
}
