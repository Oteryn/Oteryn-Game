//! Field entry and residual ticks use the actual native creature store/HP owner.
use super::{ComposedFreshAdmission, actor_spell::ChannelSpellStates};
use crate::durability::DurabilityError;
use crate::foundation::ChannelRuntimeV1;
use oteryn_gameplay_entities::carrier::runtime_actor_periodic::CreatureFieldEntry;
use oteryn_gameplay_foundation::foundation::condition::{
    ConditionDefinition, ConditionLineage, TickFacts,
};
use sha2::{Digest, Sha256};

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn tick_creature_fields(
        &self,
        map: &mut crate::movement::monster_map::CurrentMonsterMap<'_, '_>,
        runtime: &mut ChannelRuntimeV1,
        states: &ChannelSpellStates,
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
            let facts = TickFacts {
                in_protection_zone: protection_zone,
                standing_on_field: standing,
            };
            // Preview only a cloned condition store. Actual due work is consumed
            // once by the existing canonical owner after all source reads finish.
            if native
                .policies()
                .soul_war_health_policy(
                    &snapshot.state.policy.definition_key,
                    &snapshot.state.policy.definition_revision,
                )
                .is_some_and(|policy| {
                    matches!(policy.kind(),
                crate::content::native_soul_war_callbacks::SoulWarHealthCallbackKind::MirrorImage
                | crate::content::native_soul_war_callbacks::SoulWarHealthCallbackKind::FourthTaint)
                })
            {
                let mut preview = snapshot.state.conditions.clone();
                for tick in preview.take_due(now, facts) {
                    if let Some(source) = tick.provenance.source {
                        source_players.push(source);
                    }
                }
                if let Some(source) = entry.as_ref().and_then(|field| field.source) {
                    source_players.push(source);
                }
            }
            plans.push((visible.actor, entry, facts));
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
        for (actor, entry, facts) in plans {
            runtime
                .commit_creature_field_entry(actor, now, entry.as_ref())
                .map_err(|_| DurabilityError::Unavailable)?;
            let observations = match native_health_names.observe(runtime, actor) {
                Ok(observation) => observation.into_iter().collect::<Vec<_>>(),
                // Only this required health callback lacks facts. The original
                // condition/entry clock remains installed and its HP ticks
                // remain unconsumed; unrelated actors continue normally.
                Err(()) => continue,
            };
            states
                .tick_source_creature_conditions_with_health_observations(
                    runtime,
                    actor,
                    facts,
                    crate::foundation::owner_timer::SemanticTimeMicros::from_micros(now),
                    &observations,
                )
                .map_err(|_| DurabilityError::Unavailable)?;
        }
        Ok(())
    }
}
