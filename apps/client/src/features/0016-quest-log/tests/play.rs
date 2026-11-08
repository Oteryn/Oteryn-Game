use super::*;
use crate::combat_input::PlayCommand;
use oteryn_client_runtime::quest_log::{
    QuestListEntry, QuestLog, QuestLogView, QuestState, encode_quest_log,
};
use oteryn_session::GatedSnapshot;

#[path = "list_options.rs"]
mod list_options;

#[path = "snapshot.rs"]
mod snapshot;

#[test]
fn tracker_read_only_reply_preserves_the_current_quest_and_revision() -> Result<(), String> {
    use oteryn_client_runtime::quest_log::{QuestLine, QuestLogQuery};
    let (link, mut commands, outbox) = play_channel();
    let visible = QuestLog {
        view: Some(QuestLogView::Line(QuestLine {
            quest_index: 1,
            completed: false,
            missions: vec![],
        })),
        tracked: vec![],
    };
    {
        let mut m = mailbox::lock(&outbox.0);
        m.quest.state.selected = true;
        m.quest.state.revision = Some(42);
        m.quest.state.log = visible.clone();
        m.quest.tracker.auto_add = true;
        m.quest.tracker.observe(&[QuestListEntry {
            quest_index: 1,
            completed: false,
        }]);
    }
    link.quest_request(&QuestLogQuery::ListOnly);
    assert!(matches!(commands.try_recv(), Ok(PlayCommand::QuestLog(_))));
    let list = vec![
        QuestListEntry {
            quest_index: 1,
            completed: false,
        },
        QuestListEntry {
            quest_index: 2,
            completed: false,
        },
    ];
    outbox.quest_result(&oteryn_session::GatedCommandResult {
        command_id: 1,
        status: oteryn_client_runtime::CommandStatus::Accepted,
        server_sequence: 1,
        payload: encode_quest_log(&QuestLog {
            view: Some(QuestLogView::List(list.clone())),
            tracked: vec![],
        })
        .map_err(|e| format!("{e:?}"))?,
    });
    let panel = link.quest_panel();
    assert_eq!(panel.state.list, list);
    assert_eq!(panel.state.log, visible);
    assert_eq!(panel.state.revision, Some(42));
    assert!(!mailbox::lock(&outbox.0).quest_pending);
    let query = oteryn_client_ui::quest_log::tracker_query(
        &mut mailbox::lock(&outbox.0).quest,
        std::time::Instant::now(),
    )
    .ok_or("Expected automatic tracking")?;
    let expected = QuestLogQuery::Track {
        quest_indexes: vec![2],
    };
    assert_eq!(query, expected);
    link.quest_request(&query);
    let Ok(PlayCommand::QuestLog(payload)) = commands.try_recv() else {
        return Err("Expected real quest command".into());
    };
    assert_eq!(
        oteryn_client_runtime::quest_log::decode_quest_log_query(&payload),
        Ok(expected.clone())
    );
    assert!(link.quest_panel().state.log.tracked.is_empty());
    outbox.quest_result(&oteryn_session::GatedCommandResult {
        command_id: 2,
        status: oteryn_client_runtime::CommandStatus::Rejected,
        server_sequence: 2,
        payload: vec![],
    });
    let retry_at = std::time::Instant::now() + std::time::Duration::from_secs(2);
    let poll =
        oteryn_client_ui::quest_log::tracker_query(&mut mailbox::lock(&outbox.0).quest, retry_at)
            .ok_or("Expected refresh")?;
    assert_eq!(poll, QuestLogQuery::ListOnly);
    link.quest_request(&poll);
    assert!(matches!(commands.try_recv(), Ok(PlayCommand::QuestLog(_))));
    outbox.quest_result(&oteryn_session::GatedCommandResult {
        command_id: 3,
        status: oteryn_client_runtime::CommandStatus::Accepted,
        server_sequence: 3,
        payload: encode_quest_log(&QuestLog {
            view: Some(QuestLogView::List(list)),
            tracked: vec![],
        })
        .map_err(|e| format!("{e:?}"))?,
    });
    let retry = oteryn_client_ui::quest_log::tracker_query(
        &mut mailbox::lock(&outbox.0).quest,
        retry_at + std::time::Duration::from_secs(1),
    )
    .ok_or("Expected retry after refresh")?;
    assert_eq!(retry, expected);
    link.quest_request(&retry);
    let Ok(PlayCommand::QuestLog(payload)) = commands.try_recv() else {
        return Err("Expected retried command".into());
    };
    assert_eq!(
        oteryn_client_runtime::quest_log::decode_quest_log_query(&payload),
        Ok(expected)
    );
    assert!(link.quest_panel().state.log.tracked.is_empty());
    outbox.quest_result(&oteryn_session::GatedCommandResult {
        command_id: 4,
        status: oteryn_client_runtime::CommandStatus::Accepted,
        server_sequence: 4,
        payload: vec![],
    });
    // Both authoritative receipts can arrive between native UI iterations.
    for (base, tracked) in [
        (
            42,
            vec![QuestLine {
                quest_index: 2,
                completed: false,
                missions: vec![],
            }],
        ),
        (43, vec![]),
    ] {
        outbox.push(vec![SessionEvent::QuestLog(AppliedDelta {
            server_sequence: base + 1,
            base_revision: base,
            new_revision: base + 1,
            value: QuestLog {
                view: visible.view.clone(),
                tracked,
            },
        })]);
    }
    link.quest_request(&QuestLogQuery::ListOnly);
    assert!(matches!(commands.try_recv(), Ok(PlayCommand::QuestLog(_))));
    outbox.quest_result(&oteryn_session::GatedCommandResult {
        command_id: 5,
        status: oteryn_client_runtime::CommandStatus::Accepted,
        server_sequence: 7,
        payload: encode_quest_log(&QuestLog {
            view: Some(QuestLogView::List(
                (1..=3)
                    .map(|quest_index| QuestListEntry {
                        quest_index,
                        completed: false,
                    })
                    .collect(),
            )),
            tracked: vec![],
        })
        .map_err(|e| format!("{e:?}"))?,
    });
    assert_eq!(
        oteryn_client_ui::quest_log::tracker_query(
            &mut mailbox::lock(&outbox.0).quest,
            retry_at + std::time::Duration::from_secs(2)
        ),
        Some(QuestLogQuery::Track {
            quest_indexes: vec![3]
        })
    );
    Ok(())
}
