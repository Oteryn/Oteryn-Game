use super::*;
use oteryn_client_runtime::quest_log::{
    QuestListEntry, QuestLog, QuestLogView, QuestState, encode_quest_log,
};
use oteryn_session::GatedSnapshot;

#[test]
fn quest_replacement_resets_cached_list_and_revision_before_new_base_delta() -> Result<(), String> {
    let log = QuestLog {
        map_marks: Vec::new(),
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
