//! Commands and confirmed views share the session's capability, ordering and replay checks.
pub use oteryn_session::{CommandStatus, atelier, wheel};
use oteryn_session::{GatedCommandResult, Session, SessionError, SessionStream};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WheelCommand {
    pub kind: u32,
    pub payload: Vec<u8>,
    pub mutation: bool,
}

impl WheelCommand {
    /// Keep raw command construction inside the four registered feature codecs.
    pub fn new(kind: u32, payload: Vec<u8>) -> Option<Self> {
        let mutation = match kind {
            wheel::COMMAND_TYPE_WHEEL_ALLOCATION => {
                wheel::decode_wheel_request(&payload).ok()?.action != 0
            }
            atelier::COMMAND_TYPE_ATELIER_QUERY => {
                atelier::decode_query(&payload).ok()?;
                false
            }
            atelier::COMMAND_TYPE_ATELIER_SET_LOCK => {
                atelier::decode_set_lock(&payload).ok()?;
                true
            }
            atelier::COMMAND_TYPE_ATELIER_MUTATE => {
                atelier::decode_mutate(&payload).ok()?;
                true
            }
            _ => return None,
        };
        Some(Self {
            kind,
            payload,
            mutation,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WheelResponse {
    pub kind: u32,
    pub status: CommandStatus,
    /// The operation's receipt answer may describe historical state.
    pub payload: Vec<u8>,
    /// Current queries issued after the operation, including after receipt replay.
    pub wheel: Option<Vec<u8>>,
    pub atelier: Option<Vec<u8>>,
}

fn accepted_payload(result: GatedCommandResult) -> Option<Vec<u8>> {
    matches!(
        result.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    )
    .then_some(result.payload)
}

pub async fn send<S: SessionStream>(
    session: &mut Session<S>,
    request: &WheelCommand,
) -> Result<WheelResponse, SessionError> {
    let checked = WheelCommand::new(request.kind, request.payload.clone());
    if checked.as_ref() != Some(request) {
        return Err(SessionError::CapabilityNotSelected {
            capability: wheel::CAPABILITY_WHEEL_ALLOCATION_V1,
        });
    }
    let result = session
        .gated_command(
            wheel::CAPABILITY_WHEEL_ALLOCATION_V1,
            request.kind,
            &request.payload,
        )
        .await?;
    let wheel = session
        .gated_command(
            wheel::CAPABILITY_WHEEL_ALLOCATION_V1,
            wheel::COMMAND_TYPE_WHEEL_ALLOCATION,
            &[8, 0],
        )
        .await?;
    let atelier = session
        .gated_command(
            wheel::CAPABILITY_WHEEL_ALLOCATION_V1,
            atelier::COMMAND_TYPE_ATELIER_QUERY,
            &[],
        )
        .await?;
    Ok(WheelResponse {
        kind: request.kind,
        status: result.status,
        payload: result.payload,
        wheel: accepted_payload(wheel),
        atelier: accepted_payload(atelier),
    })
}
