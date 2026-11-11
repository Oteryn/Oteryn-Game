//! Stage mapping, stationary VIS-2 updates and retirements before consuming cursors.
use super::*;
use crate::gameplay_transport::appearance_change::observation::ObservationError::{
    self, Unavailable,
};
use oteryn_protocol_oteryn::world_spatial_entities::WorldSpatialEntitiesDelta;

impl SessionAppearance {
    #[allow(clippy::too_many_arguments)]
    pub(in crate::gameplay_transport::connection) async fn refresh<
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
        sequence: u64,
        revision: u64,
    ) -> Option<(u64, u64)> {
        self.refresh_result(
            stream, authority, actor, admitted, view, channel, items, sequence, revision,
        )
        .await
        .ok()
    }
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
        sequence: u64,
        revision: u64,
    ) -> Result<(u64, u64), ObservationError> {
        let mut staged_view = view.clone();
        let mut staged_items = items.as_deref().cloned();
        let update = staged_view
            .refresh(channel, &mut |entities| {
                attach_spatial_handles(staged_items.as_mut(), entities)
            })
            .map_err(|_| Unavailable)?;
        let generation = admitted.continuity.connection_generation;
        let selected = admitted.continuity.selected_capabilities;
        if !self.selected {
            let (frames, to_sequence, to_revision) = visibility_frames(
                generation,
                sequence,
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
            for frame in frames {
                write_frame(stream, &frame).await.map_err(|_| Unavailable)?;
            }
            return Ok((to_sequence, to_revision));
        }
        let mut candidate = self.clone();
        candidate.select_hirelings(channel);
        let visible = staged_view.appearance_actors(self.forge);
        let player_refs = candidate.players(&visible);
        let observation = authority
            .appearance_observation_projected_result(
                actor,
                admitted.game_session_id,
                candidate.empty_outfit,
                player_refs.clone(),
            )
            .await?;
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
