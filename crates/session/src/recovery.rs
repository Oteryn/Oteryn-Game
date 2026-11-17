//! Client continuity and recovery over a replacement transport.
use super::*;

/// CAP-NEG-1: a resume keeps the original selected set and never widens it. The client calls
/// this with the set it holds and the set a `ServerResumeAccepted` carries; a mismatch means the
/// resume must be abandoned for a fresh admission.
pub fn check_resume_selection(original: &[u32], resumed: &[u32]) -> Result<(), SessionError> {
    let same = original.len() == resumed.len() && resumed.iter().all(|id| original.contains(id));
    if same {
        Ok(())
    } else {
        Err(SessionError::ResumeSelectionChanged)
    }
}

/// Retained client continuity after closing a transport. This grants no server authority.
#[derive(Debug, Clone)]
pub struct RecoveryState {
    pub(super) game_session_id: GameSessionId,
    pub(super) world_id: [u8; 16],
    pub(super) activity_scope: Option<activity_scope::ActivityScopeSnapshot>,
    pub(super) connection_generation: u64,
    pub(super) next_command_id: u64,
    pub(super) last_server_sequence: u64,
    pub(super) selected_capabilities: Vec<u32>,
}

#[derive(Debug, Clone, Copy)]
pub struct Recovery<'a> {
    pub reconnect_material: &'a [u8],
    pub schema_revision: u32,
    pub client_build_id: &'a str,
    pub deadline: Duration,
    pub appearance_items: Option<AppearanceItemDictionary<'a>>,
}

impl<S: SessionStream> Session<S> {
    /// Close between completed commands, retaining the continuity the server must prove on recovery.
    pub fn disconnect(self) -> RecoveryState {
        RecoveryState {
            game_session_id: self.join_snapshot.game_session_id,
            world_id: self.world_id,
            activity_scope: self.activity_scope,
            connection_generation: self.connection_generation,
            next_command_id: self.next_command_id,
            last_server_sequence: self.last_server_sequence,
            selected_capabilities: self.selected_capabilities,
        }
    }

    /// Write a real step and close without reading its outcome. Recovery's next-command
    /// lower bound remains this unacknowledged id; the server snapshot decides whether it ran.
    pub async fn disconnect_after_step(
        mut self,
        direction: StepDirection,
    ) -> Result<RecoveryState, SessionError> {
        self.ensure_usable()?;
        if direction.is_diagonal()
            && !self.is_selected(world_spatial::CAPABILITY_DIAGONAL_MOVEMENT_V1)
        {
            return Err(SessionError::CapabilityNotSelected {
                capability: world_spatial::CAPABILITY_DIAGONAL_MOVEMENT_V1,
            });
        }
        self.next_command_id
            .checked_add(1)
            .ok_or(SessionError::CommandIdExhausted)?;
        let frame = self.encode_current_command(&ClientCommandValue {
            command_id: self.next_command_id,
            command_type: world_spatial::COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
            payload: &world_spatial::encode_step_intent(direction),
        })?;
        bounded(
            self.deadline,
            "pending step write",
            write_frame(&mut self.stream, &frame),
        )
        .await?;
        Ok(self.disconnect())
    }

    /// Recover the same logical session over a new transport using an existing server credential.
    pub async fn resume(
        mut stream: S,
        previous: RecoveryState,
        recovery: Recovery<'_>,
    ) -> Result<Self, SessionError> {
        use oteryn_protocol_oteryn::recovery::{
            ClientResumeValue, decode_server_resume_accepted, encode_client_resume,
        };
        let request = encode_client_resume(&ClientResumeValue {
            game_session_id: previous.game_session_id,
            reconnect_material: recovery.reconnect_material,
            last_applied_server_sequence: previous.last_server_sequence,
            schema_revision: recovery.schema_revision,
            client_build_id: recovery.client_build_id,
            supported_capabilities: &previous.selected_capabilities,
        })?;
        bounded(
            recovery.deadline,
            "ClientResume",
            write_frame(&mut stream, &request),
        )
        .await?;
        let frame = bounded(
            recovery.deadline,
            "ServerResumeAccepted",
            read_frame(&mut stream),
        )
        .await?;
        let envelope = decode_wire_envelope(&frame)?;
        envelope.validate(Direction::ServerToClient, false)?;
        // A resume the server took but cannot join (`SESSION_JOIN_UNAVAILABLE`) ends with its
        // `ProtocolError`: keep its code and trace.
        if let Some(error) =
            session_exchange::terminal_protocol_error(&envelope, &previous.selected_capabilities)?
        {
            return Err(error);
        }
        if envelope.message_type() != MessageType::ServerResumeAccepted {
            return Err(SessionError::UnexpectedMessage {
                expected: MessageType::ServerResumeAccepted,
                actual: envelope.message_type(),
            });
        }
        let accepted = decode_server_resume_accepted(envelope.payload())?;
        check_resume_selection(
            &previous.selected_capabilities,
            &accepted.selected_capabilities,
        )?;
        let successor = accepted.game_session_id != previous.game_session_id;
        if accepted.schema_revision != recovery.schema_revision
            || if successor {
                !previous.selected_capabilities.contains(&117)
                    || accepted.connection_generation != 1
                    || accepted.current_server_sequence != 1
                    || accepted.next_command_id != 1
            } else {
                accepted.connection_generation <= previous.connection_generation
                    || accepted.current_server_sequence < previous.last_server_sequence
                    || accepted.next_command_id < previous.next_command_id
            }
        {
            return Err(FoundationProtocolError::MalformedEnvelope.into());
        }
        // Decode a private temporary session. A changed UUID is never exposed before
        // its complete first snapshot proves the exact committed source lineage.
        let resumed = Self::read_join(
            stream,
            recovery.deadline,
            recovery.appearance_items,
            JoinAcceptance {
                game_session_id: accepted.game_session_id,
                world_id: previous.world_id,
                activity_scope: if successor {
                    None
                } else {
                    previous.activity_scope
                },
                connection_generation: accepted.connection_generation,
                next_command_id: accepted.next_command_id,
                selected_capabilities: accepted.selected_capabilities,
                recovery_sequence: Some(accepted.current_server_sequence),
            },
        )
        .await?;
        if successor
            && !resumed.activity_scope.as_ref().is_some_and(|scope| {
                scope.source_session == *previous.game_session_id.as_bytes()
                    && scope.session == *accepted.game_session_id.as_bytes()
            })
        {
            return Err(FoundationProtocolError::MalformedEnvelope.into());
        }
        Ok(resumed)
    }
}
