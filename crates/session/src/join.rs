//! Shared join-snapshot validation for admission and recovery.
use super::*;

pub(super) struct JoinAcceptance {
    pub(super) game_session_id: GameSessionId,
    pub(super) connection_generation: u64,
    pub(super) next_command_id: u64,
    pub(super) selected_capabilities: Vec<u32>,
    pub(super) recovery_sequence: Option<u64>,
}

impl<S: SessionStream> Session<S> {
    pub(super) async fn read_join(
        mut stream: S,
        deadline: Duration,
        appearance_items: Option<AppearanceItemDictionary<'_>>,
        accepted_fields: JoinAcceptance,
    ) -> Result<Self, SessionError> {
        let session_generation = accepted_fields.connection_generation;
        let selected_capabilities = accepted_fields.selected_capabilities.clone();

        // SnapshotBegin's full declaration: every chunk read below, and the commit that follows
        // them, is checked against it before being trusted.
        let begin_frame = bounded(deadline, "SnapshotBegin", read_frame(&mut stream)).await?;
        let begin_envelope = decode_wire_envelope(&begin_frame)?;
        // Post-admission server traffic: direction, phase, sequencing (unsequenced — FND-02 §14
        // snapshot transfer-control frames carry no `server_sequence`) and a nonzero envelope
        // `connection_generation` are checked before this frame's payload is consumed.
        begin_envelope.validate(Direction::ServerToClient, true)?;
        if begin_envelope.message_type() != MessageType::SnapshotBegin {
            return Err(SessionError::UnexpectedMessage {
                expected: MessageType::SnapshotBegin,
                actual: begin_envelope.message_type(),
            });
        }
        if begin_envelope.connection_generation() != session_generation {
            return Err(SessionError::ConnectionGenerationMismatch {
                expected: session_generation,
                actual: begin_envelope.connection_generation(),
            });
        }
        let begin = decode_snapshot_begin(begin_envelope.payload())?;
        if accepted_fields
            .recovery_sequence
            .is_some_and(|sequence| sequence != begin.target_server_sequence)
        {
            return Err(FoundationProtocolError::MalformedEnvelope.into());
        }

        // Exactly `begin.chunk_count` `SnapshotChunk` frames, strictly in order: a short transfer
        // (the server stops early) surfaces here as `UnexpectedMessage` (the next frame is
        // `SnapshotCommit` instead) or as a bounded `Timeout`/`Io` (the connection stalls or closes).
        // A multi-chunk transfer may split a `SnapshotBody` protobuf field at any byte offset, not
        // necessarily a field boundary (FND-02 §16), so every chunk's raw `data` is only ever
        // concatenated here, in `chunk_index` order, into one owned buffer — never decoded on its
        // own. `assembled_bytes` is checked against `SnapshotBegin`'s declared
        // `total_encoded_bytes` with checked arithmetic, and rejected, *before* the chunk's bytes are
        // appended (before growing the assembly), so an over-declaring or over-sending peer cannot
        // grow the buffer past the declared bound first and get rejected only afterwards.
        let mut assembled_body: Vec<u8> = Vec::new();
        let mut assembled_bytes: u64 = 0;
        for expected_index in 0..begin.chunk_count {
            let chunk_frame = bounded(deadline, "SnapshotChunk", read_frame(&mut stream)).await?;
            let chunk_envelope = decode_wire_envelope(&chunk_frame)?;
            // Post-admission server traffic, same as `SnapshotBegin` above.
            chunk_envelope.validate(Direction::ServerToClient, true)?;
            if chunk_envelope.message_type() != MessageType::SnapshotChunk {
                return Err(SessionError::UnexpectedMessage {
                    expected: MessageType::SnapshotChunk,
                    actual: chunk_envelope.message_type(),
                });
            }
            if chunk_envelope.connection_generation() != session_generation {
                return Err(SessionError::ConnectionGenerationMismatch {
                    expected: session_generation,
                    actual: chunk_envelope.connection_generation(),
                });
            }
            let chunk_snapshot_id = decode_snapshot_id(chunk_envelope.payload())?;
            if chunk_snapshot_id != begin.snapshot_id {
                return Err(SessionError::SnapshotIdMismatch {
                    expected: begin.snapshot_id,
                    actual: chunk_snapshot_id,
                });
            }
            let (chunk_index, chunk_data) =
                decode_snapshot_chunk_framing(chunk_envelope.payload())?;
            if chunk_index != expected_index {
                return Err(SessionError::ChunkIndexMismatch {
                    expected: expected_index,
                    actual: chunk_index,
                });
            }
            let chunk_len = u64::try_from(chunk_data.len()).unwrap_or(u64::MAX);
            assembled_bytes = assembled_bytes
                .checked_add(chunk_len)
                .filter(|total| *total <= begin.total_encoded_bytes)
                .ok_or(SessionError::AssembledLengthMismatch {
                    expected: begin.total_encoded_bytes,
                    actual: assembled_bytes.saturating_add(chunk_len),
                })?;
            assembled_body.extend_from_slice(chunk_data);
        }
        if assembled_bytes != begin.total_encoded_bytes {
            return Err(SessionError::AssembledLengthMismatch {
                expected: begin.total_encoded_bytes,
                actual: assembled_bytes,
            });
        }

        // The commit is accepted only once every declared chunk arrived, in order, and the
        // assembled length matched.
        let commit_snapshot_id = read_snapshot_marker(
            &mut stream,
            deadline,
            "SnapshotCommit",
            MessageType::SnapshotCommit,
            session_generation,
        )
        .await?;
        if commit_snapshot_id != begin.snapshot_id {
            return Err(SessionError::SnapshotIdMismatch {
                expected: begin.snapshot_id,
                actual: commit_snapshot_id,
            });
        }

        // Only now — every declared chunk arrived in order, the assembled length matched, and the
        // matching `SnapshotCommit` validated — is the assembled `SnapshotBody` decoded, exactly
        // once (FND-02 §16: "protobuf decode occurs only after a full bounded body is assembled" and
        // "apply is atomic only after all chunks and matching SnapshotCommit validate").
        let entities_selected = selected_capabilities.contains(&CAPABILITY_WORLD_SPATIAL_ENTITIES);
        let mut world_spatial_observation = None;
        let mut entities = None;
        let mut world_object_overlay = None;
        let mut actor_vitals = None;
        let mut combat_state = None;
        let mut combat_revision = 0;
        let attack_selected = selected_capabilities.contains(&CAPABILITY_ATTACK_V1);
        let (mut spatial_revision, mut overlay_revision, mut vitals_revision) = (0, 0, 0);
        guild_chat::check_selection(&selected_capabilities)?;
        vip_presence::check_selection(&selected_capabilities)?;
        let chat_selected = selected_capabilities.contains(&CAPABILITY_CHAT_V1);
        let mut chat = chat_selected.then(ChatLog::default);
        let mut chat_revision = 0;
        let item_selected = selected_capabilities.contains(&CAPABILITY_ITEM_VIEW_MOVE_V1);
        let mut inventory = item_selected.then(CharacterInventory::default);
        let mut open_container = item_selected.then(OpenContainer::default);
        let (mut inventory_revision, mut container_revision) = (0, 0);
        let mut gated_snapshots = Vec::new();
        let mut gated_revisions = Vec::new();
        for domain in decode_snapshot_body(&assembled_body)? {
            match (domain.domain_id, domain.snapshot_type) {
                (
                    world_spatial::STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                    world_spatial_entities::SNAPSHOT_TYPE_WORLD_SPATIAL_ENTITIES_V2,
                ) if entities_selected => {
                    let snapshot = if item_selected {
                        world_spatial_entities::decode_world_spatial_entities_snapshot_with_item_handles(
                            domain.payload,
                        )?
                    } else {
                        world_spatial_entities::decode_world_spatial_entities_snapshot(
                            domain.payload,
                        )?
                    };
                    world_spatial_observation = Some(WorldSpatialObservation {
                        content_generation: snapshot.content_generation,
                        actor_position: snapshot.actor_position,
                    });
                    entities = Some(WorldEntities::from_snapshot(&snapshot));
                    spatial_revision = domain.revision;
                }
                (
                    world_spatial::STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                    world_spatial::SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
                ) if !entities_selected => {
                    world_spatial_observation =
                        Some(world_spatial::decode_world_spatial(domain.payload)?);
                    spatial_revision = domain.revision;
                }
                (
                    world_object::STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
                    world_object::SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1,
                ) => {
                    world_object_overlay = Some(
                        world_object::decode_world_object_overlay_snapshot(domain.payload)?,
                    );
                    overlay_revision = domain.revision;
                }
                (
                    actor_spell::STATE_DOMAIN_ACTOR_VITALS,
                    actor_spell::SNAPSHOT_TYPE_ACTOR_VITALS_V1,
                ) => {
                    actor_vitals = Some(actor_spell::decode_actor_vitals(domain.payload)?);
                    vitals_revision = domain.revision;
                }
                (
                    attack::STATE_DOMAIN_ACTOR_COMBAT_STATE,
                    attack::SNAPSHOT_TYPE_ACTOR_COMBAT_STATE_V1,
                ) if attack_selected => {
                    combat_state = Some(
                        attack::decode_actor_combat_state(domain.payload)
                            .map_err(SessionError::Attack)?,
                    );
                    combat_revision = domain.revision;
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
                    inventory = Some(
                        item_view::decode_character_inventory_with_equip_drop(
                            domain.payload,
                            selected_capabilities
                                .contains(&item_view::CAPABILITY_ITEM_EQUIP_DROP_V1),
                        )
                        .map_err(SessionError::ItemView)?,
                    );
                    inventory_revision = domain.revision;
                }
                (STATE_DOMAIN_OPEN_CONTAINER, item_view::SNAPSHOT_TYPE_OPEN_CONTAINER_V1)
                    if item_selected =>
                {
                    open_container = Some(
                        item_view::decode_open_container(domain.payload)
                            .map_err(SessionError::ItemView)?,
                    );
                    container_revision = domain.revision;
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
                    chat = Some(guild_chat::load_snapshot(
                        snapshot_type,
                        domain.payload,
                        &selected_capabilities,
                    )?);
                    chat_revision = domain.revision;
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
                // A gated domain is kept raw for its capability's own packet. Unselected is a server
                // violation; an unregistered domain is ignored.
                (domain_id, snapshot_type) => match capability_of_domain(domain_id) {
                    Some(capability) if selected_capabilities.contains(&capability) => {
                        gated_revisions.push((domain_id, domain.revision));
                        gated_snapshots.push(GatedSnapshot {
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

        let appearance = appearance_state::from_snapshots(
            &selected_capabilities,
            &gated_snapshots,
            world_spatial_observation
                .as_ref()
                .ok_or(SessionError::MissingDomain(1))?
                .content_generation,
            appearance_items,
        )?;
        appearance_state::validate_entities(appearance.as_ref(), entities.as_ref())?;
        if attack_selected && combat_state.is_none() {
            return Err(SessionError::MissingDomain(
                attack::STATE_DOMAIN_ACTOR_COMBAT_STATE,
            ));
        }
        quest_log_transport::validate_join(&selected_capabilities, &gated_snapshots)?;
        tutorial_notices::validate_join(&selected_capabilities, &gated_snapshots)?;
        world_light::validate_join(&selected_capabilities, &gated_snapshots)?;
        presentation::validate_join(&selected_capabilities, &gated_snapshots)?;
        server_save_countdown::validate_join(&selected_capabilities, &gated_snapshots)?;
        achievement_notices_transport::validate_join(&selected_capabilities, &gated_snapshots)?;
        let events = server_messages::validate_join(
            &selected_capabilities,
            &gated_snapshots,
            begin.target_server_sequence,
        )?;
        let map_header = map_transport::validate_join(
            &selected_capabilities,
            &gated_snapshots,
            world_spatial_observation
                .as_ref()
                .ok_or(SessionError::MissingDomain(1))?
                .content_generation,
        )?;
        if map_header.is_some() {
            if world_object_overlay.is_some() {
                return Err(SessionError::UnselectedDomain { domain_id: 2 });
            }
            world_object_overlay = Some(Vec::new());
        }
        let snapshot = JoinSnapshot {
            game_session_id: accepted_fields.game_session_id,
            world_spatial: world_spatial_observation.ok_or(SessionError::MissingDomain(
                world_spatial::STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
            ))?,
            world_spatial_revision: spatial_revision,
            world_object_overlay: world_object_overlay.ok_or(SessionError::MissingDomain(
                world_object::STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
            ))?,
            world_object_overlay_revision: overlay_revision,
        };
        Ok(Self {
            stream,
            deadline,
            last_received: tokio::time::Instant::now(),
            connection_generation: session_generation,
            first_command_id: accepted_fields.next_command_id,
            next_command_id: accepted_fields.next_command_id,
            last_server_sequence: begin.target_server_sequence,
            spatial_revision,
            overlay_revision,
            vitals_revision,
            combat_revision,
            combat_state,
            actor_vitals,
            world_spatial: snapshot.world_spatial,
            world_object_overlay: snapshot.world_object_overlay.clone(),
            join_snapshot: snapshot,
            unusable: false,
            duplicates: Vec::new(),
            last_probe_id: 0,
            selected_capabilities,
            gated_snapshots,
            gated_revisions,
            events,
            entities,
            appearance,
            chat_revision,
            map_header,
            inventory_revision,
            container_revision,
            inventory,
            open_container,
            chat,
            vip_presence: None,
        })
    }
}
