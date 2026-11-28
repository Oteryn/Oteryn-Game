use super::*;
use oteryn_session::{GatedCommandResult, Session, SessionError, SessionStream};

impl QuestState {
    /// Constructs command 22's payload only for the selected capability and visible quests.
    pub fn command(&self, query: &QuestLogQuery) -> Result<Vec<u8>, QuestRefusal> {
        self.require_selected()?;
        let known = match query {
            QuestLogQuery::List | QuestLogQuery::ListOnly => true,
            QuestLogQuery::Quest { quest_index } => self.known(*quest_index),
            QuestLogQuery::Track { .. } if self.mission_tracking => {
                return Err(QuestRefusal::Malformed);
            }
            QuestLogQuery::Track { quest_indexes } => quest_indexes.iter().all(|q| self.known(*q)),
        };
        if !known {
            return Err(QuestRefusal::UnknownQuest);
        }
        encode_quest_log_query(query).map_err(super::state::wire_refusal)
    }
}

/// The session rechecks negotiation and command ownership before sending either tracker command.
pub async fn send<S: SessionStream>(
    session: &mut Session<S>,
    payload: &[u8],
) -> Result<GatedCommandResult, SessionError> {
    // The shell's quest command channel carries canonical payloads. A command-22 query
    // never encodes as a mission replacement: its list has an empty field-1 body, its
    // other cases use different tags, and it cannot have an empty payload.
    let (capability, command) = if decode_quest_mission_track(payload).is_ok() {
        (
            CAPABILITY_QUEST_MISSION_TRACKING_V1,
            COMMAND_TYPE_QUEST_MISSION_TRACK,
        )
    } else {
        (CAPABILITY_QUEST_LOG_V1, COMMAND_TYPE_QUEST_LOG_QUERY)
    };
    let result = session.gated_command(capability, command, payload).await?;
    let list_only = matches!(decode_quest_log_query(payload), Ok(QuestLogQuery::ListOnly));
    let valid = if list_only && result.status == CommandStatus::Accepted {
        valid_list_only(&result.payload)
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

fn valid_list_only(payload: &[u8]) -> bool {
    matches!(decode_quest_log(payload), Ok(QuestLog { map_marks: _, view: Some(QuestLogView::List(_)), tracked }) if tracked.is_empty())
}

#[cfg(test)]
#[test]
fn review10_list_only_accepts_personal_map_marks() -> Result<(), String> {
    let payload = encode_quest_log(&QuestLog {
        map_marks: vec![QuestMapMark {
            x: 100,
            y: 100,
            floor: 7,
            text: "Treasure".into(),
        }],
        view: Some(QuestLogView::List(vec![])),
        tracked: vec![],
    })
    .map_err(|error| format!("{error:?}"))?;
    assert!(valid_list_only(&payload));
    Ok(())
}
