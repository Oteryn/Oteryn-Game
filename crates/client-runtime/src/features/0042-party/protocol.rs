//! Party commands and server-confirmed party views.
pub use oteryn_session::CommandStatus;
pub use oteryn_session::EntityRef;
pub use oteryn_session::party;
use oteryn_session::{
    COMMAND_TYPE_CHAT_INTENT, GatedCommandResult, Session, SessionError, SessionStream,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartyCommand {
    pub kind: u32,
    pub payload: Vec<u8>,
}

impl PartyCommand {
    /// The server exposes shared XP through CHAT_V1, not a new party wire intent.
    pub fn shared_experience(enabled: bool) -> Result<Self, oteryn_session::ChatWireError> {
        Ok(Self {
            kind: COMMAND_TYPE_CHAT_INTENT,
            payload: oteryn_session::encode_chat_intent(&shared_intent(enabled))?,
        })
    }

    pub fn shared_setting(&self) -> Option<bool> {
        [true, false]
            .into_iter()
            .find(|enabled| Self::shared_experience(*enabled).is_ok_and(|command| command == *self))
    }

    #[must_use]
    pub fn query() -> Self {
        Self {
            kind: party::COMMAND_TYPE_PARTY_QUERY,
            payload: Vec::new(),
        }
    }

    pub fn intent(intent: party::PartyIntent) -> Result<Self, party::PartyWireError> {
        Ok(Self {
            kind: party::COMMAND_TYPE_PARTY_INTENT,
            payload: party::encode_party_intent(intent)?,
        })
    }

    fn valid(&self) -> bool {
        match self.kind {
            party::COMMAND_TYPE_PARTY_QUERY => self.payload.is_empty(),
            party::COMMAND_TYPE_PARTY_INTENT => party::decode_party_intent(&self.payload).is_ok(),
            _ => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartyResponse {
    pub status: CommandStatus,
    pub operation: Vec<u8>,
    pub view: Option<Vec<u8>>,
}

fn accepted(result: GatedCommandResult) -> Option<Vec<u8>> {
    matches!(
        result.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    )
    .then_some(result.payload)
}

pub async fn send<S: SessionStream>(
    session: &mut Session<S>,
    request: &PartyCommand,
) -> Result<PartyResponse, SessionError> {
    if !request.valid() {
        return Err(SessionError::CapabilityNotSelected {
            capability: party::CAPABILITY_PARTY_V1,
        });
    }
    let result = session
        .gated_command(party::CAPABILITY_PARTY_V1, request.kind, &request.payload)
        .await?;
    let operation = result.payload.clone();
    let status = result.status;
    let view = if request.kind == party::COMMAND_TYPE_PARTY_QUERY {
        accepted(result)
    } else {
        accepted(
            session
                .gated_command(
                    party::CAPABILITY_PARTY_V1,
                    party::COMMAND_TYPE_PARTY_QUERY,
                    &[],
                )
                .await?,
        )
    };
    Ok(PartyResponse {
        status,
        operation,
        view,
    })
}

#[must_use]
pub fn shared_intent(enabled: bool) -> oteryn_session::ChatIntent {
    oteryn_session::ChatIntent::Say {
        mode: oteryn_session::ChatSpeechMode::Say,
        text: if enabled {
            "/sharedxp on"
        } else {
            "/sharedxp off"
        }
        .into(),
    }
}

pub fn shared_response(
    outcome: &oteryn_session::ChatOutcome,
    view: Option<Vec<u8>>,
) -> PartyResponse {
    use oteryn_session::ChatDisposition;
    let disposition = match outcome.disposition {
        ChatDisposition::Ok => party::PartyDisposition::Ok,
        ChatDisposition::NotAllowed => party::PartyDisposition::NotLeader,
        ChatDisposition::ChatUnavailable => party::PartyDisposition::Unavailable,
        _ => party::PartyDisposition::Rejected,
    };
    PartyResponse {
        status: outcome.status,
        operation: party::encode_party_result(&party::PartyResult::refused(disposition))
            .unwrap_or_default(),
        view,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_commands_are_strict_existing_chat_payloads()
    -> Result<(), oteryn_session::ChatWireError> {
        for enabled in [true, false] {
            let mut command = PartyCommand::shared_experience(enabled)?;
            assert_eq!(command.kind, COMMAND_TYPE_CHAT_INTENT);
            assert_eq!(command.shared_setting(), Some(enabled));
            command.payload.push(0);
            assert_eq!(command.shared_setting(), None);
        }
        assert_eq!(PartyCommand::query().shared_setting(), None);
        Ok(())
    }
}
