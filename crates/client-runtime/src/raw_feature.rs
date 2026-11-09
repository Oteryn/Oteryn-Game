//! Bounded typed envelope used by native panels whose wire codecs live in `protocol-oteryn`.

pub use oteryn_session::property_services::house_rotation;
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
    let result = session
        .gated_command(command.capability, command.kind, &command.payload)
        .await?;
    // Older capability-199 servers accept only the empty self query. Never downgrade
    // a visibility mutation or a query for another character into a self request.
    if command.capability == 199
        && command.kind == 387
        && command.payload
            == [
                oteryn_session::cyclopedia_character::badge_visibility::PREFIX,
                0,
            ]
        && result.status == CommandStatus::Rejected
    {
        return session
            .gated_command(command.capability, command.kind, &[])
            .await;
    }
    Ok(result)
}
