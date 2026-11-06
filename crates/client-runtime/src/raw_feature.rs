//! Bounded typed envelope used by native panels whose wire codecs live in `protocol-oteryn`.

pub use oteryn_session::property_services::house_wrap;
pub use oteryn_session::property_services::{staff, staff_tools};
pub use oteryn_session::{CommandStatus, GatedCommandResult};
use oteryn_session::{Session, SessionError, SessionStream};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeatureCommand {
    pub capability: u32,
    pub kind: u32,
    pub payload: Vec<u8>,
}

pub async fn send<S: SessionStream>(
    session: &mut Session<S>,
    command: &FeatureCommand,
) -> Result<GatedCommandResult, SessionError> {
    session
        .gated_command(command.capability, command.kind, &command.payload)
        .await
}
