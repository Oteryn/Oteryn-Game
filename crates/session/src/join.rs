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

        let assembled_body =
            snapshot_transfer::read_body(&mut stream, deadline, session_generation, &begin).await?;

        // Only now — every declared chunk arrived in order, the assembled length matched, and the
        // matching `SnapshotCommit` validated — is the assembled `SnapshotBody` decoded, exactly
        // once (FND-02 §16: "protobuf decode occurs only after a full bounded body is assembled" and
        // "apply is atomic only after all chunks and matching SnapshotCommit validate").
        let snapshot_state::SnapshotState {
            world_spatial_observation,
            entities,
            world_object_overlay,
            actor_vitals,
            combat_state,
            combat_revision,
            spatial_revision,
            overlay_revision,
            vitals_revision,
            chat,
            chat_revision,
            inventory,
            open_container,
            container_views,
            inventory_revision,
            container_revision,
            gated_snapshots,
            gated_revisions,
            appearance,
            map_header,
            events,
        } = snapshot_state::SnapshotState::decode(
            &assembled_body,
            begin.target_server_sequence,
            &selected_capabilities,
            appearance_items,
            None,
        )?;
        if let Some(row) = gated_snapshots
            .iter()
            .find(|row| row.domain_id == activity_scope::STATE_DOMAIN_ACTIVITY_SCOPE)
        {
            let receipt = activity_scope::decode_activity_scope(&row.payload)
                .map_err(|_| FoundationProtocolError::MalformedEnvelope)?;
            if receipt.requires_ack
                || receipt.session != *accepted_fields.game_session_id.as_bytes()
                || receipt.connection_generation != session_generation
            {
                return Err(FoundationProtocolError::MalformedEnvelope.into());
            }
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
            bulletin: None,
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
            last_refusal_detail: None,
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
            container_views,
            chat,
            vip_presence: None,
        })
    }
}
