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
    #[allow(clippy::too_many_arguments)]
    pub(super) async fn tick_creature_fields(
        &self,
        map: &mut crate::movement::monster_map::CurrentMonsterMap<'_, '_>,
        runtime: &mut ChannelRuntimeV1,
        states: &ChannelSpellStates,
        room: &crate::content::QualifiedNativeEntryRoom,
        native: &crate::content::native_gameplay::NativeGameplayState,
        now: u64,
    ) -> Result<(), DurabilityError> {
        for visible in runtime.visible_entities().creatures {
            let Ok(snapshot) = runtime.companion_snapshot(visible.actor) else {
                continue;
            };
            let read = map
                .read_items(runtime, visible.position)
                .await
                .map_err(|_| DurabilityError::Unavailable)?;
            let cell = crate::content::LogicalCell {
                x: visible.position.x,
                y: visible.position.y,
                z: i32::from(visible.position.floor),
            };
            let in_protection_zone = if matches!(
                room.movement_cells().index(),
                crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(_)
            ) {
                // A pickupable stack makes spell collision metadata unknown, but
                // the pinned bundle still owns this tile's immutable zone flags.
                // Reuse logout's current-map proof; never infer that the tile is safe.
                self.logout_zones_for_cell(runtime, cell)
                    .and_then(|zones| zones.protection_zone)
                    .ok_or(DurabilityError::Unavailable)?
            } else {
                room.movement_cells()
                    .spell_tiles()
                    .lookup(room.movement_cells().scope(), cell)
                    .map_err(|_| DurabilityError::Unavailable)?
                    .flags()
                    .protection_zone
            };
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
                    && !in_protection_zone
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
                            runtime
                                .player_control_facts(*actor, *session)
                                .is_ok_and(|facts| facts.control_loss.is_none())
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
            runtime
                .commit_creature_field_entry(visible.actor, now, entry.as_ref())
                .map_err(|_| DurabilityError::Unavailable)?;
            states
                .tick_source_creature_conditions(
                    runtime,
                    visible.actor,
                    TickFacts {
                        in_protection_zone,
                        standing_on_field: standing,
                    },
                    crate::foundation::owner_timer::SemanticTimeMicros::from_micros(now),
                )
                .map_err(|_| DurabilityError::Unavailable)?;
        }
        Ok(())
    }
}
