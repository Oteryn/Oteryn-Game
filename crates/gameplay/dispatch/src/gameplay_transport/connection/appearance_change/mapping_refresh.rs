//! Stage mapping, stationary VIS-2 updates and retirements before consuming cursors.
use super::*;
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
        let mut staged_view = view.clone();
        let mut staged_items = items.as_deref().cloned();
        let update = staged_view
            .refresh(channel, &mut |entities| {
                attach_spatial_handles(staged_items.as_mut(), entities)
            })
            .ok()?;
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
            )?;
            *view = staged_view;
            if let (Some(items), Some(staged_items)) = (items, staged_items) {
                admitted.continuity.item_view = staged_items.continuity();
                *items = staged_items;
            }
            admitted.continuity.server_sequence = to_sequence;
            for frame in frames {
                write_frame(stream, &frame).await.ok()?;
            }
            return Some((to_sequence, to_revision));
        }
        let mut candidate = self.clone();
        let visible = staged_view.appearance_players();
        let player_refs = Self::players(&visible);
        let observation = authority
            .appearance_observation(actor, admitted.game_session_id, player_refs.clone())
            .await?;
        let old_catalogue = self.catalogue.as_ref()?;
        let upserts = candidate.upserts(observation, &player_refs)?;
        let context = candidate.catalogue.as_ref()?.context;
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
                    .find(|e| e.identity == channel.observer)?
                    .position,
                enter: Vec::new(),
                update: Vec::new(),
                leave: Vec::new(),
            },
            VisibilityUpdate::Snapshot(_) => return None,
        };
        for binding in &upserts {
            if !delta
                .enter
                .iter()
                .chain(&delta.update)
                .any(|e| e.entity == binding.entity)
            {
                delta
                    .update
                    .push(*visible.iter().find(|e| e.entity == binding.entity)?);
            }
        }
        if delta.enter.len() + delta.update.len() + delta.leave.len() > 256 {
            return None;
        }
        candidate.attach(&mut delta.enter)?;
        candidate.attach(&mut delta.update)?;
        let mut frames = Vec::new();
        let mut to_sequence = sequence;
        if old_catalogue != candidate.catalogue.as_ref()? {
            frames.push(candidate.frame(
                generation,
                &mut to_sequence,
                wire::DELTA_TYPE_APPEARANCE_CATALOGUE,
                candidate.catalogue.as_ref()?.encode().ok()?,
            )?);
        }
        if !upserts.is_empty() {
            let payload = wire::Mappings {
                context,
                upserts,
                forgets: Vec::new(),
            }
            .encode()
            .ok()?;
            frames.push(candidate.frame(
                generation,
                &mut to_sequence,
                wire::DELTA_TYPE_APPEARANCE_MAPPINGS,
                payload,
            )?);
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
        )?;
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
            .ok()?;
            frames.push(candidate.frame(
                generation,
                &mut to_sequence,
                wire::DELTA_TYPE_APPEARANCE_MAPPINGS,
                payload,
            )?);
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
            write_frame(stream, &frame).await.ok()?;
        }
        Some((to_sequence, to_revision))
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
