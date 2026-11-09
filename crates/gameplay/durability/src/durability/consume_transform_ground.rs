//! Exact current Ground custody for source-authored item actions on the existing receipt.
use super::*;
use crate::durability::item_loop_move::LoopGround;
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementLocalPosition};
use oteryn_gameplay_items::novelty_items::{self, GroundEffect};

/// A read-only snapshot from the actual Item owner. Its item and location are
/// compared in full again under the writer's locks before any durable mutation.
#[derive(Clone, Debug)]
pub struct GroundActionSource {
    item: [u8; 16],
    definition: TypedDefinitionRef,
    before: Value,
    location: Value,
    corpse_ref: Vec<u8>,
    position: MovementLocalPosition,
    target: LoopGround,
    scope: crate::foundation::RuntimeScopeRefV1,
    generation: u64,
    map_mint: bool,
    action_ground_output: bool,
}
impl GroundActionSource {
    pub fn definition(&self) -> &TypedDefinitionRef {
        &self.definition
    }
    pub fn position(&self) -> MovementLocalPosition {
        self.position
    }
    /// Read-only operand for the quest participant, which rereads custody and
    /// corpse provenance inside its own fenced transaction before any mutation.
    pub fn quest_corpse_operand(&self) -> crate::durability::reward_claim_mint::quest_creature::QuestCorpseOperand {
        crate::durability::reward_claim_mint::quest_creature::QuestCorpseOperand {
            item: self.item,
            ground: crate::durability::spell_items_abi::SpellGroundTarget {
                spatial_position: self.target.spatial_position.clone(),
                map_revision: self.target.map_revision.clone(),
                content_revision: self.target.content_revision.clone(),
                placement_context: self.target.placement_context.clone(),
            },
        }
    }
}

/// Constructed only from a current native cell and an actual positioned actor.
/// House membership belongs to the source tile, as in anniversary_balloons.lua.
#[derive(Clone, Debug)]
pub struct GroundActionContext {
    source: GroundActionSource,
    house: Option<String>,
}
impl GroundActionContext {
    pub fn from_native_owner(
        room: &crate::content::QualifiedNativeEntryRoom,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        source: GroundActionSource,
    ) -> Result<Self, &'static str> {
        let Some((positioned, current)) = runtime
            .positioned_player_for_session(session)
            .map_err(|_| "ground action actor")?
        else {
            return Err("ground action actor missing");
        };
        let at = current.position();
        if positioned != actor
            || source.scope != runtime.binding().runtime_scope()
            || source.generation != runtime.binding().scope_generation().get()
            || source.position.floor != at.floor
            || (i64::from(source.position.x) - i64::from(at.x)).abs() > 1
            || (i64::from(source.position.y) - i64::from(at.y)).abs() > 1
        {
            return Err("ground action source is not adjacent in current scope");
        }
        let mut target = LoopGround::from_native_owner(
            room,
            runtime,
            source.position,
            &source.target.map_revision,
        )?;
        if target != source.target {
            // Ordinary loop drops keep the qualified frame, including action
            // outputs later moved by that owner. A map mint or its immutable
            // Ground-action output may retain the original base-bundle encoding.
            // The action proof follows minted_transaction_id, so its original
            // lineage remains proven after this same Item's temporal stages.
            if !source.map_mint && !source.action_ground_output {
                return Err("ground action source lacks base-context receipt");
            }
            // The canonical map-mint codec names the base as sha256:<base>.
            // LoopGround above independently qualified that exact raw revision
            // against the room's activated map pin. All remaining custody fields
            // still have to match the complete qualified target below.
            if source.target.placement_context.len() != 32
                || crate::durability::map_item_mint_audit::map_revision_of(
                    &source.target.placement_context,
                ) != source.target.map_revision
            {
                return Err("map-mint source bundle differs");
            }
            target.placement_context = source.target.placement_context.clone();
        }
        if target != source.target {
            return Err("ground action source generation differs");
        }
        let cells = room.movement_cells();
        let cell = crate::content::LogicalCell {
            x: source.position.x,
            y: source.position.y,
            z: i32::from(source.position.floor),
        };
        let house = match cells.house_tiles().house_at(cells.scope(), cell) {
            Ok(house) => house.map(|house| house.house().key.clone()),
            Err(_)
                if novelty_items::house_transform_for_key(&source.definition.production_key)
                    .is_none() =>
            {
                None
            }
            Err(_) => return Err("ground action House membership unavailable"),
        };
        Ok(Self { source, house })
    }
    pub(super) async fn dispenser_row(
        &self,
        tx: &mut crate::durability::item_mint_audit::Type2Transaction<'_>,
        fence: CurrentCharacterItemFence,
        item: [u8; 16],
    ) -> Result<Option<sqlx::postgres::PgRow>, DurabilityError> {
        if item != self.source.item
            || self.source.scope != fence.runtime_scope
            || self.source.generation != fence.scope_ownership_generation.get()
        {
            return Ok(None);
        }
        sqlx::query("SELECT item_instance_id FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid FOR UPDATE")
            .bind(item.as_slice()).fetch_optional(&mut ***tx).await?;
        let (world, channel) = scope_of(&fence).map_err(|_| DurabilityError::InvalidStoredState)?;
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended(encode($1,'hex') || encode($2,'hex'),33))")
            .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).execute(&mut ***tx).await?;
        sqlx::query("SELECT to_jsonb(i) AS before_item,i.definition_family,i.definition_production_key,i.definition_revision_ref,decode(repeat('00',16),'hex') AS parent,g.stack_ordinal::text AS ordinal,3::integer AS source_kind,to_jsonb(g) AS ground_location FROM game_item_instances i JOIN game_item_ground_locations g USING(item_instance_id,world_id) WHERE i.item_instance_id=encode($1,'hex')::uuid AND i.lifecycle=1 AND i.quantity=1 AND to_jsonb(i)=$2 AND to_jsonb(g)=$3 FOR UPDATE OF i")
            .bind(item.as_slice()).bind(&self.source.before).bind(&self.source.location).fetch_optional(&mut ***tx).await.map_err(Into::into)
    }
    pub fn in_house(&self) -> bool {
        self.house.is_some()
    }
    pub fn effect_position(&self) -> GroundEffect {
        GroundEffect {
            x: self.source.position.x,
            y: self.source.position.y,
            floor: self.source.position.floor,
        }
    }
}

impl DurabilityRoot {
    pub async fn read_ground_action_source(
        &self,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        fence: CurrentCharacterItemFence,
        item: [u8; 16],
    ) -> Result<Option<GroundActionSource>, DurabilityError> {
        let Ok(_) = scope_of(&fence) else {
            return Ok(None);
        };
        let record = recovery
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = crate::durability::db::begin_semantic_transaction(holder, deadline).await?;
            crate::durability::character_authority::assert_recovery_fence(&mut tx, &record).await?;
            // Reuse the existing Ground owner lock before any item/location row lock.
    let (world, channel) = scope_of(&fence).map_err(|_| DurabilityError::InvalidStoredState)?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended(encode($1,'hex') || encode($2,'hex'),33))")
        .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).execute(&mut *tx).await?;
    let row = sqlx::query("SELECT to_jsonb(i) AS before_item,to_jsonb(g) AS location,i.definition_family,i.definition_production_key,i.definition_revision_ref,g.spatial_position,g.corpse_ref,g.map_revision,g.content_revision,g.native_room_placement_context,game_item_loop_map_mint_source(i.item_instance_id) AS map_mint,game_item_action_ground_source(i.item_instance_id) AS action_ground_output FROM game_item_instances i JOIN game_item_ground_locations g USING(item_instance_id,world_id) WHERE i.item_instance_id=encode($1,'hex')::uuid AND i.world_id=encode($2,'hex')::uuid AND g.channel_id=encode($3,'hex')::uuid AND g.runtime_scope_ownership_generation=$4::text::numeric AND i.lifecycle=1 AND i.quantity>0 AND g.stack_ordinal IS NOT NULL")
                .bind(item.as_slice()).bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice())
                .bind(fence.scope_ownership_generation.get().to_string()).fetch_optional(&mut *tx).await?;
            let result = row.map(|row| -> Result<GroundActionSource, DurabilityError> {
                let position: Vec<u8> = row.try_get("spatial_position")?;
                let cell = crate::durability::spell_items_abi::decode_ground_cell(&position)
                    .map_err(|_| DurabilityError::InvalidStoredState)?;
                Ok(GroundActionSource {
                    item, definition: TypedDefinitionRef { family: row.try_get("definition_family")?,
                        production_key: row.try_get("definition_production_key")?, revision_ref: row.try_get("definition_revision_ref")? },
                    before: row.try_get("before_item")?, location: row.try_get("location")?,
                    corpse_ref: row.try_get("corpse_ref")?,
                    position: MovementLocalPosition { x: cell.x, y: cell.y,
                        // Stored loop custody uses native -z; actor/effect facts use runtime z.
                        floor: i16::try_from(cell.z).ok().and_then(i16::checked_neg)
                            .ok_or(DurabilityError::InvalidStoredState)? },
                    target: LoopGround { spatial_position: position, map_revision: row.try_get("map_revision")?,
                        content_revision: row.try_get("content_revision")?, placement_context: row.try_get("native_room_placement_context")? },
                    scope: fence.runtime_scope, generation: fence.scope_ownership_generation.get(),
                    map_mint: row.try_get("map_mint")?,
                    action_ground_output: row.try_get("action_ground_output")?,
                })
            }).transpose()?;
            crate::durability::db::commit_semantic_transaction(tx, deadline).await?;
            Ok(result)
        })).await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn consume_dispenser_on_ground<C: ItemLoopContent>(
        &self,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: ConsumeTransformRequest,
        native: &NativeGameplayState,
        content: &C,
        target: &GroundActionContext,
    ) -> Result<Option<ConsumeTransformResult>, DurabilityError> {
        self.consume_transform_inner(
            permit,
            recovery,
            node,
            fence,
            request,
            native,
            content,
            false,
            None,
            ActionContext::DispenserTarget(target),
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn consume_transform_on_ground<C: ItemLoopContent>(
        &self,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: ConsumeTransformRequest,
        native: &NativeGameplayState,
        content: &C,
        source: &GroundActionContext,
    ) -> Result<Option<ConsumeTransformResult>, DurabilityError> {
        self.consume_transform_inner(
            permit,
            recovery,
            node,
            fence,
            request,
            native,
            content,
            false,
            None,
            ActionContext::Ground(source),
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn consume_transform_on_ground_with_cosmetic_premium<C: ItemLoopContent>(
        &self,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: ConsumeTransformRequest,
        native: &NativeGameplayState,
        content: &C,
        source: &GroundActionContext,
        premium: Option<&dyn crate::durability::consume_transform_cosmetics::ItemCosmeticPremium>,
    ) -> Result<Option<ConsumeTransformResult>, DurabilityError> {
        self.consume_transform_inner(
            permit,
            recovery,
            node,
            fence,
            request,
            native,
            content,
            false,
            premium,
            ActionContext::Ground(source),
        )
        .await
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn write<C: ItemLoopContent>(
    permit: &crate::durability::spell_owner_commit::SpellLanePermit,
    tx: &mut crate::durability::item_mint_audit::Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    request: &ConsumeTransformRequest,
    native: &NativeGameplayState,
    content: &C,
    context: &GroundActionContext,
    premium: Option<&dyn crate::durability::consume_transform_cosmetics::ItemCosmeticPremium>,
    ground_fences: &ActionGroundFences<'_>,
) -> Result<ConsumeTransformResult, DurabilityError> {
    permit.check_scope(fence.runtime_scope)?;
    let source = &context.source;
    if source.item != request.source
        || source.scope != fence.runtime_scope
        || source.generation != fence.scope_ownership_generation.get()
        || (request.target.is_some()
            && !matches!(
                source.definition.production_key.as_str(),
                "oteryn:item.tibia.i306" | "oteryn:item.tibia.i14029"
            ))
    {
        return Ok(ConsumeTransformResult::refused(UseDisposition::Rejected));
    }
    // Follow the existing Ground writer order: Character root, source item, then
    // the Ground owner lock. Re-read custody in a new statement after lock waits.
    // Ground rows deliberately have no runtime UPDATE privilege.
    let (world, channel) = scope_of(&fence).map_err(|_| DurabilityError::InvalidStoredState)?;
    let locked = sqlx::query("SELECT item_instance_id FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(request.source.as_slice()).fetch_optional(&mut ***tx).await?;
    if locked.is_none() {
        return Ok(ConsumeTransformResult::refused(UseDisposition::StaleState));
    }
    sqlx::query(
        "SELECT pg_advisory_xact_lock(hashtextextended(encode($1,'hex') || encode($2,'hex'),33))",
    )
    .bind(world.as_bytes().as_slice())
    .bind(channel.as_bytes().as_slice())
    .execute(&mut ***tx)
    .await?;
    let row = sqlx::query("SELECT to_jsonb(i) AS before_item,to_jsonb(g) AS location FROM game_item_instances i JOIN game_item_ground_locations g USING(item_instance_id,world_id) WHERE i.item_instance_id=encode($1,'hex')::uuid FOR UPDATE OF i")
        .bind(request.source.as_slice()).fetch_optional(&mut ***tx).await?;
    let Some(row) = row else {
        return Ok(ConsumeTransformResult::refused(UseDisposition::StaleState));
    };
    if row.try_get::<Value, _>("before_item")? != source.before
        || row.try_get::<Value, _>("location")? != source.location
    {
        return Ok(ConsumeTransformResult::refused(UseDisposition::StaleState));
    }
    if oteryn_gameplay_items::event_items::is_event_reward(&source.definition.production_key) {
        return write_reward(tx, fence, request, native, content, context, ground_fences).await;
    }
    use crate::durability::consume_transform_cosmetics::{self as cosmetics, Preparation};
    let plan = if request.target.is_some() {
        let Some(plan) = super::planting::prepare_ground(
            tx,
            fence,
            request,
            &source.definition,
            native,
            context.effect_position(),
        )
        .await?
        else {
            return Ok(ConsumeTransformResult::refused(
                UseDisposition::NothingToUse,
            ));
        };
        plan
    } else {
        match cosmetics::prepare_on_ground(tx, fence, request, &source.definition, native, premium)
            .await?
        {
            Preparation::Refused(disposition) => {
                return Ok(ConsumeTransformResult::refused(disposition));
            }
            Preparation::Ready(plan) => plan,
            Preparation::Unrelated => {
                let Some(plan) = novelty_items::plan_ground(
                    native,
                    &source.definition,
                    *fence.game_session_id.as_bytes(),
                    request.command.command_id().get(),
                    request.source,
                    context.effect_position(),
                    context.in_house(),
                ) else {
                    return Ok(ConsumeTransformResult::refused(UseDisposition::NotPossible));
                };
                ActionPlan {
                    recipe: plan.recipe,
                    outputs: plan.outputs,
                    consume_source: plan.consume_source,
                    source_count: u32::from(plan.consume_source),
                    effects: plan.effects,
                    extra_inputs: Vec::new(),
                }
            }
        }
    };
    let source_ordinal = source
        .location
        .get("stack_ordinal")
        .and_then(Value::as_i64)
        .filter(|ordinal| *ordinal > 0)
        .ok_or(DurabilityError::InvalidStoredState)?
        .to_string();
    let mut outputs = Vec::new();
    if !plan.outputs.is_empty() && source.before.get("quantity").and_then(Value::as_i64) != Some(1)
    {
        return Ok(ConsumeTransformResult::refused(UseDisposition::NotPossible));
    }
    for (definition, count) in &plan.outputs {
        if *count != 1 {
            return Err(DurabilityError::InvalidStoredState);
        }
        let policy = native
            .item_policy(&definition.production_key, &definition.revision_ref)
            .ok_or(DurabilityError::InvalidStoredState)?;
        let qualified =
            crate::durability::spell_items_abi::QualifiedItemDefinition::from_native_policy(policy)
                .map_err(|_| DurabilityError::InvalidStoredState)?;
        if !qualified.ground_destination
            || qualified.stack_maximum != 1
            || qualified.container_capacity.is_some()
        {
            return Ok(ConsumeTransformResult::refused(UseDisposition::NotPossible));
        }
        let decay = if let Some(expected) = plan
            .effects
            .get("timed_output")
            .filter(|value| !value.is_null())
        {
            let decay = qualified.decay.ok_or(DurabilityError::InvalidStoredState)?;
            let target = expected
                .get("decay_to")
                .and_then(Value::as_u64)
                .map(|id| format!("oteryn:item.tibia.i{id}"));
            if expected.get("duration_ms").and_then(Value::as_u64)
                != Some(u64::from(decay.duration_millis))
                || decay.target.as_ref().map(|target| &target.production_key) != target.as_ref()
            {
                return Ok(ConsumeTransformResult::refused(UseDisposition::NotPossible));
            }
            Some(
                json!({"duration_millis": decay.duration_millis, "target": decay.target.as_ref().map(|target|
                json!({"family": target.family, "key": target.production_key, "revision": target.revision_ref}))}),
            )
        } else if plan.recipe == "oteryn:planting.ice-flower" {
            if qualified.decay.as_ref().is_none_or(|decay| {
                decay.duration_millis != 72_000_000
                    || decay
                        .target
                        .as_ref()
                        .map(|target| target.production_key.as_str())
                        != Some("oteryn:item.tibia.i14032")
            }) {
                return Ok(ConsumeTransformResult::refused(UseDisposition::NotPossible));
            }
            None // The two-stage field chain owns both original absolute deadlines.
        } else if qualified.decay.is_some() {
            return Ok(ConsumeTransformResult::refused(UseDisposition::NotPossible));
        } else {
            None
        };
        let item: String = sqlx::query_scalar("SELECT game_character_uuid_v7()::text")
            .fetch_one(&mut ***tx)
            .await?;
        let mut output = json!({"item": item, "key": definition.production_key, "revision": definition.revision_ref,
            "count": count, "ordinal": source_ordinal, "decay": decay});
        if plan.recipe == "oteryn:planting.ice-flower" {
            super::super::spell_item_temporal::event_items::decorate_output(native, &mut output)?;
        }
        outputs.push(output);
    }
    let mut location = source.location.clone();
    if let Some(house) = &context.house {
        location["qualified_house"] = json!(house);
    }
    let mut committed_request = request.clone();
    if matches!(
        plan.recipe.as_str(),
        "oteryn:planting.ice-flower" | "oteryn:planting.preserve"
    ) {
        // The exact clicked seed is the separately proven extra input. Preserve the
        // original bounded wire intent while the primary Ground receipt remains unary.
        committed_request.target = None;
    }
    persist_action(
        Some(ground_fences.permit),
        tx,
        fence,
        &committed_request,
        native,
        &plan,
        &source.before,
        None,
        None,
        3,
        &source_ordinal,
        20,
        &outputs,
        &location,
        None,
    )
    .await
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
async fn write_reward<C: ItemLoopContent>(
    tx: &mut crate::durability::item_mint_audit::Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    request: &ConsumeTransformRequest,
    native: &NativeGameplayState,
    content: &C,
    context: &GroundActionContext,
    ground_fences: &ActionGroundFences<'_>,
) -> Result<ConsumeTransformResult, DurabilityError> {
    let source = &context.source;
    let (world, _) = scope_of(&fence).map_err(|_| DurabilityError::InvalidStoredState)?;
    let Some(selected) = oteryn_gameplay_items::event_items::plan(
        native,
        &source.definition,
        *fence.game_session_id.as_bytes(),
        request.command.command_id().get(),
        request.source,
        Some(context.effect_position()),
    ) else {
        return Ok(ConsumeTransformResult::refused(UseDisposition::NotPossible));
    };
    let ground = oteryn_gameplay_items::event_items::ground_output_at_source(
        &source.definition.production_key,
    );
    let mut plan = ActionPlan {
        recipe: selected.recipe.clone(),
        outputs: Vec::new(),
        consume_source: true,
        source_count: if oteryn_gameplay_items::reward_bag::event_containers::consumes_whole_stack(
            &selected.recipe,
        ) {
            u32::try_from(
                source.before["quantity"]
                    .as_i64()
                    .ok_or(DurabilityError::InvalidStoredState)?,
            )
            .map_err(|_| DurabilityError::InvalidStoredState)?
        } else {
            1
        },
        effects: selected.effects,
        extra_inputs: Vec::new(),
    };
    let mut weight = 0_u64;
    for (definition, count) in selected.outputs {
        let Some(facts) = content.loop_facts(&definition) else {
            return Ok(ConsumeTransformResult::refused(UseDisposition::NotPossible));
        };
        let Some(unit_weight) = facts.weight else {
            return Ok(ConsumeTransformResult::refused(UseDisposition::NotPossible));
        };
        let maximum = stack_maximum(facts.stack)
            .map(|(maximum, _)| maximum)
            .unwrap_or(0);
        if maximum == 0 {
            return Ok(ConsumeTransformResult::refused(UseDisposition::NotPossible));
        }
        weight = weight
            .checked_add(u64::from(unit_weight) * u64::from(count))
            .ok_or(DurabilityError::InvalidStoredState)?;
        let mut remaining = count;
        while remaining > 0 {
            let count = maximum.min(remaining);
            plan.outputs.push((definition.clone(), count));
            remaining -= count;
        }
    }
    let (parent, capacity, mut highest) = if ground {
        // special_boxes.lua creates exactly one gift on the source tile. It gets a
        // new ordinal from the existing Ground sequence, unlike an in-place toy.
        if plan.outputs.len() != 1 || plan.outputs[0].1 != 1 {
            return Err(DurabilityError::InvalidStoredState);
        }
        (None, 20_u32, 0_u64)
    } else {
        let bag = sqlx::query("SELECT uuid_send(i.item_instance_id) AS item,i.definition_family,i.definition_production_key,i.definition_revision_ref FROM game_item_container_slots b JOIN game_item_instances i USING(item_instance_id) WHERE b.character_id=encode($1,'hex')::uuid AND b.world_id=encode($2,'hex')::uuid AND i.world_id=b.world_id AND i.lifecycle=1 FOR UPDATE OF i")
            .bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).fetch_optional(&mut ***tx).await?;
        let Some(bag) = bag else {
            return Ok(ConsumeTransformResult::refused(UseDisposition::NoRoom));
        };
        let parent: Vec<u8> = bag.try_get("item")?;
        let definition = TypedDefinitionRef {
            family: bag.try_get("definition_family")?,
            production_key: bag.try_get("definition_production_key")?,
            revision_ref: bag.try_get("definition_revision_ref")?,
        };
        let capacity = content
            .loop_facts(&definition)
            .and_then(|facts| facts.container_capacity)
            .unwrap_or(0);
        let (occupied, highest): (i64, String) = sqlx::query_as("SELECT count(*),coalesce(max(placement_ordinal),0)::text FROM (SELECT placement_ordinal FROM game_item_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid UNION ALL SELECT placement_ordinal FROM game_item_nested_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid) entries")
            .bind(&parent).fetch_one(&mut ***tx).await?;
        let tree: i64 =
            sqlx::query_scalar("SELECT game_item_loop_tree_size(encode($1,'hex')::uuid)")
                .bind(fence.character_id.as_bytes().as_slice())
                .fetch_one(&mut ***tx)
                .await?;
        let outputs =
            i64::try_from(plan.outputs.len()).map_err(|_| DurabilityError::InvalidStoredState)?;
        if occupied + outputs > i64::from(capacity) || tree + outputs > 500 {
            return Ok(ConsumeTransformResult::refused(UseDisposition::NoRoom));
        }
        let carried = match carried_weight(tx, fence.character_id, *world.as_bytes(), content).await
        {
            Ok(weight) => weight,
            Err(_) => return Ok(ConsumeTransformResult::refused(UseDisposition::NotPossible)),
        };
        let limit =
            read_derived_transfer_capacity_centi_oz(tx, fence.character_id, content).await?;
        if carried
            .checked_add(weight)
            .is_none_or(|total| total > limit)
        {
            return Ok(ConsumeTransformResult::refused(UseDisposition::NotPossible));
        }
        (
            Some(parent),
            capacity,
            highest
                .parse()
                .map_err(|_| DurabilityError::InvalidStoredState)?,
        )
    };
    let mut outputs = Vec::new();
    for (definition, count) in &plan.outputs {
        let policy = native
            .item_policy(&definition.production_key, &definition.revision_ref)
            .ok_or(DurabilityError::InvalidStoredState)?;
        let qualified =
            crate::durability::spell_items_abi::QualifiedItemDefinition::from_native_policy(policy)
                .map_err(|_| DurabilityError::InvalidStoredState)?;
        if (ground && !qualified.ground_destination)
            || (!ground && !qualified.inventory_destination)
            || *count > qualified.stack_maximum
            || (qualified.container_capacity.is_some() && *count != 1)
        {
            return Ok(ConsumeTransformResult::refused(UseDisposition::NotPossible));
        }
        let ordinal = if ground {
            sqlx::query_scalar::<_, i64>("SELECT nextval('game_item_ground_stack_ordinal_seq')")
                .fetch_one(&mut ***tx)
                .await?
                .to_string()
        } else {
            highest = highest
                .checked_add(1)
                .ok_or(DurabilityError::InvalidStoredState)?;
            highest.to_string()
        };
        let item: String = sqlx::query_scalar("SELECT game_character_uuid_v7()::text")
            .fetch_one(&mut ***tx)
            .await?;
        outputs.push(json!({"item": item, "key": definition.production_key, "revision": definition.revision_ref,
            "count": count, "ordinal": ordinal, "decay": null}));
    }
    let source_ordinal = source
        .location
        .get("stack_ordinal")
        .and_then(Value::as_i64)
        .filter(|ordinal| *ordinal > 0)
        .ok_or(DurabilityError::InvalidStoredState)?
        .to_string();
    persist_action(
        Some(ground_fences.permit),
        tx,
        fence,
        request,
        native,
        &plan,
        &source.before,
        None,
        parent.as_deref(),
        3,
        &source_ordinal,
        capacity,
        &outputs,
        &source.location,
        ground.then_some((ground_fences, context)),
    )
    .await
}

/// Reserve the exact new reward root through the existing native census. The
/// caller has already applied its real source debit under the Ground owner lock.
pub(super) async fn admit_reward_output(
    tx: &mut crate::durability::item_mint_audit::Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    fences: &ActionGroundFences<'_>,
    context: &GroundActionContext,
    output: &Value,
) -> Result<Option<UseDisposition>, DurabilityError> {
    use crate::durability::ground_cell_limit::{GroundCellAdmissionError, GroundCellLimitError};
    use crate::durability::spell_item_transaction::SpellItemError;

    let source = &context.source;
    if !oteryn_gameplay_items::event_items::ground_output_at_source(
        &source.definition.production_key,
    ) {
        return Err(DurabilityError::InvalidStoredState);
    }
    let placement = crate::durability::item_mint::GroundPlacement {
        spatial_position: source.target.spatial_position.clone(),
        corpse_ref: source.corpse_ref.clone(),
        map_revision: source.target.map_revision.clone(),
        content_revision: source.target.content_revision.clone(),
        native_room_placement_context: source.target.placement_context.clone(),
    };
    let item = crate::durability::item_mint::uuid_text(
        output
            .get("item")
            .and_then(Value::as_str)
            .ok_or(DurabilityError::InvalidStoredState)?,
    )?;
    match crate::durability::ground_cell_limit::check_ground_cell_root_insert_with_recovery(
        tx,
        fences.permit,
        fences.recovery,
        fences.node,
        (fence.runtime_scope, fence.scope_ownership_generation.get()),
        &placement,
        &item,
    )
    .await
    {
        Ok(()) => Ok(None),
        Err(GroundCellAdmissionError::Limit(GroundCellLimitError::NotEnoughRoom)) => {
            Ok(Some(UseDisposition::NoRoom))
        }
        Err(GroundCellAdmissionError::Limit(GroundCellLimitError::IncompatibleInput)) => {
            Ok(Some(UseDisposition::NotPossible))
        }
        Err(GroundCellAdmissionError::Source(
            SpellItemError::Rejected(_) | SpellItemError::FieldCapabilityRefused,
        )) => Ok(Some(UseDisposition::Rejected)),
        Err(GroundCellAdmissionError::Source(SpellItemError::Database(error))) => Err(error.into()),
        Err(GroundCellAdmissionError::Source(SpellItemError::Durability(error))) => Err(error),
        Err(GroundCellAdmissionError::Source(SpellItemError::Training(_))) => {
            Err(DurabilityError::Unavailable)
        }
    }
}
