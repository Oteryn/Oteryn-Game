//! Consume the existing shared owner's committed barrier, then the fresh destination.
//! Crystal 201189f data-global/scripts/quests/soulpit/soulpit_fight.lua:79-106
//! stages participants; data-global/scripts/actions/soulpit/soulpit_arena_exit.lua:8-16
//! returns through USE. Session authority stays with the shared server owner.
use super::*;
use activity_scope::{STATE_DOMAIN_ACTIVITY_SCOPE, decode_activity_scope};
use snapshot_state::SnapshotState;

fn invalid() -> SessionError {
    FoundationProtocolError::MalformedEnvelope.into()
}

pub(super) fn validate_projection(
    selected: &[u32],
    snapshots: &[GatedSnapshot],
) -> Result<(), SessionError> {
    if let Some(row) = snapshots
        .iter()
        .find(|row| row.domain_id == STATE_DOMAIN_ACTIVITY_SCOPE)
        && (row.snapshot_type != 1
            || decode_activity_scope(&row.payload)
                .map_err(|_| invalid())?
                .selection
                != selected)
    {
        return Err(invalid());
    }
    Ok(())
}

impl<S: SessionStream> Session<S> {
    /// A domain-99-only replacement is the source stream's final committed barrier.
    /// Decode the complete destination before changing any session identity or domain store.
    pub(super) async fn adopt_activity_snapshot(
        &mut self,
        body: &[u8],
    ) -> Result<bool, SessionError> {
        let domains = decode_snapshot_body(body)?;
        let Some(row) = domains
            .iter()
            .find(|row| row.domain_id == STATE_DOMAIN_ACTIVITY_SCOPE)
        else {
            return Ok(false);
        };
        if domains.len() != 1
            || row.snapshot_type != 1
            || !self
                .is_selected(oteryn_protocol_oteryn::boss_encounter::CAPABILITY_BOSS_ENCOUNTER_V1)
        {
            return Err(invalid());
        }
        let receipt = decode_activity_scope(row.payload).map_err(|_| invalid())?;
        if receipt.source_session != *self.join_snapshot.game_session_id.as_bytes()
            || receipt.selection != self.selected_capabilities
        {
            return Err(invalid());
        }
        let frame = bounded(
            self.deadline,
            "activity destination snapshot",
            read_frame(&mut self.stream),
        )
        .await?;
        let envelope = decode_wire_envelope(&frame)?;
        envelope.validate(Direction::ServerToClient, true)?;
        if envelope.message_type() != MessageType::SnapshotBegin
            || envelope.connection_generation() != receipt.connection_generation
        {
            return Err(invalid());
        }
        let begin = decode_snapshot_begin(envelope.payload())?;
        if begin.target_server_sequence != 0 {
            return Err(invalid());
        }
        let destination = snapshot_transfer::read_body(
            &mut self.stream,
            self.deadline,
            receipt.connection_generation,
            &begin,
        )
        .await?;
        let destination_domains = decode_snapshot_body(&destination)?;
        let projected = destination_domains
            .iter()
            .find(|domain| domain.domain_id == STATE_DOMAIN_ACTIVITY_SCOPE)
            .ok_or(SessionError::MissingDomain(STATE_DOMAIN_ACTIVITY_SCOPE))?;
        let mut projected_receipt = receipt.clone();
        projected_receipt.requires_ack = false;
        if projected.snapshot_type != 1
            || decode_activity_scope(projected.payload).map_err(|_| invalid())? != projected_receipt
        {
            return Err(invalid());
        }
        let old = SnapshotState::capture(self);
        let references = old
            .appearance
            .as_ref()
            .and_then(appearance_state::item_references);
        let items = references
            .as_deref()
            .map(|definition_refs| AppearanceItemDictionary {
                content_generation: self.world_spatial.content_generation,
                definition_refs,
            });
        // Fresh snapshots must not retain origin item handles, map windows, targets or revisions.
        let next = SnapshotState::decode(&destination, 0, &receipt.selection, items, None)?;
        let spatial = next
            .world_spatial_observation
            .ok_or(SessionError::MissingDomain(1))?;
        let overlay = next
            .world_object_overlay
            .clone()
            .ok_or(SessionError::MissingDomain(2))?;
        if spatial.content_generation != self.world_spatial.content_generation {
            return Err(SessionError::ContentGenerationMismatch { domain_id: 1 });
        }
        let events = replacement_snapshot::replacement_events(&destination, 0, &old, &next)?;
        if self.events.len().saturating_add(events.len()) > MAX_QUEUED_EVENTS {
            return Err(SessionError::EventQueueOverflow {
                limit: MAX_QUEUED_EVENTS,
            });
        }
        let session = GameSessionId::decode(&receipt.session)?;
        if receipt.requires_ack {
            self.acknowledge_activity(&receipt).await?;
        }
        self.join_snapshot = JoinSnapshot {
            game_session_id: session,
            world_spatial: spatial,
            world_spatial_revision: next.spatial_revision,
            world_object_overlay: overlay,
            world_object_overlay_revision: next.overlay_revision,
        };
        next.install(self);
        self.connection_generation = receipt.connection_generation;
        self.last_server_sequence = u64::from(receipt.requires_ack);
        self.next_command_id = 1 + u64::from(receipt.requires_ack);
        self.first_command_id = 1;
        self.last_probe_id = 0;
        self.duplicates.clear();
        self.last_refusal_detail = None;
        self.vip_presence = None;
        self.last_received = tokio::time::Instant::now();
        self.events.extend(events);
        Ok(true)
    }

    /// The barrier's full fresh snapshot is validated before acknowledging it. Until the
    /// correlated result arrives, no ordinary destination command can leave this owner.
    async fn acknowledge_activity(
        &mut self,
        receipt: &activity_scope::ActivityScopeSnapshot,
    ) -> Result<(), SessionError> {
        let payload =
            activity_scope::encode_activity_scope_ack(&activity_scope::ActivityScopeAck {
                handoff: receipt.handoff,
                session: receipt.session,
            })
            .map_err(|_| invalid())?;
        let command = encode_client_command(
            receipt.connection_generation,
            &ClientCommandValue {
                command_id: 1,
                command_type: activity_scope::COMMAND_TYPE_ACTIVITY_SCOPE_ACK,
                payload: &payload,
            },
        )?;
        bounded(
            self.deadline,
            "activity acknowledgement write",
            write_frame(&mut self.stream, &command),
        )
        .await?;
        let frame = bounded(
            self.deadline,
            "activity acknowledgement result",
            read_frame(&mut self.stream),
        )
        .await?;
        let envelope = decode_wire_envelope(&frame)?;
        envelope.validate(Direction::ServerToClient, true)?;
        if envelope.message_type() != MessageType::CommandResult
            || envelope.connection_generation() != receipt.connection_generation
            || envelope.server_sequence() != 1
        {
            return Err(invalid());
        }
        let result = decode_command_result(envelope.payload())?;
        if result.command_id != 1
            || result.status != CommandStatus::Accepted
            || !result.payload.is_empty()
        {
            return Err(invalid());
        }
        Ok(())
    }
}
