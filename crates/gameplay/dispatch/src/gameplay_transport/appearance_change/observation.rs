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

/// Only a proven exhausted holder retry may defer an established observer's refresh.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObservationError {
    HolderBusy,
    /// A Character between a durable death and its respawn placement: its appearance state
    /// is fenced until the placement consumes the pending respawn. Not a lost observer.
    RespawnPending,
    Unavailable,
}

/// Why one attempt's durable read produced no value.
pub(super) enum ReadFailure {
    Durable(crate::durability::DurabilityError),
    RespawnPending,
}

pub(in crate::gameplay_transport) async fn observe(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    observer: ExactActorRef,
    session: GameSessionId,
    empty_outfit: bool,
    entities: Vec<wire::EntityRef>,
) -> Option<Observation> {
    observe_result(admission, observer, session, empty_outfit, entities)
        .await
        .ok()
}

pub(in crate::gameplay_transport) async fn observe_result(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    observer: ExactActorRef,
    session: GameSessionId,
    empty_outfit: bool,
    mut entities: Vec<wire::EntityRef>,
) -> Result<Observation, ObservationError> {
    if entities.len() > 256 {
        operator_event("event=appearance_observation_failed class=visibility");
        return Err(ObservationError::Unavailable);
    }
    entities.sort_unstable();
    if entities.windows(2).any(|p| p[0] == p[1]) {
        operator_event("event=appearance_observation_failed class=visibility");
        return Err(ObservationError::Unavailable);
    }
    let _foreground = admission.root.foreground_demand().map_err(|_| {
        operator_event("event=appearance_observation_failed class=fence");
        ObservationError::Unavailable
    })?;
    {
        // Monster profiles hold both guards while using the durable holder. Wait
        // for that existing owner before starting the command's one-second retry
        // window; the foreground demand prevents a new profile from overtaking us.
        let _runtime = admission.runtime.lock().await;
        let _states = admission.states.spell_states.lock().await;
    }
    let retry = AdmissionRetryBudget::for_command(admission.root);
    retry
        .retry_durability("appearance_observation", async || {
            let mut failure = None;
            let observation = observe_once(
                admission,
                observer,
                session,
                empty_outfit,
                &entities,
                &mut failure,
            )
            .await;
            // All runtime/spell guards belong to observe_once and have dropped
            // before retry_durability can wait for the holder. Its durable reads
            // acquire with try_issue_root/try_issue_semantic_pass, never a wait.
            match failure {
                Some(ReadFailure::Durable(error)) => Err(error),
                Some(ReadFailure::RespawnPending) => Ok(Err(ObservationError::RespawnPending)),
                None => Ok(observation.ok_or(ObservationError::Unavailable)),
            }
        })
        .await
        .map_err(|error| match error {
            crate::durability::DurabilityError::RootUnavailable => ObservationError::HolderBusy,
            _ => ObservationError::Unavailable,
        })?
}

fn read_result<T>(
    result: Result<Option<T>, crate::durability::DurabilityError>,
    failure: &mut Option<ReadFailure>,
    class: &'static str,
) -> Option<T> {
    match result {
        Ok(value) => note_none(value, class),
        Err(error) => {
            // A durable error belongs to the existing retry owner, not a logical
            // refusal. A busy attempt must not emit diagnostics on every retry.
            *failure = Some(ReadFailure::Durable(error));
            None
        }
    }
}

fn read_state_result<T>(
    result: Result<Option<T>, ReadFailure>,
    failure: &mut Option<ReadFailure>,
) -> Option<T> {
    match result {
        Ok(value) => note_none(value, "state"),
        Err(error) => {
            *failure = Some(error);
            None
        }
    }
}

fn note_none<T>(value: Option<T>, class: &'static str) -> Option<T> {
    if value.is_none() {
        operator_event(&format!(
            "event=appearance_observation_failed class={class}"
        ));
    }
    value
}

async fn observe_once(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    observer: ExactActorRef,
    session: GameSessionId,
    empty_outfit: bool,
    entities: &[wire::EntityRef],
    failure: &mut Option<ReadFailure>,
) -> Option<Observation> {
    let (context, content) = note_none(admission.appearance_context(), "content")?;
    let own_fence = read_result(
        super::super::character_gameplay_fence::current_once(admission, session, false).await,
        failure,
        "fence",
    )?;
    let own = read_state_result(
        house::read_state(admission, own_fence, content.earned_grant_keys()).await,
        failure,
    )?;
    let catalogue = note_none(
        super::catalogue::catalogue(
            &content,
            &own,
            own_fence.expected_character_revision.get(),
            context,
        ),
        "content",
    )?;
    let runtime = admission.runtime.lock().await;
    if !house::current_runtime_for_read(&runtime, observer, session, &own_fence, &content) {
        return note_none(None, "fence");
    }
    let states = admission.states.spell_states.lock().await;
    let (census, hirelings) = note_none(house::census(&runtime, observer, session), "visibility")?;
    let mut bindings = Vec::new();
    let now = admission.owner_now().get();
    for &entity in entities {
        if let Some(entry) = hirelings.iter().find(|entry| {
            entry.actor.placement_identity() == entity.identity
                && entry.actor.actor_local_generation() == entity.generation
        }) {
            bindings.push(wire::Binding {
                context,
                appearance_ref: 1,
                entity,
                root_revision: 1,
                condition_revision: entry.revision,
                owner_time_us: now,
                look: wire::Look::Outfit {
                    look_type: u32::from(entry.look_type),
                    colours: entry.colors.map(u16::from),
                    addons: 0,
                },
                selection_revision: None,
                forge_stacks: None,
            });
            continue;
        }
        let (actor, target_session) = note_none(
            census.iter().find(|(actor, _)| {
                actor.placement_identity() == entity.identity
                    && actor.actor_local_generation() == entity.generation
            }),
            "visibility",
        )?;
        if states.player_hidden_from(
            &runtime,
            observer,
            *actor,
            oteryn_simulation_determinism::SemanticTimeMicros::from_micros(now),
        ) {
            continue;
        }
        let Some(target_session) = *target_session else {
            let creature = note_none(runtime.companion_snapshot(*actor).ok(), "visibility")?;
            let look = if let Some(look_type) = creature.state.policy.outfit_appearance() {
                wire::Look::Outfit {
                    look_type,
                    colours: [0; 4],
                    addons: 0,
                }
            } else {
                let native = note_none(
                    note_none(admission.active_generation, "content")?.native_gameplay(),
                    "content",
                )?;
                let item = &note_none(
                    native.item_policy_for_source_id(note_none(
                        creature.state.object_look_type,
                        "content",
                    )?),
                    "content",
                )?
                .record()
                .production_definition;
                wire::Look::Item {
                    definition_ref: note_none(
                        note_none(native.item_index(), "content")?.definition_ref(
                            "Item",
                            &item.production_key,
                            &item.revision_ref,
                        ),
                        "content",
                    )?
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
        let fence = read_result(
            super::super::character_gameplay_fence::current_once(admission, target_session, true)
                .await,
            failure,
            "fence",
        )?;
        if target_session == session && fence != own_fence {
            return note_none(None, "revision");
        }
        if fence.runtime_scope != own_fence.runtime_scope
            || fence.scope_ownership_generation != own_fence.scope_ownership_generation
        {
            return note_none(None, "fence");
        }
        let current = match house::read_state(admission, fence, content.earned_grant_keys()).await {
            // Another player between its durable death and the respawn placement has no
            // observable appearance yet: it gets no binding, as a hidden player does, and
            // the observer's projection is not lost to it.
            Err(ReadFailure::RespawnPending) if target_session != session => continue,
            result => read_state_result(result, failure)?,
        };
        let in_house = runtime.house_actor_state(*actor, target_session).is_ok();
        let player = if in_house {
            note_none(
                states
                    .actors
                    .iter()
                    .find(|(held, held_session, _)| {
                        *held == *actor && *held_session == target_session
                    })
                    .map(|(_, _, state)| state),
                "visibility",
            )?
        } else {
            note_none(states.get(&runtime, *actor, target_session), "visibility")?
        };
        let (base, selection_revision) = super::catalogue::base(&content, &current);
        let mut look = wire::Look::Outfit {
            look_type: note_none(content.look_type(&base.outfit_key, current.sex), "content")?,
            colours: base.colours,
            addons: base.addons,
        };
        let owned = if in_house {
            note_none(
                runtime
                    .house_actor_conditions_at(*actor, target_session, now)
                    .ok(),
                "conditions",
            )?
        } else {
            note_none(
                runtime
                    .actor_conditions_at(*actor, Some(target_session), now)
                    .ok(),
                "conditions",
            )?
        };
        let spell = player.owned_conditions();
        if !spell.accepts_time(now) || !owned.accepts_time(now) {
            return note_none(None, "conditions");
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
            return note_none(None, "conditions");
        }
        let spell_look = note_none(
            temporary(
                spell_value,
                spell.outfit_at(now),
                &content,
                note_none(admission.active_generation, "content")?,
            ),
            "conditions",
        )?;
        let owned_look = note_none(
            temporary(
                owned_value,
                owned.outfit_at(now),
                &content,
                note_none(admission.active_generation, "content")?,
            ),
            "conditions",
        )?;
        if let (Some(a), Some(b)) = (&spell_look, &owned_look)
            && a != b
        {
            return note_none(None, "conditions");
        }
        // Crystal 201189f8 protocolgame.cpp:9233 and player.cpp:1383-1396: an
        // invisible player remains observable, but other players receive no outfit.
        if empty_outfit
            && target_session != session
            && (admission.states.staff_world.invisible(*actor)
                && admission.states.staff_world.group(observer) < 4
                || player.equipped_invisible()
                || spell.invisible_at(now)
                || owned.invisible_at(now))
        {
            look = wire::Look::EmptyOutfit;
        } else if let Some(temporary) = spell_look.or(owned_look) {
            look = temporary;
        }
        // The selection and root are re-read after observer preparation. Runtime pin,
        // incarnation and condition state stay locked through this bounded DB read.
        let reread = match house::read_state(admission, fence, content.earned_grant_keys()).await {
            Err(ReadFailure::RespawnPending) if target_session != session => continue,
            result => read_state_result(result, failure)?,
        };
        if reread != current {
            return note_none(None, "revision");
        }
        bindings.push(wire::Binding {
            context,
            appearance_ref: 1,
            entity,
            root_revision: fence.expected_character_revision.get(),
            condition_revision: note_none(
                player.revision().checked_add(
                    note_none(runtime.actor_staff_visibility(*actor).ok(), "visibility")?.2,
                ),
                "revision",
            )?,
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
