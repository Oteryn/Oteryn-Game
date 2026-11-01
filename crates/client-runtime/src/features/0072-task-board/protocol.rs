//! Task Board commands and decoded server-owned views.
pub use oteryn_session::{CommandStatus, task_board};
use oteryn_session::{Session, SessionError, SessionStream};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskBoardCommand(pub task_board::TaskBoardRequest);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskBoardResponse {
    pub status: CommandStatus,
    pub reply: task_board::TaskBoardReply,
}

pub async fn send<S: SessionStream>(
    session: &mut Session<S>,
    command: &TaskBoardCommand,
) -> Result<TaskBoardResponse, SessionError> {
    let payload = task_board::encode_task_board_request(&command.0).map_err(|_| {
        SessionError::CapabilityNotSelected {
            capability: task_board::CAPABILITY_TASK_BOARD_V1,
        }
    })?;
    let result = session
        .gated_command(
            task_board::CAPABILITY_TASK_BOARD_V1,
            task_board::COMMAND_TYPE_TASK_BOARD,
            &payload,
        )
        .await?;
    let reply = task_board::decode_task_board_reply(&result.payload)
        .map_err(|_| SessionError::SessionUnusable)?;
    Ok(TaskBoardResponse {
        status: result.status,
        reply,
    })
}
