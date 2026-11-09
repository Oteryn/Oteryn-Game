//! CRAFT-1: one fenced, receipt-keyed consume/mint transaction. Basic fixed cooking and
//! REWARD-CHEST-WIRE-1 reward bags, gathering and gem enchanting.
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
use oteryn_gameplay_items::item_craft::{
    carried_cooking_conversion_with_quantity, cooking_conversion, fishing_conversion,
};
use oteryn_gameplay_items::reward_bag::{
    is_reward_bag_output, reward_bag_conversion, reward_bag_draw,
};
use oteryn_protocol_oteryn::world_object::UseDisposition;
use serde_json::{Value, json};
use sqlx::Row;

pub(crate) mod audit;
mod enchanting;
mod event_containers;
mod harvest;
pub(crate) mod marriage;
mod skinning;
pub mod soulpit;
pub use enchanting::EnchantingCraft;
pub use harvest::{MapHarvestOperand, MapHarvestRequest, MapHarvestResult};
pub use skinning::SkinningRequest;

#[derive(Debug, Clone)]
pub struct ItemCraftRequest {
    pub command: CommandRef,
    pub source: [u8; 16],
    /// An unchanged carried tool for conversions such as fishing.
    pub tool: Option<[u8; 16]>,
    /// A second carried ingredient, locked and consumed in the same transaction.
    pub ingredient: Option<[u8; 16]>,
    /// The cooking station; `None` selects the reward-bag draw for this command and source.
    pub station: Option<TypedDefinitionRef>,
    /// Original bounded USE bytes. Replay compares these before resolving current handles.
    pub intent: Vec<u8>,
    pub gathering: Option<GatheringCraft>,
    pub enchanting: Option<EnchantingCraft>,
}

/// Server-resolved map operand; the terrain CAS and inventory proof commit together.
#[derive(Debug, Clone)]
pub struct GatheringCraft {
    pub scope: super::terrain_tool::TerrainScope,
    pub mutation: super::terrain_tool::TerrainMutation,
    pub attempt: oteryn_gameplay_items::fishing::GatheringAttempt,
}

impl DurabilityRoot {
    /// Probe or commit a basic cooking command. A probe needs no current handles or station;
    /// an exact historical receipt is returned before live custody checks after a restart.
    #[allow(clippy::too_many_arguments)]
    pub async fn craft_item<C: ItemLoopContent>(
        &self,
        permit: &super::spell_owner_commit::SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: ItemCraftRequest,
        native: &NativeGameplayState,
        content: &C,
        probe: bool,
    ) -> Result<Option<UseDisposition>, DurabilityError> {
        permit.check_scope(fence.runtime_scope)?;
        self.craft_item_owned::<C, ()>(recovery, node, fence, request, native, content, probe, None)
            .await
    }

    /// The same craft owner, with a retained live-state successor around COMMIT.
    #[allow(clippy::too_many_arguments)]
    pub async fn craft_item_owned<C: ItemLoopContent, T: Send + 'static>(
        &self,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: ItemCraftRequest,
        native: &NativeGameplayState,
        content: &C,
        probe: bool,
        window: Option<&mut super::spell_owner_commit::SpellCommitWindow<'_, T>>,
    ) -> Result<Option<UseDisposition>, DurabilityError> {
        // An enchanting write may not bypass the live resource owner's retained successor.
        // Probes remain available to the common replay path before resolving current handles.
        if request.enchanting.is_some() && !probe && window.is_none() {
            return Ok(Some(UseDisposition::Rejected));
        }
        if request.command.game_session_id() != fence.game_session_id
            || request.intent.is_empty()
            || request.intent.len() > oteryn_protocol_oteryn::world_object::MAX_USE_INTENT_BYTES
            || content.loop_content_digest() != native.source_digest()
        {
            return Ok(Some(UseDisposition::Rejected));
        }
        if let Some(window) = &window {
            window.permit().check_scope(fence.runtime_scope)?;
        }
        let record = recovery
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        let mut context = (native, content, window);
        self.try_issue_semantic_pass()?.run_with_context(&mut context, move |holder, deadline, context| Box::pin(async move {
            let mut tx = begin_type2_transaction(holder, deadline).await?;
            super::character_authority::assert_recovery_fence(&mut tx, &record).await?;
            // The same lock serializes probes and commits, including a cancelled acknowledgement.
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:item-craft:' || encode($1,'hex') || ':' || $2,0))")
                .bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string())
                .execute(&mut **tx).await?;
            let previous: Option<(Vec<u8>, Vec<u8>)> = sqlx::query_as("SELECT intent,uuid_send(character_id) FROM game_item_craft_records WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric")
                .bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string())
                .fetch_optional(&mut **tx).await?;
            if let Some((intent, character)) = previous {
                let result = if intent == request.intent && character == fence.character_id.as_bytes() { UseDisposition::Committed } else { UseDisposition::Rejected };
                tx.commit(deadline).await?;
                return Ok(Some(result));
            }
            if probe { tx.commit(deadline).await?; return Ok(None); }
            if assert_current_equipment_in_transaction(&mut tx, &record, &node, &fence, Some(request.command), context.1.loop_content_digest()).await.is_err() {
                return Ok(Some(UseDisposition::Rejected));
            }
            let result = write(&mut tx, fence, &request, context.0, context.1).await?;
            if result == UseDisposition::Committed {
                if let Some(window) = context.2.as_mut() {
                    super::pvp_live::commit_live_pvp_transaction(tx.into_inner(), deadline, window).await?;
                } else { tx.commit(deadline).await?; }
            }
            Ok(Some(result))
        })).await
    }
}

async fn write<C: ItemLoopContent>(
    tx: &mut super::item_mint_audit::Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    request: &ItemCraftRequest,
    native: &NativeGameplayState,
    content: &C,
) -> Result<UseDisposition, DurabilityError> {
    write_with_skinning(tx, fence, request, native, content, None).await
}

#[allow(
    clippy::too_many_lines,
    reason = "existing craft writer retains its single atomic inventory and audit owner"
)]
async fn write_with_skinning<C: ItemLoopContent>(
    tx: &mut super::item_mint_audit::Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    request: &ItemCraftRequest,
    native: &NativeGameplayState,
    content: &C,
    skinning: Option<(
        &SkinningRequest,
        &oteryn_gameplay_content::content::charm_source::CanonicalCharmCatalogue,
    )>,
) -> Result<UseDisposition, DurabilityError> {
    if request.enchanting.is_some() {
        return enchanting::write(tx, fence, request, native, content).await;
    }
    let (world, _) = scope_of(&fence).map_err(|_| DurabilityError::InvalidStoredState)?;
    // Root serialization precedes all item locks. Basic cooking accepts main-backpack leaves.
    let source = sqlx::query("SELECT to_jsonb(i) AS before_item,i.definition_family,i.definition_production_key,i.definition_revision_ref,i.quantity,uuid_send(e.parent_item_instance_id) AS parent,e.placement_ordinal::text AS ordinal,NULL::jsonb AS nested_source FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) JOIN game_item_container_slots b ON b.character_id=e.character_id AND b.item_instance_id=e.parent_item_instance_id WHERE i.item_instance_id=encode($1,'hex')::uuid AND e.character_id=encode($2,'hex')::uuid AND i.world_id=encode($3,'hex')::uuid AND i.lifecycle=1 AND e.world_id=i.world_id AND b.world_id=i.world_id FOR UPDATE OF i")
        .bind(request.source.as_slice()).bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).fetch_optional(&mut ***tx).await?;
    let source = match source {
        Some(source) => Some(source),
        None if request.station.is_none()
            && request.tool.is_none()
            && request.gathering.is_none() =>
        {
            event_containers::nested_source(tx, fence, request.source, *world.as_bytes()).await?
        }
        None => None,
    };
    let Some(source) = source else {
        return Ok(UseDisposition::StaleState);
    };
    let nested_source: Option<Value> = source.try_get("nested_source")?;
    let definition = TypedDefinitionRef {
        family: source.try_get("definition_family")?,
        production_key: source.try_get("definition_production_key")?,
        revision_ref: source.try_get("definition_revision_ref")?,
    };
    if nested_source.is_some()
        && oteryn_gameplay_items::reward_bag::event_containers::event_container_rule(
            &definition.production_key,
        )
        .is_none()
    {
        return Ok(UseDisposition::NothingToUse);
    }
    let skinning = if let Some((operand, catalogue)) = skinning {
        let Some(prepared) =
            skinning::prepare(tx, fence, operand, &definition, native, catalogue).await?
        else {
            return Ok(UseDisposition::NothingToUse);
        };
        Some(prepared)
    } else {
        None
    };
    let consumes_source = skinning.is_none()
        && request
            .gathering
            .as_ref()
            .is_none_or(|g| g.attempt.consumes_bait);
    let mut ingredient_count = 1_i64;
    let mut ingredient_weight = 0_u64;
    let mut retires_ingredient = false;
    let mut ingredient_debit = 1_u32;
    let mut ingredient_consumed = None;
    let conversion = if let Some(prepared) = &skinning {
        Some(oteryn_gameplay_items::item_craft::CookingConversion {
            recipe: prepared.conversion.recipe.clone(),
            outputs: prepared.conversion.outputs.clone(),
        })
    } else if let Some(gathering) = &request.gathering {
        let bait = if gathering.mutation.tool_key == "oteryn:item.tibia.i9306"
            && u16::try_from(gathering.mutation.before).is_ok_and(|target| {
                oteryn_gameplay_items::fishing::MECHANICAL_FISHING_WATER.contains(&target)
            }) {
            oteryn_gameplay_items::fishing::NAIL
        } else {
            oteryn_gameplay_items::fishing::WORM
        };
        let tool_id = request.tool.unwrap_or(request.source);
        let tool: Option<(String, String)> = sqlx::query_as("SELECT i.definition_production_key,i.definition_revision_ref FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) JOIN game_item_container_slots b ON b.character_id=e.character_id AND b.item_instance_id=e.parent_item_instance_id WHERE i.item_instance_id=encode($1,'hex')::uuid AND e.character_id=encode($2,'hex')::uuid AND i.world_id=encode($3,'hex')::uuid AND e.world_id=i.world_id AND b.world_id=i.world_id AND i.lifecycle=1 AND i.quantity>0 FOR UPDATE OF i")
            .bind(tool_id.as_slice()).bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).fetch_optional(&mut ***tx).await?;
        let planting = definition
            .production_key
            .strip_prefix("oteryn:item.tibia.i")
            .and_then(|id| id.parse::<u16>().ok())
            .and_then(|seed| {
                u16::try_from(gathering.mutation.before)
                    .ok()
                    .and_then(|target| {
                        oteryn_gameplay_items::event_planting::planting_attempt(seed, target)
                    })
            });
        let seed_is_qualified = planting.is_some_and(|plan| {
            plan == gathering.attempt
                && gathering.mutation.after == plan.after.map_or(0, u32::from)
                && gathering.mutation.decay
                    == plan
                        .after
                        .map(oteryn_gameplay_items::event_planting::planting_decays)
                        .unwrap_or_default()
                && request.tool.is_none()
                && tool_id == request.source
        });
        let soil = definition
            .production_key
            .strip_prefix("oteryn:item.tibia.i")
            .and_then(|id| id.parse::<u16>().ok())
            .and_then(|soil| {
                u16::try_from(gathering.mutation.before)
                    .ok()
                    .and_then(|target| oteryn_gameplay_items::elemental_soil::charge(soil, target))
            });
        let soil_is_qualified = soil.is_some_and(|plan| {
            plan == gathering.attempt
                && gathering.mutation.after == plan.after.map_or(0, u32::from)
                && gathering.mutation.decay
                    == plan
                        .after
                        .map(oteryn_gameplay_items::elemental_soil::decays)
                        .unwrap_or_default()
                && request.tool.is_none()
                && tool_id == request.source
        });
        if tool
            .as_ref()
            .map(|(key, revision)| (key.as_str(), revision.as_str()))
            != Some((
                gathering.mutation.tool_key.as_str(),
                gathering.mutation.tool_revision.as_str(),
            ))
            || tool_id != gathering.mutation.tool
            || gathering.scope.content_generation != native.source_digest()
            || (consumes_source
                && definition.production_key != bait
                && !seed_is_qualified
                && !soil_is_qualified)
        {
            return Ok(UseDisposition::StaleState);
        }
        let outputs = gathering
            .attempt
            .yield_item
            .map(|id| {
                let record = native.item_policy_for_source_id(u32::from(id))?.record();
                Some((
                    TypedDefinitionRef {
                        family: record.production_definition.family.clone(),
                        production_key: record.production_definition.production_key.clone(),
                        revision_ref: record.production_definition.revision_ref.clone(),
                    },
                    1,
                ))
            })
            .into_iter()
            .collect::<Option<Vec<_>>>();
        outputs.map(
            |outputs| oteryn_gameplay_items::item_craft::CookingConversion {
                recipe: if soil_is_qualified {
                    "oteryn:elemental_soil.crystal"
                } else if seed_is_qualified {
                    "oteryn:planting.crystal"
                } else {
                    "oteryn:gathering.crystal"
                }
                .into(),
                outputs,
            },
        )
    } else {
        match (&request.station, request.tool) {
            (Some(station), Some(tool_id)) => {
                let tool = sqlx::query("SELECT definition_family,definition_production_key,definition_revision_ref FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) WHERE i.item_instance_id=encode($1,'hex')::uuid AND e.character_id=encode($2,'hex')::uuid AND i.world_id=encode($3,'hex')::uuid AND i.lifecycle=1 FOR UPDATE OF i")
                .bind(tool_id.as_slice()).bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).fetch_optional(&mut ***tx).await?;
                tool.and_then(|tool| {
                    fishing_conversion(
                        native,
                        &TypedDefinitionRef {
                            family: tool.try_get("definition_family").ok()?,
                            production_key: tool.try_get("definition_production_key").ok()?,
                            revision_ref: tool.try_get("definition_revision_ref").ok()?,
                        },
                        &definition,
                        station,
                    )
                })
            }
            (Some(station), None) => {
                if let Some(ingredient_id) = request.ingredient {
                    if ingredient_id == request.source {
                        return Ok(UseDisposition::Rejected);
                    }
                    let ingredient = sqlx::query("SELECT uuid_send(e.parent_item_instance_id) AS parent,i.quantity,i.definition_family,i.definition_production_key,i.definition_revision_ref,(SELECT f.fluid_subtype FROM game_item_fluid_states f WHERE f.item_instance_id=i.item_instance_id FOR UPDATE) AS fluid_subtype FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) WHERE i.item_instance_id=encode($1,'hex')::uuid AND e.character_id=encode($2,'hex')::uuid AND i.world_id=encode($3,'hex')::uuid AND i.lifecycle=1 AND i.quantity>0 FOR UPDATE OF i")
                        .bind(ingredient_id.as_slice()).bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).fetch_optional(&mut ***tx).await?;
                    ingredient.and_then(|row| {
                        if row.try_get::<Vec<u8>, _>("parent").ok()?
                            != source.try_get::<Vec<u8>, _>("parent").ok()?
                        {
                    let ingredient = sqlx::query("SELECT i.quantity,uuid_send(e.parent_item_instance_id) AS parent,i.definition_family,i.definition_production_key,i.definition_revision_ref,(SELECT f.fluid_subtype FROM game_item_fluid_states f WHERE f.item_instance_id=i.item_instance_id FOR UPDATE) AS fluid_subtype FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) WHERE i.item_instance_id=encode($1,'hex')::uuid AND e.character_id=encode($2,'hex')::uuid AND i.world_id=encode($3,'hex')::uuid AND i.lifecycle=1 AND i.quantity>0 FOR UPDATE OF i")
                        .bind(ingredient_id.as_slice()).bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).fetch_optional(&mut ***tx).await?;
                    ingredient.and_then(|row| {
                        if row.try_get::<Vec<u8>, _>("parent").ok()?
                            != source.try_get::<Vec<u8>, _>("parent").ok()? {
                            return None;
                        }
                        let target = TypedDefinitionRef {
                            family: row.try_get("definition_family").ok()?,
                            production_key: row.try_get("definition_production_key").ok()?,
                            revision_ref: row.try_get("definition_revision_ref").ok()?,
                        };
                        let quantity =
                            u32::try_from(row.try_get::<i64, _>("quantity").ok()?).ok()?;
                        let resolved = carried_cooking_conversion_with_quantity(
                            native,
                            &definition,
                            &target,
                            ingredient_fluid_subtype(
                                row.try_get::<Option<i16>, _>("fluid_subtype").ok()?,
                            ),
                            quantity,
                        )?;
                        let weight = content.loop_facts(&target)?.weight?;
                        ingredient_count = i64::from(resolved.target_debit);
                        ingredient_weight = u64::from(weight) * u64::from(resolved.target_debit);
                        retires_ingredient = quantity == resolved.target_debit;
                        let quantity = u32::try_from(row.try_get::<i64, _>("quantity").ok()?).ok()?;
                        let resolved = carried_cooking_conversion_with_quantity(
                            native, &definition, &target,
                            ingredient_fluid_subtype(row.try_get::<Option<i16>, _>("fluid_subtype").ok()?),
                            quantity,
                        )?;
                        ingredient_debit = resolved.target_debit;
                        ingredient_consumed = Some((target, quantity == ingredient_debit));
                        Some(resolved.conversion)
                    })
                } else {
                    cooking_conversion(native, &definition, station)
                }
            }
            (None, _) => reward_bag_conversion(
                native,
                &definition,
                reward_bag_draw(
                    *fence.game_session_id.as_bytes(),
                    request.command.command_id().get(),
                    request.source,
                ),
            ),
        }
    };
    let Some(conversion) = conversion else {
        return Ok(UseDisposition::NothingToUse);
    };
    let before: Value = source.try_get("before_item")?;
    let quantity: i64 = source.try_get("quantity")?;
    let consumed_quantity = if !consumes_source {
        0
    } else if oteryn_gameplay_items::reward_bag::event_containers::consumes_whole_stack(
        &conversion.recipe,
    ) {
        quantity
    } else {
        1
    };
    let retires_source = consumes_source && quantity == consumed_quantity;
    let frees_output_slot = retires_source && nested_source.is_none();
    let retires_ingredient = ingredient_consumed.as_ref().is_some_and(|(_, retires)| *retires);
    let parent: Vec<u8> = source.try_get("parent")?;
    let occupied: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid").bind(&parent).fetch_one(&mut ***tx).await?;
    let tree_count: i64 = sqlx::query_scalar(
        "WITH RECURSIVE tree(item_instance_id,depth) AS ( \
           SELECT item_instance_id,1 FROM game_item_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid \
           UNION ALL SELECT e.item_instance_id,t.depth+1 FROM tree t JOIN game_item_nested_entries e \
             ON e.parent_item_instance_id=t.item_instance_id WHERE t.depth<8 \
         ) SELECT count(*) FROM (SELECT item_instance_id FROM tree LIMIT 501) bounded",
    ).bind(&parent).fetch_one(&mut ***tx).await?;
    if tree_count
        + i64::try_from(conversion.outputs.len())
            .map_err(|_| DurabilityError::InvalidStoredState)?
        - i64::from(retires_source)
        - i64::from(retires_ingredient)
        > 500
    {
        return Ok(UseDisposition::NoRoom);
    }
    let bag = sqlx::query("SELECT definition_family,definition_production_key,definition_revision_ref FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid AND lifecycle=1 FOR UPDATE")
        .bind(&parent).fetch_one(&mut ***tx).await?;
    let bag_definition = TypedDefinitionRef {
        family: bag.try_get("definition_family")?,
        production_key: bag.try_get("definition_production_key")?,
        revision_ref: bag.try_get("definition_revision_ref")?,
    };
    let capacity = content
        .loop_facts(&bag_definition)
        .and_then(|f| f.container_capacity)
        .unwrap_or(0)
        .min(20);
    if occupied
        + i64::try_from(conversion.outputs.len())
            .map_err(|_| DurabilityError::InvalidStoredState)?
        - i64::from(frees_output_slot)
        - i64::from(retires_ingredient)
        > i64::from(capacity)
    {
        return Ok(UseDisposition::NoRoom);
    }
    // Whole carried weight and capacity use the same locked projection as the item loop.
    let carried = match carried_weight(tx, fence.character_id, *world.as_bytes(), content).await {
        Ok(w) => w,
        Err(_) => return Ok(UseDisposition::NotPossible),
    };
    let Some(used_weight) = content.loop_facts(&definition).and_then(|f| f.weight) else {
        return Ok(UseDisposition::NotPossible);
    };
    let mut output_weight = 0_u64;
    for (definition, count) in &conversion.outputs {
        let Some(facts) = content.loop_facts(definition) else {
            return Ok(UseDisposition::NotPossible);
        };
        let Some(weight) = facts.weight else {
            return Ok(UseDisposition::NotPossible);
        };
        // Mint only an empty container admitted by the Primal Bag, W19 source
        // table, or captured reward catalogue; the writer creates no children.
        let empty_container = request.station.is_none()
            && *count == 1
            && (definition.production_key == "oteryn:item.tibia.i39150"
                || oteryn_gameplay_items::reward_bag::event_containers::permits_empty_container(
                    &conversion.recipe,
                    &definition.production_key,
                )
                || is_reward_bag_output(&conversion.recipe, &definition.production_key));
        if (facts.container_capacity.is_some() && !empty_container)
            || *count
                > stack_maximum(facts.stack)
                    .map(|(maximum, _)| maximum)
                    .unwrap_or(0)
        {
            return Ok(UseDisposition::NotPossible);
        }
        output_weight += u64::from(weight) * u64::from(*count);
    }
    let ingredient_weight = match &ingredient_consumed {
        Some((target, _)) => {
            let Some(weight) = content.loop_facts(target).and_then(|facts| facts.weight) else {
                return Ok(UseDisposition::NotPossible);
            };
            u64::from(weight) * u64::from(ingredient_debit)
        }
        None => 0,
    };
    let capacity = read_derived_transfer_capacity_centi_oz(tx, fence.character_id, content).await?;
    if carried
        .checked_sub(if consumes_source {
            u64::from(used_weight)
                * u64::try_from(consumed_quantity)
                    .map_err(|_| DurabilityError::InvalidStoredState)?
        } else {
            0
        })
        .and_then(|w| w.checked_sub(ingredient_weight))
        .and_then(|w| w.checked_add(output_weight))
        .is_none_or(|w| w > capacity)
    {
        return Ok(UseDisposition::NotPossible);
    }
    let highest: String = sqlx::query_scalar("SELECT coalesce(max(placement_ordinal),0)::text FROM game_item_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid").bind(&parent).fetch_one(&mut ***tx).await?;
    let mut ordinal: u64 = highest
        .parse()
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let mut outputs = Vec::new();
    for (position, (definition, count)) in conversion.outputs.iter().enumerate() {
        let output_ordinal = if frees_output_slot && position == 0 {
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
        outputs.push(json!({"item": item, "key": definition.production_key, "revision": definition.revision_ref, "count": count, "ordinal": output_ordinal}));
    }
    let (transaction, event, occurred): (String, String, i64) = sqlx::query_as("SELECT game_character_uuid_v7()::text,game_character_uuid_v7()::text,(extract(epoch FROM clock_timestamp())*1000)::bigint").fetch_one(&mut ***tx).await?;
    let ingredient_before: Option<Value> = match request.ingredient {
        Some(ingredient) => sqlx::query_scalar("SELECT to_jsonb(i) FROM game_item_instances i WHERE item_instance_id=encode($1,'hex')::uuid")
            .bind(ingredient.as_slice()).fetch_optional(&mut ***tx).await?,
        None => None,
    };
    let tuple = tx.tuple();
    let envelope = if let Some(prepared) = &skinning {
        audit::encode_map_harvest(
            fence,
            request.command,
            json!({
                "recipe": conversion.recipe, "intent": request.intent, "character": fence.character_id.as_bytes(), "before": before, "station": null, "gathering": null,
                "corpse": prepared.before, "skinning": prepared.proof, "spent": {"key": prepared.spent.production_key, "revision": prepared.spent.revision_ref},
                "outputs": outputs, "content_digest": native.source_digest(),
                "lease_generation": fence.character_lease_generation,
                "scope_generation": fence.scope_ownership_generation.get(),
            }),
            &transaction,
            &event,
            occurred,
            tuple.tuple(),
        )?
    } else {
        audit::encode_with_ingredient(
            fence,
            request,
            &conversion.recipe,
            native.source_digest(),
            &before,
            &outputs,
            ingredient_before
                .as_ref()
                .map(|before| (before, ingredient_count)),
            &transaction,
            &event,
            occurred,
            tuple.tuple(),
        )?
    };
    if let Some(gathering) = &request.gathering
        && !super::terrain_tool::apply_in_transaction(
            tx,
            fence,
            request.command,
            &request.intent,
            &gathering.scope,
            &gathering.mutation,
        )
        .await?
    {
        return Ok(UseDisposition::StaleState);
    }
    if let Some(prepared) = &skinning {
        skinning::record(tx, fence, prepared, &transaction).await?;
    }
    sqlx::query("INSERT INTO game_item_craft_records(transaction_id,event_id,game_session_id,command_id,character_id,world_id,intent,recipe,source_item_instance_id,parent_item_instance_id,source_ordinal,before_item,outputs,occurred_at,envelope_sha256,consume_source,ingredient_item_instance_id,ingredient_before_item,event_source_location,ingredient_count) VALUES($1::uuid,$2::uuid,encode($3,'hex')::uuid,$4::text::numeric,encode($5,'hex')::uuid,encode($6,'hex')::uuid,$7,$8,encode($9,'hex')::uuid,encode($10,'hex')::uuid,$11::text::numeric,$12,$13,$14,sha256($15),$16,encode($17,'hex')::uuid,$18,$19,$20)")
        .bind(&transaction).bind(&event).bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string()).bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).bind(&request.intent).bind(&conversion.recipe).bind(request.source.as_slice()).bind(&parent).bind(source.try_get::<String,_>("ordinal")?).bind(&before).bind(json!(outputs)).bind(occurred).bind(&envelope).bind(consumes_source).bind(request.ingredient.map(|id| id.to_vec())).bind(ingredient_before).bind(&nested_source).bind(i32::try_from(ingredient_count).map_err(|_| DurabilityError::InvalidStoredState)?).execute(&mut ***tx).await?;
    sqlx::query("INSERT INTO game_item_craft_records(transaction_id,event_id,game_session_id,command_id,character_id,world_id,intent,recipe,source_item_instance_id,parent_item_instance_id,source_ordinal,before_item,outputs,occurred_at,envelope_sha256,consume_source,ingredient_item_instance_id,ingredient_before_item,event_source_location,ingredient_consumed_quantity) VALUES($1::uuid,$2::uuid,encode($3,'hex')::uuid,$4::text::numeric,encode($5,'hex')::uuid,encode($6,'hex')::uuid,$7,$8,encode($9,'hex')::uuid,encode($10,'hex')::uuid,$11::text::numeric,$12,$13,$14,sha256($15),$16,encode($17,'hex')::uuid,$18,$19,$20)")
        .bind(&transaction).bind(&event).bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string()).bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).bind(&request.intent).bind(&conversion.recipe).bind(request.source.as_slice()).bind(&parent).bind(source.try_get::<String,_>("ordinal")?).bind(&before).bind(json!(outputs)).bind(occurred).bind(&envelope).bind(consumes_source).bind(request.ingredient.map(|id| id.to_vec())).bind(ingredient_before).bind(&nested_source).bind(i64::from(ingredient_debit)).execute(&mut ***tx).await?;
    if let Some(prepared) = &skinning {
        skinning::transform(tx, prepared, &transaction).await?;
    }
    if consumes_source {
        sqlx::query("UPDATE game_item_instances SET quantity=quantity-$3,lifecycle=CASE WHEN quantity=$3 THEN 2 ELSE 1 END,last_transaction_id=$2::uuid WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(request.source.as_slice()).bind(&transaction).bind(consumed_quantity).execute(&mut ***tx).await?;
    }
    if let Some(ingredient) = request.ingredient {
        sqlx::query("UPDATE game_item_instances SET quantity=quantity-$3,lifecycle=CASE WHEN quantity=$3 THEN 2 ELSE 1 END,last_transaction_id=$2::uuid WHERE item_instance_id=encode($1,'hex')::uuid")
            .bind(ingredient.as_slice()).bind(&transaction).bind(ingredient_count).execute(&mut ***tx).await?;
            .bind(ingredient.as_slice()).bind(&transaction).bind(i64::from(ingredient_debit)).execute(&mut ***tx).await?;
        sqlx::query("DELETE FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid AND (SELECT lifecycle=2 FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid)")
            .bind(ingredient.as_slice()).execute(&mut ***tx).await?;
    }
    if retires_source {
        let deletion = if nested_source.is_some() {
            "DELETE FROM game_item_nested_entries WHERE item_instance_id=encode($1,'hex')::uuid"
        } else {
            "DELETE FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid"
        };
        sqlx::query(deletion)
            .bind(request.source.as_slice())
            .execute(&mut ***tx)
            .await?;
    }
    for output in outputs {
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(($1->>'item')::uuid,encode($2,'hex')::uuid,'Item',$1->>'key',$1->>'revision',($1->>'count')::bigint,1,$3::uuid)")
            .bind(&output).bind(world.as_bytes().as_slice()).bind(&transaction).execute(&mut ***tx).await?;
        sqlx::query("INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(($1->>'item')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,($1->>'ordinal')::numeric,$5::uuid)")
            .bind(&output).bind(world.as_bytes().as_slice()).bind(fence.character_id.as_bytes().as_slice()).bind(&parent).bind(&transaction).execute(&mut ***tx).await?;
    }
    if conversion.recipe == "oteryn:planting.crystal"
        && request
            .gathering
            .as_ref()
            .is_some_and(|g| g.mutation.before == 14029)
    {
        super::account_achievement::grant_missing(
            tx,
            fence.character_id,
            &["oteryn:achievement/preservationist"],
            "map_seed_planting",
            &super::item_mint::uuid_text(&event)?,
        )
        .await?;
    }
    sqlx::query("INSERT INTO game_item_audit_outbox(event_id,transaction_id,transaction_ordinal,transaction_count,event_type_id,schema_revision,retention_profile_id,item_instance_id,occurred_at,expires_at,envelope,envelope_sha256,publication_state) VALUES($1::uuid,$2::uuid,1,1,2,$3,$4,encode($5,'hex')::uuid,$6,$6+$7,$8,sha256($8),1)")
        .bind(&event).bind(&transaction).bind(i64::from(tuple.schema_revision())).bind(tuple.retention_profile_id()).bind(request.source.as_slice()).bind(occurred).bind(super::item_mint_audit::AUDIT_RETENTION_P90D_MS).bind(&envelope).execute(&mut ***tx).await?;
    if skinning.is_some() {
        super::stag_beaks::record(tx, fence, &transaction).await?;
    }
    // The successful yield, counter and grant share the craft receipt transaction.
    // Exact replay returned before this writer, so a retry never advances the counter.
    use oteryn_gameplay_progression::achievement_granters;
    let achievement = if let Some(gathering) = &request.gathering {
        achievement_granters::gathering(&gathering.mutation.tool_key, gathering.attempt.yield_item)
    } else {
        (conversion.recipe == "oteryn:fishing.global.standard")
            .then_some(achievement_granters::HERE_FISHY_FISHY)
    };
    if let Some(achievement) = achievement {
        super::account_achievement::add_progress(
            tx,
            fence.character_id,
            achievement.key,
            achievement.total,
            achievement_granters::CRAFT_SOURCE_KIND,
            event.as_bytes(),
        )
        .await?;
    }
    Ok(UseDisposition::Committed)
}

fn ingredient_fluid_subtype(stored: Option<i16>) -> Option<u32> {
    stored.and_then(|value| u32::try_from(value).ok())
}

#[cfg(test)]
mod fluid_ingredient_tests {
    use super::*;

    #[test]
    fn cooking_fluid_subtype_comes_from_fluid_state() {
        assert_eq!(ingredient_fluid_subtype(Some(9)), Some(9));
        assert_eq!(ingredient_fluid_subtype(None), None);
        assert_eq!(ingredient_fluid_subtype(Some(-1)), None);
    }
}
