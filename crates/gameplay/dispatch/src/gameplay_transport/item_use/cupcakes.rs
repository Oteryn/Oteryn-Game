//! Source-backed cupcake USE. The three independent exhaustions apply all year.
use super::gourmet::{PendingUse, current, finish_with_cupcake, gate, resolve};
use super::*;
use crate::durability::character_condition_snapshot::CharacterConditionSnapshot;
use crate::durability::item_use::cupcakes::{CupcakeUseRequest, cupcake_id};
use crate::foundation::RuntimeScopeRefV1;
use crate::spell::actor_conditions::stage_cupcake;
fn binding(source: &UseSource) -> Option<[u8; 32]> {
    let mut hash = Sha256::new();
    hash.update(b"oteryn:cupcake-use:v1\0");
    match source {
        UseSource::Handle(ItemKey::Instance(id)) => {
            hash.update([1]);
            hash.update(id);
        }
        UseSource::Definition(id) => {
            hash.update([2]);
            hash.update(id.get().to_be_bytes());
        }
        _ => return None,
    }
    Some(hash.finalize().into())
}
pub(super) async fn use_cupcake(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    source: &UseSource,
) -> Option<Result<UseDisposition, UseDisposition>> {
    let fence = command.item_fence?;
    if !resolve(owner, actor, command.game_session_id).await {
        return Some(Err(UseDisposition::Rejected));
    }
    let wire = binding(source)?;
    match owner
        .root
        .read_cupcake_use_receipt(owner.character, fence, command.command_id, wire)
        .await
    {
        Ok(true) => return Some(Ok(UseDisposition::Committed)),
        Ok(false) => {}
        Err(_) => return Some(Err(UseDisposition::Rejected)),
    }
    // A new definition hotkey can decline this action before a carried-tree read.
    if let UseSource::Definition(id) = source {
        let runtime = owner.runtime.lock().await;
        let index =
            super::super::item_ref_admission::pinned_index(&runtime, owner.active_generation)?;
        if !index
            .definition_at(*id)
            .is_some_and(|(key, _)| cupcake_id(key).is_some())
        {
            return None;
        }
    }
    let inventory = match fence.runtime_scope {
        RuntimeScopeRefV1::Channel { .. } => owner
            .root
            .read_item_use_inventory(owner.character, owner.holder, fence)
            .await
            .ok(),
        RuntimeScopeRefV1::Instance { .. } => owner
            .root
            .read_hireling_use_inventory(
                owner.character,
                owner.holder,
                fence,
                *owner.channel_id.as_bytes(),
            )
            .await
            .ok(),
    };
    let Some(inventory) = inventory else {
        return Some(Err(UseDisposition::Rejected));
    };
    let native = owner.active_generation?.native_gameplay()?;
    let source_id = match source {
        UseSource::Handle(ItemKey::Instance(id)) => *id,
        UseSource::Definition(wanted) => {
            let runtime = owner.runtime.lock().await;
            let index =
                super::super::item_ref_admission::pinned_index(&runtime, owner.active_generation)?;
            let (key, revision) = index.definition_at(*wanted)?;
            let wanted = TypedDefinitionRef {
                family: "Item".into(),
                production_key: key.into(),
                revision_ref: revision.into(),
            };
            let mut states = owner.states.spell_states.lock().await;
            let Some(gate) = gate(&mut states, &runtime, actor, command.game_session_id, fence)
            else {
                return Some(Err(UseDisposition::Rejected));
            };
            match inventory.first_unreserved(&wanted, &gate.unspendable) {
                Ok(id) => id,
                Err(_) => return Some(Err(UseDisposition::NothingToUse)),
            }
        }
        _ => return None,
    };
    let entry = inventory
        .backpack
        .as_ref()
        .into_iter()
        .flat_map(|b| b.entries.iter())
        .chain(inventory.nested.iter().map(|(_, e)| e))
        .find(|e| e.item.item_instance_id == source_id)?;
    let item_id = cupcake_id(&entry.item.definition.production_key)?;
    if record(native, &entry.item.definition).is_none() {
        return Some(Err(UseDisposition::Rejected));
    }
    let Some(_permit) = owner.spell_lane_permit().await else {
        return Some(Err(UseDisposition::Rejected));
    };
    if !owner
        .states
        .player_trades
        .physical(command.game_session_id, Some(source_id))
    {
        return Some(Err(UseDisposition::Rejected));
    }
    let session = command.game_session_id;
    // Retain the actual physical owner lock through the native commit and successor
    // publication: a concurrent attack/cast/House handoff cannot invalidate PREPARE.
    let runtime = owner.runtime.lock().await;
    let mut states = owner.states.spell_states.lock().await;
    if !current(&runtime, actor, session, fence, false)
        || states.is_dead(actor)
        || states.has_pending_spell_commit(actor, session)
    {
        return Some(Err(UseDisposition::Rejected));
    }
    if matches!(fence.runtime_scope, RuntimeScopeRefV1::Channel { .. })
        && !states.spell_idle(&runtime, actor, session)
    {
        return Some(Err(UseDisposition::Rejected));
    }
    let state_index = states
        .actors
        .iter()
        .position(|(a, s, _)| *a == actor && *s == session)?;
    let now_ms = owner.owner_ms();
    let next = match stage_cupcake(&states.actors[state_index].2, item_id, now_ms) {
        Ok(next) => next,
        Err(_) => return Some(Err(UseDisposition::Rejected)),
    };
    let source_gate = gate(&mut states, &runtime, actor, session, fence)?;
    if let Err(result) = source_gate.admit(command.command_id, now_ms) {
        return Some(Err(result));
    }
    if source_gate.unspendable.contains(&source_id) {
        return Some(Err(UseDisposition::StaleState));
    }
    if let Err(result) = source_gate.start_drink(now_ms) {
        return Some(Err(result));
    }
    let (Ok(transaction_id), Ok(event_id), Ok(occurred_at_unix_ms)) = (
        crate::operator_files::uuid_v7(),
        crate::operator_files::uuid_v7(),
        unix_ms(),
    ) else {
        return Some(Err(UseDisposition::Rejected));
    };
    let request = CupcakeUseRequest {
        wire_binding: wire,
        conditions: CharacterConditionSnapshot {
            lease_generation: fence.character_lease_generation,
            content_digest: native.source_digest(),
            conditions: match next
                .owned_conditions()
                .persistent_snapshot(now_ms.saturating_mul(1000))
            {
                Ok(snapshot) => snapshot,
                Err(_) => return Some(Err(UseDisposition::Rejected)),
            },
        },
        item: ItemUseRequest {
            kind: ItemUseKind::Cupcake {
                origin_channel: *owner.channel_id.as_bytes(),
            },
            command_id: command.command_id,
            content_revision: inventory.content_revision.clone(),
            ruleset_revision: inventory.ruleset_revision.clone(),
            sim_revision: entry_chest::SIM_REVISION.into(),
            transaction_id,
            event_id,
            occurred_at_unix_ms,
            server_build_id: SERVER_BUILD_ID.into(),
            source_item_instance_id: source_id,
            source_definition: entry.item.definition.clone(),
            source_quantity_before: entry.item.quantity,
            flask: ItemUseFlaskPlan::None,
        },
    };
    let before_revision = states.actors[state_index].2.revision();
    if !PendingUse::new_cupcake(
        actor,
        session,
        fence,
        request.clone(),
        next,
        before_revision,
    )
    .park(&mut states)
    {
        if let Some(gate) = gate(&mut states, &runtime, actor, session, fence) {
            gate.settle(command.command_id, UseDisposition::Rejected, &[]);
        }
        return Some(Err(UseDisposition::Rejected));
    }
    let mut result = owner
        .root
        .commit_cupcake_use(owner.character, owner.holder, fence, request)
        .await;
    if matches!(result, Err(ItemUseError::Unavailable(_))) {
        // Reconcile while the prepared native vitals successor remains protected
        // by the same owner lock. This read serializes behind the original writer.
        if let Ok(true) = owner
            .root
            .read_cupcake_use_receipt(owner.character, fence, command.command_id, wire)
            .await
        {
            result = Ok(ItemUseOutcome::Written {
                shape: crate::durability::item_use_audit::ItemUseShape::Burn,
            });
        }
    }
    Some(Ok(finish_with_cupcake(
        owner,
        &runtime,
        &mut states,
        actor,
        session,
        result,
        owner.owner_now().get(),
    )
    .unwrap_or(UseDisposition::Rejected)))
}
