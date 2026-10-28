use super::*;
use oteryn_session::{GatedCommandResult, Session, SessionError, SessionStream};

impl QuestState {
    /// Constructs command 22's payload only for the selected capability and visible quests.
    pub fn command(&self, query: &QuestLogQuery) -> Result<Vec<u8>, QuestRefusal> {
        self.require_selected()?;
        let known = match query {
            QuestLogQuery::List => true,
            QuestLogQuery::Quest { quest_index } => self.known(*quest_index),
            QuestLogQuery::Track { quest_indexes } => quest_indexes.iter().all(|q| self.known(*q)),
        };
        if !known {
            return Err(QuestRefusal::UnknownQuest);
        }
        encode_quest_log_query(query).map_err(super::state::wire_refusal)
    }
}

/// The session rechecks selection and ownership before sending command 22.
pub async fn send<S: SessionStream>(
    session: &mut Session<S>,
    payload: &[u8],
) -> Result<GatedCommandResult, SessionError> {
    let result = session
        .gated_command(
            CAPABILITY_QUEST_LOG_V1,
            COMMAND_TYPE_QUEST_LOG_QUERY,
            payload,
        )
        .await?;
    if !result.payload.is_empty() {
        return Err(SessionError::InconsistentCommandResult {
            command_id: result.command_id,
            status: result.status,
        });
    }
    Ok(result)
}
