//! Stage mapping, stationary VIS-2 updates and retirements before consuming cursors.
use super::*;
use crate::gameplay_transport::appearance_change::observation::Observation;
use crate::gameplay_transport::appearance_change::observation::ObservationError::{
    self, HolderBusy, Unavailable,
};
use oteryn_protocol_oteryn::world_spatial_entities::WorldSpatialEntitiesDelta;

impl SessionAppearance {
    #[allow(clippy::too_many_arguments)]
    pub(in crate::gameplay_transport::connection) async fn refresh_result<
        S: AsyncWrite + Unpin,
        A: FreshAdmissionAuthority,
    >(
        &mut self,
        stream: &mut S,
        authority: &A,
        actor: ExactActorRef,
        admitted: &mut AdmittedSession,
        view: &mut SessionVisibility,
        channel: &ChannelEntities,
        items: Option<&mut SessionItemView>,
        (sequence, revision): (u64, u64),
        moved: bool,
    ) -> Result<(u64, u64), ObservationError> {
        let mut staged_view = view.clone();
        let mut staged_items = items.as_deref().cloned();
        let mut update = staged_view
            .refresh(channel, &mut |entities| {
                attach_spatial_handles(staged_items.as_mut(), entities)
            })
            .map_err(|_| Unavailable)?;
        let generation = admitted.continuity.connection_generation;
        let selected = admitted.continuity.selected_capabilities;
        if !self.selected {
            let mut to_sequence = sequence;
            let death = death_frame(channel, view, &staged_view, admitted, &mut to_sequence)?;
            let (frames, to_sequence, to_revision) = visibility_frames(
                generation,
                to_sequence,
                revision,
                selected.as_slice(),
                staged_view.lights(),
                &update,
            )
            .ok_or(Unavailable)?;
            *view = staged_view;
            if let (Some(items), Some(staged_items)) = (items, staged_items) {
                admitted.continuity.item_view = staged_items.continuity();
                *items = staged_items;
            }
            admitted.continuity.server_sequence = to_sequence;
            if let Some((_, revision)) = &death {
                admitted.continuity.presentation_revision = *revision;
            }
            for frame in death.into_iter().map(|(frame, _)| frame).chain(frames) {
                write_frame(stream, &frame).await.map_err(|_| Unavailable)?;
            }
            return Ok((to_sequence, to_revision));
        }
        let mut candidate = self.clone();
        candidate.select_hirelings(channel);
        let visible = staged_view.appearance_actors(self.forge);
        let mut player_refs = candidate.players(&visible);
        let observation = authority
            .appearance_observation_projected_result(
                actor,
                admitted.game_session_id,
                candidate.empty_outfit,
                player_refs.clone(),
            )
            .await;
        let observation = match observation {
            Err(HolderBusy) if moved => {
                // The movement already committed. Publish the new position using the
                // exact incarnations the client knows. A later refresh will bind entrants.
                let mut bindings = Vec::new();
                let mut skipped = Vec::new();
                for entity in &player_refs {
                    if let Some(binding) = self.bindings.get(entity) {
                        bindings.push(binding.clone());
                    } else {
                        skipped.push(*entity);
                    }
                }
                Observation {
                    catalogue: self.catalogue.clone().ok_or(Unavailable)?,
                    bindings,
                    skipped,
                }
            }
            other => other?,
        };
        player_refs.retain(|entity| !observation.skipped.contains(entity));
        if !observation.skipped.is_empty() {
            let mut channel = channel.clone();
            channel.entities.retain(|entity| {
                !observation.skipped.contains(&WireEntityRef {
                    identity: entity.identity,
                    generation: entity.generation,
                })
            });
            staged_view = view.clone();
            staged_items = items.as_deref().cloned();
            update = staged_view
                .refresh(&channel, &mut |entities| {
                    attach_spatial_handles(staged_items.as_mut(), entities)
                })
                .map_err(|_| Unavailable)?;
        }
        let old_catalogue = self.catalogue.as_ref().ok_or(Unavailable)?;
        let upserts = candidate
            .upserts(observation, &player_refs)
            .ok_or(Unavailable)?;
        let context = candidate.catalogue.as_ref().ok_or(Unavailable)?.context;
        let forgets: Vec<_> = candidate
            .bindings
            .keys()
            .filter(|e| !player_refs.contains(e))
            .copied()
            .collect();
        let mut delta = match update {
            VisibilityUpdate::Delta(delta) => delta,
            VisibilityUpdate::Unchanged => WorldSpatialEntitiesDelta {
                content_generation: channel.content_generation,
                actor_position: channel
                    .entities
                    .iter()
                    .find(|e| e.identity == channel.observer)
                    .ok_or(Unavailable)?
                    .position,
                enter: Vec::new(),
                update: Vec::new(),
                leave: Vec::new(),
            },
            VisibilityUpdate::Snapshot(_) => return Err(Unavailable),
        };
        for binding in &upserts {
            if !delta
                .enter
                .iter()
                .chain(&delta.update)
                .any(|e| e.entity == binding.entity)
            {
                delta.update.push(
                    *visible
                        .iter()
                        .find(|e| e.entity == binding.entity)
                        .ok_or(Unavailable)?,
                );
            }
        }
        if delta.enter.len() + delta.update.len() + delta.leave.len() > 256 {
            return Err(Unavailable);
        }
        candidate.attach(&mut delta.enter).ok_or(Unavailable)?;
        candidate.attach(&mut delta.update).ok_or(Unavailable)?;
        let mut frames = Vec::new();
        let mut to_sequence = sequence;
        if old_catalogue != candidate.catalogue.as_ref().ok_or(Unavailable)? {
            frames.push(
                candidate
                    .frame(
                        generation,
                        &mut to_sequence,
                        wire::DELTA_TYPE_APPEARANCE_CATALOGUE,
                        candidate
                            .catalogue
                            .as_ref()
                            .ok_or(Unavailable)?
                            .encode()
                            .map_err(|_| Unavailable)?,
                    )
                    .ok_or(Unavailable)?,
            );
        }
        if !upserts.is_empty() {
            let payload = wire::Mappings {
                context,
                upserts,
                forgets: Vec::new(),
            }
            .encode()
            .map_err(|_| Unavailable)?;
            frames.push(
                candidate
                    .frame(
                        generation,
                        &mut to_sequence,
                        wire::DELTA_TYPE_APPEARANCE_MAPPINGS,
                        payload,
                    )
                    .ok_or(Unavailable)?,
            );
        }
        let update = if delta.enter.is_empty() && delta.update.is_empty() && delta.leave.is_empty()
        {
            VisibilityUpdate::Unchanged
        } else {
            VisibilityUpdate::Delta(delta)
        };
        let presentation_revision =
            death_frame(channel, view, &staged_view, admitted, &mut to_sequence)?.map(
                |(frame, revision)| {
                    frames.push(frame);
                    revision
                },
            );
        let (spatial, next_sequence, to_revision) = visibility_frames(
            generation,
            to_sequence,
            revision,
            selected.as_slice(),
            staged_view.lights(),
            &update,
        )
        .ok_or(Unavailable)?;
        frames.extend(spatial);
        to_sequence = next_sequence;
        // The VIS-2 leave is staged before retiring its exact-generation mapping.
        if !forgets.is_empty() {
            let payload = wire::Mappings {
                context,
                upserts: Vec::new(),
                forgets: forgets.clone(),
            }
            .encode()
            .map_err(|_| Unavailable)?;
            frames.push(
                candidate
                    .frame(
                        generation,
                        &mut to_sequence,
                        wire::DELTA_TYPE_APPEARANCE_MAPPINGS,
                        payload,
                    )
                    .ok_or(Unavailable)?,
            );
            for entity in forgets {
                candidate.bindings.remove(&entity);
            }
        }
        *view = staged_view;
        if let (Some(items), Some(staged_items)) = (items, staged_items) {
            admitted.continuity.item_view = staged_items.continuity();
            *items = staged_items;
        }
        *self = candidate;
        if let Some(revision) = presentation_revision {
            admitted.continuity.presentation_revision = revision;
        }
        admitted.continuity.server_sequence = to_sequence;
        for frame in frames {
            write_frame(stream, &frame).await.map_err(|_| Unavailable)?;
        }
        Ok((to_sequence, to_revision))
    }
    fn frame(
        &mut self,
        generation: u64,
        sequence: &mut u64,
        kind: u32,
        payload: Vec<u8>,
    ) -> Option<Vec<u8>> {
        let next_sequence = sequence.checked_add(1)?;
        let next_revision = self.revision.checked_add(1)?;
        let frame = encode_state_delta(
            generation,
            next_sequence,
            wire::STATE_DOMAIN_APPEARANCE,
            self.revision,
            next_revision,
            kind,
            &payload,
        )
        .ok()?;
        *sequence = next_sequence;
        self.revision = next_revision;
        Some(frame)
    }
}

// The same Channel observation supplied the exact actor departure and the
// entering corpse. Publish the committed HP loss before that spatial frame.
fn death_frame(
    channel: &ChannelEntities,
    before: &SessionVisibility,
    after: &SessionVisibility,
    admitted: &AdmittedSession,
    sequence: &mut u64,
) -> Result<Option<(Vec<u8>, u64)>, ObservationError> {
    use crate::gameplay_transport::world_spatial::VisibleKind;
    use oteryn_protocol_oteryn::presentation as wire;
    if !admitted
        .continuity
        .selected_capabilities
        .contains(wire::CAPABILITY_PRESENTATION_V1)
    {
        return Ok(None);
    }
    let events: Vec<_> = channel
        .entities
        .iter()
        .filter_map(|entity| {
            let VisibleKind::Corpse {
                lethal_hit: Some(hit),
                ..
            } = entity.kind
            else {
                return None;
            };
            let corpse = WireEntityRef {
                identity: entity.identity,
                generation: entity.generation,
            };
            if !before.shows(hit.target) || after.shows(hit.target) || !after.shows_corpse(corpse) {
                return None;
            }
            Some(wire::PresentationEventKind::ValueText {
                kind: if hit.attacker.identity == channel.observer {
                    wire::PresentationValueKind::DamageDealt
                } else {
                    wire::PresentationValueKind::DamageOthers
                },
                position: entity.position,
                target: hit.target,
                attacker: before.shows(hit.attacker).then_some(hit.attacker),
                primary: wire::PresentationValue {
                    value: hit.value.get(),
                    color: wire::PresentationColor::Red,
                },
                secondary: None,
            })
        })
        .take(wire::MAX_PRESENTATION_EVENTS)
        .enumerate()
        .map(|(index, kind)| wire::PresentationEvent {
            decision_ordinal: (index + 1) as u32,
            emission_ordinal: 1,
            kind,
        })
        .collect();
    if events.is_empty() {
        return Ok(None);
    }
    let payload = wire::encode_presentation_batch(&events).map_err(|_| Unavailable)?;
    let current = admitted.continuity.presentation_revision;
    let revision = current.checked_add(1).ok_or(Unavailable)?;
    let next = sequence.checked_add(1).ok_or(Unavailable)?;
    let frame = oteryn_protocol_oteryn::encode_state_delta(
        admitted.continuity.connection_generation,
        next,
        wire::STATE_DOMAIN_WORLD_PRESENTATION,
        current,
        revision,
        wire::DELTA_TYPE_WORLD_PRESENTATION_BATCH_V1,
        &payload,
    )
    .map_err(|_| Unavailable)?;
    *sequence = next;
    Ok(Some((frame, revision)))
}
