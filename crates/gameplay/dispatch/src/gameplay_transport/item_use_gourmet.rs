//! Closed gourmet actions composed with the existing native USE, condition and
//! vitals owners. A House observation proves actual named custody; its origin
//! Channel is never substituted for the admitted inventory fence.
use super::*;
use crate::durability::item_use::gourmet::{GourmetUseRequest, gourmet_item_id, is_gourmet_key};
use crate::foundation::RuntimeScopeRefV1;
use crate::spell::actor_conditions::{gourmet_conditions, stage_gourmet_food};
use oteryn_gameplay_domain::domain::hireling::food::GourmetConditions;
#[path = "item_use_gourmet/recovery.rs"]
mod recovery;
pub(in crate::gameplay_transport) use recovery::restore_house;
#[path = "item_use_gourmet/pending.rs"]
mod pending;
pub(in crate::gameplay_transport) use pending::PendingUse;
pub(in crate::gameplay_transport) use pending::resolve;

pub(in crate::gameplay_transport) fn remember(
    states: &mut ChannelSpellStates,
    actor: ExactActorRef,
    session: GameSessionId,
    saved: GourmetConditions,
    now_us: u64,
) {
    states
        .gourmet_checkpoints
        .retain(|(a, s, _, _)| *a != actor || *s != session);
    if saved != GourmetConditions::default() {
        states
            .gourmet_checkpoints
            .push((actor, session, saved, now_us));
    }
}

fn binding(source: &UseSource) -> Option<[u8; 32]> {
    let mut hash = Sha256::new();
    hash.update(b"oteryn:gourmet-food-use:v1\0");
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

fn current(
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    session: GameSessionId,
    fence: CurrentCharacterItemFence,
    allow_control_loss: bool,
) -> bool {
    let facts = match fence.runtime_scope {
        RuntimeScopeRefV1::Channel {
            world_id,
            channel_id,
        } => {
            let held = runtime.binding();
            if held.world_id() != world_id
                || held.channel_id() != channel_id
                || held.scope_generation() != fence.scope_ownership_generation
            {
                return false;
            }
            runtime.player_control_facts(actor, session)
        }
        RuntimeScopeRefV1::Instance { .. } => {
            let Ok(house) = runtime.house_actor_state(actor, session) else {
                return false;
            };
            if house.scope != fence.runtime_scope
                || house.generation != fence.scope_ownership_generation
            {
                return false;
            }
            runtime.house_player_control_facts(actor, session)
        }
    };
    facts.is_ok_and(|facts| allow_control_loss || facts.control_loss.is_none())
}

/// Reuse the same item-use rate and in-flight gate after independent House proof.
/// The generic gate still refuses Instance actors for every unrelated consumer.
fn gate<'a>(
    states: &'a mut ChannelSpellStates,
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    session: GameSessionId,
    fence: CurrentCharacterItemFence,
) -> Option<&'a mut ItemUseGate> {
    if !current(runtime, actor, session, fence, false) {
        return None;
    }
    let gates = &mut states.item_uses.0;
    gates.retain(|(a, s, _)| {
        runtime.player_control_facts(*a, *s).is_ok()
            || runtime.house_player_control_facts(*a, *s).is_ok()
    });
    let index = if let Some(index) = gates
        .iter()
        .position(|(a, s, _)| *a == actor && *s == session)
    {
        index
    } else {
        gates.push((actor, session, ItemUseGate::default()));
        gates.len() - 1
    };
    gates.get_mut(index).map(|(_, _, gate)| gate)
}

pub(super) async fn use_gourmet(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    source: &UseSource,
) -> Option<Result<UseDisposition, UseDisposition>> {
    let fence = command.item_fence?;
    if !pending::resolve(owner, actor, command.game_session_id).await {
        return Some(Err(UseDisposition::Rejected));
    }
    let wire = binding(source)?;
    match owner
        .root
        .read_gourmet_use_receipt(owner.character, fence, command.command_id, wire)
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
            .is_some_and(|(key, _)| is_gourmet_key(key))
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
    let item_id = gourmet_item_id(&entry.item.definition.production_key)?;
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
    let next = match stage_gourmet_food(&states.actors[state_index].2, item_id, now_ms) {
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
    let request = GourmetUseRequest {
        wire_binding: wire,
        conditions: gourmet_conditions(&next, now_ms.saturating_mul(1000)),
        item: ItemUseRequest {
            kind: ItemUseKind::HirelingFood {
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
    if !PendingUse::new(
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
        .commit_gourmet_use(owner.character, owner.holder, fence, request)
        .await;
    if matches!(result, Err(ItemUseError::Unavailable(_))) {
        // Reconcile while the prepared native vitals successor remains protected
        // by the same owner lock. This read serializes behind the original writer.
        if let Ok(true) = owner
            .root
            .read_gourmet_use_receipt(owner.character, fence, command.command_id, wire)
            .await
        {
            result = Ok(ItemUseOutcome::Written {
                shape: crate::durability::item_use_audit::ItemUseShape::Burn,
            });
        }
    }
    Some(Ok(pending::finish(
        &mut states,
        actor,
        session,
        result,
        owner.owner_now().get(),
    )
    .unwrap_or(UseDisposition::Rejected)))
}

/// Periodic and terminal checkpoints come from the same live native conditions.
/// The durable writer only reduces ticks, so this cannot introduce a new buff.
pub(in crate::gameplay_transport) async fn checkpoint(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
    force: bool,
) -> bool {
    checkpoint_inner(owner, actor, session, force, false).await
}

/// Actor-end owners may continue after a known stale authority refusal, just as
/// their existing monk/training saves do. This does not claim a checkpoint was
/// written: the last valid durable condition projection remains. Unknown native
/// USE or database completion still holds the actor and its exact successor.
pub(in crate::gameplay_transport) async fn checkpoint_at_actor_end(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
) -> bool {
    checkpoint_inner(owner, actor, session, true, true).await
}

async fn checkpoint_inner(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
    force: bool,
    allow_fenced_out: bool,
) -> bool {
    if !super::super::hireling::resolve_pending_lamp(owner, actor, session).await {
        return false;
    }
    if !pending::resolve(owner, actor, session).await {
        return false;
    }
    {
        let states = owner.states.spell_states.lock().await;
        let Some((_, _, _, at)) = states
            .gourmet_checkpoints
            .iter()
            .find(|(a, s, _, _)| *a == actor && *s == session)
        else {
            return true;
        };
        if !force && owner.owner_now().get().saturating_sub(*at) < 1_000_000 {
            return true;
        }
    }
    let Some(_permit) = owner.spell_lane_permit().await else {
        return false;
    };
    let store =
        crate::durability::fresh_admission::FreshAdmissionStore::from_root(owner.root.clone());
    let Ok((current_session, _)) = store.current_session_at(session).await else {
        return false;
    };
    let Some(fence) = super::super::item_fence_of(current_session) else {
        return false;
    };
    let runtime = owner.runtime.lock().await;
    let mut states = owner.states.spell_states.lock().await;
    if !current(&runtime, actor, session, fence, true) {
        return allow_fenced_out;
    }
    let Some((_, _, state)) = states
        .actors
        .iter()
        .find(|(a, s, _)| *a == actor && *s == session)
    else {
        return true;
    };
    let conditions = if states.is_dead(actor) {
        Default::default()
    } else {
        gourmet_conditions(state, owner.owner_now().get())
    };
    match owner
        .root
        .save_gourmet_conditions(
            owner.character,
            owner.holder,
            fence,
            *owner.channel_id.as_bytes(),
            conditions,
        )
        .await
    {
        Ok(()) => {
            remember(
                &mut states,
                actor,
                session,
                conditions,
                owner.owner_now().get(),
            );
            true
        }
        Err(ItemUseError::AuthorityRejected) => allow_fenced_out,
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    #[test]
    fn fishing_food_reaches_the_actual_live_skill_and_catch_consumer() {
        let mut actor = crate::spell::cast::PlayerSpellState::new(
            CharacterCastFacts {
                vocation: crate::spell::Vocation::Knight,
                level: 8,
                magic_level: 10,
                max_health: 185,
                max_mana: 90,
                max_soul: 100,
            },
            0,
            0,
        )
        .unwrap();
        let build = crate::durability::character_build::DurableBuildState::new(
            "knight",
            (10, 0),
            [(10, 0); 7],
        )
        .unwrap();
        actor.load_owned_build(&build).unwrap();
        let next = stage_gourmet_food(&actor, 29413, 0).unwrap();
        let original = u16::try_from(actor.live_skill(6, 0).unwrap()).unwrap();
        let boosted = u16::try_from(next.live_skill(6, 0).unwrap()).unwrap();
        assert_eq!((original, boosted), (10, 40));
        assert!((1..=100_000).any(
            |roll| oteryn_gameplay_items::fishing::catches(boosted, roll)
                && !oteryn_gameplay_items::fishing::catches(original, roll)
        ));
        assert_eq!(next.live_skill(6, 3_600_000_000), Some(10));
    }

    #[test]
    fn chilli_food_changes_native_step_duration_then_expires() {
        let actor = crate::spell::cast::PlayerSpellState::new(
            CharacterCastFacts {
                vocation: crate::spell::Vocation::Knight,
                level: 8,
                magic_level: 10,
                max_health: 185,
                max_mana: 90,
                max_soul: 100,
            },
            0,
            0,
        )
        .unwrap();
        let next = stage_gourmet_food(&actor, 29412, 0).unwrap();
        let speed =
            |state, now| crate::spell::actor_conditions::movement_speed_with(state, now, 0, 0);
        let base = speed(&actor, 0);
        let boosted = speed(&next, 0);
        assert_eq!(boosted, base + 80);
        let cell = crate::content::LogicalCell {
            x: 100,
            y: 100,
            z: 7,
        };
        let duration = |speed| {
            crate::movement::speed::player_step_duration(
                &crate::movement::speed::EngineeringGroundSpeed,
                cell,
                speed,
            )
            .unwrap()
        };
        assert!(duration(boosted) < duration(base));
        assert_eq!(speed(&next, 3_600_000_000), base);
        assert_eq!(duration(speed(&next, 3_600_000_000)), duration(base));
    }
}
