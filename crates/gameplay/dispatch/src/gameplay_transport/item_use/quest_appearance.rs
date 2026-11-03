//! Crystal's reusable mummy disguise enters the existing item-use and condition owners.
//! No durable item or appearance entitlement changes: the source item is reusable.
use super::*;
use crate::content::native_spell_appearances::CompiledSpellAppearances;
use crate::durability::spell_owner_commit::SpellLanePermit;
use crate::spell::cast::PlayerSpellState;

const MUMMY_CREATURE: &str = "oteryn:creature.mummy";
const CREATURE_REVISION: &str = "definition-r1";
// zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:
// data-global/scripts/quests/what_a_foolish_quest/action_disguise.lua:1-14.
const MUMMY_LOOK: u32 = 65;
const MUMMY_DURATION_MS: u32 = 10_000;
const MESSAGE: &str =
    "You are now disguised as a mummy for 10 seconds. Hurry up and scare the caliph!";

fn stage_mummy(
    before: &PlayerSpellState,
    appearances: &CompiledSpellAppearances,
    now_ms: u64,
) -> Result<PlayerSpellState, UseDisposition> {
    let member = appearances
        .for_creature(MUMMY_CREATURE, CREATURE_REVISION)
        .ok_or(UseDisposition::Rejected)?;
    let selection = member.selection();
    if member.look_type() != MUMMY_LOOK
        || selection.colours != [0; 4]
        || selection.addons != 0
        || selection.mount_key.is_some()
    {
        return Err(UseDisposition::Rejected);
    }
    let mut next = before.clone();
    crate::spell::actor_conditions::apply_appearance(
        &mut next,
        selection,
        member.look_type(),
        MUMMY_DURATION_MS,
        now_ms,
    )
    .map_err(|_| UseDisposition::Rejected)?;
    next.advance_batch_revision()
        .map_err(|_| UseDisposition::Rejected)?;
    Ok(next)
}

/// The caller already admitted this CommandRef, resolved the owned source instance
/// under the current item fence, and holds the spell/item lane through publication.
/// The same gate retains the terminal receipt; replay cannot restart the condition.
pub(super) async fn use_foolish_disguise(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    _permit: &SpellLanePermit,
    native: &NativeGameplayState,
    entry: &BackpackEntry,
    record: Option<&ItemProfileRecord>,
) -> Option<Result<UseDisposition, UseDisposition>> {
    if record?.production_binding.external_id != "144" {
        return None;
    }
    Some(apply(admission, actor, command, native, entry).await)
}

async fn apply(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    native: &NativeGameplayState,
    entry: &BackpackEntry,
) -> Result<UseDisposition, UseDisposition> {
    let fence = command.item_fence.ok_or(UseDisposition::Rejected)?;
    let session = command.game_session_id;
    let runtime = admission.runtime.lock().await;
    let mut states = admission.states.spell_states.lock().await;
    let appearances = native.spell_appearances().ok_or(UseDisposition::Rejected)?;
    if runtime.owner_fence().is_err()
        || native.source_digest() != runtime.content_pin().server_artifact_digest()
        || appearances.source_digest() != runtime.content_pin().server_artifact_digest()
        || super::super::item_ref_admission::pinned_index(&runtime, admission.active_generation)
            .is_none()
        || fence.game_session_id != session
        || states.is_dead(actor)
        || !states.spell_idle(&runtime, actor, session)
        || !runtime
            .player_control_facts(actor, session)
            .is_ok_and(|facts| facts.control_loss.is_none())
        || !runtime
            .bound_attacker_lease(actor, session)
            .is_ok_and(|lease| {
                lease.character_id().as_bytes() == fence.character_id.as_bytes()
                    && lease.generation() == fence.character_lease_generation
            })
    {
        return Err(UseDisposition::Rejected);
    }
    // Resolve every fallible check before publishing either the condition or its memo.
    let gate_index = states
        .item_uses
        .0
        .iter()
        .position(|(held_actor, held_session, _)| *held_actor == actor && *held_session == session)
        .ok_or(UseDisposition::Rejected)?;
    let gate = &states.item_uses.0[gate_index].2;
    if gate.unspendable.contains(&entry.item.item_instance_id) || gate.in_flight_since_ms.is_some()
    {
        return Err(UseDisposition::StaleState);
    }
    let before = states
        .get(&runtime, actor, session)
        .ok_or(UseDisposition::Rejected)?;
    let next = stage_mummy(before, appearances, admission.owner_ms())?;
    if !states.commit(&runtime, actor, session, next) {
        return Err(UseDisposition::Rejected);
    }
    states.item_uses.0[gate_index]
        .2
        .settle(command.command_id, UseDisposition::Committed, &[]);
    admission.states.chat.push_quest_feedback(session, MESSAGE);
    Ok(UseDisposition::Committed)
}

#[cfg(test)]
mod tests;
