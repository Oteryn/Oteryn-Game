//! World callback inputs share one atomic quest receipt and consumption proof path.
pub mod carried_step;
pub(super) mod completion;
pub(crate) mod ground_stack;
pub mod shared_count;
pub mod shared_switch;
use super::{
    DurabilityError, character_progression::CurrentCharacterGameplayFence,
    spell_owner_commit::SpellLanePermit,
};
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
use serde::{Deserialize, Serialize};
use sqlx::Row;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuestWorldContext {
    pub map_revision: String,
    pub content_revision: String,
    pub placement_context: Vec<u8>,
    pub reset_epoch: u64,
    pub actor_placement: [u8; 16],
    pub movement_revision: u64,
}

#[path = "quest_world_callback/reward_inputs.rs"]
mod reward_inputs;

struct Input {
    item: String,
    before: serde_json::Value,
    location: serde_json::Value,
    kind: String,
    quantity: i64,
    count: u16,
    replacement: Option<String>,
}

/// Resolve all carried stacks before touching any of them. Closed nested containers
/// are traversed by the existing custody owner; filled containers are never destroyed.
pub(super) async fn consume_reward_inputs(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterGameplayFence,
    request: &super::quest_state::QuestTransitionRequest,
    callback: &oteryn_gameplay_quests::quest::reward_callbacks::RewardCallback,
    carried_use: Option<&super::reward_claim_mint::QuestCarriedUse>,
) -> Result<bool, DurabilityError> {
    let super::quest_state::QuestCause::Command(_) = request.cause else {
        return Ok(false);
    };
    let keys: Vec<_> = callback
        .take
        .iter()
        .chain(&callback.present)
        .map(|i| i.item.as_str())
        .collect();
    let rows=sqlx::query("SELECT i.item_instance_id::text AS item,to_jsonb(i) AS before_item,c.location,c.kind,i.quantity,i.definition_production_key FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid) c JOIN game_item_instances i ON i.item_instance_id=c.item WHERE i.lifecycle=1 AND i.definition_family='Item' AND i.definition_production_key=ANY($3::text[]) ORDER BY i.item_instance_id LIMIT 501 FOR UPDATE OF i")
        .bind(fence.character_id.as_bytes().as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice()).bind(&keys).fetch_all(&mut **tx).await?;
    if let Some(use_) = carried_use {
        let selected = sqlx::types::Uuid::from_bytes(use_.source).to_string();
        if !rows.iter().any(|row| {
            row.try_get::<&str, _>("item").ok() == Some(selected.as_str())
                && row.try_get::<&str, _>("definition_production_key").ok()
                    == callback.carried_item.as_deref()
        }) {
            return Ok(false);
        }
    }
    if rows.len() > 500 {
        return Ok(false);
    }
    if callback.exact_take {
        for input in &callback.take {
            let mut total = 0_i64;
            for row in &rows {
                if row.try_get::<&str, _>("definition_production_key")? == input.item {
                    total = total
                        .checked_add(row.try_get::<i64, _>("quantity")?)
                        .ok_or(DurabilityError::InvalidStoredState)?;
                }
            }
            if total != i64::from(input.count) {
                return Ok(false);
            }
        }
    }
    // Read-only hand-ins (Palimuth's crest) prove custody without replacing
    // the original instance or manufacturing a second MINT receipt.
    for evidence in &callback.present {
        let mut needed = i64::from(evidence.count);
        for row in &rows {
            if row.try_get::<&str, _>("definition_production_key")? == evidence.item {
                let quantity: i64 = row.try_get("quantity")?;
                needed -= needed.min(quantity);
            }
        }
        if needed != 0 {
            return Ok(false);
        }
    }
    let mut remaining: std::collections::BTreeMap<_, _> = callback
        .take
        .iter()
        .map(|i| (i.item.as_str(), i64::from(i.count)))
        .collect();
    let mut inputs = Vec::new();
    for row in rows {
        if carried_use.is_some_and(|use_| {
            row.try_get::<&str, _>("item").ok()
                != Some(
                    sqlx::types::Uuid::from_bytes(use_.source)
                        .to_string()
                        .as_str(),
                )
        }) {
            continue;
        }
        let key: String = row.try_get("definition_production_key")?;
        let Some(required) = remaining.get_mut(key.as_str()) else {
            continue;
        };
        let quantity: i64 = row.try_get("quantity")?;
        let count = (*required).min(quantity);
        if count == 0 {
            continue;
        }
        let input = Input {
            item: row.try_get("item")?,
            before: row.try_get("before_item")?,
            location: row.try_get("location")?,
            kind: row.try_get("kind")?,
            quantity,
            count: u16::try_from(count).map_err(|_| DurabilityError::InvalidStoredState)?,
            replacement: None,
        };
        *required -= count;
        inputs.push(input);
    }
    if remaining.values().any(|n| *n != 0) {
        return Ok(false);
    }
    reward_inputs::apply(tx, fence, request, callback, carried_use, &inputs).await
}

pub(super) async fn consume(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    permit: &SpellLanePermit,
    fence: CurrentCharacterGameplayFence,
    request: &super::quest_state::QuestTransitionRequest,
    callback: &Callback,
    context: &QuestWorldContext,
    scope_guard: (
        &crate::character_recovery_fence::CharacterRecoveryFenceV1,
        &super::runtime_scope_assignment::NodeIncarnationProof,
    ),
) -> Result<bool, DurabilityError> {
    let (super::quest_state::QuestCause::Use(command)
    | super::quest_state::QuestCause::Step(command)) = request.cause
    else {
        return Ok(false);
    };
    let crate::foundation::RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = fence.runtime_scope
    else {
        return Ok(false);
    };
    if !context_is_current(tx, permit, fence, context).await? {
        return Ok(false);
    }
    if let Some(change) = &callback.world_change
        && !super::world_event::change_active(tx, world_id, change).await?
    {
        return Ok(false);
    }
    if !super::world_event::changes::callbacks::require(
        tx,
        world_id,
        context.reset_epoch,
        &callback.key,
    )
    .await?
    {
        return Ok(false);
    }
    // M03: promotion is a permanent Character purchase, not a client vocation
    // name or copied quest storage. The outer writer holds the Character root.
    if callback.admission.as_ref().is_some_and(|a| a.promotion)
        && !super::character_promotion::read_durable_promotion(tx, fence.character_id).await?
    {
        return Ok(false);
    }
    if let Some(minimum) = callback.minimum_level {
        let level: Option<i64> = sqlx::query_scalar(
            "SELECT level FROM game_character_progression_state \
             WHERE character_id=encode($1,'hex')::uuid",
        )
        .bind(fence.character_id.as_bytes().as_slice())
        .fetch_optional(&mut **tx)
        .await?;
        if level.is_none_or(|level| level < i64::from(minimum)) {
            return Ok(false);
        }
    }
    if !ground_stack::require(
        tx,
        permit,
        scope_guard,
        fence,
        context,
        &callback.world_requirements,
    )
    .await?
    {
        return Ok(false);
    }
    if callback.offerings.is_empty() && callback.item_on_target.is_none() {
        return Ok(true);
    }
    consume_inputs(
        tx, permit, fence, request, callback, context, command, world_id, channel_id,
    )
    .await
}

/// Shared current map/content/reset proof for source-qualified quest world actions.
pub(super) async fn context_is_current(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    permit: &SpellLanePermit,
    fence: CurrentCharacterGameplayFence,
    context: &QuestWorldContext,
) -> Result<bool, DurabilityError> {
    let crate::foundation::RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = fence.runtime_scope
    else {
        return Ok(false);
    };
    permit.check_channel(world_id, channel_id)?;
    context_facts_are_current(tx, fence, context).await
}

/// Current persisted map/reset facts, shared by stateful and stateless callbacks.
/// The caller has checked and retains the live lane permit for this exact scope.
pub(super) async fn context_facts_are_current(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterGameplayFence,
    context: &QuestWorldContext,
) -> Result<bool, DurabilityError> {
    let crate::foundation::RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = fence.runtime_scope
    else {
        return Ok(false);
    };
    let epoch: String = sqlx::query_scalar(
        "SELECT coalesce((SELECT reset_epoch FROM game_world_reset_active \
         WHERE world_id=encode($1,'hex')::uuid),0)::text",
    )
    .bind(world_id.as_bytes().as_slice())
    .fetch_one(&mut **tx)
    .await?;
    if epoch != context.reset_epoch.to_string() {
        return Ok(false);
    }
    let pinned: bool = sqlx::query_scalar(
        "SELECT coalesce((SELECT ('sha256:'||encode(server_artifact_digest,'hex')=$3 \
         AND frame_binding_digest=$4) FROM game_content_activations \
         WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid \
         ORDER BY activation_sequence DESC LIMIT 1),false)",
    )
    .bind(world_id.as_bytes().as_slice())
    .bind(channel_id.as_bytes().as_slice())
    .bind(&context.content_revision)
    .bind(&context.placement_context)
    .fetch_one(&mut **tx)
    .await?;
    Ok(pinned)
}

#[allow(clippy::too_many_arguments)]
async fn consume_inputs(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    permit: &SpellLanePermit,
    fence: CurrentCharacterGameplayFence,
    request: &super::quest_state::QuestTransitionRequest,
    callback: &Callback,
    context: &QuestWorldContext,
    command: crate::foundation::CommandRef,
    world_id: crate::foundation::WorldId,
    channel_id: crate::foundation::ChannelId,
) -> Result<bool, DurabilityError> {
    // Resolve and lock every input before any write, including replacement preconditions.
    let mut inputs = Vec::new();
    if let Some(required) = &callback.item_on_target {
        let accepted: Vec<&str> = std::iter::once(required.item.as_str())
            .chain(required.alternatives.iter().map(String::as_str))
            .collect();
        let row = sqlx::query(
            "SELECT i.item_instance_id::text AS item,to_jsonb(i) AS before_item, \
             c.location,c.kind,i.quantity FROM \
             game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid) c \
             JOIN game_item_instances i ON i.item_instance_id=c.item \
             WHERE i.world_id=encode($2,'hex')::uuid AND i.lifecycle=1 \
             AND i.definition_family='Item' AND i.definition_production_key=ANY($3::text[]) \
             AND i.quantity >= $4 ORDER BY i.item_instance_id LIMIT 1 FOR UPDATE OF i",
        )
        .bind(fence.character_id.as_bytes().as_slice())
        .bind(world_id.as_bytes().as_slice())
        .bind(&accepted)
        .bind(i64::from(required.count))
        .fetch_optional(&mut **tx)
        .await?;
        let Some(row) = row else { return Ok(false) };
        let input = Input {
            item: row.try_get("item")?,
            before: row.try_get("before_item")?,
            location: row.try_get("location")?,
            kind: row.try_get("kind")?,
            quantity: row.try_get("quantity")?,
            count: required.count,
            replacement: required.replacement.clone(),
        };
        if input.replacement.is_some() && (input.count != 1 || input.quantity != 1) {
            return Ok(false);
        }
        lock_location(tx, &input).await?;
        if callback.key == oteryn_gameplay_quests::quest::rotten_blood::SACRIFICE {
            // Crystal actions_sacrifice.lua:15-18: stage one needs a real
            // carried tear but consumes none. All other stages use the same
            // existing atomic BURN and Quest receipt path below.
            let access: i64 = sqlx::query_scalar(
                "SELECT coalesce((SELECT value FROM game_character_quest_tracks \
                 WHERE character_id=encode($1,'hex')::uuid AND track_key=$2),0)",
            )
            .bind(fence.character_id.as_bytes().as_slice())
            .bind(oteryn_gameplay_quests::quest::rotten_blood::ACCESS_TRACK)
            .fetch_one(&mut **tx)
            .await?;
            let state = oteryn_gameplay_quests::quest::rotten_blood::RottenBloodState::from_stored(
                u8::try_from(access).map_err(|_| DurabilityError::InvalidStoredState)?,
                0,
                0,
                false,
                false,
                None,
            )
            .map_err(|_| DurabilityError::InvalidStoredState)?;
            let Ok(change) = state.sacrifice(
                u32::try_from(input.quantity).map_err(|_| DurabilityError::InvalidStoredState)?,
            ) else {
                return Ok(false);
            };
            if change.tears_consumed == 0 {
                return Ok(true);
            }
            if change.tears_consumed != u32::from(input.count) {
                return Err(DurabilityError::InvalidStoredState);
            }
        }
        // Retained tools still require a positive quantity, live custody and the
        // same location lock. The enclosing Quest receipt binds this read to the
        // action without inventing an item consumption or changing its identity.
        if !required.reusable {
            inputs.push(input);
        }
    }
    // Stable tile order and then instance order. One item cannot satisfy two altars.
    let mut offerings = callback.offerings.iter().collect::<Vec<_>>();
    offerings.sort_by_key(|o| o.position);
    for offering in offerings {
        let Position { x, y, floor } = offering.position;
        let mut spatial = i32::from(x).to_be_bytes().to_vec();
        spatial.extend_from_slice(&i32::from(y).to_be_bytes());
        spatial.extend_from_slice(&i16::from(floor).to_be_bytes());
        let row = sqlx::query(
            "SELECT i.item_instance_id::text AS item,to_jsonb(i) AS before_item, \
             to_jsonb(g) AS before_ground,i.quantity FROM game_item_ground_locations g \
             JOIN game_item_instances i USING(item_instance_id) \
             WHERE g.world_id=encode($1,'hex')::uuid AND g.channel_id=encode($2,'hex')::uuid \
             AND g.spatial_position=$3 AND g.map_revision=$4 AND g.content_revision=$5 \
             AND g.native_room_placement_context=$6 \
             AND g.runtime_scope_ownership_generation=$7::text::numeric \
             AND i.lifecycle=1 AND i.definition_family='Item' AND i.definition_production_key=$8 \
             AND i.quantity >= $9 AND game_item_loop_drop_ref(g.corpse_ref) \
             ORDER BY i.item_instance_id LIMIT 1 FOR UPDATE OF i,g",
        )
        .bind(world_id.as_bytes().as_slice())
        .bind(channel_id.as_bytes().as_slice())
        .bind(spatial)
        .bind(&context.map_revision)
        .bind(&context.content_revision)
        .bind(&context.placement_context)
        .bind(fence.scope_ownership_generation.get().to_string())
        .bind(&offering.item)
        .bind(i64::from(offering.count))
        .fetch_optional(&mut **tx)
        .await?;
        let Some(row) = row else { return Ok(false) };
        let item: String = row.try_get("item")?;
        if inputs.iter().any(|input| input.item == item) {
            return Ok(false);
        }
        inputs.push(Input {
            item,
            before: row.try_get("before_item")?,
            location: row.try_get("before_ground")?,
            kind: "ground".into(),
            quantity: row.try_get("quantity")?,
            count: offering.count,
            replacement: None,
        });
    }
    let transaction: String = sqlx::query_scalar("SELECT game_character_uuid_v7()::text")
        .fetch_one(&mut **tx)
        .await?;
    for input in inputs {
        let proof = serde_json::to_vec(&serde_json::json!({
            "callback": callback, "context": context,
            "character": fence.character_id.as_bytes(), "session": command.game_session_id().as_bytes(),
            "command": command.command_id().get(), "connection": fence.connection_generation.get(),
            "lease": fence.character_lease_generation, "scope": fence.scope_ownership_generation.get(),
            "before": input.before, "location": input.location, "kind": input.kind,
            "count": input.count, "replacement": input.replacement,
        })).map_err(|_| DurabilityError::InvalidStoredState)?;
        sqlx::query(
            "INSERT INTO game_quest_callback_consumptions(transaction_id,item_instance_id, \
             character_id,cause_id,cause_ordinal,transition_key,before_item,before_ground, \
             quantity,proof,proof_sha256,before_location,location_kind,replacement) \
             VALUES($1::uuid,$2::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid, \
             $5::text::numeric,$6,$7,$8,$9,$10,sha256($10),$11,$12,$13)",
        )
        .bind(&transaction)
        .bind(&input.item)
        .bind(fence.character_id.as_bytes().as_slice())
        .bind(command.game_session_id().as_bytes().as_slice())
        .bind(command.command_id().get().to_string())
        .bind(&request.transition_key)
        .bind(&input.before)
        .bind((input.kind == "ground").then_some(&input.location))
        .bind(i32::from(input.count))
        .bind(proof)
        .bind((input.kind != "ground").then_some(&input.location))
        .bind(&input.kind)
        .bind(&input.replacement)
        .execute(&mut **tx)
        .await?;
        apply_input(tx, permit, fence, &input, &transaction).await?;
    }
    Ok(true)
}

/// Closed native NPC handover reuses the existing carried Quest consumption owner.
/// The Quest writer owns its command receipt; this participant never commits.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn consume_native_npc_one(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    permit: &SpellLanePermit,
    fence: CurrentCharacterGameplayFence,
    request: &super::quest_state::QuestTransitionRequest,
    command: crate::foundation::CommandRef,
    key: &str,
    source: &serde_json::Value,
) -> Result<bool, DurabilityError> {
    if request.transition_key != super::world_event::changes::counters::benevola::HANDOVER
        || key != super::world_event::changes::counters::benevola::WOLF
        || request.cause != super::quest_state::QuestCause::Command(command)
    {
        return Ok(false);
    }
    let row = sqlx::query("SELECT i.item_instance_id::text AS item,to_jsonb(i) AS before_item,c.location,c.kind,i.quantity FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid) c JOIN game_item_instances i ON i.item_instance_id=c.item WHERE i.world_id=encode($2,'hex')::uuid AND i.lifecycle=1 AND i.definition_family='Item' AND i.definition_production_key=$3 AND i.quantity>=1 ORDER BY i.item_instance_id LIMIT 1 FOR UPDATE OF i")
        .bind(fence.character_id.as_bytes().as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice())
        .bind(key).fetch_optional(&mut **tx).await?;
    let Some(row) = row else {
        return Ok(false);
    };
    let input = Input {
        item: row.try_get("item")?,
        before: row.try_get("before_item")?,
        location: row.try_get("location")?,
        kind: row.try_get("kind")?,
        quantity: row.try_get("quantity")?,
        count: 1,
        replacement: None,
    };
    lock_location(tx, &input).await?;
    let transaction: String = sqlx::query_scalar("SELECT game_character_uuid_v7()::text")
        .fetch_one(&mut **tx)
        .await?;
    let proof = serde_json::to_vec(&serde_json::json!({"native_npc": source,
        "character": fence.character_id.as_bytes(), "session": command.game_session_id().as_bytes(),
        "command": command.command_id().get(), "connection": fence.connection_generation.get(),
        "lease": fence.character_lease_generation, "scope": fence.scope_ownership_generation.get(),
        "before": input.before, "location": input.location, "kind": input.kind, "count": 1
    }))
    .map_err(|_| DurabilityError::InvalidStoredState)?;
    sqlx::query("INSERT INTO game_quest_callback_consumptions(transaction_id,item_instance_id,character_id,cause_id,cause_ordinal,transition_key,before_item,before_ground,quantity,proof,proof_sha256,before_location,location_kind,replacement) VALUES($1::uuid,$2::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5::text::numeric,$6,$7,NULL,1,$8,sha256($8),$9,$10,NULL)")
        .bind(&transaction).bind(&input.item).bind(fence.character_id.as_bytes().as_slice())
        .bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string())
        .bind(&request.transition_key).bind(&input.before).bind(proof).bind(&input.location).bind(&input.kind)
        .execute(&mut **tx).await?;
    apply_input(tx, permit, fence, &input, &transaction).await?;
    Ok(true)
}

fn location_queries(kind: &str) -> Result<(&'static str, &'static str), DurabilityError> {
    match kind {
        "container" => Ok((
            "SELECT to_jsonb(l) FROM game_item_container_entries l \
             WHERE item_instance_id=$1::uuid FOR UPDATE",
            "DELETE FROM game_item_container_entries WHERE item_instance_id=$1::uuid",
        )),
        "nested" => Ok((
            "SELECT to_jsonb(l) FROM game_item_nested_entries l \
             WHERE item_instance_id=$1::uuid FOR UPDATE",
            "DELETE FROM game_item_nested_entries WHERE item_instance_id=$1::uuid",
        )),
        "backpack" => Ok((
            "SELECT to_jsonb(l) FROM game_item_container_slots l \
             WHERE item_instance_id=$1::uuid FOR UPDATE",
            "SELECT * FROM game_quest_callback_remove_backpack($1::uuid)",
        )),
        "equipment" => Ok((
            "SELECT to_jsonb(l) FROM game_character_equipment_slots l \
             WHERE item_instance_id=$1::uuid FOR UPDATE",
            "DELETE FROM game_character_equipment_slots WHERE item_instance_id=$1::uuid",
        )),
        _ => Err(DurabilityError::InvalidStoredState),
    }
}

async fn lock_location(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    input: &Input,
) -> Result<(), DurabilityError> {
    let query = if input.kind == "ground" {
        "SELECT to_jsonb(l) FROM game_item_ground_locations l \
         WHERE item_instance_id=$1::uuid FOR UPDATE"
    } else {
        location_queries(&input.kind)?.0
    };
    let actual: Option<serde_json::Value> = sqlx::query_scalar(query)
        .bind(&input.item)
        .fetch_optional(&mut **tx)
        .await?;
    if actual.as_ref() != Some(&input.location) {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(())
}

async fn apply_input(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    permit: &SpellLanePermit,
    fence: CurrentCharacterGameplayFence,
    input: &Input,
    transaction: &str,
) -> Result<(), DurabilityError> {
    let crate::foundation::RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = fence.runtime_scope
    else {
        return Err(DurabilityError::InvalidStoredState);
    };
    permit.check_channel(world_id, channel_id)?;
    let changed = sqlx::query(
        "UPDATE game_item_instances SET \
         quantity=CASE WHEN $4::text IS NULL THEN quantity-$2 ELSE quantity END, \
         lifecycle=CASE WHEN $4::text IS NULL AND quantity=$2 THEN 2 ELSE 1 END, \
         definition_production_key=coalesce($4,definition_production_key), \
         last_transaction_id=$3::uuid WHERE item_instance_id=$1::uuid AND lifecycle=1 AND quantity=$5",
    )
    .bind(&input.item).bind(i64::from(input.count)).bind(transaction)
    .bind(&input.replacement).bind(input.quantity)
    .execute(&mut **tx).await?;
    if changed.rows_affected() != 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    if input.replacement.is_none() && input.quantity == i64::from(input.count) {
        let query = if input.kind == "ground" {
            "DELETE FROM game_item_ground_locations WHERE item_instance_id=$1::uuid"
        } else {
            location_queries(&input.kind)?.1
        };
        let removed = sqlx::query(query)
            .bind(&input.item)
            .execute(&mut **tx)
            .await?;
        if removed.rows_affected() != 1 {
            return Err(DurabilityError::InvalidStoredState);
        }
    }
    Ok(())
}

pub(super) async fn consume_npc_inputs(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    permit: &SpellLanePermit,
    fence: CurrentCharacterGameplayFence,
    request: &super::quest_state::QuestTransitionRequest,
    plan: &oteryn_gameplay_quests::quest::npc_rules::NpcRulePlan,
) -> Result<bool, DurabilityError> {
    let super::quest_state::QuestCause::Command(command) = request.cause else {
        return Ok(false);
    };
    use oteryn_gameplay_quests::quest::npc_rules::NpcRuleEffect;
    let mut remaining = std::collections::BTreeMap::<String, i64>::new();
    for effect in &plan.effects {
        if let NpcRuleEffect::TakeItem {
            item,
            count,
            subtype: None,
            optional: false,
        } = effect
        {
            let total = remaining.entry(item.clone()).or_default();
            *total = total
                .checked_add(i64::from(*count))
                .ok_or(DurabilityError::InvalidStoredState)?;
            if *total > 100 {
                return Ok(false);
            }
        }
    }
    if remaining.is_empty() {
        return Ok(true);
    }
    if remaining.len() > 8 {
        return Ok(false);
    }
    let keys: Vec<_> = remaining.keys().cloned().collect();
    let rows=sqlx::query("SELECT i.item_instance_id::text AS item,to_jsonb(i) AS before_item,c.location,c.kind,i.quantity,i.definition_production_key FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid) c JOIN game_item_instances i ON i.item_instance_id=c.item WHERE i.lifecycle=1 AND i.definition_family='Item' AND i.definition_production_key=ANY($3::text[]) ORDER BY i.item_instance_id LIMIT 501 FOR UPDATE OF i")
        .bind(fence.character_id.as_bytes().as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice()).bind(&keys).fetch_all(&mut **tx).await?;
    if rows.len() > 500 {
        return Ok(false);
    }
    let mut inputs = Vec::new();
    for row in rows {
        let key: String = row.try_get("definition_production_key")?;
        let Some(required) = remaining.get_mut(&key) else {
            continue;
        };
        let quantity: i64 = row.try_get("quantity")?;
        let count = (*required).min(quantity);
        if count == 0 {
            continue;
        }
        let input = Input {
            item: row.try_get("item")?,
            before: row.try_get("before_item")?,
            location: row.try_get("location")?,
            kind: row.try_get("kind")?,
            quantity,
            count: u16::try_from(count).map_err(|_| DurabilityError::InvalidStoredState)?,
            replacement: None,
        };
        lock_location(tx, &input).await?;
        let plain:bool=sqlx::query_scalar("SELECT NOT (EXISTS(SELECT 1 FROM game_item_keys WHERE item_instance_id=$1::uuid) OR EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=$1::uuid) OR EXISTS(SELECT 1 FROM game_item_texts WHERE item_instance_id=$1::uuid) OR EXISTS(SELECT 1 FROM game_item_imbuements WHERE item_instance_id=$1::uuid) OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=$1::uuid) OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=$1::uuid))")
            .bind(&input.item).fetch_one(&mut **tx).await?;
        if !plain {
            return Ok(false);
        }
        *required -= count;
        inputs.push(input);
    }
    if remaining.values().any(|n| *n != 0) {
        return Ok(false);
    }
    let transaction: String = sqlx::query_scalar("SELECT game_character_uuid_v7()::text")
        .fetch_one(&mut **tx)
        .await?;
    for input in inputs {
        let proof=serde_json::to_vec(&serde_json::json!({"npc_program_digest":plan.program_digest,"selected_rules":plan.selected_rule_keys,"effects":plan.effects,"before":input.before,"location":input.location,"count":input.count})).map_err(|_|DurabilityError::InvalidStoredState)?;
        if proof.len() > 16384 {
            return Ok(false);
        }
        sqlx::query("INSERT INTO game_quest_callback_consumptions(transaction_id,item_instance_id,character_id,cause_id,cause_ordinal,transition_key,before_item,before_ground,quantity,proof,proof_sha256,before_location,location_kind) VALUES($1::uuid,$2::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5::text::numeric,$6,$7,NULL,$8,$9,sha256($9),$10,$11)")
            .bind(&transaction).bind(&input.item).bind(fence.character_id.as_bytes().as_slice()).bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string()).bind(&request.transition_key).bind(&input.before).bind(i32::from(input.count)).bind(proof).bind(&input.location).bind(&input.kind).execute(&mut **tx).await?;
        // This owner admits carried equipment/container inputs only. Ground
        // deletion remains behind apply_input's channel SpellLanePermit.
        if input.kind == "ground" {
            return Err(DurabilityError::InvalidStoredState);
        }
        apply_input(tx, permit, fence, &input, &transaction).await?;
    }
    Ok(true)
}

/// Exercise the exact durable predicate from auto-mounted PostgreSQL cases.
#[cfg(any(test, feature = "test-support"))]
pub async fn matches_ground_stack_for_test(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    authority: &super::spell_item_transaction::SpellItemScopeAuthority,
    target: &super::spell_items_abi::SpellGroundTarget,
    expected: &[oteryn_gameplay_quests::quest::world_callbacks::GroundStackEntry],
) -> Result<bool, DurabilityError> {
    ground_stack::matches(tx, authority, target, expected).await
}
