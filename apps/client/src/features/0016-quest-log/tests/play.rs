use super::*;
use crate::combat_input::PlayCommand;
use oteryn_client_runtime::quest_log::{
    QuestListEntry, QuestLog, QuestLogView, QuestState, encode_quest_log,
};
use oteryn_session::GatedSnapshot;

#[test]
fn quest_replacement_resets_cached_list_and_revision_before_new_base_delta() -> Result<(), String> {
    let log = QuestLog {
        view: Some(QuestLogView::List(vec![QuestListEntry {
            quest_index: 1,
            completed: false,
        }])),
        tracked: vec![],
    };
    let snapshot = |revision, log: &QuestLog| -> Result<GatedSnapshot, String> {
        Ok(GatedSnapshot {
            domain_id: 16,
            snapshot_type: 1,
            revision,
            payload: encode_quest_log(log).map_err(|e| format!("{e:?}"))?,
        })
    };
    let (link, _commands, outbox) = play_channel();
    mailbox::lock(&outbox.0).quest.state =
        QuestState::from_snapshot(&[16], Some(&snapshot(1, &log)?))
            .map_err(|e| format!("{e:?}"))?;
    outbox.push(vec![
        SessionEvent::QuestLog(AppliedDelta {
            server_sequence: 2,
            base_revision: 1,
            new_revision: 2,
            value: log,
        }),
        SessionEvent::GatedSnapshotReplaced(AppliedDelta {
            server_sequence: 3,
            base_revision: 2,
            new_revision: 9,
            value: snapshot(9, &QuestLog::default())?,
        }),
        SessionEvent::QuestLog(AppliedDelta {
            server_sequence: 4,
            base_revision: 9,
            new_revision: 10,
            value: QuestLog::default(),
        }),
    ]);
    let panel = link.quest_panel();
    assert_eq!(panel.state.revision, Some(10));
    assert!(panel.state.list.is_empty());
    assert_eq!(panel.state.refusal, None);
    Ok(())
}

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
        Ok(expected)
    );
    assert!(link.quest_panel().state.log.tracked.is_empty());
    Ok(())
}
