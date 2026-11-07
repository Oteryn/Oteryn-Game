//! HouseInterior balloon USE: the same receipt/audit, with named-House custody retained.
use super::*;
#[path = "consume_transform_house_construction.rs"]
mod construction;
#[path = "consume_transform_house_cupcake.rs"]
mod cupcakes;
#[path = "consume_transform_house_dracoyle.rs"]
mod dracoyle;
#[path = "consume_transform_house_santa.rs"]
mod santa;
#[path = "consume_transform_house_static.rs"]
mod static_source;
use crate::durability::house_interior::{
    HouseInteriorSnapshot, load_house_interior_in_transaction,
};
use crate::durability::house_scope_handoff::HouseId;
use crate::durability::runtime_scope_assignment::prove_current_incarnation;
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, GameSessionId};
use oteryn_gameplay_items::novelty_items::{self, GroundEffect};

/// The actual House actor owner constructs this context. Neither wire positions nor
/// a Channel equipment fence can stand in for the current named-House lease.
#[derive(Clone, Debug)]
pub struct HouseActionContext {
    house: HouseId,
    position: [i32; 3],
    fence: CurrentCharacterItemFence,
    construction_cell_walkable: bool,
    server_artifact_digest: [u8; 32],
}

impl HouseActionContext {
    pub fn from_native_owner(
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        fence: CurrentCharacterItemFence,
    ) -> Result<Self, &'static str> {
        let state = runtime
            .house_actor_state(actor, session)
            .map_err(|_| "House actor missing")?;
        if session != fence.game_session_id
            || state.scope != fence.runtime_scope
            || state.generation != fence.scope_ownership_generation
            || state.lease.generation() != fence.character_lease_generation
            || state.lease.character_id().as_bytes() != fence.character_id.as_bytes()
        {
            return Err("House actor fence differs");
        }
        let house =
            HouseId::new(fence.runtime_scope.world_id(), &state.house).ok_or("House identity")?;
        if house.runtime_scope().map_err(|_| "House scope")? != fence.runtime_scope {
            return Err("House scope differs");
        }
        Ok(Self {
            house,
            position: [
                state.position.x,
                state.position.y,
                i32::from(state.position.floor),
            ],
            fence,
            construction_cell_walkable: false,
            server_artifact_digest: runtime.content_pin().server_artifact_digest(),
        })
    }
    /// Decode the existing named-House custody format: three big-endian i32s.
    /// This representation alone grants no current House or item authority.
    pub fn decode_spatial_position(bytes: &[u8]) -> Option<[i32; 3]> {
        let bytes: &[u8; 12] = bytes.try_into().ok()?;
        Some([
            i32::from_be_bytes(bytes[0..4].try_into().ok()?),
            i32::from_be_bytes(bytes[4..8].try_into().ok()?),
            i32::from_be_bytes(bytes[8..12].try_into().ok()?),
        ])
    }
    /// The existing pinned full-world collision index, also used by command402.
    pub fn qualify_furniture_cell(
        mut self,
        cells: &oteryn_gameplay_content::content::NativeEntryMovementCells,
        blocked: &std::collections::BTreeSet<oteryn_gameplay_content::content::LogicalCell>,
    ) -> Self {
        self.construction_cell_walkable =
            cells.index().lookup(
                cells.scope(),
                oteryn_gameplay_content::content::LogicalCell {
                    x: self.position[0],
                    y: self.position[1],
                    z: self.position[2],
                },
            ) == Ok(oteryn_gameplay_content::content::CollisionClass::Walkable)
                && !blocked.contains(&oteryn_gameplay_content::content::LogicalCell {
                    x: self.position[0],
                    y: self.position[1],
                    z: self.position[2],
                });
        self
    }
    pub(crate) fn item_fence(&self) -> CurrentCharacterItemFence {
        self.fence
    }
    pub(crate) fn server_artifact_digest(&self) -> [u8; 32] {
        self.server_artifact_digest
    }
    pub fn position(&self) -> [i32; 3] {
        self.position
    }
    pub fn house(&self) -> &HouseId {
        &self.house
    }
    pub(super) fn dispenser_proof(&self, node: &NodeIncarnationProof) -> Value {
        json!({"house":self.house.house_key(), "node":uuid_text(*node.fact().node_id().as_bytes()),
            "node_revision":node.fact().registration_revision(), "actor_position":self.position})
    }
    pub(super) async fn dispenser_current(
        &self,
        tx: &mut crate::durability::item_mint_audit::Type2Transaction<'_>,
        node: &NodeIncarnationProof,
    ) -> Result<bool, DurabilityError> {
        if !prove_current_incarnation(tx, node).await?
            || !fenced(CURRENT, node, self)
                .fetch_one(&mut ***tx)
                .await?
                .try_get::<bool, _>(0)?
        {
            return Ok(false);
        }
        Ok(sqlx::query("SELECT character_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND lifecycle=1 FOR UPDATE")
            .bind(self.fence.character_id.as_bytes().as_slice()).bind(self.house.world_id().as_bytes().as_slice())
            .fetch_optional(&mut ***tx).await?.is_some())
    }
    pub(super) async fn dispenser_row(
        &self,
        tx: &mut crate::durability::item_mint_audit::Type2Transaction<'_>,
        node: &NodeIncarnationProof,
        item: [u8; 16],
    ) -> Result<Option<sqlx::postgres::PgRow>, DurabilityError> {
        let row=fenced("SELECT s.before_item,s.before_item->>'definition_family' AS definition_family,s.before_item->>'definition_production_key' AS definition_production_key,s.before_item->>'definition_revision_ref' AS definition_revision_ref,decode(repeat('00',16),'hex') AS parent,s.stack_ordinal AS ordinal,5::integer AS source_kind,jsonb_build_object('house',s.location->'house','provenance',s.provenance) AS ground_location,s.spatial_position FROM game_item_action_house_source(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,$4::text::numeric,$5::text::numeric,encode($6,'hex')::uuid,$7::text::numeric,encode($8,'hex')::uuid,$9,encode($10,'hex')::uuid) s WHERE s.location->>'custody_kind'='house'",node,self)
            .bind(item.to_vec()).fetch_optional(&mut ***tx).await?;
        let Some(row) = row else { return Ok(None) };
        let at = Self::decode_spatial_position(&row.try_get::<Vec<u8>, _>("spatial_position")?)
            .ok_or(DurabilityError::InvalidStoredState)?;
        if at[2] != self.position[2]
            || i64::from(at[0]).abs_diff(i64::from(self.position[0])) > 1
            || i64::from(at[1]).abs_diff(i64::from(self.position[1])) > 1
        {
            return Ok(None);
        }
        Ok(Some(row))
    }
}

fn fenced(
    sql: &'static str,
    node: &NodeIncarnationProof,
    context: &HouseActionContext,
) -> sqlx::query::Query<'static, sqlx::Postgres, sqlx::postgres::PgArguments> {
    let fence = context.fence;
    sqlx::query(sql)
        .bind(fence.game_session_id.as_bytes().to_vec())
        .bind(fence.character_id.as_bytes().to_vec())
        .bind(fence.connection_generation.get().to_string())
        .bind(fence.character_lease_generation.to_string())
        .bind(fence.scope_ownership_generation.get().to_string())
        .bind(node.fact().node_id().as_bytes().to_vec())
        .bind(node.fact().registration_revision().to_string())
        .bind(context.house.world_id().as_bytes().to_vec())
        .bind(context.house.house_key().to_owned())
}

const CURRENT: &str = "SELECT game_item_action_house_current(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,$4::text::numeric,$5::text::numeric,encode($6,'hex')::uuid,$7::text::numeric,encode($8,'hex')::uuid,$9)";
const SOURCE: &str = "SELECT * FROM game_item_action_house_source(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,$4::text::numeric,$5::text::numeric,encode($6,'hex')::uuid,$7::text::numeric,encode($8,'hex')::uuid,$9,encode($10,'hex')::uuid)";

impl DurabilityRoot {
    #[allow(clippy::too_many_arguments)]
    pub async fn consume_dispenser_in_house<C: ItemLoopContent>(
        &self,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        context: &HouseActionContext,
        request: ConsumeTransformRequest,
        native: &NativeGameplayState,
        content: &C,
        probe: bool,
    ) -> Result<Option<ConsumeTransformResult>, DurabilityError> {
        self.consume_transform_inner(
            permit,
            recovery,
            node,
            context.fence,
            request,
            native,
            content,
            probe,
            None,
            ActionContext::DispenserHouseTarget(context),
            None,
            None,
        )
        .await
    }

    /// A current named-House read fence for the existing carried-container observer.
    /// This proves no future mutation and does not read or project House contents.
    pub async fn house_action_current(
        &self,
        node: &NodeIncarnationProof,
        context: &HouseActionContext,
    ) -> Result<bool, DurabilityError> {
        let node = node.clone();
        let context = context.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx =
                        crate::durability::db::begin_semantic_transaction(holder, deadline).await?;
                    let current = prove_current_incarnation(&mut tx, &node).await?
                        && fenced(CURRENT, &node, &context)
                            .fetch_one(&mut *tx)
                            .await?
                            .try_get::<bool, _>(0)?;
                    crate::durability::db::commit_semantic_transaction(tx, deadline).await?;
                    Ok(current)
                })
            })
            .await
    }

    /// Reuse the bounded HouseInterior loader. This snapshot grants no later mutation.
    pub async fn read_house_action_items(
        &self,
        node: &NodeIncarnationProof,
        context: HouseActionContext,
    ) -> Result<Option<HouseInteriorSnapshot>, DurabilityError> {
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx =
                        crate::durability::db::begin_semantic_transaction(holder, deadline).await?;
                    if !prove_current_incarnation(&mut tx, &node).await?
                        || !fenced(CURRENT, &node, &context)
                            .fetch_one(&mut *tx)
                            .await?
                            .try_get::<bool, _>(0)?
                    {
                        return Ok(None);
                    }
                    let snapshot = load_house_interior_in_transaction(
                        &mut tx,
                        &context.house,
                        context.fence.scope_ownership_generation.get(),
                        &node,
                    )
                    .await
                    .map_err(|error| match error {
                        crate::durability::house_interior::HouseInteriorLoadError::Durability(
                            error,
                        ) => error,
                        _ => DurabilityError::InvalidStoredState,
                    })?;
                    crate::durability::db::commit_semantic_transaction(tx, deadline).await?;
                    Ok(Some(snapshot))
                })
            })
            .await
    }

    /// Replay precedes the live item read. A new turn re-proves both the native House
    /// actor and the durable named-House session, then transforms exactly one instance.
    pub async fn consume_transform_in_house(
        &self,
        node: &NodeIncarnationProof,
        context: HouseActionContext,
        request: ConsumeTransformRequest,
        native: &NativeGameplayState,
    ) -> Result<ConsumeTransformResult, DurabilityError> {
        self.transform_in_house(node, context, request, native, false, None, None)
            .await
    }

    pub async fn rotate_in_house(
        &self,
        node: &NodeIncarnationProof,
        context: HouseActionContext,
        request: ConsumeTransformRequest,
        native: &NativeGameplayState,
        expected: Option<TypedDefinitionRef>,
    ) -> Result<ConsumeTransformResult, DurabilityError> {
        self.transform_in_house(node, context, request, native, true, expected, None)
            .await
    }

    pub async fn consume_house_interactive(
        &self,
        node: &NodeIncarnationProof,
        context: HouseActionContext,
        request: ConsumeTransformRequest,
        native: &NativeGameplayState,
        expected: TypedDefinitionRef,
        revision: u64,
    ) -> Result<ConsumeTransformResult, DurabilityError> {
        self.transform_in_house(
            node,
            context,
            request,
            native,
            false,
            Some(expected),
            Some(revision),
        )
        .await
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "Node, House, character authority and observed item state are independent admission proofs"
    )]
    async fn transform_in_house(
        &self,
        node: &NodeIncarnationProof,
        context: HouseActionContext,
        request: ConsumeTransformRequest,
        native: &NativeGameplayState,
        rotate: bool,
        expected: Option<TypedDefinitionRef>,
        expected_revision: Option<u64>,
    ) -> Result<ConsumeTransformResult, DurabilityError> {
        if request.command.game_session_id() != context.fence.game_session_id
            || request.target.is_some()
            || request.intent.is_empty()
            || request.intent.len() > oteryn_protocol_oteryn::world_object::MAX_USE_INTENT_BYTES
        {
            return Ok(ConsumeTransformResult::refused(UseDisposition::Rejected));
        }
        let node = node.clone();
        let mut native = native;
        self.try_issue_semantic_pass()?.run_with_context(&mut native, move |holder, deadline, native| Box::pin(async move {
            let mut tx = begin_type2_transaction(holder, deadline).await?;
            crate::durability::db::lock_admission_relations(&mut tx).await?;
            if !prove_current_incarnation(&mut tx, &node).await?
                || !fenced(CURRENT, &node, &context).fetch_one(&mut **tx).await?.try_get::<bool, _>(0)? {
                return Ok(ConsumeTransformResult::refused(UseDisposition::StaleState));
            }
            let fence = context.fence;
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:consume-transform:' || encode($1,'hex') || ':' || $2,0))")
                .bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string())
                .execute(&mut **tx).await?;
            let previous: Option<(Vec<u8>, Vec<u8>, Value, String)> = sqlx::query_as("SELECT intent,uuid_send(character_id),effects,recipe FROM game_item_consume_transform_records WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric")
                .bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string())
                .fetch_optional(&mut **tx).await?;
            if let Some((intent, character, effects, recipe)) = previous {
                let result = if intent == request.intent && character == fence.character_id.as_bytes() {
                    ConsumeTransformResult { disposition: UseDisposition::Committed, fresh: false, effects, recipe: Some(recipe) }
                } else { ConsumeTransformResult::refused(UseDisposition::Rejected) };
                tx.commit(deadline).await?;
                return Ok(result);
            }
            if !rotate && let Some(result)=construction::replay(&mut tx,&context,&request).await? {
                tx.commit(deadline).await?;
                return Ok(result);
            }
            // A candidate selects the constructor's existing exclusive lock order;
            // the locked source below and the writer independently prove authority.
            let constructor=!rotate&&construction::candidate(&mut tx,&context,&request).await?;
            let source = fenced(if constructor {construction::SOURCE} else {SOURCE}, &node, &context).bind(request.source.to_vec())
                .fetch_optional(&mut **tx).await?;
            let Some(source) = source else {
                return Ok(ConsumeTransformResult::refused(UseDisposition::StaleState));
            };
            let before: Value = source.try_get("before_item")?;
            let location: Value = source.try_get("location")?;
            let provenance: Value = source.try_get("provenance")?;
            let definition = TypedDefinitionRef {
                family: before.get("definition_family").and_then(Value::as_str).ok_or(DurabilityError::InvalidStoredState)?.into(),
                production_key: before.get("definition_production_key").and_then(Value::as_str).ok_or(DurabilityError::InvalidStoredState)?.into(),
                revision_ref: before.get("definition_revision_ref").and_then(Value::as_str).ok_or(DurabilityError::InvalidStoredState)?.into(),
            };
            if !rotate && !constructor && oteryn_gameplay_items::furniture::construction_to(&definition.production_key).is_some()
                && !(oteryn_gameplay_items::furniture::is_carpet(&definition.production_key)
                    &&location.get("custody_kind").and_then(Value::as_str)==Some("house")) {
                return Ok(ConsumeTransformResult::refused(UseDisposition::StaleState));
            }
            if !rotate&&constructor {
                if !context.construction_cell_walkable {
                    return Ok(ConsumeTransformResult::refused(UseDisposition::NoRoom));
                }
                let result=construction::apply(&mut tx,&node,&context,&request,&before,native).await?;
                if result.disposition==UseDisposition::Committed { tx.commit(deadline).await?; }
                return Ok(result);
            }
            if !rotate&&oteryn_gameplay_items::furniture::requires_house_preflight(&definition.production_key) {
                if expected_revision.is_none(){return Ok(ConsumeTransformResult::refused(UseDisposition::NothingToUse))}
                if expected.as_ref()!=Some(&definition)||before.get("state_revision").and_then(Value::as_u64)!=expected_revision {
                    return Ok(ConsumeTransformResult::refused(UseDisposition::StaleState));
                }
                if oteryn_gameplay_items::furniture::is_carpet(&definition.production_key)
                    && !sqlx::query_scalar::<_,bool>("SELECT game_house_furniture_single_item(encode($1,'hex')::uuid)")
                        .bind(request.source.as_slice()).fetch_one(&mut **tx).await? {
                    return Ok(ConsumeTransformResult::refused(UseDisposition::NotPossible));
                }

            }
            // Crystal game.cpp:4533 binds Rotate to the item type the client saw.
            // Immutable replay above remains valid after the original handle retires.
            if rotate && expected.as_ref()!=Some(&definition) {
                return Ok(ConsumeTransformResult::refused(UseDisposition::StaleState));
            }
            if (rotate && oteryn_gameplay_items::furniture::rotate_to(&definition.production_key).is_none())
                || (!rotate && novelty_items::house_transform_for_key(&definition.production_key).is_none()
                    && oteryn_gameplay_items::furniture::transform_to(&definition.production_key).is_none()
                    && oteryn_gameplay_items::furniture::side_rule(&definition.production_key).is_none()) {
                return Ok(ConsumeTransformResult::refused(UseDisposition::NothingToUse));
            }
            if before.get("quantity").and_then(Value::as_i64) != Some(1) {
                return Ok(ConsumeTransformResult::refused(UseDisposition::NotPossible));
            }
            // Crystal Item::getTile follows carried/nested parents to the actor's
            // tile. HouseInterior sources retain their own actual placement.
            let position: Option<Vec<u8>> = source.try_get("spatial_position")?;
            let at = if let Some(position) = position {
                HouseActionContext::decode_spatial_position(&position)
                    .ok_or(DurabilityError::InvalidStoredState)?
            } else { context.position };
            if at[2] != context.position[2] || (i64::from(at[0]) - i64::from(context.position[0])).abs() > 1
                || (i64::from(at[1]) - i64::from(context.position[1])).abs() > 1 {
                return Ok(ConsumeTransformResult::refused(UseDisposition::NotPossible));
            }
            if !rotate && oteryn_gameplay_items::furniture::is_golden_display(&definition.production_key) {
                let full=sqlx::query_scalar::<_,bool>("SELECT EXISTS(SELECT 1 FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND quest_key='oteryn:quest.golden_outfits_quest' AND track_key='oteryn:quest-progress/crystalserver/outfit_quest/golden_outfit' AND value=3)")
                    .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
                if !full { return Ok(ConsumeTransformResult {disposition:UseDisposition::NotPossible,fresh:false,effects:json!({"golden_display_locked":true}),recipe:None}); }
            }
            let selected=if rotate { oteryn_gameplay_items::furniture::plan_rotate(native,&definition) }
                // The finite portable catalogue has a different Channel/Ground
                // receipt. House custody must retain its original kind5 owner.
                else if oteryn_gameplay_items::furniture::portable_use_to(&definition.production_key).is_some()
                    || oteryn_gameplay_items::furniture::speech(&definition.production_key).is_some() {
                    oteryn_gameplay_items::furniture::plan_house(native,&definition)
                }
                else {novelty_items::plan_ground(native, &definition,
                    *fence.game_session_id.as_bytes(), request.command.command_id().get(), request.source,
                    GroundEffect { x: at[0], y: at[1], floor: i16::try_from(at[2]).map_err(|_| DurabilityError::InvalidStoredState)? }, true)
                    .or_else(|| oteryn_gameplay_items::furniture::plan_house(native, &definition))
                    .or_else(||oteryn_gameplay_items::furniture::plan_interactive(native,&definition,
                        *fence.game_session_id.as_bytes(),request.command.command_id().get(),request.source,
                        GroundEffect{x:at[0],y:at[1],floor:i16::try_from(at[2]).ok()?},false))};
            let Some(mut selected)=selected
            else { return Ok(ConsumeTransformResult::refused(UseDisposition::NotPossible)); };
            if !rotate && oteryn_gameplay_items::furniture::speech(&definition.production_key).is_some() {
                selected.effects["ground"]=serde_json::to_value(GroundEffect{x:at[0],y:at[1],floor:i16::try_from(at[2]).map_err(|_|DurabilityError::InvalidStoredState)?})
                    .map_err(|_|DurabilityError::InvalidStoredState)?;
            }
            let [(after, 1)] = selected.outputs.as_slice() else { return Err(DurabilityError::InvalidStoredState); };
            let qualified = native.item_policy(&after.production_key, &after.revision_ref)
                .and_then(|policy| crate::durability::spell_items_abi::QualifiedItemDefinition::from_native_policy(policy).ok());
            // The source transforms exactly one instance; a stack-capable target
            // still admits that quantity. Custody and quantity remain unchanged.
            let paused=oteryn_gameplay_items::furniture::pauses_duration(&after.production_key)
                &&native.item_policy(&after.production_key,&after.revision_ref).is_some_and(|policy|{
                    let record=policy.record();record.admission.materializable
                    &&record.admission.stack_class==oteryn_gameplay_content::content::native_gameplay::NativeItemStackClass::NonStackable
                    &&record.admission.legal_destinations.contains(&oteryn_gameplay_content::content::native_gameplay::NativeItemDestination::Ground)});
            if !paused&&qualified.as_ref().is_none_or(|policy| policy.stack_maximum == 0
                || !policy.ground_destination) {
                return Ok(ConsumeTransformResult::refused(UseDisposition::NotPossible));
            }
            let timer_before: Option<Value> = sqlx::query_scalar("SELECT to_jsonb(t) FROM game_item_timed_states t WHERE item_instance_id=encode($1,'hex')::uuid FOR UPDATE")
                .bind(request.source.as_slice()).fetch_optional(&mut **tx).await?;
            let decay=if paused {None}else{qualified.as_ref().and_then(|policy|policy.decay.as_ref())};
            let absolute:Option<Value>=sqlx::query_scalar("SELECT to_jsonb(t) FROM game_spell_item_temporal_schedules t WHERE item_instance_id=encode($1,'hex')::uuid AND NOT EXISTS(SELECT 1 FROM game_spell_item_temporal_receipts done WHERE done.item_instance_id=t.item_instance_id) AND NOT EXISTS(SELECT 1 FROM game_house_furniture_absolute_expiries done WHERE done.item_instance_id=t.item_instance_id) FOR UPDATE")
                .bind(request.source.as_slice()).fetch_optional(&mut **tx).await?;
            if let Some(absolute)=absolute {
                if !rotate || timer_before.is_some() { return Ok(ConsumeTransformResult::refused(UseDisposition::NotPossible)); }
                let Some(target)=decay.and_then(|d|d.target.as_ref()) else { return Ok(ConsumeTransformResult::refused(UseDisposition::NotPossible)); };
                selected.effects["house_absolute"]=json!({"before":absolute,"target":target.production_key});
            } else if timer_before.is_some() || decay.is_some() {
                selected.effects["house_temporal"]=json!({"before":timer_before,
                    "duration":decay.map(|d| d.duration_millis),
                    "target":decay.and_then(|d| d.target.as_ref()).map(|d| &d.production_key)});
            }
            selected.effects["house_after"] = json!({"key": after.production_key, "revision": after.revision_ref});
            let plan = ActionPlan { recipe: selected.recipe, outputs: Vec::new(), consume_source: false,
                source_count: 0, effects: selected.effects, extra_inputs: Vec::new() };
            let mut source_location = location;
            let location = source_location.as_object_mut().ok_or(DurabilityError::InvalidStoredState)?;
            location.insert("provenance".into(), provenance);
            location.insert("actor_position".into(), json!(context.position));
            location.insert("node".into(), json!(uuid_text(*node.fact().node_id().as_bytes())));
            location.insert("node_revision".into(), json!(node.fact().registration_revision().to_string()));
            let ordinal: String = source.try_get("stack_ordinal")?;
            let result = persist_action(None, &mut tx, fence, &request, native, &plan, &before, None, None, 5,
                &ordinal, 20, &[], &source_location, None).await?;
            tx.commit(deadline).await?;
            Ok(result)
        })).await
    }
}

pub(super) async fn apply_in_transaction(
    tx: &mut crate::durability::item_mint_audit::Type2Transaction<'_>,
    transaction: &str,
) -> Result<(), DurabilityError> {
    sqlx::query("SELECT game_item_action_house_apply($1::uuid)")
        .bind(transaction)
        .execute(&mut ***tx)
        .await?;
    sqlx::query("SELECT game_house_furniture_absolute_apply($1::uuid)")
        .bind(transaction)
        .execute(&mut ***tx)
        .await?;
    sqlx::query("SELECT game_house_furniture_timer_apply($1::uuid)")
        .bind(transaction)
        .execute(&mut ***tx)
        .await?;
    Ok(())
}

#[derive(Clone, Debug)]
pub struct HouseInteractiveSource {
    pub item_instance_id: [u8; 16],
    pub definition: TypedDefinitionRef,
    pub state_revision: u64,
    pub position: [i32; 3],
    pub placed: bool,
}
impl DurabilityRoot {
    /// Server preparation only; the mutation rechecks this source revision under its own fence.
    pub async fn read_house_interactive_source(
        &self,
        node: &NodeIncarnationProof,
        context: HouseActionContext,
        item: [u8; 16],
    ) -> Result<Option<HouseInteractiveSource>, DurabilityError> {
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx =
                        crate::durability::db::begin_semantic_transaction(holder, deadline).await?;
                    if !prove_current_incarnation(&mut tx, &node).await?
                        || !fenced(CURRENT, &node, &context)
                            .fetch_one(&mut *tx)
                            .await?
                            .try_get::<bool, _>(0)?
                    {
                        return Ok(None);
                    }
                    let row = fenced(SOURCE, &node, &context)
                        .bind(item.to_vec())
                        .fetch_optional(&mut *tx)
                        .await?;
                    let Some(row) = row else { return Ok(None) };
                    let before: Value = row.try_get("before_item")?;
                    let position: Option<Vec<u8>> = row.try_get("spatial_position")?;
                    let result = HouseInteractiveSource {
                        item_instance_id: item,
                        definition: TypedDefinitionRef {
                            family: before
                                .get("definition_family")
                                .and_then(Value::as_str)
                                .ok_or(DurabilityError::InvalidStoredState)?
                                .into(),
                            production_key: before
                                .get("definition_production_key")
                                .and_then(Value::as_str)
                                .ok_or(DurabilityError::InvalidStoredState)?
                                .into(),
                            revision_ref: before
                                .get("definition_revision_ref")
                                .and_then(Value::as_str)
                                .ok_or(DurabilityError::InvalidStoredState)?
                                .into(),
                        },
                        state_revision: before
                            .get("state_revision")
                            .and_then(Value::as_u64)
                            .ok_or(DurabilityError::InvalidStoredState)?,
                        position: position
                            .as_ref()
                            .map(|bytes| {
                                HouseActionContext::decode_spatial_position(bytes)
                                    .ok_or(DurabilityError::InvalidStoredState)
                            })
                            .transpose()?
                            .unwrap_or(context.position),
                        placed: position.is_some(),
                    };
                    crate::durability::db::commit_semantic_transaction(tx, deadline).await?;
                    Ok(Some(result))
                })
            })
            .await
    }
}
