//! VIP commands sent through the selected admitted session.
pub use oteryn_session::{CommandStatus, vip};
use oteryn_session::{Session, SessionError, SessionStream};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VipCommand {
    pub kind: u32,
    pub payload: Vec<u8>,
}

impl VipCommand {
    pub fn new(request: &vip::Request) -> Option<Self> {
        let (kind, payload) = vip::encode_request(request).ok()?;
        Some(Self { kind, payload })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VipResponse {
    pub status: CommandStatus,
    pub result: vip::ResultView,
}

pub async fn send<S: SessionStream>(
    session: &mut Session<S>,
    command: &VipCommand,
) -> Result<VipResponse, SessionError> {
    vip::decode_request(command.kind, &command.payload).map_err(|_| {
        SessionError::CapabilityNotSelected {
            capability: vip::CAPABILITY_VIP_V1,
        }
    })?;
    let result = session
        .gated_command(vip::CAPABILITY_VIP_V1, command.kind, &command.payload)
        .await?;
    let decoded =
        vip::decode_result(&result.payload).map_err(|_| SessionError::CapabilityNotSelected {
            capability: vip::CAPABILITY_VIP_V1,
        })?;
    Ok(VipResponse {
        status: result.status,
        result: decoded,
    })
}
