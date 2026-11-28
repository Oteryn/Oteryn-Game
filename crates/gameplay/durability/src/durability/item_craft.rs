//! CRAFT-1: one fenced, receipt-keyed consume/mint transaction. Basic fixed cooking and
//! REWARD-CHEST-WIRE-1 reward bags, gathering and gem enchanting.
use super::character_authority::ReconciledCharacterAuthority;
use super::character_equipment::assert_current_equipment_in_transaction;
use super::db::begin_type2_transaction;
use super::item_loop_move::{ItemLoopContent, LoopGround, carried_weight};
use super::item_mint::TypedDefinitionRef;
use super::item_transfer::{
    CurrentCharacterItemFence, read_derived_transfer_capacity_centi_oz, scope_of, stack_maximum,
};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use crate::foundation::CommandRef;
use oteryn_gameplay_content::content::native_gameplay::NativeGameplayState;
use oteryn_gameplay_items::fishing;
use oteryn_gameplay_items::item_craft::{
    CookingConversion, carried_cooking_conversion_with_quantity, cooking_conversion,
    fishing_conversion,
};
use oteryn_gameplay_items::reward_bag::{
    is_reward_bag_output, reward_bag_conversion, reward_bag_draw,
};
use oteryn_protocol_oteryn::world_object::UseDisposition;
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::Row;

pub(crate) mod audit;
mod elemental_energy;
mod enchanting;
mod event_containers;
mod harvest;
pub(crate) mod marriage;
mod sculpture;
mod skinning;
pub mod soulpit;
pub use enchanting::EnchantingCraft;
pub use event_containers::{BLUE_PRESENT, EventContainerReplaySource, REWARD_BAG_INSTANCE_INTENT_TAG};
pub use harvest::{MapHarvestOperand, MapHarvestRequest, MapHarvestResult};
pub use sculpture::{GroundSculpture, SculptureGroundTarget};
pub use skinning::{SkinningOutcome, SkinningRequest};

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

enum CraftVariant<'a, 'attempt> {
    Ordinary,
    Skinning(&'a mut skinning::Attempt<'attempt>),
    BluePresentDrop(event_containers::DropAdmission<'a>),
    Sculpture(sculpture::GroundContext<'a>),
}

#[cfg(feature = "test-support")]
tokio::task_local! { static LOSE_GROUND_SCULPTURE_ACK: (); }

/// Simulate an acknowledgement lost after the real sculpture COMMIT.
#[cfg(feature = "test-support")]
pub async fn lose_ground_sculpture_ack<T>(craft: impl std::future::Future<Output = T>) -> T {
    LOSE_GROUND_SCULPTURE_ACK.scope((), craft).await
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
        self.craft_item_with_lane::<C, ()>(
            recovery,
            node,
            fence,
            request,
            native,
            content,
            probe,
            None,
            Some(permit),
            None,
            None,
        )
        .await
    }

    /// Carried Blue Present USE keeps its craft receipt and supplies an optional
    /// actor tile for the donor's full-backpack fallback.
    pub async fn craft_blue_present<C: ItemLoopContent>(
        &self,
        authority: (
            &super::spell_owner_commit::SpellLanePermit,
            &ReconciledCharacterAuthority<'_, '_>,
            &NodeIncarnationProof,
        ),
        fence: CurrentCharacterItemFence,
        request: ItemCraftRequest,
        native: &NativeGameplayState,
        content: &C,
        drop: &LoopGround,
    ) -> Result<Option<UseDisposition>, DurabilityError> {
        let (permit, recovery, node) = authority;
        self.craft_item_with_lane::<C, ()>(
            recovery,
            node,
            fence,
            request,
            native,
            content,
            false,
            None,
            Some(permit),
            Some(drop),
            None,
        )
        .await
    }

    /// Sculpt at a current Ground target or use the actor's tile for a marble yield.
    #[allow(clippy::too_many_arguments)]
    pub async fn craft_item_on_ground<C: ItemLoopContent>(
        &self,
        permit: &super::spell_owner_commit::SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: ItemCraftRequest,
        native: &NativeGameplayState,
        content: &C,
        ground: GroundSculpture,
    ) -> Result<Option<UseDisposition>, DurabilityError> {
        self.craft_item_with_lane::<C, ()>(
            recovery, node, fence, request, native, content, false, None, Some(permit), None, Some(ground),
        ).await
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
        self.craft_item_with_lane(
            recovery, node, fence, request, native, content, probe, window, None, None, None,
        )
        .await
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "shared craft owner keeps current fences and the borrowed lane/window proof distinct"
    )]
    async fn craft_item_with_lane<C: ItemLoopContent, T: Send + 'static>(
        &self,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: ItemCraftRequest,
        native: &NativeGameplayState,
        content: &C,
        probe: bool,
        window: Option<&mut super::spell_owner_commit::SpellCommitWindow<'_, T>>,
        permit: Option<&super::spell_owner_commit::SpellLanePermit>,
        drop: Option<&LoopGround>,
        ground: Option<GroundSculpture>,
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
        if let Some(permit) = permit {
            permit.check_scope(fence.runtime_scope)?;
        }
        let mut context = (native, content, window, permit, drop);
        self.try_issue_semantic_pass()?.run_with_context(&mut context, move |holder, deadline, context| Box::pin(async move {
            let mut tx = begin_type2_transaction(holder, deadline).await?;
            super::character_authority::assert_recovery_fence(&mut tx, &record).await?;
            // The same lock serializes probes and commits, including a cancelled acknowledgement.
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:item-craft:' || encode($1,'hex') || ':' || $2,0))")
                .bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string())
                .execute(&mut **tx).await?;
            let previous: Option<(Vec<u8>, Vec<u8>, bool)> = sqlx::query_as(
                "SELECT intent, uuid_send(character_id), \
                        source_item_instance_id=encode($3,'hex')::uuid \
                            AND (recipe LIKE 'oteryn:event-container.%' OR recipe=$4) \
                 FROM game_item_craft_records \
                 WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric",
            )
                .bind(fence.game_session_id.as_bytes().as_slice())
                .bind(request.command.command_id().get().to_string())
                .bind(request.source.as_slice())
                .bind(event_containers::BLUE_PRESENT_RECIPE)
                .fetch_optional(&mut **tx).await?;
            if let Some((intent, character, event_source_matches)) = previous {
                // The server's completion probe only retains the resolved instance after
                // restart. An incoming wire retry still has to match its original bytes.
                let completed_instance = probe
                    && event_source_matches
                    && !intent.starts_with(REWARD_BAG_INSTANCE_INTENT_TAG)
                    && request.intent.strip_prefix(REWARD_BAG_INSTANCE_INTENT_TAG)
                        == Some(request.source.as_slice());
                let result = if character == fence.character_id.as_bytes()
                    && (intent == request.intent || completed_instance)
                {
                    UseDisposition::Committed
                } else {
                    UseDisposition::Rejected
                };
                tx.commit(deadline).await?;
                return Ok(Some(result));
            }
            if probe { tx.commit(deadline).await?; return Ok(None); }
            if assert_current_equipment_in_transaction(&mut tx, &record, &node, &fence, Some(request.command), context.1.loop_content_digest()).await.is_err() {
                return Ok(Some(UseDisposition::Rejected));
            }
            // Historical receipts and probes above need no fresh write authority.
            // A new craft must retain the real outer permit or its live window.
            let Some(permit) = context.2.as_ref().map(|window| window.permit()).or(context.3) else {
                return Ok(Some(UseDisposition::Rejected));
            };
            let variant = if let Some(operand) = ground.as_ref() {
                // Serialize the next Ground read with this transaction,
                // including when the command's COMMIT outcome is unknown.
                super::spell_item_transaction::lock_ground_scope(&mut tx, fence.runtime_scope)
                    .await?;
                CraftVariant::Sculpture(sculpture::GroundContext {
                    operand, recovery: &record, node: &node,
                })
            } else {
                context.4.map_or(CraftVariant::Ordinary, |target| {
                    CraftVariant::BluePresentDrop(event_containers::DropAdmission {
                        target, recovery: &record, node: &node,
                    })
                })
            };
            let result = write(&mut tx, permit, fence, &request, context.0, context.1, variant).await?;
            if result == UseDisposition::Committed {
                if let Some(window) = context.2.as_mut() {
                    super::pvp_live::commit_live_pvp_transaction(tx.into_inner(), deadline, window).await?;
                } else {
                    tx.commit(deadline).await?;
                    #[cfg(feature = "test-support")]
                    if ground.is_some() && LOSE_GROUND_SCULPTURE_ACK.try_with(|()| ()).is_ok() {
                        return Err(DurabilityError::CommitOutcomeUnknown);
                    }
                }
            }
            Ok(Some(result))
        })).await
    }
}

// Crystal lucky_charm.lua:8-16 and actions_corpse.lua share the active hunt gate.
async fn rottin_charm_hunt_active(
    tx: &mut super::item_mint_audit::Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
) -> Result<bool, DurabilityError> {
    let active: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM game_character_quest_tracks
         WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2
           AND track_key IN ($3,$4) AND value=1)",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(oteryn_gameplay_quests::quest::rottin_walls::QUEST)
    .bind("oteryn:quest-progress/crystalserver/quest/u8_7/rottin_wood_and_the_married_men/mission01")
    .bind("oteryn:quest-progress/crystalserver/quest/u8_7/rottin_wood_and_the_married_men/task01")
    .fetch_one(&mut ***tx)
    .await?;
    Ok(active)
}

#[allow(
    clippy::too_many_lines, clippy::too_many_arguments,
    reason = "existing craft writer retains its single atomic inventory and audit owner"
)]
async fn write<C: ItemLoopContent>(
    tx: &mut super::item_mint_audit::Type2Transaction<'_>,
    permit: &super::spell_owner_commit::SpellLanePermit,
    fence: CurrentCharacterItemFence,
    request: &ItemCraftRequest,
    native: &NativeGameplayState,
    content: &C,
    variant: CraftVariant<'_, '_>,
) -> Result<UseDisposition, DurabilityError> {
    if request.enchanting.is_some() {
        return enchanting::write(tx, fence, request, native, content).await;
    }
    let (skinning, ground, blue_present) = match variant {
        CraftVariant::Ordinary => (None, None, None),
        CraftVariant::Skinning(attempt) => (Some(attempt), None, None),
        CraftVariant::Sculpture(context) => (None, Some(context), None),
        CraftVariant::BluePresentDrop(drop) => (None, None, Some(drop)),
    };
    let world = if skinning.is_some() {
        fence.runtime_scope.world_id()
    } else {
        scope_of(&fence).map_err(|_| DurabilityError::InvalidStoredState)?.0
    };
    // Root serialization precedes all item locks. Only opted-in actions accept nested leaves.
    let source = sqlx::query("SELECT to_jsonb(i) AS before_item,i.definition_family,i.definition_production_key,i.definition_revision_ref,i.quantity,uuid_send(e.parent_item_instance_id) AS parent,e.placement_ordinal::text AS ordinal,NULL::jsonb AS nested_source FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) JOIN game_item_container_slots b ON b.character_id=e.character_id AND b.item_instance_id=e.parent_item_instance_id WHERE i.item_instance_id=encode($1,'hex')::uuid AND e.character_id=encode($2,'hex')::uuid AND i.world_id=encode($3,'hex')::uuid AND i.lifecycle=1 AND e.world_id=i.world_id AND b.world_id=i.world_id FOR UPDATE OF i")
        .bind(request.source.as_slice()).bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).fetch_optional(&mut ***tx).await?;
    let sculpture_target = request.station.as_ref().and_then(source_item_id);
    let source = match source {
        Some(source) => Some(source),
        None if request.tool.is_none()
            && ((request.station.is_none() && request.gathering.is_none())
                || ((request.ingredient.is_some() || request.gathering.is_some())
                    && sculpture_target.is_some_and(|target| fishing::is_sculpture(5908, target)))) =>
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
    let sculpture_station = source_item_id(&definition)
        .zip(sculpture_target)
        .is_some_and(|(tool, target)| fishing::is_sculpture(tool, target));
    if nested_source.is_some() && !sculpture_station && request.station.is_some() {
        return Ok(UseDisposition::StaleState);
    }
    if nested_source.is_some()
        && !sculpture_station
        && skinning.is_none()
        && oteryn_gameplay_items::reward_bag::event_containers::legacy_conversion_rule(
            &definition.production_key,
        )
        .is_none()
    {
        return Ok(UseDisposition::NothingToUse);
    }
    let skinning = if let Some(attempt) = skinning {
        let Some(prepared) = skinning::prepare(
            tx,
            fence,
            attempt.request,
            &definition,
            native,
            attempt.catalogue,
        )
        .await?
        else {
            return Ok(UseDisposition::NothingToUse);
        };
        if nested_source.is_some() && prepared.rabbit_feet.is_none() {
            return Ok(UseDisposition::NothingToUse);
        }
        attempt.rabbit_feet = prepared.rabbit_feet;
        Some(prepared)
    } else {
        None
    };
    let carried_sculpture = request.ingredient.is_some()
        && request.gathering.is_none()
        && request.tool.is_none()
        && sculpture_station;
    let consumes_source = !carried_sculpture
        && skinning.is_none()
        && request
            .gathering
            .as_ref()
            .is_none_or(|g| g.attempt.consumes_bait);
    let mut ingredient_count = 1_i64;
    let mut ingredient_debit = 1_u32;
    let mut ingredient_consumed = None;
    let mut carried_attempt = None;
    let mut ingredient_location: Option<Value> = None;
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
        let tool: Option<(String, String)> = if tool_id == request.source && sculpture_station {
            Some((definition.production_key.clone(), definition.revision_ref.clone()))
        } else { sqlx::query_as("SELECT i.definition_production_key,i.definition_revision_ref FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) JOIN game_item_container_slots b ON b.character_id=e.character_id AND b.item_instance_id=e.parent_item_instance_id WHERE i.item_instance_id=encode($1,'hex')::uuid AND e.character_id=encode($2,'hex')::uuid AND i.world_id=encode($3,'hex')::uuid AND e.world_id=i.world_id AND b.world_id=i.world_id AND i.lifecycle=1 AND i.quantity>0 FOR UPDATE OF i")
            .bind(tool_id.as_slice()).bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).fetch_optional(&mut ***tx).await? };
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
        gathering_outputs(native, gathering.attempt.yield_item).map(|outputs| CookingConversion {
            recipe: if soil_is_qualified && definition.production_key == "oteryn:item.tibia.i945" {
                "oteryn:elemental_energy_soil.crystal"
            } else if soil_is_qualified {
                "oteryn:elemental_soil.crystal"
            } else if seed_is_qualified {
                "oteryn:planting.crystal"
            } else {
                "oteryn:gathering.crystal"
            }
            .into(),
            outputs,
        })
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
                    let direct = sqlx::query("SELECT uuid_send(e.parent_item_instance_id) AS parent,i.quantity,i.definition_family,i.definition_production_key,i.definition_revision_ref,(SELECT f.fluid_subtype FROM game_item_fluid_states f WHERE f.item_instance_id=i.item_instance_id FOR UPDATE) AS fluid_subtype FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) WHERE i.item_instance_id=encode($1,'hex')::uuid AND e.character_id=encode($2,'hex')::uuid AND i.world_id=encode($3,'hex')::uuid AND i.lifecycle=1 AND i.quantity>0 FOR UPDATE OF i")
                        .bind(ingredient_id.as_slice()).bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).fetch_optional(&mut ***tx).await?;
                    let expected_ground = ground.as_ref().and_then(|ctx| ctx.operand.target.as_ref());
                    let ingredient = match (direct, carried_sculpture, expected_ground) {
                        (None, true, Some(target)) => sculpture::lock_ground_material(tx, fence, ingredient_id, target, ground.as_ref().ok_or(DurabilityError::InvalidStoredState)?.operand).await?,
                        (None, true, None) => {
                            let nested = event_containers::nested_source(tx, fence, ingredient_id, *world.as_bytes()).await?;
                            if let Some(row) = &nested {
                                ingredient_location = row.try_get("nested_source")?;
                            }
                            nested
                        }
                        (direct, _, None) => direct,
                        _ => None,
                    };
                    if expected_ground.is_some() {
                        ingredient_location = ingredient.as_ref().map(|row| row.try_get("ground_source")).transpose()?;
                    }
                    ingredient.and_then(|row| {
                        if expected_ground.is_none()
                            && row.try_get::<Vec<u8>, _>("parent").ok()?
                                != source.try_get::<Vec<u8>, _>("parent").ok()?
                        {
                            return None;
                        }
                        let target = TypedDefinitionRef {
                            family: row.try_get("definition_family").ok()?,
                            production_key: row.try_get("definition_production_key").ok()?,
                            revision_ref: row.try_get("definition_revision_ref").ok()?,
                        };
                        let quantity =
                            u32::try_from(row.try_get::<i64, _>("quantity").ok()?).ok()?;
                        if carried_sculpture {
                            if &target != station || (expected_ground.is_some() && quantity != 1) {
                                return None;
                            }
                            let input = source_item_id(&target)?;
                            let tool = source_item_id(&definition)?;
                            let roll = gathering_roll(
                                native.source_digest(),
                                fence.game_session_id,
                                request.command.command_id().get(),
                                &request.intent,
                            )?;
                            let attempt = fishing::gathering_attempt(tool, input, 10, 1, roll)?;
                            // Crystal removes carried ice, then creates the next stage
                            // at the target position. Container addresses have no tile.
                            let output = if expected_ground.is_some() {
                                attempt.after.or(attempt.yield_item)
                            } else {
                                attempt.yield_item
                            };
                            let outputs = gathering_outputs(native, output)?;
                            ingredient_consumed = Some((target, quantity == 1));
                            carried_attempt = Some((input, attempt));
                            return Some(CookingConversion {
                                recipe: "oteryn:sculpture.carried".into(),
                                outputs,
                            });
                        }
                        let resolved = carried_cooking_conversion_with_quantity(
                            native,
                            &definition,
                            &target,
                            ingredient_fluid_subtype(
                                row.try_get::<Option<i16>, _>("fluid_subtype").ok()?,
                            ),
                            quantity,
                        )?;
                        content.loop_facts(&target)?.weight?;
                        ingredient_count = i64::from(resolved.target_debit);
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
    if conversion.recipe == "oteryn:quest-craft/rottin-wood/lucky-charm"
        && !rottin_charm_hunt_active(tx, fence).await?
    {
        return Ok(UseDisposition::NothingToUse);
    }
    if conversion.recipe == "oteryn:elemental_energy_soil.crystal"
        && !elemental_energy::has_room(
            tx,
            permit,
            fence,
            &request
                .gathering
                .as_ref()
                .ok_or(DurabilityError::InvalidStoredState)?
                .mutation
                .placement,
        )
        .await?
    {
        return Ok(UseDisposition::NotPossible);
    }
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
    let retires_ingredient = ingredient_consumed
        .as_ref()
        .is_some_and(|(_, retires)| *retires);
    let parent: Vec<u8> = source.try_get("parent")?;
    let occupied: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid").bind(&parent).fetch_one(&mut ***tx).await?;
    let tree_count: i64 = sqlx::query_scalar(
        "WITH RECURSIVE tree(item_instance_id,depth) AS ( \
           SELECT item_instance_id,1 FROM game_item_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid \
           UNION ALL SELECT e.item_instance_id,t.depth+1 FROM tree t JOIN game_item_nested_entries e \
             ON e.parent_item_instance_id=t.item_instance_id WHERE t.depth<8 \
         ) SELECT count(*) FROM (SELECT item_instance_id FROM tree LIMIT 501) bounded",
    ).bind(&parent).fetch_one(&mut ***tx).await?;
    let marble_yield = carried_attempt
        .is_some_and(|(input, attempt)| input == 10426 && attempt.yield_item.is_some())
        || request
            .gathering
            .as_ref()
            .is_some_and(|g| g.mutation.before == 10426 && g.attempt.yield_item.is_some());
    // Crystal adds the figurine before removing the marble. The marble
    // still occupies its slot, counts toward the tree limit, and weighs
    // the same when choosing between carried and Ground custody.
    let retired_carried_ingredient = !marble_yield
        && retires_ingredient
        && ground
            .as_ref()
            .is_none_or(|ctx| ctx.operand.target.is_none());
    let output_count =
        i64::try_from(conversion.outputs.len()).map_err(|_| DurabilityError::InvalidStoredState)?;
    let tree_full = tree_count + output_count
        - i64::from(retires_source)
        - i64::from(retired_carried_ingredient)
        > 500;
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
    let bag_full = occupied + output_count
        - i64::from(frees_output_slot)
        - i64::from(retired_carried_ingredient && ingredient_location.is_none())
        > i64::from(capacity);
    let nested_destination = if marble_yield
        && !tree_full
        && let [(output, _)] = conversion.outputs.as_slice()
    {
        sculpture::nested_marble_destination(
            tx, fence.character_id, &parent, !bag_full, output, content,
        ).await?
    } else {
        None
    };
    let no_carried_room = tree_full || (bag_full && nested_destination.is_none());
    // The donor tries to give the present's gift before removing the present.
    let drop = blue_present.filter(|_| {
        occupied + output_count > i64::from(capacity)
            && conversion.recipe == event_containers::BLUE_PRESENT_RECIPE
            && definition.production_key == BLUE_PRESENT
            && consumed_quantity == 1
            && request.ingredient.is_none()
            && output_count == 1
    });
    if no_carried_room && ground.is_none() && drop.is_none() {
        return Ok(UseDisposition::NoRoom);
    }
    let overweight = if drop.is_none() {
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
            Some((target, _))
                if !marble_yield
                    && ground
                        .as_ref()
                        .is_none_or(|ctx| ctx.operand.target.is_none()) =>
            {
                let Some(weight) = content.loop_facts(target).and_then(|facts| facts.weight) else {
                    return Ok(UseDisposition::NotPossible);
                };
                u64::from(weight) * u64::from(ingredient_debit)
            }
            _ => 0,
        };
        let capacity = read_derived_transfer_capacity_centi_oz(tx, fence.character_id, content).await?;
        carried
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
    } else {
        false
    };
    let ground_output = ground.as_ref().and_then(|ctx| {
        let ground_ice =
            ctx.operand.target.is_some() && carried_attempt.is_some_and(|(id, _)| id != 10426);
        if ground_ice || no_carried_room || overweight {
            Some(ctx.placement(ground_ice))
        } else {
            None
        }
    });
    if overweight && ground_output.is_none() {
        return Ok(UseDisposition::NotPossible);
    }
    let nested_destination = nested_destination.filter(|_| ground_output.is_none());
    let (transaction, event, occurred): (String, String, i64) = sqlx::query_as("SELECT game_character_uuid_v7()::text,game_character_uuid_v7()::text,(extract(epoch FROM clock_timestamp())*1000)::bigint").fetch_one(&mut ***tx).await?;
    let mut ordinal = if drop.is_none() {
        let highest: String = sqlx::query_scalar("SELECT coalesce(max(placement_ordinal),0)::text FROM game_item_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid").bind(&parent).fetch_one(&mut ***tx).await?;
        highest
            .parse::<u64>()
            .map_err(|_| DurabilityError::InvalidStoredState)?
    } else {
        0
    };
    let mut outputs = Vec::new();
    for (position, (definition, count)) in conversion.outputs.iter().enumerate() {
        let item: String = sqlx::query_scalar("SELECT game_character_uuid_v7()::text")
            .fetch_one(&mut ***tx)
            .await?;
        let mut output = json!({"item": item, "key": definition.production_key, "revision": definition.revision_ref, "count": count});
        let nested = nested_destination.as_ref();
        let output_ordinal = if let Some(drop) = &drop {
            let Some(policy) =
                native.item_policy(&definition.production_key, &definition.revision_ref)
            else {
                return Ok(UseDisposition::NotPossible);
            };
            let Ok(qualified) =
                super::spell_items_abi::QualifiedItemDefinition::from_native_policy(policy)
            else {
                return Ok(UseDisposition::NotPossible);
            };
            if !qualified.ground_destination || *count == 0 || *count > qualified.stack_maximum {
                return Ok(UseDisposition::NotPossible);
            }
            let ground = match drop
                .admit(tx, permit, fence, &transaction, &item, capacity)
                .await?
            {
                Ok(ground) => ground,
                Err(disposition) => return Ok(disposition),
            };
            output["ground"] = ground;
            sqlx::query_scalar::<_, i64>("SELECT nextval('game_item_ground_stack_ordinal_seq')")
                .fetch_one(&mut ***tx)
                .await?
                .to_string()
        } else if let Some((_, ordinal)) = nested {
            ordinal.to_string()
        } else if frees_output_slot && position == 0 {
            source.try_get::<String, _>("ordinal")?
        } else {
            ordinal = ordinal
                .checked_add(1)
                .ok_or(DurabilityError::InvalidStoredState)?;
            ordinal.to_string()
        };
        output["ordinal"] = json!(output_ordinal);
        if let Some(placement) = &ground_output {
            output["ground"] = sculpture::ground_proof(fence, placement)?;
        } else if let Some((parent, _)) = nested {
            output["nested_parent"] = json!(parent);
        }
        outputs.push(output);
    }
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
                .map(|before| (before, ingredient_count, ingredient_location.as_ref())),
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
    sqlx::query("INSERT INTO game_item_craft_records(transaction_id,event_id,game_session_id,command_id,character_id,world_id,intent,recipe,source_item_instance_id,parent_item_instance_id,source_ordinal,before_item,outputs,occurred_at,envelope_sha256,consume_source,ingredient_item_instance_id,ingredient_before_item,event_source_location,ingredient_count,ingredient_consumed_quantity,ingredient_location) VALUES($1::uuid,$2::uuid,encode($3,'hex')::uuid,$4::text::numeric,encode($5,'hex')::uuid,encode($6,'hex')::uuid,$7,$8,encode($9,'hex')::uuid,encode($10,'hex')::uuid,$11::text::numeric,$12,$13,$14,sha256($15),$16,encode($17,'hex')::uuid,$18,$19,$20,$21,$22)")
        .bind(&transaction).bind(&event).bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string()).bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).bind(&request.intent).bind(&conversion.recipe).bind(request.source.as_slice()).bind(&parent).bind(source.try_get::<String,_>("ordinal")?).bind(&before).bind(json!(outputs)).bind(occurred).bind(&envelope).bind(consumes_source).bind(request.ingredient.map(|id| id.to_vec())).bind(ingredient_before).bind(&nested_source).bind(i32::try_from(ingredient_count).map_err(|_| DurabilityError::InvalidStoredState)?).bind(i64::from(ingredient_debit)).bind(&ingredient_location).execute(&mut ***tx).await?;
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
        if retires_ingredient {
            let removal = match &ingredient_location {
                Some(location) if location.get("spatial_position").is_some() =>
                    "DELETE FROM game_item_ground_locations WHERE item_instance_id=encode($1,'hex')::uuid",
                Some(_) => "DELETE FROM game_item_nested_entries WHERE item_instance_id=encode($1,'hex')::uuid",
                None => "DELETE FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid",
            };
            sqlx::query(removal).bind(ingredient.as_slice()).execute(&mut ***tx).await?;
        }
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
    let gathering_award = request
        .gathering
        .as_ref()
        .map(|g| (g.mutation.tool_key.as_str(), g.mutation.before, g.attempt))
        .or_else(|| {
            carried_attempt.map(|(input, attempt)| {
                (
                    definition.production_key.as_str(),
                    u32::from(input),
                    attempt,
                )
            })
        });
    let figurine = gathering_award
        .filter(|(_, before, _)| *before == 10426)
        .and_then(|(_, _, attempt)| attempt.yield_item);
    let inscription = if let Some(figurine) = figurine {
        let name: String = sqlx::query_scalar(
            "SELECT name FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
        )
        .bind(fence.character_id.as_bytes().as_slice())
        .fetch_one(&mut ***tx)
        .await?;
        fishing::marble_inscription(figurine, &name)
    } else {
        None
    };
    for output in outputs {
        if let (Some(ctx), Some(placement)) = (&ground, &ground_output)
            && let Err(disposition) = ctx.check_room(tx, permit, fence, placement, &output).await?
        {
            return Ok(disposition);
        }
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(($1->>'item')::uuid,encode($2,'hex')::uuid,'Item',$1->>'key',$1->>'revision',($1->>'count')::bigint,1,$3::uuid)")
            .bind(&output).bind(world.as_bytes().as_slice()).bind(&transaction).execute(&mut ***tx).await?;
        if let Some(placement) = &ground_output {
            sculpture::place_ground(tx, permit, fence, &output, placement).await?;
        } else if drop.is_some() {
            event_containers::place_drop(tx, permit, &output).await?;
        } else if nested_destination.is_some() {
            sqlx::query("INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(($1->>'item')::uuid,encode($2,'hex')::uuid,($1->>'nested_parent')::uuid,($1->>'ordinal')::numeric,$3::uuid)")
                .bind(&output).bind(world.as_bytes().as_slice()).bind(&transaction).execute(&mut ***tx).await?;
        } else {
            sqlx::query("INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(($1->>'item')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,($1->>'ordinal')::numeric,$5::uuid)")
            .bind(&output).bind(world.as_bytes().as_slice()).bind(fence.character_id.as_bytes().as_slice()).bind(&parent).bind(&transaction).execute(&mut ***tx).await?;
        }
        if let Some(inscription) = &inscription {
            let item = output["item"]
                .as_str()
                .ok_or(DurabilityError::InvalidStoredState)?;
            super::item_text::initialize_authored_reward_text(
                tx,
                super::item_mint::uuid_text(item)?,
                super::item_mint::uuid_text(&transaction)?,
                inscription,
                occurred,
            )
            .await?;
        }
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
    let achievement = if let Some((tool, before, attempt)) = gathering_award {
        let (award, progress) = match (tool, before, attempt.after, attempt.yield_item) {
            // Crystal 201189f8, skinning.lua:153-191. Earlier stages or
            // lower-quality figurines do not earn either sculpture award.
            ("oteryn:item.tibia.i5908" | "oteryn:item.tibia.i5942", 7445, Some(7446), None) => (
                Some("oteryn:achievement/ice_sculptor"),
                Some(achievement_granters::COLD_AS_ICE),
            ),
            ("oteryn:item.tibia.i5908" | "oteryn:item.tibia.i5942", 10426, None, Some(10429)) => (
                Some("oteryn:achievement/marblelous"),
                Some(achievement_granters::MARBLE_MADNESS),
            ),
            // Crystal data-crystal/scripts/lib/register_actions.lua:418-444
            // counts successful scythe and sickle harvests. The Global sickle
            // operates on 5463, per data/scripts/actions/tools/sickle.lua:4-7.
            (_, 3653, Some(3651), Some(3605)) => (None, Some(achievement_granters::HAPPY_FARMER)),
            (_, 5463, Some(5462), Some(5466)) | (_, 5464, Some(5463), Some(5466)) => {
                (None, Some(achievement_granters::NATURAL_SWEETENER))
            }
            _ => (
                None,
                achievement_granters::gathering(tool, attempt.yield_item),
            ),
        };
        if let Some(key) = award {
            super::account_achievement::grant_missing(
                tx,
                fence.character_id,
                &[key],
                achievement_granters::CRAFT_SOURCE_KIND,
                event.as_bytes(),
            )
            .await?;
        }
        progress
    } else if conversion.recipe.starts_with("oteryn:reward-bag.") {
        achievement_granters::christmas_bundle(&definition.production_key)
    } else if conversion.recipe.starts_with("oteryn:item-conversion.cookie-tray-") {
        // TibiaWiki page 62446, revision 753128: one bake, regardless of cookie count.
        Some(achievement_granters::COOKIE_MONSTER)
    } else if conversion.recipe == "oteryn:fishing.global.standard" {
        Some(achievement_granters::HERE_FISHY_FISHY)
    } else {
        achievement_granters::baking(&conversion.recipe)
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
    if conversion.recipe == "oteryn:elemental_energy_soil.crystal" {
        elemental_energy::create(tx, permit, fence, request, native, &transaction, occurred)
            .await?;
    }
    Ok(UseDisposition::Committed)
}

fn gathering_outputs(
    native: &NativeGameplayState,
    output: Option<u16>,
) -> Option<Vec<(TypedDefinitionRef, u32)>> {
    let Some(item) = output else {
        return Some(Vec::new());
    };
    let definition = &native
        .item_policy_for_source_id(u32::from(item))?
        .record()
        .production_definition;
    Some(vec![(
        TypedDefinitionRef {
            family: definition.family.clone(),
            production_key: definition.production_key.clone(),
            revision_ref: definition.revision_ref.clone(),
        },
        1,
    )])
}

/// Shared deterministic draw for map and carried gathering targets.
pub fn gathering_roll(
    digest: [u8; 32],
    session: crate::foundation::GameSessionId,
    command_id: u64,
    intent: &[u8],
) -> Option<u32> {
    let digest = Sha256::new()
        .chain_update(b"oteryn.gathering.yield.v1")
        .chain_update(session.as_bytes())
        .chain_update(command_id.to_be_bytes())
        .chain_update(intent)
        .chain_update(digest)
        .finalize();
    let mut occurrence = [0; 16];
    occurrence.copy_from_slice(&digest[..16]);
    let draw = deterministic_decision_u64(
        &GameplayDecisionRoot::from_bytes(digest.into()),
        DecisionOccurrenceId::from_bytes(occurrence),
        "gathering.yield",
        0,
    )
    .ok()?;
    u32::try_from(oteryn_gameplay_domain::domain::uniform_draw::uniform_draw(
        draw, 1, 100_000,
    ))
    .ok()
}

fn source_item_id(definition: &TypedDefinitionRef) -> Option<u16> {
    definition.production_key.strip_prefix("oteryn:item.tibia.i")?.parse().ok()
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
