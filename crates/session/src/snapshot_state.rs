//! Atomic domain decoding shared by admission and replacement snapshots.
use super::*;

#[derive(Clone, Default)]
pub(super) struct SnapshotState {
    pub(super) world_spatial_observation: Option<WorldSpatialObservation>,
    pub(super) entities: Option<WorldEntities>,
    pub(super) world_object_overlay: Option<Vec<WorldObjectOverlayEntry>>,
    pub(super) actor_vitals: Option<ActorVitals>,
    pub(super) own_vitals_house_binding: Option<Vec<u8>>,
    pub(super) combat_state: Option<ActorCombatState>,
    pub(super) combat_revision: u64,
    pub(super) spatial_revision: u64,
    pub(super) overlay_revision: u64,
    pub(super) vitals_revision: u64,
    pub(super) chat: Option<ChatLog>,
    pub(super) chat_revision: u64,
    pub(super) inventory: Option<CharacterInventory>,
    pub(super) container_views: Option<container_tree::ContainerViews>,
    pub(super) open_container: Option<OpenContainer>,
    pub(super) inventory_revision: u64,
    pub(super) container_revision: u64,
    pub(super) gated_snapshots: Vec<GatedSnapshot>,
    pub(super) gated_revisions: Vec<(u32, u64)>,
    pub(super) appearance: Option<appearance_state::AppearanceState>,
    pub(super) map_header: Option<world_map::MapViewHeader>,
    pub(super) events: Vec<SessionEvent>,
}

impl SnapshotState {
    pub(super) fn decode(
        body: &[u8],
        sequence: u64,
        selected_capabilities: &[u32],
        appearance_items: Option<AppearanceItemDictionary<'_>>,
        previous: Option<&Self>,
    ) -> Result<Self, SessionError> {
        let entities_selected = selected_capabilities.contains(&CAPABILITY_WORLD_SPATIAL_ENTITIES);
        let attack_selected = selected_capabilities.contains(&CAPABILITY_ATTACK_V1);
        let chat_selected = selected_capabilities.contains(&CAPABILITY_CHAT_V1);
        let item_selected = selected_capabilities.contains(&CAPABILITY_ITEM_VIEW_MOVE_V1);
        guild_chat::check_selection(selected_capabilities)?;
        vip_presence::check_selection(selected_capabilities)?;
        let mut state = previous.cloned().unwrap_or_default();
        if previous.is_none() {
            state.chat = chat_selected.then(ChatLog::default);
            state.inventory = item_selected.then(CharacterInventory::default);
            state.open_container = item_selected.then(OpenContainer::default);
        }
        let domains = decode_snapshot_body(body)?;
        for domain in &domains {
            match (domain.domain_id, domain.snapshot_type) {
                (
                    world_spatial::STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                    world_spatial_entities::SNAPSHOT_TYPE_WORLD_SPATIAL_ENTITIES_V2,
                ) if entities_selected => {
                    let (snapshot, lights) =
                        world_spatial_entities::decode_world_spatial_entities_snapshot_with_fields(
                            domain.payload,
                            world_spatial_entities::EntityFields::selected(selected_capabilities),
                        )?;
                    state.world_spatial_observation = Some(WorldSpatialObservation {
                        content_generation: snapshot.content_generation,
                        actor_position: snapshot.actor_position,
                    });
                    state.entities = Some(WorldEntities::from_snapshot(&snapshot, lights));
                    state.spatial_revision = domain.revision;
                }
                (
                    world_spatial::STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                    world_spatial::SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
                ) if !entities_selected => {
                    state.world_spatial_observation =
                        Some(world_spatial::decode_world_spatial(domain.payload)?);
                    state.spatial_revision = domain.revision;
                }
                (
                    world_object::STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
                    world_object::SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1,
                ) => {
                    state.world_object_overlay = Some(
                        world_object::decode_world_object_overlay_snapshot(domain.payload)?,
                    );
                    state.overlay_revision = domain.revision;
                }
                (
                    actor_spell::STATE_DOMAIN_ACTOR_VITALS,
                    actor_spell::SNAPSHOT_TYPE_ACTOR_VITALS_V1,
                ) => {
                    state.actor_vitals = Some(actor_spell::decode_actor_vitals(domain.payload)?);
                    state.vitals_revision = domain.revision;
                }
                (
                    attack::STATE_DOMAIN_ACTOR_COMBAT_STATE,
                    attack::SNAPSHOT_TYPE_ACTOR_COMBAT_STATE_V1,
                ) if attack_selected => {
                    state.combat_state = Some(
                        attack::decode_actor_combat_state(domain.payload)
                            .map_err(SessionError::Attack)?,
                    );
                    state.combat_revision = domain.revision;
                }
                (attack::STATE_DOMAIN_ACTOR_COMBAT_STATE, snapshot_type) if attack_selected => {
                    return Err(SessionError::UnregisteredSnapshotType {
                        domain_id: attack::STATE_DOMAIN_ACTOR_COMBAT_STATE,
                        snapshot_type,
                    });
                }
                (attack::STATE_DOMAIN_ACTOR_COMBAT_STATE, _) => {
                    return Err(SessionError::UnselectedDomain {
                        domain_id: attack::STATE_DOMAIN_ACTOR_COMBAT_STATE,
                    });
                }
                (
                    STATE_DOMAIN_CHARACTER_INVENTORY,
                    item_view::SNAPSHOT_TYPE_CHARACTER_INVENTORY_V1,
                ) if item_selected => {
                    state.inventory = Some(
                        item_view::decode_character_inventory_with_equip_drop(
                            domain.payload,
                            selected_capabilities
                                .contains(&item_view::CAPABILITY_ITEM_EQUIP_DROP_V1),
                        )
                        .map_err(SessionError::ItemView)?,
                    );
                    state.inventory_revision = domain.revision;
                }
                (STATE_DOMAIN_OPEN_CONTAINER, item_view::SNAPSHOT_TYPE_OPEN_CONTAINER_V1)
                    if item_selected =>
                {
                    state.open_container = Some(
                        item_view::decode_open_container(domain.payload)
                            .map_err(SessionError::ItemView)?,
                    );
                    state.container_revision = domain.revision;
                }
                (
                    domain_id @ (STATE_DOMAIN_CHARACTER_INVENTORY | STATE_DOMAIN_OPEN_CONTAINER),
                    snapshot_type,
                ) if item_selected => {
                    return Err(SessionError::UnregisteredSnapshotType {
                        domain_id,
                        snapshot_type,
                    });
                }
                (
                    domain_id @ (STATE_DOMAIN_CHARACTER_INVENTORY | STATE_DOMAIN_OPEN_CONTAINER),
                    _,
                ) => {
                    return Err(SessionError::UnselectedDomain { domain_id });
                }
                (STATE_DOMAIN_CHAT, snapshot_type) if chat_selected => {
                    state.chat = Some(guild_chat::load_snapshot(
                        snapshot_type,
                        domain.payload,
                        selected_capabilities,
                    )?);
                    state.chat_revision = domain.revision;
                }
                (STATE_DOMAIN_CHAT, _) => {
                    return Err(SessionError::UnselectedDomain {
                        domain_id: STATE_DOMAIN_CHAT,
                    });
                }
                // PROTOCOL_OTERYN_V1_REGISTRY.json registers exactly one snapshot_type (1) for
                // domains 1, 2 and 3; anything else naming one of those domains is a registry
                // violation, not a domain this client merely doesn't need.
                (
                    domain_id @ world_spatial::STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                    snapshot_type,
                )
                | (domain_id @ world_object::STATE_DOMAIN_WORLD_OBJECT_OVERLAY, snapshot_type)
                | (domain_id @ actor_spell::STATE_DOMAIN_ACTOR_VITALS, snapshot_type) => {
                    return Err(SessionError::UnregisteredSnapshotType {
                        domain_id,
                        snapshot_type,
                    });
                }
                // A gated domain is kept raw for its capability's own packet.
                // Unselected is a server violation; an unregistered domain is ignored.
                (domain_id, snapshot_type) => match capability_of_domain(domain_id) {
                    Some(capability) if selected_capabilities.contains(&capability) => {
                        if domain_id == cast::STATE_DOMAIN_CAST_CASTER_CHAT {
                            cast_chat::validate_snapshot(
                                snapshot_type,
                                domain.revision,
                                domain.payload,
                            )?;
                        }
                        if domain_id == analyser_transport::STATE_DOMAIN_ACTOR_ANALYSER {
                            analyser_transport::validate_snapshot(
                                &GatedSnapshot {
                                    domain_id,
                                    snapshot_type,
                                    revision: domain.revision,
                                    payload: domain.payload.to_vec(),
                                },
                                previous.and_then(|state| {
                                    state
                                        .gated_revisions
                                        .iter()
                                        .find(|(id, _)| *id == domain_id)
                                        .map(|(_, revision)| *revision)
                                }),
                            )?;
                        }
                        if domain_id == spell_catalog::STATE_DOMAIN_ACTOR_SPELL_CATALOG {
                            spell_catalog::validate_snapshot(
                                &GatedSnapshot {
                                    domain_id,
                                    snapshot_type,
                                    revision: domain.revision,
                                    payload: domain.payload.to_vec(),
                                },
                                previous.and_then(|state| {
                                    state
                                        .gated_revisions
                                        .iter()
                                        .find(|(id, _)| *id == domain_id)
                                        .map(|(_, revision)| *revision)
                                }),
                            )?;
                        }
                        state.gated_revisions.retain(|(id, _)| *id != domain_id);
                        state.gated_revisions.push((domain_id, domain.revision));
                        state
                            .gated_snapshots
                            .retain(|snapshot| snapshot.domain_id != domain_id);
                        state.gated_snapshots.push(GatedSnapshot {
                            domain_id,
                            snapshot_type,
                            revision: domain.revision,
                            payload: domain.payload.to_vec(),
                        });
                    }
                    Some(_) => return Err(SessionError::UnselectedDomain { domain_id }),
                    None => {}
                },
            }
        }

        if previous.is_none() || domains.iter().any(|domain| domain.domain_id == 21) {
            let references = previous
                .and_then(|state| state.appearance.as_ref())
                .and_then(appearance_state::item_references);
            let appearance_items = references
                .as_deref()
                .map(|definition_refs| AppearanceItemDictionary {
                    content_generation: state
                        .world_spatial_observation
                        .as_ref()
                        .map_or([0; 32], |value| value.content_generation),
                    definition_refs,
                })
                .or(appearance_items);
            state.appearance = appearance_state::from_snapshots(
                selected_capabilities,
                &state.gated_snapshots,
                state
                    .world_spatial_observation
                    .as_ref()
                    .ok_or(SessionError::MissingDomain(1))?
                    .content_generation,
                appearance_items,
            )?;
        }
        appearance_state::validate_entities(state.appearance.as_ref(), state.entities.as_ref())?;
        if attack_selected && state.combat_state.is_none() {
            return Err(SessionError::MissingDomain(
                attack::STATE_DOMAIN_ACTOR_COMBAT_STATE,
            ));
        }
        state.container_views =
            container_tree::initial(selected_capabilities, &state.gated_snapshots)?;
        channel_list::validate_join(selected_capabilities, &state.gated_snapshots)?;
        if selected_capabilities.contains(&spell_catalog::CAPABILITY_ACTOR_SPELL_CATALOG_V1)
            && !state
                .gated_snapshots
                .iter()
                .any(|row| row.domain_id == spell_catalog::STATE_DOMAIN_ACTOR_SPELL_CATALOG)
        {
            return Err(SessionError::MissingDomain(
                spell_catalog::STATE_DOMAIN_ACTOR_SPELL_CATALOG,
            ));
        }
        cast_chat::validate_join(selected_capabilities, &state.gated_snapshots)?;
        analyser_transport::validate_join(selected_capabilities, &state.gated_snapshots)?;
        weather::validate_join(selected_capabilities, &state.gated_snapshots)?;
        quest_log_transport::validate_join(selected_capabilities, &state.gated_snapshots)?;
        tutorial_notices::validate_join(selected_capabilities, &state.gated_snapshots)?;
        pvp_expert::validate_join(selected_capabilities, &state.gated_snapshots)?;
        world_light::validate_join(selected_capabilities, &state.gated_snapshots)?;
        presentation::validate_join(selected_capabilities, &state.gated_snapshots)?;
        server_save_countdown::validate_join(selected_capabilities, &state.gated_snapshots)?;
        achievement_notices_transport::validate_join(
            selected_capabilities,
            &state.gated_snapshots,
        )?;
        state.events = server_messages::validate_join(
            selected_capabilities,
            &state.gated_snapshots,
            sequence,
        )?;
        if previous.is_some() && !domains.iter().any(|domain| domain.domain_id == 57) {
            state.events.clear();
        }
        if previous.is_none() || domains.iter().any(|domain| domain.domain_id == 17) {
            state.map_header = map_transport::validate_join(
                selected_capabilities,
                &state.gated_snapshots,
                state
                    .world_spatial_observation
                    .as_ref()
                    .ok_or(SessionError::MissingDomain(1))?
                    .content_generation,
            )?;
        }
        if state.map_header.is_some() {
            if domains.iter().any(|domain| domain.domain_id == 2) {
                return Err(SessionError::UnselectedDomain { domain_id: 2 });
            }
            state.world_object_overlay = Some(Vec::new());
        }
        if let Some(old) = previous.and_then(|state| state.world_spatial_observation)
            && state
                .world_spatial_observation
                .is_some_and(|value| value.content_generation != old.content_generation)
        {
            return Err(SessionError::ContentGenerationMismatch { domain_id: 1 });
        }
        if domains
            .iter()
            .any(|domain| domain.domain_id == actor_spell::STATE_DOMAIN_ACTOR_VITALS)
        {
            state.own_vitals_house_binding = state
                .gated_snapshots
                .iter()
                .find(|snapshot| snapshot.domain_id == 51)
                .map(|snapshot| snapshot.payload.clone());
        }
        Ok(state)
    }
}
#[cfg(test)]
mod house_vitals_provenance_tests {
    use super::*;
    use oteryn_protocol_oteryn::{
        DomainSnapshot, decode_snapshot_chunk_framing, decode_wire_envelope,
        encode_single_chunk_snapshot,
    };

    fn replacement(
        previous: &SnapshotState,
        domains: &[DomainSnapshot<'_>],
    ) -> Result<SnapshotState, String> {
        let frames = encode_single_chunk_snapshot(1, 1, 1, domains)
            .map_err(|error| format!("valid actual snapshot encoding: {error:?}"))?;
        let envelope = decode_wire_envelope(&frames[1])
            .map_err(|error| format!("actual snapshot frame: {error:?}"))?;
        let (_, body) = decode_snapshot_chunk_framing(envelope.payload())
            .map_err(|error| format!("actual snapshot body: {error:?}"))?;
        SnapshotState::decode(body, 1, &[86], None, Some(previous))
            .map_err(|error| format!("atomic replacement: {error:?}"))
    }

    #[test]
    fn srv7_own_house_vitals_provenance_requires_actual_domain_and_ignores_domain_order()
    -> Result<(), String> {
        use oteryn_protocol_oteryn::house_transfer::{HouseTransferAck, encode_house_transfer_ack};
        let mut id = [0; 16];
        id[6] = 0x70;
        id[8] = 0x80;
        id[15] = 1;
        let mut ack = HouseTransferAck {
            transfer: id,
            session: id,
            world: id,
            channel: id,
            house: Some("oteryn:content.house.test".into()),
            scope_generation: 1,
            connection_generation: 1,
            lease_generation: 1,
            selection: vec![86],
            position: [100, 200, 7],
        };
        let old_payload = encode_house_transfer_ack(&ack)
            .map_err(|error| format!("actual valid House ACK: {error:?}"))?;
        ack.lease_generation += 1;
        let next_payload = encode_house_transfer_ack(&ack)
            .map_err(|error| format!("actual successor House ACK: {error:?}"))?;
        let old = GatedSnapshot {
            domain_id: 51,
            snapshot_type: 1,
            revision: 1,
            payload: old_payload.clone(),
        };
        let previous = SnapshotState {
            actor_vitals: Some(ActorVitals {
                health: 0,
                max_health: 150,
                ..Default::default()
            }),
            gated_snapshots: vec![old],
            own_vitals_house_binding: Some(old_payload.clone()),
            ..Default::default()
        };
        let house = DomainSnapshot {
            domain_id: 51,
            snapshot_type: 1,
            revision: 2,
            payload: &next_payload,
        };
        let retained = replacement(&previous, &[house])?;
        assert_eq!(
            retained.own_vitals_house_binding,
            Some(old_payload),
            "replacing House ACK alone cannot rebind retained HP0"
        );
        let payload = actor_spell::encode_actor_vitals(&ActorVitals {
            health: 0,
            max_health: 150,
            ..Default::default()
        })
        .map_err(|error| format!("valid own vitals: {error:?}"))?;
        let vitals = DomainSnapshot {
            domain_id: 3,
            snapshot_type: 1,
            revision: 2,
            payload: &payload,
        };
        for domains in [[vitals, house], [house, vitals]] {
            let fresh = replacement(&previous, &domains)?;
            assert_eq!(
                fresh.own_vitals_house_binding,
                Some(next_payload.clone()),
                "atomic snapshot binds actual own-vitals to its final House witness regardless of domain order"
            );
        }
        Ok(())
    }
}
