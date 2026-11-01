//! Party commands and server-confirmed party views.
pub use oteryn_session::CommandStatus;
pub use oteryn_session::EntityRef;
pub use oteryn_session::party;
use oteryn_session::{GatedCommandResult, Session, SessionError, SessionStream};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartyCommand {
    pub kind: u32,
    pub payload: Vec<u8>,
}

impl PartyCommand {
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
