use super::*;
use oteryn_client_runtime::quest_log::{
    QuestLine, QuestLogQuery, QuestMission, decode_quest_log_query,
};
use oteryn_client_ui::quest_log::{TrackerFilter, tracker_query, tracker_text};

#[test]
fn tracker_hides_completed_missions_and_removes_quests_only_after_server_confirmation()
-> Result<(), String> {
    let (link, mut commands, outbox) = play_channel();
    outbox.quest_initialize(QuestState {
        selected: true,
        revision: Some(0),
        ..Default::default()
    });
    let mission = |index, done| QuestMission {
        mission_index: index,
        name: format!("Stage {index}"),
        text: format!("Objective {index}"),
        done,
    };
    let active = QuestLine {
        quest_index: 1,
        completed: false,
        missions: vec![mission(1, true), mission(2, false)],
    };
    let completed = QuestLine {
        quest_index: 2,
        completed: true,
        missions: vec![],
    };
    link.quest_request(&QuestLogQuery::List);
    assert!(matches!(commands.try_recv(), Ok(PlayCommand::QuestLog(_))));
    outbox.quest_result(&oteryn_session::GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: oteryn_client_runtime::CommandStatus::Accepted,
        payload: vec![],
    });
    outbox.push(vec![oteryn_session::SessionEvent::QuestLog(
        oteryn_session::AppliedDelta {
            server_sequence: 2,
            base_revision: 0,
            new_revision: 1,
            value: QuestLog { map_marks: Vec::new(), view: Some(QuestLogView::List(vec![])),
                tracked: vec![active.clone(), completed],
            },
        },
    )]);
    let query = {
        let mut m = mailbox::lock(&outbox.0);
        m.quest.tracker.auto_remove = true;
        m.quest.tracker.filter = TrackerFilter::Active;
        m.quest.tracker.hide_completed_missions = true;
        let text = tracker_text(&m.quest);
        assert!(!text.contains("Stage 1"));
        assert!(text.contains("Stage 2: Objective 2"));
        assert!(!text.contains("Quest 2"));
        tracker_query(&mut m.quest, std::time::Instant::now()).ok_or("auto-remove")?
    };
    link.quest_request(&query);
    let Ok(PlayCommand::QuestLog(bytes)) = commands.try_recv() else {
        return Err("tracking command".into());
    };
    assert_eq!(
        decode_quest_log_query(&bytes).map_err(|e| format!("{e:?}"))?,
        QuestLogQuery::Track {
            quest_indexes: vec![1]
        }
    );
    assert_eq!(link.quest_panel().state.log.tracked.len(), 2);
    outbox.quest_result(&oteryn_session::GatedCommandResult {
        command_id: 2,
        server_sequence: 3,
        status: oteryn_client_runtime::CommandStatus::Accepted,
        payload: vec![],
    });
    assert_eq!(
        link.quest_panel().state.log.tracked.len(),
        2,
        "an ack alone cannot change tracking"
    );
    outbox.push(vec![oteryn_session::SessionEvent::QuestLog(
        oteryn_session::AppliedDelta {
            server_sequence: 4,
            base_revision: 1,
            new_revision: 2,
            value: QuestLog { map_marks: Vec::new(), view: Some(QuestLogView::List(vec![])),
                tracked: vec![active],
            },
        },
    )]);
    assert_eq!(link.quest_panel().state.log.tracked.len(), 1);
    Ok(())
}
