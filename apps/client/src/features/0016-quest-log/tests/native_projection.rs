use crate::play::{mailbox, play_channel};
use oteryn_client_runtime::quest_log::{QuestListEntry, QuestLogView};

#[test]
fn native_journal_stops_when_visibility_is_toggled_off() {
    let (link, _commands, outbox) = play_channel();
    let mut mailbox = mailbox::lock(&outbox.0);
    mailbox.quest.open = true;
    assert!(!super::window_should_close(&mailbox));
    drop(mailbox);
    link.quest_set_open(false);
    assert!(super::window_should_close(&mailbox::lock(&outbox.0)));
}

#[test]
fn native_inline_tracker_hides_completed_missions() {
    use oteryn_client_runtime::quest_log::{QuestLine, QuestMission};
    let mut panel = oteryn_client_ui::quest_log::QuestPanel::default();
    panel.state.selected = true;
    panel.state.log.tracked.push(QuestLine {
        quest_index: 1,
        completed: false,
        missions: vec![
            QuestMission {
                mission_index: 1,
                name: "Finished task".into(),
                text: "Done".into(),
                done: true,
            },
            QuestMission {
                mission_index: 2,
                name: "Active task".into(),
                text: "Work".into(),
                done: false,
            },
        ],
    });
    assert!(super::inline_tracker_text(&panel).contains("Finished task"));
    panel.tracker.hide_completed_missions = true;
    let text = super::inline_tracker_text(&panel);
    assert!(!text.contains("Finished task"));
    assert!(text.contains("Active task"));
}

#[test]
fn native_quest_rows_repaint_after_mission_search_dependencies_change() {
    use oteryn_client_runtime::quest_log::{QuestLine, QuestMission};
    let mut panel = oteryn_client_ui::quest_log::QuestPanel::default();
    panel.state.selected = true;
    panel.state.list.push(QuestListEntry {
        quest_index: 1,
        completed: false,
    });
    panel.list_search = "dragon".into();
    let old = panel.clone();
    let line = QuestLine {
        quest_index: 1,
        completed: false,
        missions: vec![QuestMission {
            mission_index: 1,
            name: "Dragon task".into(),
            text: "Hunt".into(),
            done: false,
        }],
    };
    for tracked in [true, false] {
        panel = old.clone();
        if tracked {
            panel.state.log.tracked.push(line.clone());
        } else {
            panel.state.log.view = Some(QuestLogView::Line(line.clone()));
        }
        assert!(super::quest_rows_changed(Some(&old), &panel));
        assert!(super::quest_rows_changed(Some(&panel), &old));
    }
}
