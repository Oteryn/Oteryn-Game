//! HouseInterior balloon USE: the same receipt/audit, with named-House custody retained.
use super::*;
use crate::durability::house_interior::{
    HouseInteriorSnapshot, load_house_interior_in_transaction,
};
use crate::durability::house_scope_handoff::HouseId;
use crate::durability::runtime_scope_assignment::prove_current_incarnation;
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, GameSessionId};
use oteryn_gameplay_items::novelty_items::{self, GroundEffect};

#[path = "consume_transform_house/decorations.rs"]
mod decorations;
#[path = "consume_transform_house/timed_decorations.rs"]
mod timed_decorations;

/// The actual House actor owner constructs this context. Neither wire positions nor
/// a Channel equipment fence can stand in for the current named-House lease.
#[derive(Clone, Debug)]
pub struct HouseActionContext {
    house: HouseId,
    position: [i32; 3],
    fence: CurrentCharacterItemFence,
    occupied: Vec<[i32; 3]>,
    base_items: Vec<HouseDecorationBaseItem>,
    bed_target: Option<Value>,
    rotation: Option<u64>,
}

/// Immutable native base facts captured by the current House command owner.
#[derive(Clone, Debug)]
pub struct HouseDecorationBaseItem {
    pub position: [i32; 3],
    pub ordinal: u64,
    pub definition: TypedDefinitionRef,
}

impl HouseActionContext {
    pub fn from_native_owner(
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
        fence: CurrentCharacterItemFence,
    ) -> Result<Self, &'static str> {
        if !runtime
            .house_player_control_facts(actor, session)
            .is_ok_and(|facts| facts.control_loss.is_none())
        {
            return Err("House actor control is lost");
        }
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
        let position = [
            state.position.x,
            state.position.y,
            i32::from(state.position.floor),
        ];
        let mut occupied = vec![position];
        for dx in -1..=1 {
            for dy in -1..=1 {
                let Some(x) = position[0].checked_add(dx) else {
                    continue;
                };
                let Some(y) = position[1].checked_add(dy) else {
                    continue;
                };
                let candidate = crate::foundation::LocalPosition {
                    x,
                    y,
                    floor: state.position.floor,
                };
                if runtime
                    .house_cell_occupied_except(actor, session, candidate)
                    .map_err(|_| "House occupancy missing")?
                {
                    occupied.push([x, y, position[2]]);
                }
            }
        }
        Ok(Self {
            house,
            position,
            fence,
            occupied,
            base_items: Vec::new(),
            bed_target: None,
            rotation: None,
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
    pub fn position(&self) -> [i32; 3] {
        self.position
    }
    pub fn house(&self) -> &HouseId {
        &self.house
    }
    pub fn fence(&self) -> CurrentCharacterItemFence {
        self.fence
    }
    pub fn with_rotation(mut self, expected_revision: u64) -> Self {
        self.rotation = Some(expected_revision);
        self.bed_target = None;
        self
    }
    pub fn with_bed_target(mut self, target: Value) -> Self {
        self.rotation = None;
        self.bed_target = Some(target);
        self
    }
    pub fn with_decoration_base_items(mut self, items: Vec<HouseDecorationBaseItem>) -> Self {
        self.base_items = items;
        self
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
    /// Probe an immutable House USE receipt before resolving a consumed kit or
    /// a bed whose base revision changed in the original atomic operation.
    pub async fn replay_house_item_action(
        &self,
        node: &NodeIncarnationProof,
        context: &HouseActionContext,
        request: &ConsumeTransformRequest,
    ) -> Result<Option<ConsumeTransformResult>, DurabilityError> {
        if request.command.game_session_id() != context.fence.game_session_id
            || request.intent.is_empty()
            || request.intent.len() > oteryn_protocol_oteryn::world_object::MAX_USE_INTENT_BYTES
        {
            return Ok(Some(ConsumeTransformResult::refused(
                UseDisposition::Rejected,
            )));
        }
        let node = node.clone();
        let context = context.clone();
        let request = request.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = crate::durability::db::begin_semantic_transaction(holder, deadline).await?;
            if !prove_current_incarnation(&mut tx, &node).await?
                || !fenced(CURRENT, &node, &context).fetch_one(&mut *tx).await?.try_get::<bool,_>(0)? {
                return Ok(Some(ConsumeTransformResult::refused(UseDisposition::StaleState)));
            }
            let previous: Option<(Vec<u8>,Vec<u8>,Value,String)> = sqlx::query_as("SELECT intent,uuid_send(character_id),effects,recipe FROM game_item_consume_transform_records WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric")
                .bind(context.fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string())
                .fetch_optional(&mut *tx).await?;
            let result = previous.map(|(intent, character, effects, recipe)| {
                if intent == request.intent && character == context.fence.character_id.as_bytes() {
                    ConsumeTransformResult {disposition:UseDisposition::Committed,fresh:false,effects,recipe:Some(recipe)}
                } else {ConsumeTransformResult::refused(UseDisposition::Rejected)}
            });
            crate::durability::db::commit_semantic_transaction(tx, deadline).await?;
            Ok(result)
        })).await
    }

    /// Classification only: the committing owner re-reads and locks this source.
    /// A normal key or fluid must retain its own USE_WITH route inside a House.
    pub async fn house_source_is_bed_kit(
        &self,
        node: &NodeIncarnationProof,
        context: &HouseActionContext,
        source: [u8; 16],
    ) -> Result<bool, DurabilityError> {
        let node = node.clone();
        let context = context.clone();
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
                        return Err(DurabilityError::Unavailable);
                    }
                    let row = fenced(SOURCE, &node, &context)
                        .bind(source.to_vec())
                        .fetch_optional(&mut *tx)
                        .await?;
                    let result = if let Some(row) = row {
                        let before: Value = row.try_get("before_item")?;
                        let key = before
                            .get("definition_production_key")
                            .and_then(Value::as_str)
                            .ok_or(DurabilityError::InvalidStoredState)?;
                        decorations::is_bed_kit(key)?
                    } else {
                        false
                    };
                    crate::durability::db::commit_semantic_transaction(tx, deadline).await?;
                    Ok(result)
                })
            })
            .await
    }

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
        if !rotation_binding_valid(&context, &request)
            || request.command.game_session_id() != context.fence.game_session_id
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
            let source = fenced(SOURCE, &node, &context).bind(request.source.to_vec())
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
            if context.rotation.is_some_and(|expected| before.get("state_revision").and_then(Value::as_u64)!=Some(expected)) {
                return Ok(ConsumeTransformResult::refused(UseDisposition::StaleState));
            }
            if before.get("quantity").and_then(Value::as_i64) != Some(1) {
                return Ok(ConsumeTransformResult::refused(UseDisposition::NotPossible));
            }
            // Crystal Item::getTile follows carried/nested parents to the actor's
            // tile. HouseInterior sources retain their own actual placement.
            let position: Option<Vec<u8>> = source.try_get("spatial_position")?;
            let placed = position.is_some();
            let at = if let Some(position) = position {
                HouseActionContext::decode_spatial_position(&position)
                    .ok_or(DurabilityError::InvalidStoredState)?
            } else { context.position };
            if at[2] != context.position[2] || (i64::from(at[0]) - i64::from(context.position[0])).abs() > 1
                || (i64::from(at[1]) - i64::from(context.position[1])).abs() > 1 {
                return Ok(ConsumeTransformResult::refused(UseDisposition::NotPossible));
            }
            let plan = match decorations::plan(&mut tx, &context, &request, native, &definition, at, placed).await? {
                Some(Ok(plan)) => plan,
                Some(Err(disposition)) => return Ok(ConsumeTransformResult::refused(disposition)),
                None => {
                if novelty_items::house_transform_for_key(&definition.production_key).is_none() {
                    return Ok(ConsumeTransformResult::refused(UseDisposition::NothingToUse));
                }
                let Some(mut selected) = novelty_items::plan_ground(native, &definition,
                *fence.game_session_id.as_bytes(), request.command.command_id().get(), request.source,
                GroundEffect { x: at[0], y: at[1], floor: i16::try_from(at[2]).map_err(|_| DurabilityError::InvalidStoredState)? }, true)
            else { return Ok(ConsumeTransformResult::refused(UseDisposition::NotPossible)); };
            let [(after, 1)] = selected.outputs.as_slice() else { return Err(DurabilityError::InvalidStoredState); };
            let qualified = native.item_policy(&after.production_key, &after.revision_ref)
                .and_then(|policy| crate::durability::spell_items_abi::QualifiedItemDefinition::from_native_policy(policy).ok());
            // The source transforms exactly one instance; a stack-capable target
            // still admits that quantity. Custody and quantity remain unchanged.
            if qualified.as_ref().is_none_or(|policy| policy.stack_maximum == 0
                || policy.container_capacity.is_some() || policy.decay.is_some() || !policy.ground_destination) {
                return Ok(ConsumeTransformResult::refused(UseDisposition::NotPossible));
            }
            selected.effects["house_after"] = json!({"key": after.production_key, "revision": after.revision_ref});
                ActionPlan { recipe: selected.recipe, outputs: Vec::new(), consume_source: false,
                    source_count: 0, effects: selected.effects, extra_inputs: Vec::new() }
                }
            };
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
    Ok(())
}

fn rotation_binding_valid(context: &HouseActionContext, request: &ConsumeTransformRequest) -> bool {
    use oteryn_protocol_oteryn::house_rotation as wire;
    let prefix = wire::COMMAND.to_be_bytes();
    let encoded = request.intent.strip_prefix(prefix.as_slice());
    match (context.rotation, encoded) {
        (None, None) => true,
        (Some(expected), Some(bytes)) => {
            matches!(wire::Request::decode(bytes),Some(wire::Request::Rotate(wire::Target::Item{item,expected_revision})) if item==request.source&&expected_revision==expected)
        }
        _ => false,
    }
}
