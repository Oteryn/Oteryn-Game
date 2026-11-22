//! Field entry and residual ticks use the actual native creature store/HP owner.
use super::{
    ComposedFreshAdmission,
    actor_spell::{ChannelSpellStates, CreatureConditionTurn},
};
use crate::durability::DurabilityError;
use crate::foundation::{ChannelRuntimeV1, ExactActorRef};
use oteryn_gameplay_domain::domain::carrier_values::CarrierError;
use oteryn_gameplay_entities::carrier::runtime_actor_periodic::CreatureFieldEntry;
use oteryn_gameplay_foundation::foundation::condition::{
    ConditionDefinition, ConditionLineage, TickFacts,
};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone)]
pub(super) struct DeferredCreatureFieldEntry {
    actor: ExactActorRef,
    position_revision: u64,
    entry: CreatureFieldEntry,
    application: Option<u32>,
}

impl ChannelSpellStates {
    pub(super) fn forget_deferred_creature_field(&mut self, actor: ExactActorRef) {
        if !self.deferred_creature_field_entries.is_empty() {
            self.deferred_creature_field_entries
                .remove(&actor.placement_identity());
        }
    }

    pub(super) fn prune_deferred_creature_fields(&mut self, runtime: &ChannelRuntimeV1) {
        self.deferred_creature_field_entries.retain(|_, queue| {
            queue
                .front()
                .is_some_and(|pending| runtime.contains_live_creature(pending.actor))
        });
    }
}

pub(super) fn periodic_source_busy(
    runtime: &ChannelRuntimeV1,
    states: &ChannelSpellStates,
    source: ExactActorRef,
) -> bool {
    matches!(
        runtime.assert_actor_spell_unreserved(source),
        Err(CarrierError::PlanConflict)
    ) || states.actors.iter().any(|(actor, session, _)| {
        *actor == source && states.has_pending_spell_commit(source, *session)
    })
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn prune_transferred_creature_fields(&self) {
        let runtime = self.runtime.lock().await;
        self.states
            .spell_states
            .lock()
            .await
            .prune_deferred_creature_fields(&runtime);
    }

    pub(super) async fn tick_creature_fields(
        &self,
        map: &mut crate::movement::monster_map::CurrentMonsterMap<'_, '_>,
        runtime: &mut ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        native: &crate::content::native_gameplay::NativeGameplayState,
        now: u64,
        native_health_names: &super::source_names::NativeHealthNamesData,
    ) -> Result<(), DurabilityError> {
        native_health_names
            .validate_frame(runtime)
            .map_err(|_| DurabilityError::Unavailable)?;
        let mut plans = Vec::new();
        let mut source_players = Vec::new();
        let creatures = runtime.visible_entities().creatures;
        // Passive source creatures are outside the AI table, but still own field conditions.
        let positions = creatures
            .iter()
            .map(|visible| visible.position)
            .collect::<Vec<_>>();
        map.prefetch_empty_tiles(runtime, &positions)
            .await
            .map_err(|_| DurabilityError::Unavailable)?;
        for visible in creatures {
            let Ok(snapshot) = runtime.companion_snapshot(visible.actor) else {
                continue;
            };
            let read = map
                .read_items(runtime, visible.position)
                .await
                .map_err(|_| DurabilityError::Unavailable)?;
            let protection_zone = map
                .protection_zone(runtime, visible.position)
                .ok_or(DurabilityError::Unavailable)?;
            let mut entry = None;
            let mut standing = None;
            for item in &read.items {
                let policy = native
                    .item_policy(
                        &item.definition.production_key,
                        &item.definition.revision_ref,
                    )
                    .ok_or(DurabilityError::Unavailable)?;
                let Some(recipe) = &policy.record().attributes.field_condition else {
                    continue;
                };
                standing = recipe.element().map_err(|_| DurabilityError::Unavailable)?;
                if let Some(values) = recipe
                    .condition_values()
                    .map_err(|_| DurabilityError::Unavailable)?
                    && !protection_zone
                {
                    let definition = ConditionDefinition::new(
                        &format!(
                            "native.field.{}",
                            policy.record().production_binding.external_id
                        ),
                        1,
                        values,
                    )
                    .ok_or(DurabilityError::Unavailable)?;
                    let source = item
                        .field_origin
                        .as_ref()
                        .and_then(|origin| {
                            states.actors.iter().find(|(actor, session, _)| {
                                actor.placement_identity() == origin.actor_placement_digest
                                    && *session.as_bytes() == origin.game_session_id
                            })
                        })
                        .filter(|(actor, session, _)| {
                            runtime.player_control_facts(*actor, *session).is_ok()
                        })
                        .map(|(actor, _, _)| *actor);
                    let lineage =
                        source
                            .and(item.field_origin.as_ref())
                            .map(|origin| ConditionLineage {
                                source_character: origin.character_id,
                                source_session: origin.game_session_id,
                                source_lease_generation: origin.character_lease_generation,
                                source_actor_placement: origin.actor_placement_digest,
                                source_scope_generation: origin.source_scope_generation,
                                source_content_digest: origin.content_digest,
                                source_catalog_digest: origin.catalog_digest,
                                source_creation_command: origin.creation_command_id,
                            });
                    let digest = Sha256::new()
                        .chain_update(visible.actor.placement_identity())
                        .chain_update(visible.revision.to_be_bytes())
                        .chain_update(item.item_instance_id)
                        .finalize();
                    let mut occurrence = [0; 16];
                    occurrence.copy_from_slice(&digest[..16]);
                    entry = Some(CreatureFieldEntry {
                        definition,
                        source,
                        lineage,
                        occurrence: oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes(
                            occurrence,
                        ),
                    });
                }
                break;
            }
            if !snapshot.state.conditions.accepts_time(now) {
                continue;
            }
            let reserved = match runtime.assert_actor_spell_unreserved(visible.actor) {
                Ok(()) => false,
                Err(CarrierError::PlanConflict) => true,
                Err(_) => return Err(DurabilityError::Unavailable),
            };
            let key = visible.actor.placement_identity();
            let mut contacts = states
                .deferred_creature_field_entries
                .get(&key)
                .cloned()
                .unwrap_or_default();
            if let Some(entry) = entry
                && !runtime
                    .creature_field_entry_is_current(
                        visible.actor,
                        snapshot.position_revision,
                        &entry,
                    )
                    .map_err(|_| DurabilityError::Unavailable)?
                && !contacts.iter().any(|pending| {
                    pending.position_revision == snapshot.position_revision
                        && pending.entry.occurrence == entry.occurrence
                        && pending.entry.definition == entry.definition
                })
            {
                contacts.push_back(DeferredCreatureFieldEntry {
                    actor: visible.actor,
                    position_revision: snapshot.position_revision,
                    entry,
                    application: None,
                });
            }
            for pending in &mut contacts {
                // A retained recipe survives the exact caster's retirement.
                if pending
                    .entry
                    .source
                    .is_some_and(|source| !runtime.borrow_exact_actor_lookup().contains(source))
                {
                    pending.entry.source = None;
                    pending.entry.lineage = None;
                }
            }
            let facts = TickFacts {
                in_protection_zone: protection_zone,
                standing_on_field: standing,
            };
            let needs_source_facts = native
                .policies()
                .soul_war_health_policy(
                    &snapshot.state.policy.definition_key,
                    &snapshot.state.policy.definition_revision,
                )
                .is_some_and(|policy| {
                    matches!(
                        policy.kind(),
                        crate::content::native_soul_war_callbacks::SoulWarHealthCallbackKind::MirrorImage
                            | crate::content::native_soul_war_callbacks::SoulWarHealthCallbackKind::FourthTaint
                    )
                });
            if !reserved {
                let due = runtime
                    .preview_creature_field_due(
                        snapshot,
                        now,
                        contacts.iter().map(|pending| {
                            (
                                pending.position_revision,
                                &pending.entry,
                                pending.application,
                            )
                        }),
                        facts,
                    )
                    .map_err(|_| DurabilityError::Unavailable)?;
                if needs_source_facts
                    && !runtime
                        .encounter_hooks()
                        .is_some_and(|hooks| hooks.contains(visible.actor))
                {
                    source_players.extend(
                        due.into_iter()
                            .filter_map(|tick| tick.provenance.source)
                            .filter(|&source| !periodic_source_busy(runtime, states, source)),
                    );
                }
            }
            plans.push((visible.actor, key, contacts, facts, reserved));
        }
        if let Some(active) = self.active_generation {
            let room = map.room();
            let (tx, authority) = map.scope_transaction();
            super::soul_war_callbacks::player_source::publish_condition_sources(
                tx,
                self.root,
                authority,
                active,
                room,
                runtime,
                states,
                &source_players,
                now,
            )
            .await?;
        }
        if !map.validate_transaction(runtime).await {
            return Err(DurabilityError::Unavailable);
        }
        states.prune_deferred_creature_fields(runtime);
        for (_, key, contacts, _, _) in &mut plans {
            if !contacts.is_empty() {
                states
                    .deferred_creature_field_entries
                    .insert(*key, std::mem::take(contacts));
            }
        }
        for (actor, key, _, facts, reserved) in plans {
            if reserved {
                continue;
            }
            let hooked = runtime
                .encounter_hooks()
                .is_some_and(|hooks| hooks.contains(actor));
            let can_tick = apply_field_contacts(
                runtime,
                states,
                actor,
                key,
                facts,
                now,
                native_health_names,
                hooked,
            )?;
            if can_tick {
                tick_field_conditions(
                    runtime,
                    states,
                    actor,
                    facts,
                    now,
                    native_health_names,
                    None,
                )?;
            }
        }
        Ok(())
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "the contact and HP turns share the qualified field owner inputs"
)]
fn apply_field_contacts(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    actor: ExactActorRef,
    key: [u8; 16],
    facts: TickFacts,
    now: u64,
    names: &super::source_names::NativeHealthNamesData,
    hooked: bool,
) -> Result<bool, DurabilityError> {
    while let Some(pending) = states
        .deferred_creature_field_entries
        .get_mut(&key)
        .and_then(|queue| queue.front_mut())
    {
        let application = match pending.application {
            Some(sequence) => Some(sequence),
            None => runtime
                .commit_creature_field_entry_at(
                    actor,
                    now,
                    pending.position_revision,
                    &pending.entry,
                )
                .map_err(|_| DurabilityError::Unavailable)?,
        };
        if let Some(sequence) = application {
            pending.application = Some(sequence);
            if hooked {
                return Ok(false);
            }
            if !tick_field_conditions(runtime, states, actor, facts, now, names, Some(sequence))? {
                // A busy application must wait, but an older lethal tick can still
                // end the creature before the native owner reaches that source.
                tick_field_conditions(runtime, states, actor, facts, now, names, None)?;
                if !runtime.contains_live_creature(actor) {
                    states.forget_deferred_creature_field(actor);
                }
                return Ok(false);
            }
            if !runtime.contains_live_creature(actor) {
                states.forget_deferred_creature_field(actor);
                return Ok(false);
            }
            let snapshot = runtime
                .companion_snapshot(actor)
                .map_err(|_| DurabilityError::Unavailable)?;
            if snapshot.state.conditions.has_pending_application(sequence) {
                return Ok(false);
            }
        }
        if let Some(queue) = states.deferred_creature_field_entries.get_mut(&key) {
            queue.pop_front();
        }
    }
    states.forget_deferred_creature_field(actor);
    Ok(!hooked)
}

fn tick_field_conditions(
    runtime: &mut ChannelRuntimeV1,
    states: &ChannelSpellStates,
    actor: ExactActorRef,
    facts: TickFacts,
    now: u64,
    names: &super::source_names::NativeHealthNamesData,
    application: Option<u32>,
) -> Result<bool, DurabilityError> {
    let Ok(observation) = names.observe(runtime, actor) else {
        return Ok(false);
    };
    let observations: Vec<_> = observation.into_iter().collect();
    states
        .tick_source_creature_conditions_with_health_observations(
            runtime,
            actor,
            facts,
            crate::foundation::owner_timer::SemanticTimeMicros::from_micros(now),
            &observations,
            CreatureConditionTurn::Field(application),
        )
        .map(|receipt| receipt.is_some())
        .map_err(|_| DurabilityError::Unavailable)
}
