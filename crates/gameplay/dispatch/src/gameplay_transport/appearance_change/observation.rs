//! Current owner observations. Visibility supplies the exact player identities to read.
use super::*;
use oteryn_gameplay_foundation::foundation::condition::TemporaryDisplayedAppearance;

#[derive(Debug, Clone)]
pub struct Observation {
    pub catalogue: wire::Catalogue,
    pub bindings: Vec<wire::Binding>,
}

pub(in crate::gameplay_transport) async fn observe(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    observer: ExactActorRef,
    session: GameSessionId,
    mut entities: Vec<wire::EntityRef>,
) -> Option<Observation> {
    if entities.len() > 256 {
        return None;
    }
    entities.sort_unstable();
    if entities.windows(2).any(|p| p[0] == p[1]) {
        return None;
    }
    let (context, content) = admission.appearance_context()?;
    let own_fence =
        super::super::character_gameplay_fence::current(admission, session, false).await?;
    let own = admission
        .root
        .read_current_character_appearance_state(
            admission.character,
            admission.holder,
            own_fence,
            content.earned_grant_keys(),
        )
        .await
        .ok()?;
    let catalogue = super::catalogue::catalogue(
        &content,
        &own,
        own_fence.expected_character_revision.get(),
        context,
    )?;
    let runtime = admission.runtime.lock().await;
    if !current_runtime(&runtime, observer, session, &own_fence, &content) {
        return None;
    }
    let states = admission.spell_states.lock().await;
    let census = runtime.positioned_melee_census().ok()?;
    let mut bindings = Vec::new();
    let now = admission.owner_now().get();
    for entity in entities {
        let (actor, _, target_session) = census.iter().find(|(actor, _, _)| {
            actor.placement_identity() == entity.identity
                && actor.actor_local_generation() == entity.generation
        })?;
        let target_session = (*target_session)?;
        let fence =
            super::super::character_gameplay_fence::current(admission, target_session, true)
                .await?;
        if target_session == session && fence != own_fence {
            return None;
        }
        if fence.runtime_scope != own_fence.runtime_scope
            || fence.scope_ownership_generation != own_fence.scope_ownership_generation
        {
            return None;
        }
        let current = admission
            .root
            .read_current_character_appearance_state(
                admission.character,
                admission.holder,
                fence,
                content.earned_grant_keys(),
            )
            .await
            .ok()?;
        let player = states.get(&runtime, *actor, target_session)?;
        let (base, selection_revision) = super::catalogue::base(&content, &current);
        let mut look = wire::Look::Outfit {
            look_type: content.look_type(&base.outfit_key, current.sex)?,
            colours: base.colours,
            addons: base.addons,
        };
        let owned = runtime
            .actor_conditions_at(*actor, Some(target_session), now)
            .ok()?;
        let spell = player.owned_conditions();
        if !spell.accepts_time(now) || !owned.accepts_time(now) {
            return None;
        }
        let spell_value = spell.displayed_temporary_appearance_at(now);
        let owned_value = owned.displayed_temporary_appearance_at(now);
        if (spell
            .active_at(crate::domain::condition::ConflictKey::Outfit, now)
            .is_some()
            && spell_value.is_none())
            || (owned
                .active_at(crate::domain::condition::ConflictKey::Outfit, now)
                .is_some()
                && owned_value.is_none())
        {
            return None;
        }
        let spell_look = temporary(
            spell_value,
            spell.outfit_at(now),
            &content,
            admission.active_generation?,
        )?;
        let owned_look = temporary(
            owned_value,
            owned.outfit_at(now),
            &content,
            admission.active_generation?,
        )?;
        if let (Some(a), Some(b)) = (&spell_look, &owned_look)
            && a != b
        {
            return None;
        }
        if let Some(temporary) = spell_look.or(owned_look) {
            look = temporary;
        }
        // The selection and root are re-read after observer preparation. Runtime pin,
        // incarnation and condition state stay locked through this bounded DB read.
        if admission
            .root
            .read_current_character_appearance_state(
                admission.character,
                admission.holder,
                fence,
                content.earned_grant_keys(),
            )
            .await
            .ok()?
            != current
        {
            return None;
        }
        bindings.push(wire::Binding {
            context,
            appearance_ref: 1,
            entity,
            root_revision: fence.expected_character_revision.get(),
            condition_revision: player.revision(),
            owner_time_us: now,
            look,
            selection_revision,
        });
    }
    Some(Observation {
        catalogue,
        bindings,
    })
}
fn temporary(
    value: Option<TemporaryDisplayedAppearance<'_>>,
    look_type: Option<u32>,
    content: &NativeCosmeticsView,
    generation: &crate::content::ActiveGeneration,
) -> Option<Option<wire::Look>> {
    Some(match value {
        None => None,
        Some(TemporaryDisplayedAppearance::Outfit(selection)) => {
            let registry = generation.native_gameplay()?.spell_appearances()?;
            let member = registry.for_selection(look_type?, selection)?;
            if member.source_digest() != content.source_digest()
                || selection
                    .colours
                    .iter()
                    .any(|c| *c >= content.palette_size())
            {
                return None;
            }
            Some(wire::Look::Outfit {
                look_type: member.look_type(),
                colours: selection.colours,
                addons: selection.addons,
            })
        }
        Some(TemporaryDisplayedAppearance::Item(item)) => {
            if item.artifact_digest() != content.source_digest() {
                return None;
            }
            generation
                .native_gameplay()?
                .spell_appearances()?
                .for_item(item.definition_key(), item.revision_ref())?;
            Some(wire::Look::Item {
                definition_ref: generation
                    .native_gameplay()?
                    .item_index()?
                    .definition_ref("Item", item.definition_key(), item.revision_ref())?
                    .get(),
            })
        }
    })
}
