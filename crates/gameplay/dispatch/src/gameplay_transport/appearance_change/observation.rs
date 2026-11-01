//! Current owner observations. Visibility supplies the exact player identities to read.
use super::*;
use oteryn_gameplay_foundation::foundation::condition::TemporaryDisplayedAppearance;
#[path = "observation/house.rs"]
mod house;

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
    let own = house::read_state(admission, own_fence, content.earned_grant_keys()).await?;
    let catalogue = super::catalogue::catalogue(
        &content,
        &own,
        own_fence.expected_character_revision.get(),
        context,
    )?;
    let runtime = admission.runtime.lock().await;
    if !house::current_runtime_for_read(&runtime, observer, session, &own_fence, &content) {
        return None;
    }
    let states = admission.states.spell_states.lock().await;
    let (census, hirelings) = house::census(&runtime, observer, session)?;
    let mut bindings = Vec::new();
    let now = admission.owner_now().get();
    for entity in entities {
        if let Some(entry) = hirelings.iter().find(|entry| entry.actor.placement_identity() == entity.identity
            && entry.actor.actor_local_generation() == entity.generation) {
            bindings.push(wire::Binding {
                context,
                appearance_ref: 1,
                entity,
                root_revision: 1,
                condition_revision: entry.revision,
                owner_time_us: now,
                look: wire::Look::Outfit { look_type: u32::from(entry.look_type), colours: entry.colors.map(u16::from), addons: 0 },
                selection_revision: None,
                forge_stacks: None,
            });
            continue;
        }
        let (actor, target_session) = census.iter().find(|(actor, _)| {
            actor.placement_identity() == entity.identity
                && actor.actor_local_generation() == entity.generation
        })?;
        let Some(target_session) = *target_session else {
            let creature = runtime.companion_snapshot(*actor).ok()?;
            let look = if let Some(look_type) = creature.state.policy.outfit_appearance() {
                wire::Look::Outfit {
                    look_type,
                    colours: [0; 4],
                    addons: 0,
                }
            } else {
                let native = admission.active_generation?.native_gameplay()?;
                let item = &native
                    .item_policy_for_source_id(creature.state.object_look_type?)?
                    .record()
                    .production_definition;
                wire::Look::Item {
                    definition_ref: native
                        .item_index()?
                        .definition_ref("Item", &item.production_key, &item.revision_ref)?
                        .get(),
                }
            };
            bindings.push(wire::Binding {
                context,
                appearance_ref: 1,
                entity,
                root_revision: 1,
                condition_revision: creature.state.lifecycle_epoch,
                owner_time_us: now,
                look,
                selection_revision: None,
                forge_stacks: creature.forge.map(|f| f.reward_stacks()),
            });
            continue;
        };
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
        let current = house::read_state(admission, fence, content.earned_grant_keys()).await?;
        let in_house = runtime.house_actor_state(*actor, target_session).is_ok();
        let player = if in_house {
            states.actors.iter().find(|(held, held_session, _)| *held == *actor && *held_session == target_session).map(|(_, _, state)| state)?
        } else {
            states.get(&runtime, *actor, target_session)?
        };
        let (base, selection_revision) = super::catalogue::base(&content, &current);
        let mut look = wire::Look::Outfit {
            look_type: content.look_type(&base.outfit_key, current.sex)?,
            colours: base.colours,
            addons: base.addons,
        };
        let owned = if in_house {
            runtime.house_actor_conditions_at(*actor, target_session, now).ok()?
        } else {
            runtime.actor_conditions_at(*actor, Some(target_session), now).ok()?
        };
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
        if house::read_state(admission, fence, content.earned_grant_keys()).await? != current
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
            forge_stacks: None,
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
