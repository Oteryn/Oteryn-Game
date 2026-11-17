//! World callback inputs share one atomic quest receipt and consumption proof path.
pub mod carried_step;
pub(super) mod carrot;
pub(crate) mod character_requirements;
mod completed_quests;
pub(super) mod completion;
mod ground_read;
pub(crate) mod ground_stack;
#[path = "quest_world_callback_npc_input.rs"]
mod npc_input;
mod selected_tool;
mod tool_reads;
pub use selected_tool::QuestWorldToolContext;

/// Test-only entry for the lifted PostgreSQL harness; the opaque transaction
/// authority and all production Ground locks are checked by the same participant.
#[cfg(any(test, feature = "test-support"))]
pub async fn test_ground_stack_matches_in_transaction(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    authority: &super::spell_item_transaction::SpellItemScopeAuthority,
    target: &super::spell_items_abi::SpellGroundTarget,
    expected: &[oteryn_gameplay_quests::quest::world_callbacks::GroundStackEntry],
) -> Result<bool, super::DurabilityError> {
    ground_stack::matches(tx, authority, target, expected).await
}

pub mod shared_count;
mod shared_input;
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub party: Vec<QuestWorldPartyMember>,
}

/// Frozen station membership is part of the original immutable offering receipt.
/// The runtime proves placement; the writer rechecks every session/lease/scope
/// and base vocation before consuming any altar input.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuestWorldPartyMember {
    pub character: [u8; 16],
    pub session: [u8; 16],
    pub connection_generation: u64,
    pub lease_generation: u64,
    pub scope_generation: u64,
    pub actor_placement: [u8; 16],
    pub movement_revision: u64,
}

#[path = "quest_world_callback/carried_batch.rs"]
mod carried_batch;

#[path = "quest_world_callback/repair_inputs.rs"]
mod repair_inputs;
#[path = "quest_world_callback/reward_inputs.rs"]
mod reward_inputs;
#[path = "quest_world_callback/reward_inventory.rs"]
mod reward_inventory;

impl super::DurabilityRoot {
    /// Read a literal callback's quest gates under the live session and Character
    /// root fences. This path never writes Quest state, receipts or revisions.
    #[allow(
        clippy::too_many_arguments,
        reason = "the read owner requires the same authority and world context as callback writes"
    )]
    pub async fn check_character_world_callback(
        &self,
        authority: &super::character_authority::ReconciledCharacterAuthority<'_, '_>,
        node: &super::runtime_scope_assignment::NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        callback_key: &str,
        catalogue: std::sync::Arc<oteryn_gameplay_quests::quest::QuestStateCatalogue>,
        permit: &SpellLanePermit,
        context: QuestWorldContext,
    ) -> Result<bool, super::character_progression::CharacterProgressionError> {
        use super::character_authority::assert_recovery_fence;
        use super::character_progression::{CharacterProgressionError, assert_item_only_fence};
        use super::db::{
            begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
        };
        let callback = oteryn_gameplay_quests::quest::world_callbacks::load()
            .map_err(|_| CharacterProgressionError::InvalidInput)?
            .into_iter()
            .find(|callback| {
                callback.key == callback_key
                    && (callback.is_read_only() || callback.effects.scene.is_some())
            })
            .ok_or(CharacterProgressionError::InvalidInput)?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        let mut context = (permit, context);
        self.try_issue_semantic_pass()?.run_with_context(&mut context, move |holder, deadline, (permit, context)| {
            Box::pin(async move {
                let mut tx = begin_semantic_transaction(holder, deadline).await?;
                assert_recovery_fence(&mut tx, &recovery).await?;
                lock_admission_relations(&mut tx).await?;
                if let Err(error) = assert_item_only_fence(&mut tx, &fence.item_fence(), &node).await? {
                    return Ok(Err(error));
                }
                if !context_is_current(&mut tx, permit, fence, context).await? {
                    return Ok(Ok(false));
                }
                let current: Option<String> = sqlx::query_scalar(
                    "SELECT content_revision FROM game_character_progression_state \
                     WHERE character_id=encode($1,'hex')::uuid",
                ).bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
                if current.as_deref() != Some(catalogue.content_revision()) {
                    return Ok(Err(CharacterProgressionError::ProgressionContextMismatch));
                }
                let pending: bool = sqlx::query_scalar(
                    "SELECT EXISTS(SELECT 1 FROM game_character_pending_respawns \
                     WHERE character_id=encode($1,'hex')::uuid)",
                ).bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
                if pending {
                    return Ok(Err(CharacterProgressionError::RespawnPending));
                }
                let pin = sqlx::query(
                    "SELECT pinned_content_revision,definition_hash,completed_character_revision IS NOT NULL AS completed \
                     FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2",
                ).bind(fence.character_id.as_bytes().as_slice()).bind(&callback.quest)
                    .fetch_optional(&mut *tx).await?;
                if let Some(pin) = pin {
                    let pin = super::quest_state::QuestStateRecord {
                        pinned_content_revision: pin.try_get("pinned_content_revision")?,
                        definition_hash: pin.try_get::<Vec<u8>, _>("definition_hash")?.try_into()
                            .map_err(|_| DurabilityError::InvalidStoredState)?,
                        completed: pin.try_get("completed")?,
                    };
                    if !read_only_pin_allows(&pin, &catalogue, &callback.quest)? {
                        return Ok(Err(CharacterProgressionError::ProgressionContextMismatch));
                    }
                }
                for storage in &callback.storage {
                    let Some(track) = catalogue.track(&storage.track).filter(|track| track.quest == callback.quest) else {
                        return Ok(Err(CharacterProgressionError::InvalidInput));
                    };
                    let value: Option<i64> = sqlx::query_scalar(
                        "SELECT value FROM game_character_quest_tracks \
                         WHERE character_id=encode($1,'hex')::uuid AND track_key=$2 AND quest_key=$3",
                    ).bind(fence.character_id.as_bytes().as_slice()).bind(&storage.track)
                        .bind(&callback.quest).fetch_optional(&mut *tx).await?;
                    if !(storage.minimum..=storage.maximum).contains(&value.unwrap_or(track.initial)) {
                        return Ok(Ok(false));
                    }
                }
                commit_semantic_transaction(tx, deadline).await?;
                Ok(Ok(true))
            })
        }).await?
    }
}

fn read_only_pin_allows(
    stored: &super::quest_state::QuestStateRecord,
    catalogue: &oteryn_gameplay_quests::quest::QuestStateCatalogue,
    quest: &str,
) -> Result<bool, DurabilityError> {
    let current_hash = catalogue
        .definition_hash(quest)
        .ok_or(DurabilityError::InvalidStoredState)?;
    // QUEST-STATE-0 §6: unchanged definitions retain their historical content
    // pin; completed quests keep access even when the definition changes.
    Ok(stored.definition_hash == current_hash || stored.completed)
}

struct Input {
    item: String,
    before: serde_json::Value,
    location: serde_json::Value,
    kind: String,
    quantity: i64,
    count: u16,
    replacement: Option<String>,
}

/// The sealed source movement uses the same Ground consumption participant and
/// Quest receipt. It never creates a client command or a second Item writer.
#[allow(
    clippy::too_many_arguments,
    reason = "the sampled quest clock and current scope authority are independent owner inputs"
)]
pub(crate) async fn consume_soul_war_source_step(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    permit: &SpellLanePermit,
    fence: CurrentCharacterGameplayFence,
    request: &super::quest_state::QuestTransitionRequest,
    step: &super::quest_state::soul_war_source::step::SoulWarSourceStep,
    quest_clock_seconds: i64,
    context: &QuestWorldToolContext,
    scope_guard: (
        &crate::character_recovery_fence::CharacterRecoveryFenceV1,
        &super::runtime_scope_assignment::NodeIncarnationProof,
    ),
) -> Result<bool, DurabilityError> {
    use super::quest_state::soul_war_source::step::SourceStepKind;
    use super::spell_item_transaction::{
        assert_spell_item_scope_with_recovery, read_creature_scope_tile_in_transaction,
    };
    let transition = match step.kind() {
        SourceStepKind::SpiteFire => "oteryn:quest-transition/soul-war/source-items/spite-fire",
        SourceStepKind::Remorse => "oteryn:quest-transition/soul-war/source-items/remorse",
        SourceStepKind::Energy => return Ok(false),
    };
    if !step.matches_context(context)
        || step.game_session_id() != fence.game_session_id
        || step.runtime_scope() != fence.runtime_scope
        || step.scope_generation() != fence.scope_ownership_generation
        || !matches!(request.cause, super::quest_state::QuestCause::SourceStep(_))
        || request.transition_key != transition
    {
        return Ok(false);
    }
    let scope = assert_spell_item_scope_with_recovery(
        tx,
        permit,
        scope_guard.0,
        scope_guard.1,
        fence.runtime_scope,
        fence.scope_ownership_generation.get(),
    )
    .await
    .map_err(|_| DurabilityError::Unavailable)?;
    let epoch:String=sqlx::query_scalar("SELECT coalesce((SELECT reset_epoch FROM game_world_reset_active WHERE world_id=encode($1,'hex')::uuid),0)::text")
        .bind(fence.runtime_scope.world_id().as_bytes().as_slice()).fetch_one(&mut **tx).await?;
    if epoch != context.reset_epoch.to_string() {
        return Ok(false);
    }
    let current = read_creature_scope_tile_in_transaction(tx, &scope, step.target())
        .await
        .map_err(|_| DurabilityError::Unavailable)?;
    let selected = current
        .items
        .iter()
        .find(|item| item.definition == *step.source_definition());
    if selected.is_none_or(|item| item.item_instance_id != step.item_instance_id()) {
        return Ok(false);
    }
    let row=sqlx::query("SELECT i.item_instance_id::text AS item,to_jsonb(i) AS before_item,to_jsonb(g) AS before_ground,i.quantity FROM game_item_instances i JOIN game_item_ground_locations g USING(item_instance_id) WHERE i.item_instance_id=encode($1,'hex')::uuid AND i.lifecycle=1 FOR UPDATE OF i")
        .bind(step.item_instance_id().as_slice()).fetch_optional(&mut **tx).await?;
    let Some(row) = row else {
        return Ok(false);
    };
    if row.try_get::<i64, _>("quantity")? != 1 {
        return Ok(false);
    }
    let input = Input {
        item: row.try_get("item")?,
        before: row.try_get("before_item")?,
        location: row.try_get("before_ground")?,
        kind: "ground".into(),
        quantity: 1,
        count: 1,
        replacement: None,
    };
    let proof =
        serde_json::to_vec(&step.audit_source(&fence, scope_guard.1, request, quest_clock_seconds))
            .map_err(|_| DurabilityError::InvalidStoredState)?;
    let transaction: String = sqlx::query_scalar("SELECT game_character_uuid_v7()::text")
        .fetch_one(&mut **tx)
        .await?;
    let (cause, ordinal) = request.cause.key();
    sqlx::query("INSERT INTO game_quest_callback_consumptions(transaction_id,item_instance_id,character_id,cause_id,cause_ordinal,transition_key,before_item,before_ground,quantity,proof,proof_sha256,before_location,location_kind,replacement) VALUES($1::uuid,$2::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5::text::numeric,$6,$7,$8,1,$9,sha256($9),NULL,'ground',NULL)")
        .bind(&transaction).bind(&input.item).bind(fence.character_id.as_bytes().as_slice()).bind(cause.as_slice()).bind(ordinal.to_string())
        .bind(&request.transition_key).bind(&input.before).bind(&input.location).bind(proof).execute(&mut **tx).await?;
    apply_input_in_scope(tx, permit, fence, &input, &transaction).await?;
    Ok(true)
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
    match request.cause {
        super::quest_state::QuestCause::Command(_) if callback.world_claim.is_none() => {}
        super::quest_state::QuestCause::Use(_) if callback.world_claim.is_some() => {}
        _ => return Ok(false),
    }
    let mut keys: Vec<_> = callback
        .take
        .iter()
        .chain(&callback.present)
        .map(|i| i.item.as_str())
        .collect();
    keys.extend(callback.inventory.iter().map(|rule| rule.item.as_str()));
    keys.sort_unstable();
    keys.dedup();
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
    if let Some(target_key) = &callback.carried_target {
        let Some(use_) = carried_use else {
            return Ok(false);
        };
        let Some(target) = use_.recipe_target else {
            return Ok(false);
        };
        if target == use_.source {
            return Ok(false);
        }
        let selected = sqlx::types::Uuid::from_bytes(target).to_string();
        if !rows.iter().any(|row| {
            row.try_get::<&str, _>("item").ok() == Some(selected.as_str())
                && row.try_get::<&str, _>("definition_production_key").ok()
                    == Some(target_key.as_str())
        }) {
            return Ok(false);
        }
    }
    if !reward_inventory::holds(&rows, &callback.inventory)? {
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
        let key: String = row.try_get("definition_production_key")?;
        if let Some(use_) = carried_use {
            let selected = if callback.carried_item.as_deref() == Some(key.as_str()) {
                Some(use_.source)
            } else if callback.carried_target.as_deref() == Some(key.as_str()) {
                use_.recipe_target
            } else if callback.carried_target.is_some() || callback.carried_exchange {
                None // Additional recipe ingredients use the ordinary bounded stack allocator.
            } else {
                continue;
            };
            if let Some(selected) = selected {
                let selected = sqlx::types::Uuid::from_bytes(selected).to_string();
                if row.try_get::<&str, _>("item")? != selected {
                    continue;
                }
            }
        }
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

#[allow(
    clippy::too_many_arguments,
    reason = "the existing fenced participant also receives the exact locked quest plan"
)]
pub(super) async fn consume(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    permit: &SpellLanePermit,
    fence: CurrentCharacterGameplayFence,
    request: &super::quest_state::QuestTransitionRequest,
    callback: &Callback,
    changes: &[oteryn_gameplay_quests::quest::QuestTrackChange],
    context: &QuestWorldToolContext,
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
    // The Quest writer returned any exact immutable receipt before this fresh
    // participant. Legacy repeated-seal counters cannot bypass distinct targets.
    if oteryn_gameplay_quests::quest::ice_seal_paint::historical_only(&callback.key) {
        return Ok(false);
    }
    let crate::foundation::RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = fence.runtime_scope
    else {
        return Ok(false);
    };
    let exact = callback
        .item_on_target
        .as_ref()
        .is_some_and(|item| item.selected_instance);
    if exact != context.selected_tool.is_some() {
        return Ok(false);
    }
    if !context_is_current(tx, permit, fence, context).await? {
        return Ok(false);
    }
    if !completed_quests::require(tx, fence, &callback.required_completed_quests).await? {
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
    if let Some(admission) = &callback.admission {
        if admission.promotion
            && !super::character_promotion::read_durable_promotion(tx, fence.character_id).await?
        {
            return Ok(false);
        }
        if admission.regular_blessings {
            let held: Vec<String> = sqlx::query_scalar(
                "SELECT blessing_key FROM game_character_blessings WHERE character_id=encode($1,'hex')::uuid ORDER BY blessing_key LIMIT 33",
            ).bind(fence.character_id.as_bytes().as_slice()).fetch_all(&mut **tx).await?;
            let catalogue =
                oteryn_gameplay_content::content::blessing_catalogue::BlessingCatalogue::load()
                    .map_err(|_| DurabilityError::InvalidStoredState)?;
            if !catalogue
                .regular()
                .all(|blessing| held.contains(&blessing.storage_key))
            {
                return Ok(false);
            }
        }
    }
    for requirement in &callback.world_requirements {
        let oteryn_gameplay_quests::quest::world_callbacks::WorldRequirement {
            position,
            ground: Some(ground),
            ..
        } = requirement
        else {
            continue;
        };
        if !callback.has_personal_state()
            || !ground_read::matches(
                tx,
                &ground_read::Context {
                    world: world_id.as_bytes(),
                    channel: channel_id.as_bytes(),
                    generation: fence.scope_ownership_generation.get(),
                    map: &context.map_revision,
                    content: &context.content_revision,
                    placement: &context.placement_context,
                },
                *position,
                ground,
            )
            .await?
        {
            return Ok(false);
        }
    }
    if let Some(requirements) = &callback.character_requirements
        && !character_requirements::satisfied(tx, fence.character_id, requirements).await?
    {
        return Ok(false);
    }
    // The Character root and current session are already fenced. Lock the actual
    // carried instances before testing quantities; map objects never prove ownership.
    for evidence in &callback.present {
        let quantities: Vec<i64> = sqlx::query_scalar(
            "SELECT i.quantity FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid) c \
             JOIN game_item_instances i ON i.item_instance_id=c.item \
             WHERE i.lifecycle=1 AND i.definition_family='Item' AND i.definition_production_key=$3 \
             ORDER BY i.item_instance_id LIMIT 501 FOR UPDATE OF i",
        ).bind(fence.character_id.as_bytes().as_slice()).bind(world_id.as_bytes().as_slice())
            .bind(&evidence.item).fetch_all(&mut **tx).await?;
        if quantities.len() > 500 || quantities.iter().sum::<i64>() < i64::from(evidence.count) {
            return Ok(false);
        }
    }
    // NPC travel locks the Character root but does not advance its revision.
    // A trip admitted after the quest milestone therefore carries exactly the
    // milestone's committed revision. Every trip serialized before it has a
    // strictly smaller revision because the quest milestone itself advances it.
    for route in &callback.required_travel {
        let arrived: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM game_newhaven_travel_receipts r \
             LEFT JOIN game_item_fee_burns f ON f.transaction_id=r.fee_transaction_id \
             WHERE r.character_id=encode($1,'hex')::uuid AND r.route=$2 AND r.arrived_at IS NOT NULL \
             AND ($3::text IS NULL OR EXISTS(SELECT 1 FROM game_character_quest_receipts q \
                 WHERE q.character_id=r.character_id AND q.quest_key=$4 AND q.transition_key=$3 \
                 AND f.character_id=r.character_id \
                 AND f.committed_character_revision>=q.committed_character_revision)))",
        ).bind(fence.character_id.as_bytes().as_slice()).bind(&route.route)
            .bind(&route.after_transition).bind(&callback.quest).fetch_one(&mut **tx).await?;
        if !arrived {
            return Ok(false);
        }
    }
    if callback.offerings.is_empty()
        && callback.item_on_target.is_none()
        && callback.effects.carried_inputs.is_empty()
    {
        return Ok(true);
    }
    consume_inputs(
        tx, permit, fence, request, callback, changes, context, command, world_id, channel_id,
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
    changes: &[oteryn_gameplay_quests::quest::QuestTrackChange],
    context: &QuestWorldToolContext,
    command: crate::foundation::CommandRef,
    world_id: crate::foundation::WorldId,
    channel_id: crate::foundation::ChannelId,
) -> Result<bool, DurabilityError> {
    // Resolve and lock every input before any write, including replacement preconditions.
    let mut inputs = Vec::new();
    let solid_repair = callback
        .passages
        .iter()
        .any(|p| p.replacement_passable == Some(false));
    if solid_repair {
        let Some(required) = &callback.item_on_target else {
            return Ok(false);
        };
        let Some(resolved) = repair_inputs::collect(tx, fence, required).await? else {
            return Ok(false);
        };
        inputs.extend(resolved);
    } else if let Some(required) = &callback.item_on_target {
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
             AND i.quantity >= $4 AND ($5::bytea IS NULL OR i.item_instance_id=encode($5,'hex')::uuid) ORDER BY i.item_instance_id LIMIT 1 FOR UPDATE OF i",
        )
        .bind(fence.character_id.as_bytes().as_slice())
        .bind(world_id.as_bytes().as_slice())
        .bind(&accepted)
        .bind(i64::from(required.count))
        .bind(context.selected_tool.as_ref().map(|tool| tool.source.to_vec()))
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
        if required.reusable
            && oteryn_gameplay_quests::quest::dreamer_fire_bug::shared_input(callback)
        {
            shared_input::retain(tx, fence, request, callback, context, &input).await?;
        }
        let consume = match callback.native_rule {
            Some(oteryn_gameplay_quests::quest::native_rules::NativeRule::IceSealPaint {
                seal,
            }) => {
                use oteryn_gameplay_quests::quest::ice_islands_seals;
                if required.item != ice_islands_seals::PAINT
                    || required.count != 1
                    || !required.reusable
                    || required.replacement.is_some()
                    || required.target_replacement.is_some()
                    || !required.alternatives.is_empty()
                {
                    return Err(DurabilityError::InvalidStoredState);
                }
                ice_islands_seals::consumes_paint(seal, changes)
                    .map_err(|_| DurabilityError::InvalidStoredState)?
            }
            _ => !required.reusable,
        };
        if consume {
            inputs.push(input);
        } else if required.selected_instance
            && !tool_reads::record(tx, fence, request, callback, context, &input).await?
        {
            return Ok(false);
        }
        for material in &required.ingredients {
            let row = sqlx::query(
                "SELECT i.item_instance_id::text AS item,to_jsonb(i) AS before_item, \
                 c.location,c.kind,i.quantity FROM \
                 game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid) c \
                 JOIN game_item_instances i ON i.item_instance_id=c.item \
                 WHERE i.world_id=encode($2,'hex')::uuid AND i.lifecycle=1 \
                 AND i.definition_family='Item' AND i.definition_production_key=$3 \
                 AND i.quantity >= $4 ORDER BY i.item_instance_id LIMIT 1 FOR UPDATE OF i",
            )
            .bind(fence.character_id.as_bytes().as_slice())
            .bind(world_id.as_bytes().as_slice())
            .bind(&material.item)
            .bind(i64::from(material.count))
            .fetch_optional(&mut **tx)
            .await?;
            let Some(row) = row else { return Ok(false) };
            let input = Input {
                item: row.try_get("item")?,
                before: row.try_get("before_item")?,
                location: row.try_get("location")?,
                kind: row.try_get("kind")?,
                quantity: row.try_get("quantity")?,
                count: material.count,
                replacement: None,
            };
            if inputs.iter().any(|prior| prior.item == input.item) {
                return Ok(false);
            }
            lock_location(tx, &input).await?;

            inputs.push(input);
        }
    }
    // Distinct definitions prevent one instance satisfying multiple requirements.
    // Resolve every carried input before the shared BURN loop changes any item.
    let mut carried = callback.effects.carried_inputs.iter().collect::<Vec<_>>();
    carried.sort_by(|left, right| left.item.cmp(&right.item));
    for required in carried {
        let Some(resolved) = carried_batch::resolve(tx, fence, required).await? else {
            return Ok(false);
        };
        for input in &resolved {
            lock_location(tx, input).await?;
        }
        inputs.extend(resolved);
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
            count: if callback.effects.party_transfer.is_some() {
                let quantity: i64 = row.try_get("quantity")?;
                let Ok(count) = u16::try_from(quantity) else {
                    return Ok(false);
                };
                // Retain the existing altar consumption proof bound. A source
                // remove() takes the whole matching stack, never a partial burn.
                if !(1..=100).contains(&count) {
                    return Ok(false);
                }
                count
            } else {
                offering.count
            },
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
    apply_input_in_scope(tx, permit, fence, input, transaction).await
}

async fn apply_input_in_scope(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    permit: &SpellLanePermit,
    fence: CurrentCharacterGameplayFence,
    input: &Input,
    transaction: &str,
) -> Result<(), DurabilityError> {
    permit.check_scope(fence.runtime_scope)?;
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
    let mut remaining = std::collections::BTreeMap::<(String, Option<u16>), i64>::new();
    for effect in &plan.effects {
        if let NpcRuleEffect::TakeItem {
            item,
            count,
            subtype,
            optional: false,
        } = effect
        {
            let total = remaining.entry((item.clone(), *subtype)).or_default();
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
    let cooking = plan.quest == oteryn_gameplay_quests::quest::hot_cuisine::QUEST
        && plan.selected_rule_keys.first().is_some_and(|key| {
            key.starts_with("oteryn:quest-npc/hot_cuisine_quest/jean_pierre/native-cook-")
        });
    if remaining.len() > if cooking { 16 } else { 8 } {
        return Ok(false);
    }
    let keys: Vec<_> = remaining.keys().map(|(item, _)| item.clone()).collect();
    let rows=sqlx::query("SELECT i.item_instance_id::text AS item,to_jsonb(i) AS before_item,c.location,c.kind,i.quantity,i.definition_production_key,(SELECT f.fluid_subtype FROM game_item_fluid_states f WHERE f.item_instance_id=i.item_instance_id) AS fluid_subtype FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid) c JOIN game_item_instances i ON i.item_instance_id=c.item WHERE i.lifecycle=1 AND i.definition_family='Item' AND i.definition_production_key=ANY($3::text[]) ORDER BY i.item_instance_id LIMIT 501 FOR UPDATE OF i")
        .bind(fence.character_id.as_bytes().as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice()).bind(&keys).fetch_all(&mut **tx).await?;
    if rows.len() > 500 {
        return Ok(false);
    }
    let mut inputs = Vec::new();
    for row in rows {
        let key: String = row.try_get("definition_production_key")?;
        let subtype = row
            .try_get::<Option<i16>, _>("fluid_subtype")?
            .map(u16::try_from)
            .transpose()
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        let Some(required) = remaining.get_mut(&(key, subtype)) else {
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
        npc_input::apply(tx, permit, fence, &input, &transaction).await?;
    }
    Ok(true)
}

/// Admission locks already serialize lifecycle writers. Read every member under
/// its real Item fence before the first offering mutation; a copied runtime
/// vocation or a stale connection cannot admit the rite.
pub(super) async fn party_is_current(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    node: &super::runtime_scope_assignment::NodeIncarnationProof,
    initiator: CurrentCharacterGameplayFence,
    callback: &Callback,
    context: &QuestWorldContext,
) -> Result<bool, DurabilityError> {
    use crate::foundation::{ConnectionGeneration, GameSessionId, ScopeOwnershipGeneration};
    let Some(party) = &callback.effects.party_transfer else {
        return Ok(context.party.is_empty());
    };
    if context.party.len() != party.stations.len() || !(1..=8).contains(&context.party.len()) {
        return Ok(false);
    }
    let mut characters = std::collections::BTreeSet::new();
    let mut sessions = std::collections::BTreeSet::new();
    for (member, station) in context.party.iter().zip(&party.stations) {
        if !characters.insert(member.character)
            || !sessions.insert(member.session)
            || member.actor_placement == [0; 16]
            || member.movement_revision == 0
            || member.scope_generation != initiator.scope_ownership_generation.get()
        {
            return Ok(false);
        }
        let (
            Ok(character_id),
            Ok(game_session_id),
            Ok(connection_generation),
            Ok(scope_ownership_generation),
        ) = (
            crate::domain::CharacterId::from_bytes(member.character),
            GameSessionId::decode(&member.session),
            ConnectionGeneration::new(member.connection_generation),
            ScopeOwnershipGeneration::new(member.scope_generation),
        )
        else {
            return Ok(false);
        };
        let fence = super::item_transfer::CurrentCharacterItemFence {
            character_id,
            game_session_id,
            connection_generation,
            character_lease_generation: member.lease_generation,
            scope_ownership_generation,
            ..initiator.item_fence()
        };
        if super::character_progression::assert_item_only_fence(tx, &fence, node)
            .await?
            .is_err()
        {
            return Ok(false);
        }
        let vocation: Option<String> = sqlx::query_scalar(
            "SELECT vocation FROM game_character_build_state WHERE character_id=encode($1,'hex')::uuid FOR UPDATE",
        )
        .bind(member.character.as_slice())
        .fetch_optional(&mut **tx)
        .await?;
        let base = match vocation.as_deref() {
            Some("druid" | "elder_druid") => "druid",
            Some("sorcerer" | "master_sorcerer") => "sorcerer",
            Some("paladin" | "royal_paladin") => "paladin",
            Some("knight" | "elite_knight") => "knight",
            Some("monk" | "exalted_monk") => "monk",
            _ => return Ok(false),
        };
        if base != station.vocation {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Read consumed once claims under the same Character fence and physical quest transaction.
/// The once identity is family/key; a later definition revision never reopens a claim.
pub(super) async fn completion_claims_are_current(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterGameplayFence,
    callback: &Callback,
) -> Result<bool, DurabilityError> {
    let Some(claimed_rewards) = callback
        .effects
        .quest_completion
        .as_ref()
        .and_then(|completion| completion.claimed_rewards())
    else {
        return Ok(true);
    };
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_reward_claims WHERE character_id=encode($1,'hex')::uuid AND claim_family='RewardClaim' AND claim_production_key=ANY($2::text[])")
        .bind(fence.character_id.as_bytes().as_slice())
        .bind(claimed_rewards)
        .fetch_one(&mut **tx).await?;
    Ok(usize::try_from(count).ok() == Some(claimed_rewards.len()))
}

#[cfg(test)]
mod read_only_pin_tests {
    use super::*;
    use oteryn_gameplay_quests::quest::{QuestStateCatalogue, QuestTrack};

    #[test]
    fn read_only_callback_keeps_access_on_an_unchanged_historical_content_pin() -> Result<(), String>
    {
        let quest = "oteryn:quest.fixture.read_only";
        let catalogue = QuestStateCatalogue::new(
            "content-current",
            vec![QuestTrack {
                key: "oteryn:quest-progress/fixture/read-only".into(),
                quest: quest.into(),
                initial: -1,
                min: -1,
                max: 13,
            }],
            vec![],
        )
        .map_err(|error| format!("{error:?}"))?;
        let mut stored = super::super::quest_state::QuestStateRecord {
            pinned_content_revision: "content-previous".into(),
            definition_hash: catalogue.definition_hash(quest).ok_or("hash")?,
            completed: false,
        };
        assert_ne!(stored.pinned_content_revision, catalogue.content_revision());
        assert!(
            read_only_pin_allows(&stored, &catalogue, quest)
                .map_err(|error| format!("{error:?}"))?
        );
        stored.definition_hash[0] ^= 1;
        assert!(
            !read_only_pin_allows(&stored, &catalogue, quest)
                .map_err(|error| format!("{error:?}"))?
        );
        stored.completed = true;
        assert!(
            read_only_pin_allows(&stored, &catalogue, quest)
                .map_err(|error| format!("{error:?}"))?
        );
        Ok(())
    }
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

#[cfg(test)]
#[path = "quest_world_callback/entry_postgres_tests.rs"]
mod entry_postgres_tests;

#[cfg(any(test, feature = "test-support"))]
pub use ground_stack::fixture_matches as fixture_ground_stack_matches;
