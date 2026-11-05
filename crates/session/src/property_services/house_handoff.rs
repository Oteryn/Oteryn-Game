//! Adopt only a server-committed HOUSE-TRANSFER-1 destination and its first snapshot.
use super::house_transfer as wire;
use crate::*;

impl<S: SessionStream> Session<S> {
    pub(crate) async fn house_handoff(
        &mut self,
        request: &[u8],
        result: &GatedCommandResult,
    ) -> Result<(), SessionError> {
        if !matches!(
            result.status,
            CommandStatus::Accepted | CommandStatus::DuplicateReplay
        ) {
            return Ok(());
        }
        let request = wire::decode_house_transfer_intent(request)
            .map_err(|_| SessionError::SessionUnusable)?;
        let ack = wire::decode_house_transfer_ack(&result.payload)
            .map_err(|_| SessionError::SessionUnusable)?;
        if ack.selection != self.selected_capabilities
            || ack.house.as_deref()
                != if request.exit {
                    None
                } else {
                    Some(request.house.as_str())
                }
        {
            return Err(SessionError::SessionUnusable);
        }
        // The committed destination has its own command and sequence namespace.
        // Never retry the source command against it; a failed snapshot poisons the session.
        self.connection_generation = ack.connection_generation;
        self.last_server_sequence = 0;
        self.next_command_id = 1;
        self.first_command_id = 1;
        self.last_probe_id = 0;
        self.duplicates.clear();
        self.join_snapshot.game_session_id = GameSessionId::decode(&ack.session)?;
        self.gated_snapshots
            .retain(|domain| domain.domain_id != wire::STATE_DOMAIN_HOUSE_SCOPE);
        self.gated_revisions
            .retain(|(domain, _)| *domain != wire::STATE_DOMAIN_HOUSE_SCOPE);
        let frame = bounded(
            self.deadline,
            "house handoff snapshot",
            read_frame(&mut self.stream),
        )
        .await?;
        let envelope = decode_wire_envelope(&frame)?;
        envelope.validate(Direction::ServerToClient, true)?;
        self.check_generation(&envelope)?;
        if envelope.message_type() != MessageType::SnapshotBegin {
            return Err(SessionError::UnexpectedMessage {
                expected: MessageType::SnapshotBegin,
                actual: envelope.message_type(),
            });
        }
        // House serve emits sequence 1; Channel setup emits its FRESH baseline 0.
        self.replace_snapshot_at(envelope.payload(), if request.exit { 0 } else { 1 })
            .await?;
        if !request.exit {
            let snapshot = self
                .gated_snapshot(wire::STATE_DOMAIN_HOUSE_SCOPE)
                .ok_or(SessionError::MissingDomain(wire::STATE_DOMAIN_HOUSE_SCOPE))?;
            let scope = wire::decode_house_transfer_ack(&snapshot.payload)
                .map_err(|_| SessionError::SessionUnusable)?;
            if scope != ack {
                return Err(SessionError::SessionUnusable);
            }
        }
        self.last_received = tokio::time::Instant::now();
        Ok(())
    }
}
