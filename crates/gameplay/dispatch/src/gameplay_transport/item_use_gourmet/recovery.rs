//! The authenticated House recovery path restores the existing native state owner.
use super::*;
use crate::durability::character_last_position::LastPositionRecord;
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::gameplay_transport::connection::AdmittedSession;
use crate::gameplay_transport::spell_character_facts::{self, CastFactsLoad};

/// The House handoff recovery caller retains its existing spell-lane permit.
/// Acquiring another permit here would deadlock that same owner lane.
pub(in crate::gameplay_transport) async fn restore_house(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    saved: &LastPositionRecord,
    fresh: bool,
) -> Option<()> {
    let actor = admitted.runtime_actor?;
    let fence = admitted.item_fence?;
    let session = admitted.game_session_id;
    if !matches!(fence.runtime_scope, RuntimeScopeRefV1::Instance { .. })
        || admitted.world_id != owner.world_id
        || admitted.channel_id != owner.channel_id
        || fence.game_session_id != session
    {
        return None;
    }
    {
        let runtime = owner.runtime.lock().await;
        if !current(&runtime, actor, session, fence, false) {
            return None;
        }
    }
    let character = fence.character_id;
    let (facts, revision) = match spell_character_facts::load_character_cast_facts(
        owner.root,
        owner.character,
        character,
    )
    .await
    {
        CastFactsLoad::Ready {
            facts,
            character_revision,
        }
        | CastFactsLoad::NoVocation {
            facts,
            character_revision,
        } => (facts, character_revision),
        CastFactsLoad::Unavailable => return None,
    };
    let monk = owner.load_monk_state(character).await?;
    let build = owner
        .root
        .read_character_build_state(owner.character, character)
        .await
        .ok()?;
    let learned = owner
        .root
        .read_character_known_spells(owner.character, character)
        .await
        .ok()?;
    let stance = owner
        .root
        .read_character_stance(owner.character, character)
        .await
        .ok()?;
    let stamina = owner
        .root
        .read_hunting_stamina(owner.character, character)
        .await
        .ok()?;
    let saved_conditions = if fresh {
        owner
            .root
            .read_character_conditions(owner.character, character)
            .await
            .ok()?
    } else {
        None
    };
    let gourmet = owner
        .root
        .read_gourmet_conditions(owner.character, character)
        .await
        .ok()?;
    let effects = owner
        .read_house_admission_effects(actor, session, fence)
        .await
        .ok()?;
    let latest = owner
        .root
        .read_current_character(owner.character, character)
        .await
        .ok()?;
    let (held_session, _) = FreshAdmissionStore::from_root(owner.root.clone())
        .current_session_at(session)
        .await
        .ok()?;
    if latest.revision != revision
        || held_session.current_transport() != Some(admitted.controller?.transport)
        || crate::gameplay_transport::item_fence_of(held_session) != Some(fence)
    {
        return None;
    }
    owner.reserve_precondition().await.ok()?;
    let runtime = owner.runtime.lock().await;
    if !current(&runtime, actor, session, fence, false) {
        return None;
    }
    let mut states = owner.states.spell_states.lock().await;
    let now = owner.owner_now();
    let restored_conditions = crate::gameplay_transport::condition_persistence::prepare(
        saved_conditions.as_ref(),
        fence.character_lease_generation,
        runtime.content_pin().server_artifact_digest(),
        now.get(),
    )?;
    let mut prepared = crate::spell::cast::PlayerSpellState::new(facts, monk.0, monk.1)?;
    prepared.install_learned_spells(learned);
    prepared.make_playable(now).ok()?;
    if let Some(vitals) = saved.vitals {
        prepared.restore_saved_vitals(vitals.health(), vitals.mana());
    }
    if let Some(rest) = saved.rest {
        prepared.restore_bed_rest(rest, now.get())?;
    }
    prepared.load_owned_build(&build).ok()?;
    prepared.load_owned_stance(&stance, owner.spells).ok()?;
    if let Some(formula) = owner
        .active_generation
        .and_then(|a| a.native_gameplay())
        .and_then(|native| native.training_formula())
    {
        prepared.enable_owned_training(&build, formula).ok()?;
    }
    crate::gameplay_transport::condition_persistence::restore(
        &mut prepared,
        restored_conditions.as_ref(),
        gourmet,
        now.get(),
        saved.rest,
    )?;
    if states.install_house_admitted(&runtime, actor, session, fence, prepared)? {
        remember(&mut states, actor, session, gourmet, now.get());
        states.load_house_stamina(&runtime, actor, session, fence, stamina, now.get())?;
    }
    drop(states);
    owner
        .install_house_admission_suppressions(&runtime, actor, session, fence, effects)
        .await?;
    Some(())
}
