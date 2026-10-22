//! Apply typed domain deltas to this connection after envelope validation.
use super::*;

impl<S: SessionStream> Session<S> {
    /// The domain store: applies one validated `StateDelta` by its own domain. It must be based
    /// on exactly the stored revision, carry the domain's registered `delta_type` and, where the
    /// domain has one, the loaded `content_generation`. Nothing is stored unless every check and
    /// the typed decode pass.
    pub(crate) fn apply_state_delta(
        &mut self,
        server_sequence: u64,
        payload: &[u8],
    ) -> Result<SessionEvent, SessionError> {
        let delta = decode_state_delta(payload)?;
        let (domain_id, base_revision, new_revision) =
            (delta.domain_id, delta.base_revision, delta.new_revision);
        fn applied<T>(sequence: u64, base: u64, new: u64, value: T) -> AppliedDelta<T> {
            AppliedDelta {
                server_sequence: sequence,
                base_revision: base,
                new_revision: new,
                value,
            }
        }
        match domain_id {
            oteryn_protocol_oteryn::appearance::STATE_DOMAIN_APPEARANCE => {
                self.appearance_delta(server_sequence, delta)
            }
            world_spatial::STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY if self.entities.is_some() => {
                check_delta(
                    &delta,
                    world_spatial_entities::DELTA_TYPE_WORLD_SPATIAL_ENTITIES_V2,
                    self.spatial_revision,
                )?;
                let entities_delta = if self.is_selected(CAPABILITY_ITEM_VIEW_MOVE_V1) {
                    world_spatial_entities::decode_world_spatial_entities_delta_with_item_handles(
                        delta.payload,
                    )?
                } else {
                    world_spatial_entities::decode_world_spatial_entities_delta(delta.payload)?
                };
                if entities_delta.content_generation != self.world_spatial.content_generation {
                    return Err(SessionError::ContentGenerationMismatch { domain_id });
                }
                let next = match &self.entities {
                    Some(stored) => stored.after(&entities_delta)?,
                    None => {
                        return Err(SessionError::UnselectedDomain { domain_id });
                    }
                };
                appearance_state::validate_entities(self.appearance.as_ref(), Some(&next))?;
                self.entities = Some(next);
                self.world_spatial.actor_position = entities_delta.actor_position;
                self.spatial_revision = new_revision;
                Ok(SessionEvent::WorldSpatialEntities(applied(
                    server_sequence,
                    base_revision,
                    new_revision,
                    entities_delta,
                )))
            }
            world_spatial::STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY => {
                check_delta(
                    &delta,
                    world_spatial::DELTA_TYPE_WORLD_SPATIAL_V1,
                    self.spatial_revision,
                )?;
                let observation = world_spatial::decode_world_spatial(delta.payload)?;
                if observation.content_generation != self.world_spatial.content_generation {
                    return Err(SessionError::ContentGenerationMismatch { domain_id });
                }
                self.world_spatial = observation;
                self.spatial_revision = new_revision;
                Ok(SessionEvent::WorldSpatial(applied(
                    server_sequence,
                    base_revision,
                    new_revision,
                    observation,
                )))
            }
            world_object::STATE_DOMAIN_WORLD_OBJECT_OVERLAY => {
                check_delta(
                    &delta,
                    world_object::DELTA_TYPE_WORLD_OBJECT_OVERLAY_V1,
                    self.overlay_revision,
                )?;
                let entry = world_object::decode_world_object_overlay_delta(delta.payload)?;
                if entry.revision != new_revision {
                    return Err(SessionError::OverlayEntryRevisionMismatch {
                        new_revision,
                        entry_revision: entry.revision,
                    });
                }
                if entry.content_generation != self.world_spatial.content_generation {
                    return Err(SessionError::ContentGenerationMismatch { domain_id });
                }
                match self
                    .world_object_overlay
                    .iter_mut()
                    .find(|existing| existing.placement == entry.placement)
                {
                    Some(existing) => *existing = entry.clone(),
                    None => self.world_object_overlay.push(entry.clone()),
                }
                self.overlay_revision = new_revision;
                Ok(SessionEvent::WorldObjectOverlay(applied(
                    server_sequence,
                    base_revision,
                    new_revision,
                    entry,
                )))
            }
            actor_spell::STATE_DOMAIN_ACTOR_VITALS => {
                check_delta(
                    &delta,
                    actor_spell::DELTA_TYPE_ACTOR_VITALS_V1,
                    self.vitals_revision,
                )?;
                let vitals = actor_spell::decode_actor_vitals(delta.payload)?;
                self.actor_vitals = Some(vitals);
                self.vitals_revision = new_revision;
                Ok(SessionEvent::ActorVitals(applied(
                    server_sequence,
                    base_revision,
                    new_revision,
                    vitals,
                )))
            }
            attack::STATE_DOMAIN_ACTOR_COMBAT_STATE if self.is_selected(CAPABILITY_ATTACK_V1) => {
                check_delta(
                    &delta,
                    attack::DELTA_TYPE_ACTOR_COMBAT_STATE_V1,
                    self.combat_revision,
                )?;
                let state = attack::decode_actor_combat_state(delta.payload)
                    .map_err(SessionError::Attack)?;
                self.combat_state = Some(state);
                self.combat_revision = new_revision;
                Ok(SessionEvent::ActorCombatState(applied(
                    server_sequence,
                    base_revision,
                    new_revision,
                    state,
                )))
            }
            attack::STATE_DOMAIN_ACTOR_COMBAT_STATE => {
                Err(SessionError::UnselectedDomain { domain_id })
            }
            STATE_DOMAIN_CHAT if self.chat.is_some() => {
                // Both delta types are registered for domain 12; each is checked against the
                // one stored revision and decoded before anything is stored.
                let delta_type = delta.delta_type;
                check_delta(&delta, delta_type, self.chat_revision)?;
                let event = match delta_type {
                    chat::DELTA_TYPE_CHAT_LINE_V1 => {
                        let line = chat::decode_chat_line(delta.payload)?;
                        SessionEvent::ChatLine(applied(
                            server_sequence,
                            base_revision,
                            new_revision,
                            line,
                        ))
                    }
                    chat::DELTA_TYPE_CHAT_ROOMS_V1 => {
                        let rooms = chat::decode_chat_rooms(delta.payload)?;
                        SessionEvent::ChatRooms(applied(
                            server_sequence,
                            base_revision,
                            new_revision,
                            rooms,
                        ))
                    }
                    _ => {
                        return Err(SessionError::UnregisteredDeltaType {
                            domain_id,
                            delta_type,
                        });
                    }
                };
                if let Some(log) = self.chat.as_mut() {
                    match &event {
                        SessionEvent::ChatLine(line) => log.push(line.value.clone()),
                        SessionEvent::ChatRooms(rooms) => log.rooms = rooms.value,
                        _ => {}
                    }
                }
                self.chat_revision = new_revision;
                Ok(event)
            }
            STATE_DOMAIN_CHAT => Err(SessionError::UnselectedDomain { domain_id }),
            STATE_DOMAIN_CHARACTER_INVENTORY if self.inventory.is_some() => {
                check_delta(
                    &delta,
                    item_view::DELTA_TYPE_CHARACTER_INVENTORY_V1,
                    self.inventory_revision,
                )?;
                let value = item_view::decode_character_inventory(delta.payload)
                    .map_err(SessionError::ItemView)?;
                self.inventory = Some(value.clone());
                self.inventory_revision = new_revision;
                Ok(SessionEvent::Inventory(applied(
                    server_sequence,
                    base_revision,
                    new_revision,
                    value,
                )))
            }
            STATE_DOMAIN_OPEN_CONTAINER if self.open_container.is_some() => {
                check_delta(
                    &delta,
                    item_view::DELTA_TYPE_OPEN_CONTAINER_V1,
                    self.container_revision,
                )?;
                let value = item_view::decode_open_container(delta.payload)
                    .map_err(SessionError::ItemView)?;
                self.open_container = Some(value.clone());
                self.container_revision = new_revision;
                Ok(SessionEvent::OpenContainer(applied(
                    server_sequence,
                    base_revision,
                    new_revision,
                    value,
                )))
            }
            _ => Err(match capability_of_domain(domain_id) {
                Some(capability) if !self.is_selected(capability) => {
                    SessionError::UnselectedDomain { domain_id }
                }
                _ => SessionError::UnsupportedPushedDomain { domain_id },
            }),
        }
    }
}
