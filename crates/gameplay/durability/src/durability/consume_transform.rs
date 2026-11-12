//! W19: fenced, receipt-keyed consume/transform and donor-table event rewards.
use super::character_authority::ReconciledCharacterAuthority;
use super::character_equipment::assert_current_equipment_in_transaction;
use super::db::begin_type2_transaction;
use super::item_loop_move::{ItemLoopContent, carried_weight};
use super::item_mint::TypedDefinitionRef;
use super::item_transfer::{
    CurrentCharacterItemFence, read_derived_transfer_capacity_centi_oz, scope_of, stack_maximum,
};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use crate::foundation::CommandRef;
use oteryn_gameplay_content::content::native_gameplay::NativeGameplayState;
use oteryn_gameplay_items::reward_bag::reward_bag_bounded_draw;
use oteryn_gameplay_items::rust_lottery::{lottery_conversion, rust_conversion};
use oteryn_protocol_oteryn::world_object::UseDisposition;
use serde_json::{Value, json};
use sqlx::Row;

use super::consume_transform_audit as audit;
mod cake;
pub use cake::CakeBakerContext;
mod dispenser;
#[path = "consume_transform_ground.rs"]
mod ground;
#[path = "consume_transform_planting.rs"]
mod planting;
pub use ground::{GroundActionContext, GroundActionSource};
#[path = "consume_transform_house.rs"]
mod house;
pub use house::{HouseActionContext, HouseDecorationBaseItem};

#[derive(Debug, Clone)]
pub struct ConsumeTransformRequest {
    pub command: CommandRef,
    pub source: [u8; 16],
    pub target: Option<[u8; 16]>,
    /// Original bounded USE bytes. Replay compares these before resolving current handles.
    pub intent: Vec<u8>,
}

/// Immutable semantic outcome of the same consume/transform owner. `fresh` is true
/// only for the writer that committed this receipt; recovery returns its stored effects.
#[derive(Debug, Clone)]
pub struct ConsumeTransformResult {
    pub disposition: UseDisposition,
    pub fresh: bool,
    pub effects: Value,
    pub recipe: Option<String>,
}

impl ConsumeTransformResult {
    fn refused(disposition: UseDisposition) -> Self {
        Self {
            disposition,
            fresh: false,
            effects: json!({}),
            recipe: None,
        }
    }
}

/// One source-authored action plan, shared by rewards, novelty and cosmetic helpers.
#[derive(Debug)]
pub(super) struct ActionPlan {
    pub recipe: String,
    pub outputs: Vec<(TypedDefinitionRef, u32)>,
    pub consume_source: bool,
    pub source_count: u32,
    pub effects: Value,
    pub extra_inputs: Vec<ExtraInput>,
}

#[derive(Debug)]
pub(super) struct ExtraInput {
    pub item: [u8; 16],
    pub before: Value,
    pub parent: [u8; 16],
    pub ordinal: String,
    pub count: u32,
    pub source_kind: i32,
}

#[derive(Clone, Copy, Default)]
enum ActionContext<'a> {
    #[default]
    Carried,
    CakeBaker(&'a CakeBakerContext),
    Ground(&'a GroundActionContext),
    DispenserTarget(&'a GroundActionContext),
    DispenserHouseTarget(&'a HouseActionContext),
}

// Borrow the existing transaction fences; the Ground capacity owner independently
// revalidates them in this same physical transaction before inserting a new root.
struct ActionGroundFences<'a> {
    permit: &'a super::spell_owner_commit::SpellLanePermit,
    recovery: &'a crate::character_recovery_fence::CharacterRecoveryFenceV1,
    node: &'a NodeIncarnationProof,
}

impl DurabilityRoot {
    /// Compatibility surface for existing rust, lottery and reward callers.
    #[allow(clippy::too_many_arguments)]
    pub async fn consume_transform<C: ItemLoopContent>(
        &self,
        permit: &super::spell_owner_commit::SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: ConsumeTransformRequest,
        native: &NativeGameplayState,
        content: &C,
        probe: bool,
    ) -> Result<Option<UseDisposition>, DurabilityError> {
        Ok(self
            .consume_transform_detailed(
                permit, recovery, node, fence, request, native, content, probe,
            )
            .await?
            .map(|result| result.disposition))
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn consume_transform_detailed<C: ItemLoopContent>(
        &self,
        permit: &super::spell_owner_commit::SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: ConsumeTransformRequest,
        native: &NativeGameplayState,
        content: &C,
        probe: bool,
    ) -> Result<Option<ConsumeTransformResult>, DurabilityError> {
        self.consume_transform_with_cosmetic_premium(
            permit, recovery, node, fence, request, native, content, probe, None,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn consume_transform_with_cosmetic_premium<C: ItemLoopContent>(
        &self,
        permit: &super::spell_owner_commit::SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: ConsumeTransformRequest,
        native: &NativeGameplayState,
        content: &C,
        probe: bool,
        premium: Option<&dyn super::consume_transform_cosmetics::ItemCosmeticPremium>,
    ) -> Result<Option<ConsumeTransformResult>, DurabilityError> {
        self.consume_transform_inner(
            permit,
            recovery,
            node,
            fence,
            request,
            native,
            content,
            probe,
            premium,
            ActionContext::Carried,
        )
        .await
    }

    /// Probe or commit an action. Replay precedes live handle and custody resolution;
    /// an unknown acknowledgement cannot select a second outcome or repeat its grant.
    #[allow(clippy::too_many_arguments)]
    async fn consume_transform_inner<C: ItemLoopContent>(
        &self,
        permit: &super::spell_owner_commit::SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: ConsumeTransformRequest,
        native: &NativeGameplayState,
        content: &C,
        probe: bool,
        premium: Option<&dyn super::consume_transform_cosmetics::ItemCosmeticPremium>,
        ground: ActionContext<'_>,
    ) -> Result<Option<ConsumeTransformResult>, DurabilityError> {
        if request.command.game_session_id() != fence.game_session_id
            || request.intent.is_empty()
            || request.intent.len() > oteryn_protocol_oteryn::world_object::MAX_USE_INTENT_BYTES
            || content.loop_content_digest() != native.source_digest()
        {
            return Ok(Some(ConsumeTransformResult::refused(
                UseDisposition::Rejected,
            )));
        }
        if !matches!(ground, ActionContext::DispenserHouseTarget(_)) {
            permit.check_scope(fence.runtime_scope)?;
        }
        let record = recovery
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        let mut context = (native, content, premium, ground, permit);
        self.try_issue_semantic_pass()?.run_with_context(&mut context, move |holder, deadline, context| Box::pin(async move {
            let mut tx = begin_type2_transaction(holder, deadline).await?;
            super::character_authority::assert_recovery_fence(&mut tx, &record).await?;
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:consume-transform:' || encode($1,'hex') || ':' || $2,0))")
                .bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string())
                .execute(&mut **tx).await?;
            if matches!(context.3, ActionContext::CakeBaker(_)) &&
                assert_current_equipment_in_transaction(&mut tx,&record,&node,&fence,Some(request.command),context.1.loop_content_digest()).await.is_err() {
                return Ok(Some(ConsumeTransformResult::refused(UseDisposition::Rejected)));
            }
            let previous: Option<(Vec<u8>, Vec<u8>, Value, String)> = sqlx::query_as("SELECT intent,uuid_send(character_id),effects,recipe FROM game_item_consume_transform_records WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric")
                .bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string())
                .fetch_optional(&mut **tx).await?;
            if let Some((intent, character, effects, recipe)) = previous {
                let caller_matches = match context.3 {
                    ActionContext::CakeBaker(caller) => recipe==cake::RECIPE
                        && cake::is_baker(&caller.npc_key)
                        && effects.get("npc_key").and_then(Value::as_str)==Some(caller.npc_key.as_str())
                        && effects.get("reset_epoch").and_then(Value::as_str)==Some(caller.expected_epoch.to_string().as_str()),
                    _ => recipe!=cake::RECIPE,
                };
                let result = if caller_matches && intent == request.intent && character == fence.character_id.as_bytes() {
                    ConsumeTransformResult { disposition: UseDisposition::Committed, fresh: false, effects, recipe: Some(recipe.clone()) }
                } else { ConsumeTransformResult { recipe: Some(recipe), ..ConsumeTransformResult::refused(UseDisposition::Rejected) } };
                tx.commit(deadline).await?;
                return Ok(Some(result));
            }
            if probe { tx.commit(deadline).await?; return Ok(None); }
            if let ActionContext::DispenserHouseTarget(house) = context.3 {
                super::db::lock_admission_relations(&mut tx).await?;
                if !house.dispenser_current(&mut tx, &node).await? {
                    return Ok(Some(ConsumeTransformResult::refused(UseDisposition::StaleState)));
                }
            } else if assert_current_equipment_in_transaction(&mut tx, &record, &node, &fence, Some(request.command), context.1.loop_content_digest()).await.is_err() {
                return Ok(Some(ConsumeTransformResult::refused(UseDisposition::Rejected)));
            }
            let ground_fences = ActionGroundFences { permit: context.4, recovery: &record, node: &node };
            let result = write(&mut tx, fence, &request, context.0, context.1, context.2, context.3, &ground_fences).await?;
            if result.disposition == UseDisposition::Committed { tx.commit(deadline).await?; }
            Ok(Some(result))
        })).await
    }
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
async fn write<C: ItemLoopContent>(
    tx: &mut super::item_mint_audit::Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    request: &ConsumeTransformRequest,
    native: &NativeGameplayState,
    content: &C,
    premium: Option<&dyn super::consume_transform_cosmetics::ItemCosmeticPremium>,
    ground: ActionContext<'_>,
    ground_fences: &ActionGroundFences<'_>,
) -> Result<ConsumeTransformResult, DurabilityError> {
    if let ActionContext::Ground(context) = ground {
        return ground::write(
            ground_fences.permit,
            tx,
            fence,
            request,
            native,
            content,
            context,
            premium,
            ground_fences,
        )
        .await;
    }
    // Named-House dispenser actions have already proved their current House fence;
    // they do not write Channel Ground and cannot borrow a Channel lane proof.
    let permit =
        (!matches!(ground, ActionContext::DispenserHouseTarget(_))).then_some(ground_fences.permit);
    let world = fence.runtime_scope.world_id();
    // The full Character fence/root lock precedes item locks. The only equipment
    // source admitted here is the unchanged party hat in the actual head slot.
    let source = sqlx::query("SELECT to_jsonb(i) AS before_item,i.definition_family,i.definition_production_key,i.definition_revision_ref,i.quantity,uuid_send(coalesce(e.parent_item_instance_id,n.parent_item_instance_id)) AS parent,coalesce(e.placement_ordinal,n.placement_ordinal,h.slot,1)::text AS ordinal,CASE WHEN h.item_instance_id IS NOT NULL THEN 2 WHEN n.item_instance_id IS NOT NULL THEN 4 ELSE 1 END AS custody,CASE WHEN h.item_instance_id IS NOT NULL THEN to_jsonb(h) WHEN n.item_instance_id IS NOT NULL THEN to_jsonb(n) ELSE to_jsonb(e) END AS source_location FROM game_item_instances i LEFT JOIN game_item_container_entries e ON e.item_instance_id=i.item_instance_id AND e.character_id=encode($2,'hex')::uuid LEFT JOIN game_item_container_slots b ON b.character_id=e.character_id AND b.item_instance_id=e.parent_item_instance_id LEFT JOIN game_item_nested_entries n ON n.item_instance_id=i.item_instance_id LEFT JOIN LATERAL game_item_loop_owner(n.parent_item_instance_id) owner ON n.item_instance_id IS NOT NULL LEFT JOIN game_character_equipment_slots h ON h.item_instance_id=i.item_instance_id AND h.character_id=encode($2,'hex')::uuid AND (h.slot=1 OR $4) AND h.world_id=i.world_id WHERE i.item_instance_id=encode($1,'hex')::uuid AND i.world_id=encode($3,'hex')::uuid AND i.lifecycle=1 AND ((e.world_id=i.world_id AND b.world_id=i.world_id) OR h.item_instance_id IS NOT NULL OR (n.world_id=i.world_id AND owner.owner_character=encode($2,'hex')::uuid AND owner.depth BETWEEN 0 AND 7)) FOR UPDATE OF i")
        .bind(request.source.as_slice()).bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).bind(matches!(ground, ActionContext::CakeBaker(_))).fetch_optional(&mut ***tx).await?;
    let Some(source) = source else {
        return Ok(ConsumeTransformResult::refused(UseDisposition::StaleState));
    };
    let definition = TypedDefinitionRef {
        family: source.try_get("definition_family")?,
        production_key: source.try_get("definition_production_key")?,
        revision_ref: source.try_get("definition_revision_ref")?,
    };
    // Seed use keeps its exact operand in the already proven extra-input slot, so
    // main and nested leaves share the same custody proof without widening rust.
    let cake_plan = if let ActionContext::CakeBaker(context) = ground {
        match cake::prepare(
            tx,
            fence,
            request,
            &definition,
            source.try_get("quantity")?,
            context,
            source.try_get::<i32, _>("custody")? == 2,
        )
        .await?
        {
            Some(plan) => Some(plan),
            None => {
                return Ok(ConsumeTransformResult::refused(
                    UseDisposition::NothingToUse,
                ));
            }
        }
    } else {
        None
    };
    let planting_plan = if cake_plan.is_some() {
        cake_plan
    } else {
        match dispenser::prepare(
            tx,
            fence,
            request,
            &definition,
            source.try_get("quantity")?,
            native,
            ground,
            ground_fences.node,
        )
        .await?
        {
            Some(plan) => Some(plan),
            None if matches!(
                ground,
                ActionContext::DispenserTarget(_) | ActionContext::DispenserHouseTarget(_)
            ) =>
            {
                return Ok(ConsumeTransformResult::refused(
                    UseDisposition::NothingToUse,
                ));
            }
            None => planting::prepare_carried(tx, fence, request, &definition, native).await?,
        }
    };
    let mut planting_request = request.clone();
    if planting_plan.is_some() {
        planting_request.target = None;
    }
    let request = if planting_plan.is_some() {
        &planting_request
    } else {
        request
    };
    let secondary = if let Some(target_id) = request.target {
        sqlx::query("SELECT to_jsonb(i) AS before_item,i.definition_family,i.definition_production_key,i.definition_revision_ref,i.quantity,uuid_send(e.parent_item_instance_id) AS parent,e.placement_ordinal::text AS ordinal FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) JOIN game_item_container_slots b ON b.character_id=e.character_id AND b.item_instance_id=e.parent_item_instance_id WHERE i.item_instance_id=encode($1,'hex')::uuid AND e.character_id=encode($2,'hex')::uuid AND i.world_id=encode($3,'hex')::uuid AND i.lifecycle=1 AND e.world_id=i.world_id AND b.world_id=i.world_id FOR UPDATE OF i")
            .bind(target_id.as_slice()).bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).fetch_optional(&mut ***tx).await?
    } else {
        None
    };
    if request.target.is_some() && secondary.is_none() {
        return Ok(ConsumeTransformResult::refused(UseDisposition::StaleState));
    }
    let session = *fence.game_session_id.as_bytes();
    let command = request.command.command_id().get();
    let draw = if secondary.is_some() {
        let chance = reward_bag_bounded_draw(session, command, request.source, 1, 10_000)
            .ok_or(DurabilityError::InvalidStoredState)?;
        // LCM(1..=7): reducing this independent draw by any eligible count stays uniform.
        let selection = reward_bag_bounded_draw(session, command, request.source, 2, 420)
            .ok_or(DurabilityError::InvalidStoredState)?;
        selection * 10_000 + chance
    } else {
        reward_bag_bounded_draw(session, command, request.source, 3, 98)
            .ok_or(DurabilityError::InvalidStoredState)?
    };
    let conversion = if let Some(tool) = &secondary {
        let tool_definition = TypedDefinitionRef {
            family: tool.try_get("definition_family")?,
            production_key: tool.try_get("definition_production_key")?,
            revision_ref: tool.try_get("definition_revision_ref")?,
        };
        oteryn_gameplay_items::event_planting::seed_conversion(
            native,
            &tool_definition,
            &definition,
        )
        .or_else(|| rust_conversion(native, &tool_definition, &definition, draw))
    } else {
        lottery_conversion(native, &definition, draw).or_else(|| {
            oteryn_gameplay_items::event_items::conversion(
                native,
                &definition,
                session,
                command,
                request.source,
            )
        })
    };
    let mut conversion = if let Some(planting) = planting_plan {
        planting
    } else if let Some(conversion) = conversion {
        let effects = if conversion.recipe == "oteryn:planting.ice-flower" {
            oteryn_gameplay_items::event_planting::seed_effects(None)
        } else {
            oteryn_gameplay_items::event_items::effects(&definition.production_key, None)
                .map(serde_json::to_value)
                .transpose()
                .map_err(|_| DurabilityError::InvalidStoredState)?
                .unwrap_or_else(|| json!({}))
        };
        ActionPlan {
            source_count:
                if oteryn_gameplay_items::reward_bag::event_containers::consumes_whole_stack(
                    &conversion.recipe,
                ) {
                    u32::try_from(source.try_get::<i64, _>("quantity")?)
                        .map_err(|_| DurabilityError::InvalidStoredState)?
                } else {
                    1
                },
            recipe: conversion.recipe,
            outputs: conversion.outputs,
            consume_source: true,
            effects,
            extra_inputs: Vec::new(),
        }
    } else if secondary.is_none() {
        if let Some(plan) = oteryn_gameplay_items::novelty_items::plan(
            native,
            &definition,
            session,
            command,
            request.source,
        ) {
            ActionPlan {
                recipe: plan.recipe,
                outputs: plan.outputs,
                consume_source: plan.consume_source,
                source_count: u32::from(plan.consume_source),
                effects: plan.effects,
                extra_inputs: Vec::new(),
            }
        } else {
            use super::consume_transform_cosmetics::{self as cosmetics, Preparation};
            match cosmetics::prepare(tx, fence, request, &definition, native, premium).await? {
                Preparation::Ready(plan) => plan,
                Preparation::Refused(disposition) => {
                    return Ok(ConsumeTransformResult::refused(disposition));
                }
                Preparation::Unrelated => {
                    return Ok(ConsumeTransformResult::refused(
                        UseDisposition::NothingToUse,
                    ));
                }
            }
        }
    } else {
        return Ok(ConsumeTransformResult::refused(
            UseDisposition::NothingToUse,
        ));
    };
    let source_kind: i32 = source.try_get("custody")?;
    let require_head = conversion
        .effects
        .get("require_head")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if conversion.recipe != cake::RECIPE
        && (require_head != (source_kind == 2) || (source_kind == 2 && conversion.consume_source))
    {
        return Ok(ConsumeTransformResult::refused(UseDisposition::NotPossible));
    }
    let before: Value = source.try_get("before_item")?;
    let source_ordinal: String = source.try_get("ordinal")?;
    let source_location: Value = source.try_get("source_location")?;
    let parent: Option<Vec<u8>> = source.try_get("parent")?;
    if conversion.recipe == cake::RECIPE {
        let capacity = if let Some(parent) = &parent {
            let bag = sqlx::query("SELECT definition_family,definition_production_key,definition_revision_ref FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid AND lifecycle=1 FOR UPDATE")
                .bind(parent).fetch_one(&mut ***tx).await?;
            let definition = TypedDefinitionRef {
                family: bag.try_get("definition_family")?,
                production_key: bag.try_get("definition_production_key")?,
                revision_ref: bag.try_get("definition_revision_ref")?,
            };
            content
                .loop_facts(&definition)
                .and_then(|facts| facts.container_capacity)
                .ok_or(DurabilityError::InvalidStoredState)?
        } else {
            0
        };
        return persist_action(
            permit,
            tx,
            fence,
            request,
            native,
            &conversion,
            &before,
            None,
            parent.as_deref(),
            source_kind,
            &source_ordinal,
            capacity,
            &[],
            &source_location,
            None,
        )
        .await;
    }
    if !conversion.consume_source {
        if !conversion.outputs.is_empty()
            || secondary.is_some()
            || (!conversion.extra_inputs.is_empty()
                && conversion.recipe != "oteryn:planting.preserve")
        {
            return Err(DurabilityError::InvalidStoredState);
        }
        return persist_action(
            permit,
            tx,
            fence,
            request,
            native,
            &conversion,
            &before,
            None,
            parent.as_deref(),
            source_kind,
            &source_ordinal,
            20,
            &[],
            &source_location,
            None,
        )
        .await;
    }
    let parent = parent.ok_or(DurabilityError::InvalidStoredState)?;
    let event_reward = conversion.recipe.starts_with("oteryn:event-container.");
    if event_reward {
        // A donor count can denote several nonstackable decorations. Each gets a fresh
        // identity and slot; never mint an illegal quantity on a nonstackable instance.
        let mut split = Vec::new();
        for (definition, count) in conversion.outputs {
            let maximum = content
                .loop_facts(&definition)
                .and_then(|facts| stack_maximum(facts.stack).ok().map(|(maximum, _)| maximum))
                .unwrap_or(0);
            if maximum == 0 {
                return Ok(ConsumeTransformResult::refused(UseDisposition::NotPossible));
            }
            let mut remaining = count;
            while remaining > 0 {
                let quantity = remaining.min(maximum);
                split.push((definition.clone(), quantity));
                remaining -= quantity;
            }
        }
        conversion.outputs = split;
    }
    let quantity: i64 = source.try_get("quantity")?;
    let additional_freed = conversion
        .extra_inputs
        .iter()
        .filter(|input| {
            input.parent.as_slice() == parent.as_slice()
                && input.before.get("quantity").and_then(Value::as_i64)
                    == Some(i64::from(input.count))
        })
        .count();
    let additional_freed =
        i64::try_from(additional_freed).map_err(|_| DurabilityError::InvalidStoredState)?;
    let occupied: i64 = sqlx::query_scalar("SELECT (SELECT count(*) FROM game_item_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid)+(SELECT count(*) FROM game_item_nested_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid)").bind(&parent).fetch_one(&mut ***tx).await?;
    let tree_count: i64 =
        sqlx::query_scalar("SELECT game_item_loop_tree_size(encode($1,'hex')::uuid)")
            .bind(fence.character_id.as_bytes().as_slice())
            .fetch_one(&mut ***tx)
            .await?;
    if tree_count
        + i64::try_from(conversion.outputs.len())
            .map_err(|_| DurabilityError::InvalidStoredState)?
        - i64::from(quantity == i64::from(conversion.source_count))
        - additional_freed
        > 500
    {
        return Ok(ConsumeTransformResult::refused(UseDisposition::NoRoom));
    }
    let bag = sqlx::query("SELECT definition_family,definition_production_key,definition_revision_ref FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid AND lifecycle=1 FOR UPDATE")
        .bind(&parent).fetch_one(&mut ***tx).await?;
    let bag_definition = TypedDefinitionRef {
        family: bag.try_get("definition_family")?,
        production_key: bag.try_get("definition_production_key")?,
        revision_ref: bag.try_get("definition_revision_ref")?,
    };
    let authored_capacity = content
        .loop_facts(&bag_definition)
        .and_then(|f| f.container_capacity)
        .unwrap_or(0);
    let capacity = if event_reward
        || conversion.recipe == cake::RECIPE
        || conversion.recipe.starts_with("oteryn:novelty-item.")
        || conversion.recipe.starts_with("oteryn:item-cosmetic.")
    {
        authored_capacity
    } else {
        authored_capacity.min(20)
    };
    if occupied
        + i64::try_from(conversion.outputs.len())
            .map_err(|_| DurabilityError::InvalidStoredState)?
        - i64::from(quantity == i64::from(conversion.source_count))
        - additional_freed
        > i64::from(capacity)
    {
        return Ok(ConsumeTransformResult::refused(UseDisposition::NoRoom));
    }
    // Whole carried weight and capacity use the same locked projection as the item loop.
    let carried = match carried_weight(tx, fence.character_id, *world.as_bytes(), content).await {
        Ok(w) => w,
        Err(_) => return Ok(ConsumeTransformResult::refused(UseDisposition::NotPossible)),
    };
    let Some(used_weight) = content.loop_facts(&definition).and_then(|f| f.weight) else {
        return Ok(ConsumeTransformResult::refused(UseDisposition::NotPossible));
    };
    let mut output_weight = 0_u64;
    for (definition, count) in &conversion.outputs {
        let Some(facts) = content.loop_facts(definition) else {
            return Ok(ConsumeTransformResult::refused(UseDisposition::NotPossible));
        };
        let Some(weight) = facts.weight else {
            return Ok(ConsumeTransformResult::refused(UseDisposition::NotPossible));
        };
        if (facts.container_capacity.is_some() && !(event_reward && *count == 1))
            || *count
                > stack_maximum(facts.stack)
                    .map(|(maximum, _)| maximum)
                    .unwrap_or(0)
        {
            return Ok(ConsumeTransformResult::refused(UseDisposition::NotPossible));
        }
        output_weight += u64::from(weight) * u64::from(*count);
    }
    let secondary_weight = if let Some(tool) = &secondary {
        let tool_definition = TypedDefinitionRef {
            family: tool.try_get("definition_family")?,
            production_key: tool.try_get("definition_production_key")?,
            revision_ref: tool.try_get("definition_revision_ref")?,
        };
        let Some(weight) = content.loop_facts(&tool_definition).and_then(|f| f.weight) else {
            return Ok(ConsumeTransformResult::refused(UseDisposition::NotPossible));
        };
        u64::from(weight)
    } else {
        0
    };
    let mut extra_weight = 0_u64;
    // A depleted placed dispenser is retired atomically, but its Ground weight
    // was never part of the carried projection and cannot free carrying capacity.
    for input in conversion
        .extra_inputs
        .iter()
        .filter(|input| matches!(input.source_kind, 1 | 4))
    {
        let definition = TypedDefinitionRef {
            family: input
                .before
                .get("definition_family")
                .and_then(Value::as_str)
                .ok_or(DurabilityError::InvalidStoredState)?
                .to_owned(),
            production_key: input
                .before
                .get("definition_production_key")
                .and_then(Value::as_str)
                .ok_or(DurabilityError::InvalidStoredState)?
                .to_owned(),
            revision_ref: input
                .before
                .get("definition_revision_ref")
                .and_then(Value::as_str)
                .ok_or(DurabilityError::InvalidStoredState)?
                .to_owned(),
        };
        let Some(weight) = content
            .loop_facts(&definition)
            .and_then(|facts| facts.weight)
        else {
            return Ok(ConsumeTransformResult::refused(UseDisposition::NotPossible));
        };
        extra_weight = extra_weight
            .checked_add(u64::from(weight) * u64::from(input.count))
            .ok_or(DurabilityError::InvalidStoredState)?;
    }
    let parent_capacity = capacity;
    let capacity = read_derived_transfer_capacity_centi_oz(tx, fence.character_id, content).await?;
    if carried
        .checked_sub(u64::from(used_weight) * u64::from(conversion.source_count))
        .and_then(|w| w.checked_sub(secondary_weight))
        .and_then(|w| w.checked_sub(extra_weight))
        .and_then(|w| w.checked_add(output_weight))
        .is_none_or(|w| w > capacity)
    {
        return Ok(ConsumeTransformResult::refused(UseDisposition::NotPossible));
    }
    let highest: String = sqlx::query_scalar("SELECT coalesce(max(placement_ordinal),0)::text FROM (SELECT placement_ordinal FROM game_item_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid UNION ALL SELECT placement_ordinal FROM game_item_nested_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid) entries").bind(&parent).fetch_one(&mut ***tx).await?;
    let mut ordinal: u64 = highest
        .parse()
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let mut outputs = Vec::new();
    for (position, (definition, count)) in conversion.outputs.iter().enumerate() {
        let output_ordinal = if quantity == i64::from(conversion.source_count) && position == 0 {
            source.try_get::<String, _>("ordinal")?
        } else {
            ordinal = ordinal
                .checked_add(1)
                .ok_or(DurabilityError::InvalidStoredState)?;
            ordinal.to_string()
        };
        let item: String = sqlx::query_scalar("SELECT game_character_uuid_v7()::text")
            .fetch_one(&mut ***tx)
            .await?;
        let decay = if let Some(expected) = conversion
            .effects
            .get("timed_output")
            .filter(|value| !value.is_null())
        {
            let policy = native
                .item_policy(&definition.production_key, &definition.revision_ref)
                .ok_or(DurabilityError::InvalidStoredState)?;
            let qualified =
                super::spell_items_abi::QualifiedItemDefinition::from_native_policy(policy)
                    .map_err(|_| DurabilityError::InvalidStoredState)?;
            let decay = qualified.decay.ok_or(DurabilityError::InvalidStoredState)?;
            let expected_target = expected
                .get("decay_to")
                .and_then(Value::as_u64)
                .map(|id| format!("oteryn:item.tibia.i{id}"));
            if *count != 1
                || qualified.stack_maximum != 1
                || qualified.container_capacity.is_some()
                || expected.get("duration_ms").and_then(Value::as_u64)
                    != Some(u64::from(decay.duration_millis))
                || decay.target.as_ref().map(|target| &target.production_key)
                    != expected_target.as_ref()
            {
                return Ok(ConsumeTransformResult::refused(UseDisposition::NotPossible));
            }
            Some(
                json!({"duration_millis": decay.duration_millis, "target": decay.target.as_ref().map(|target|
                json!({"family": target.family, "key": target.production_key, "revision": target.revision_ref}))}),
            )
        } else {
            None
        };
        let mut output = json!({"item": item, "key": definition.production_key, "revision": definition.revision_ref,
            "count": count, "ordinal": output_ordinal, "decay": decay});
        if conversion.recipe == "oteryn:planting.ice-flower" {
            super::spell_item_temporal::event_items::decorate_output(native, &mut output)?;
        }
        outputs.push(output);
    }
    let secondary_before = secondary
        .as_ref()
        .map(|row| row.try_get::<Value, _>("before_item"))
        .transpose()?;
    persist_action(
        permit,
        tx,
        fence,
        request,
        native,
        &conversion,
        &before,
        secondary_before.as_ref(),
        Some(&parent),
        source_kind,
        &source_ordinal,
        parent_capacity,
        &outputs,
        &source_location,
        None,
    )
    .await
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
async fn persist_action(
    permit: Option<&super::spell_owner_commit::SpellLanePermit>,
    tx: &mut super::item_mint_audit::Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    request: &ConsumeTransformRequest,
    native: &NativeGameplayState,
    plan: &ActionPlan,
    before: &Value,
    secondary_before: Option<&Value>,
    parent: Option<&[u8]>,
    source_kind: i32,
    source_ordinal: &str,
    parent_capacity: u32,
    outputs: &[Value],
    source_location: &Value,
    ground_admission: Option<(&ActionGroundFences<'_>, &GroundActionContext)>,
) -> Result<ConsumeTransformResult, DurabilityError> {
    // Named-House transforms do not touch key-33 tables; every Ground path must
    // retain and validate its caller's lane proof before writing the receipt or items.
    if source_kind == 3 && permit.is_none() {
        return Err(DurabilityError::InvalidStoredState);
    }
    if let Some(permit) = permit {
        permit.check_scope(fence.runtime_scope)?;
    }
    let ground_reward = source_kind == 3
        && parent.is_none()
        && plan.effects.get("kind").and_then(Value::as_str) == Some("reward");
    if ground_admission.is_some() != ground_reward {
        return Err(DurabilityError::InvalidStoredState);
    }
    if if plan.recipe == cake::RECIPE {
        !cake::valid_shape(plan)
    } else {
        plan.consume_source != (plan.source_count > 0)
            || plan.source_count > 100
            || plan.extra_inputs.len() > 8
            || plan.extra_inputs.iter().any(|input| {
                input.count == 0 || input.count != 1 || !matches!(input.source_kind, 1 | 3 | 4 | 5)
            })
    } {
        return Err(DurabilityError::InvalidStoredState);
    }
    let world = fence.runtime_scope.world_id();
    let (transaction, event, occurred): (String, String, i64) = sqlx::query_as("SELECT game_character_uuid_v7()::text,game_character_uuid_v7()::text,(extract(epoch FROM clock_timestamp())*1000)::bigint").fetch_one(&mut ***tx).await?;
    let tuple = tx.tuple();
    let extra_inputs = plan.extra_inputs.iter().map(|input| json!({
        "item": uuid_text(input.item), "before": input.before,
        "parent": uuid_text(input.parent), "ordinal": input.ordinal, "count": input.count, "source_kind": input.source_kind,
    })).collect::<Vec<_>>();
    let proof = audit::action_proof(
        fence,
        request,
        &plan.recipe,
        native.source_digest(),
        before,
        secondary_before,
        outputs,
        plan.consume_source,
        plan.source_count,
        &plan.effects,
        &extra_inputs,
        source_kind,
        source_location,
    )?;
    let audit_context = json!({"scope_generation": fence.scope_ownership_generation.get(),
        "lease_generation": fence.character_lease_generation,
        "connection_generation": fence.connection_generation.get()});
    let (proof, audit_commitment) = if audit::payload_fits(&proof) {
        (proof, None)
    } else {
        // The database defines the single canonical byte representation. Hash the
        // complete prospective receipt there; the deferred proof recomputes it from
        // the actual immutable row and decodes the actual outbox Craft commitment.
        let basis = json!({"transaction_id": transaction, "event_id": event,
            "game_session_id": uuid_text(*fence.game_session_id.as_bytes()),
            "command_id": request.command.command_id().get().to_string(),
            "character_id": uuid_text(*fence.character_id.as_bytes()),
            "world_id": uuid_text(*world.as_bytes()), "intent": bytea_text(&request.intent),
            "recipe": plan.recipe, "source_item_instance_id": uuid_text(request.source),
            "parent_item_instance_id": parent.map(|id| <[u8; 16]>::try_from(id).map(uuid_text)).transpose()
                .map_err(|_| DurabilityError::InvalidStoredState)?,
            "source_ordinal": source_ordinal, "secondary_item_instance_id": uuid_text(request.target.unwrap_or(request.source)),
            "before_secondary": secondary_before.unwrap_or(before), "before_item": before,
            "outputs": outputs, "occurred_at": occurred, "parent_capacity": parent_capacity,
            "consume_source": plan.consume_source, "source_count": plan.source_count,
            "effects": plan.effects, "extra_inputs": extra_inputs, "source_kind": source_kind,
            "content_digest": bytea_text(&native.source_digest()), "source_location": source_location,
            "audit_context": audit_context});
        let digest: Vec<u8> = sqlx::query_scalar("SELECT sha256(convert_to(game_item_action_commitment_basis(jsonb_populate_record(NULL::game_item_consume_transform_records,$1))::text,'UTF8'))")
            .bind(basis).fetch_one(&mut ***tx).await?;
        (audit::compact_proof(&transaction, &digest)?, Some(digest))
    };
    let envelope = audit::encode(
        fence,
        request,
        proof,
        &transaction,
        &event,
        occurred,
        tuple.tuple(),
    )?;
    // Achievement writes use their existing in-transaction owner and the same event.
    if plan.recipe == "oteryn:potion.demonic-blood.v1" {
        super::account_achievement::add_progress(
            tx,
            fence.character_id,
            "oteryn:achievement/demonic_barkeeper",
            250,
            oteryn_gameplay_progression::achievement_granters::TRANSFORM_SOURCE_KIND,
            event.as_bytes(),
        )
        .await?;
    }
    apply_achievements(tx, fence, &plan.effects, &event).await?;
    // Keep the established rust-remover achievement. Seed use is another two-input
    // conversion, so source recipe rather than operand count qualifies this progress.
    if plan.recipe.starts_with("oteryn:rust-remover.") && !plan.outputs.is_empty() {
        use oteryn_gameplay_progression::achievement_granters::{POLISHER, TRANSFORM_SOURCE_KIND};
        super::account_achievement::add_progress(
            tx,
            fence.character_id,
            POLISHER.key,
            POLISHER.total,
            TRANSFORM_SOURCE_KIND,
            event.as_bytes(),
        )
        .await?;
    }
    sqlx::query("INSERT INTO game_item_consume_transform_records(transaction_id,event_id,game_session_id,command_id,character_id,world_id,intent,recipe,source_item_instance_id,parent_item_instance_id,source_ordinal,secondary_item_instance_id,before_secondary,before_item,outputs,occurred_at,envelope_sha256,parent_capacity,consume_source,effects,extra_inputs,source_kind,content_digest,source_location,source_count,audit_context,audit_commitment_sha256) VALUES($1::uuid,$2::uuid,encode($3,'hex')::uuid,$4::text::numeric,encode($5,'hex')::uuid,encode($6,'hex')::uuid,$7,$8,encode($9,'hex')::uuid,encode($10,'hex')::uuid,$11::text::numeric,encode($12,'hex')::uuid,$13,$14,$15,$16,sha256($17),$18,$19,$20,$21,$22,$23,$24,$25,$26,$27)")
        .bind(&transaction).bind(&event).bind(fence.game_session_id.as_bytes().as_slice())
        .bind(request.command.command_id().get().to_string()).bind(fence.character_id.as_bytes().as_slice())
        .bind(world.as_bytes().as_slice()).bind(&request.intent).bind(&plan.recipe).bind(request.source.as_slice())
        .bind(parent).bind(source_ordinal).bind(request.target.unwrap_or(request.source).as_slice())
        .bind(secondary_before.unwrap_or(before)).bind(before).bind(json!(outputs)).bind(occurred).bind(&envelope)
        .bind(i32::try_from(parent_capacity).map_err(|_| DurabilityError::InvalidStoredState)?)
        .bind(plan.consume_source).bind(&plan.effects).bind(json!(extra_inputs)).bind(source_kind)
        .bind(native.source_digest().as_slice()).bind(source_location).bind(i64::from(plan.source_count)).bind(&audit_context).bind(audit_commitment).execute(&mut ***tx).await?;
    super::consume_transform_cosmetics::apply(tx, fence, request, plan, &transaction).await?;
    if source_kind == 5 {
        house::apply_in_transaction(tx, &transaction).await?;
    }
    if plan.consume_source
        && !(source_kind == 5
            && plan.effects.get("kind").and_then(Value::as_str) == Some("house_decoration"))
    {
        consume_count(
            permit,
            tx,
            request.source,
            before,
            &transaction,
            source_kind,
            plan.source_count,
        )
        .await?;
        if let Some(target) = request.target {
            consume_count(
                permit,
                tx,
                target,
                secondary_before.ok_or(DurabilityError::InvalidStoredState)?,
                &transaction,
                1,
                1,
            )
            .await?;
        }
    }
    // Afflicted Ground USE leaves the clicked trigger intact and spends all six
    // carried pieces. SQL admits this exact exception with the same owner proofs.
    let dispenser = plan.recipe == "oteryn:dispenser.fill";
    if !dispenser {
        for input in &plan.extra_inputs {
            consume_count(
                permit,
                tx,
                input.item,
                &input.before,
                &transaction,
                input.source_kind,
                input.count,
            )
            .await?;
        }
    }
    cake::apply_equipment(tx, fence, plan, &transaction).await?;
    for (ordinal, output) in outputs.iter().enumerate() {
        if let Some((fences, context)) = ground_admission {
            // Actual source consumption above determines whether its root remains.
            // A refusal drops this entire transaction, including source and receipt.
            if let Some(disposition) =
                ground::admit_reward_output(tx, fence, fences, context, output).await?
            {
                return Ok(ConsumeTransformResult::refused(disposition));
            }
        }
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(($1->>'item')::uuid,encode($2,'hex')::uuid,'Item',$1->>'key',$1->>'revision',($1->>'count')::bigint,1,$3::uuid)")
            .bind(output).bind(world.as_bytes().as_slice()).bind(&transaction).execute(&mut ***tx).await?;
        match if source_kind == 3 && parent.is_some() {
            1
        } else {
            source_kind
        } {
            3 => {
                sqlx::query("INSERT INTO game_item_ground_locations(item_instance_id,world_id,channel_id,runtime_scope_ownership_generation,spatial_position,corpse_ref,map_revision,content_revision,native_room_placement_context,stack_ordinal) SELECT ($1->>'item')::uuid,g.world_id,g.channel_id,g.runtime_scope_ownership_generation,g.spatial_position,g.corpse_ref,g.map_revision,g.content_revision,g.native_room_placement_context,($1->>'ordinal')::bigint FROM jsonb_populate_record(NULL::game_item_ground_locations,$2) g")
                    .bind(output).bind(source_location).execute(&mut ***tx).await?;
            }
            4 => {
                sqlx::query("INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(($1->>'item')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,($1->>'ordinal')::numeric,$4::uuid)")
                    .bind(output).bind(world.as_bytes().as_slice()).bind(parent).bind(&transaction).execute(&mut ***tx).await?;
            }
            _ => {
                sqlx::query("INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(($1->>'item')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,($1->>'ordinal')::numeric,$5::uuid)")
                    .bind(output).bind(world.as_bytes().as_slice()).bind(fence.character_id.as_bytes().as_slice()).bind(parent).bind(&transaction).execute(&mut ***tx).await?;
            }
        }
        super::spell_item_temporal::schedule_action_item_in_transaction(
            tx,
            &transaction,
            u32::try_from(ordinal + 1).map_err(|_| DurabilityError::InvalidStoredState)?,
            output,
            native.source_digest(),
            occurred,
        )
        .await?;
        super::spell_item_temporal::event_items::schedule_consume_output(
            tx,
            output,
            &transaction,
            i32::try_from(ordinal + 1).map_err(|_| DurabilityError::InvalidStoredState)?,
            occurred,
        )
        .await?;
    }
    // Crystal cask_and_kegs.lua:43-54: retain the leftover empty flasks first,
    // create the filled potions, then debit/remove the dispenser. The source
    // debit above leaves every unfilled flask untouched before potion creation.
    if dispenser {
        dispenser::apply(tx, fence, plan, &transaction).await?;
        for input in &plan.extra_inputs {
            consume_count(
                permit,
                tx,
                input.item,
                &input.before,
                &transaction,
                input.source_kind,
                input.count,
            )
            .await?;
        }
    }
    sqlx::query("INSERT INTO game_item_audit_outbox(event_id,transaction_id,transaction_ordinal,transaction_count,event_type_id,schema_revision,retention_profile_id,item_instance_id,occurred_at,expires_at,envelope,envelope_sha256,publication_state) VALUES($1::uuid,$2::uuid,1,1,2,$3,$4,encode($5,'hex')::uuid,$6,$6+$7,$8,sha256($8),1)")
        .bind(&event).bind(&transaction).bind(i64::from(tuple.schema_revision())).bind(tuple.retention_profile_id())
        .bind(request.source.as_slice()).bind(occurred).bind(super::item_mint_audit::AUDIT_RETENTION_P90D_MS)
        .bind(&envelope).execute(&mut ***tx).await?;
    Ok(ConsumeTransformResult {
        disposition: UseDisposition::Committed,
        fresh: true,
        effects: plan.effects.clone(),
        recipe: Some(plan.recipe.clone()),
    })
}

async fn consume_count(
    permit: Option<&super::spell_owner_commit::SpellLanePermit>,
    tx: &mut super::item_mint_audit::Type2Transaction<'_>,
    item: [u8; 16],
    before: &Value,
    transaction: &str,
    source_kind: i32,
    count: u32,
) -> Result<(), DurabilityError> {
    if source_kind == 3 && permit.is_none() {
        return Err(DurabilityError::InvalidStoredState);
    }
    let quantity = before
        .get("quantity")
        .and_then(Value::as_i64)
        .ok_or(DurabilityError::InvalidStoredState)?;
    if count == 0 || i64::from(count) > quantity || !matches!(source_kind, 1..=5) {
        return Err(DurabilityError::InvalidStoredState);
    }
    let changed = sqlx::query("UPDATE game_item_instances SET quantity=quantity-$3,lifecycle=CASE WHEN quantity=$3 THEN 2 ELSE 1 END,last_transaction_id=$2::uuid WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(item.as_slice()).bind(transaction).bind(i64::from(count)).execute(&mut ***tx).await?.rows_affected();
    if changed != 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    if quantity == i64::from(count) {
        if source_kind == 5 {
            sqlx::query("SELECT game_item_dispenser_house_retire($1::uuid)")
                .bind(transaction)
                .execute(&mut ***tx)
                .await?;
            return Ok(());
        }
        let sql = match source_kind {
            2 => {
                "DELETE FROM game_character_equipment_slots WHERE item_instance_id=encode($1,'hex')::uuid"
            }
            3 => {
                "DELETE FROM game_item_ground_locations WHERE item_instance_id=encode($1,'hex')::uuid"
            }
            4 => {
                "DELETE FROM game_item_nested_entries WHERE item_instance_id=encode($1,'hex')::uuid"
            }
            _ => {
                "DELETE FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid"
            }
        };
        if sqlx::query(sql)
            .bind(item.as_slice())
            .execute(&mut ***tx)
            .await?
            .rows_affected()
            != 1
        {
            return Err(DurabilityError::InvalidStoredState);
        }
    }
    Ok(())
}

async fn apply_achievements(
    tx: &mut super::item_mint_audit::Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    effects: &Value,
    event: &str,
) -> Result<(), DurabilityError> {
    if effects.get("kind").and_then(Value::as_str) == Some("house_decoration") {
        let event = super::item_mint::uuid_text(event)?;
        let progress = effects
            .get("achievement_progress")
            .and_then(Value::as_array)
            .ok_or(DurabilityError::InvalidStoredState)?;
        // Crystal data-global/scripts/actions/other/music.lua:65-66: each use
        // advances the existing Rockstar achievement; no alternate achievement writer.
        for row in progress {
            if effects.get("decoration_family").and_then(Value::as_str) != Some("music")
                || row.get("key").and_then(Value::as_str) != Some("oteryn:achievement/rockstar")
                || row.get("target").and_then(Value::as_u64) != Some(10000)
            {
                return Err(DurabilityError::InvalidStoredState);
            }
            super::account_achievement::add_progress(
                tx,
                fence.character_id,
                "oteryn:achievement/rockstar",
                10000,
                oteryn_gameplay_progression::item_use_achievement::SOURCE_KIND,
                &event,
            )
            .await?;
        }
        return Ok(());
    }
    let Ok(effects) = serde_json::from_value::<oteryn_gameplay_items::novelty_items::NoveltyEffects>(
        effects.clone(),
    ) else {
        return Ok(());
    };
    let event = super::item_mint::uuid_text(event)?;
    let kind = oteryn_gameplay_progression::item_use_achievement::SOURCE_KIND;
    if let Some(key) = &effects.achievement {
        super::account_achievement::grant_missing(
            tx,
            fence.character_id,
            &[key.as_str()],
            kind,
            &event,
        )
        .await?;
    }
    for progress in &effects.achievement_progress {
        super::account_achievement::add_progress(
            tx,
            fence.character_id,
            &progress.key,
            progress.target,
            kind,
            &event,
        )
        .await?;
    }
    if effects.dice_value == Some(6) {
        let previous: Vec<i32> = sqlx::query_scalar("SELECT (effects->>'dice_value')::integer FROM game_item_consume_transform_records WHERE character_id=encode($1,'hex')::uuid AND effects->>'kind'='dice' ORDER BY receipt_sequence DESC LIMIT 2")
            .bind(fence.character_id.as_bytes().as_slice()).fetch_all(&mut ***tx).await?;
        if previous == [6, 6] {
            super::account_achievement::grant_missing(
                tx,
                fence.character_id,
                &["oteryn:achievement/number_of_the_beast"],
                kind,
                &event,
            )
            .await?;
        }
    }
    Ok(())
}

fn uuid_text(bytes: [u8; 16]) -> String {
    let hex = bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..]
    )
}

fn bytea_text(bytes: &[u8]) -> String {
    format!(
        "\\x{}",
        bytes
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    )
}
