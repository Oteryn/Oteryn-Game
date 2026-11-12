use super::*;
use oteryn_session::{GatedCommandResult, Session, SessionError, SessionStream};

impl QuestState {
    /// Constructs command 22's payload only for the selected capability and visible quests.
    pub fn command(&self, query: &QuestLogQuery) -> Result<Vec<u8>, QuestRefusal> {
        self.require_selected()?;
        let known = match query {
            QuestLogQuery::List | QuestLogQuery::ListOnly => true,
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
    let list_only = matches!(decode_quest_log_query(payload), Ok(QuestLogQuery::ListOnly));
    let valid = if list_only && result.status == CommandStatus::Accepted {
        matches!(decode_quest_log(&result.payload), Ok(QuestLog { view: Some(QuestLogView::List(_)), tracked }) if tracked.is_empty())
    } else {
        result.payload.is_empty()
    };
    if !valid {
        return Err(SessionError::InconsistentCommandResult {
            command_id: result.command_id,
            status: result.status,
        });
    }
    Ok(result)
}
