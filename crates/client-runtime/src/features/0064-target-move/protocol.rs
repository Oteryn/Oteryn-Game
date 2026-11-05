//! Typed capability-64 walk request and server receipt. Positions remain server-owned.
pub use oteryn_session::{ActorPosition, CommandStatus};
use oteryn_session::{GatedCommandResult, Session, SessionError, SessionStream};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WalkDisposition {
    Accepted,
    NoPath,
    Rejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WalkResponse {
    pub status: CommandStatus,
    pub disposition: Option<WalkDisposition>,
}

pub fn project(reply: &GatedCommandResult) -> Result<WalkResponse, SessionError> {
    let disposition = match reply.payload.as_slice() {
        [8, 1] => Some(WalkDisposition::Accepted),
        [8, 2] => Some(WalkDisposition::NoPath),
        [8, 3] => Some(WalkDisposition::Rejected),
        [] if !matches!(
            reply.status,
            CommandStatus::Accepted | CommandStatus::DuplicateReplay
        ) =>
        {
            None
        }
        _ => return Err(SessionError::SessionUnusable),
    };
    if matches!(
        reply.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    ) && !matches!(
        disposition,
        Some(WalkDisposition::Accepted | WalkDisposition::NoPath)
    ) {
        return Err(SessionError::SessionUnusable);
    }
    Ok(WalkResponse {
        status: reply.status,
        disposition,
    })
}

pub async fn send<S: SessionStream>(
    session: &mut Session<S>,
    position: ActorPosition,
) -> Result<WalkResponse, SessionError> {
    project(&session.target_walk(position).await?)
}

#[cfg(test)]
mod tests;
