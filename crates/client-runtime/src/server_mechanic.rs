//! Shared command/result vocabulary for server-owned native mechanic panels.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerMechanicCommand {
    pub capability: u32,
    pub command_type: u32,
    pub payload: Vec<u8>,
}

pub use oteryn_session::CommandStatus;

pub type ServerMechanicResponse = oteryn_session::GatedCommandResult;
