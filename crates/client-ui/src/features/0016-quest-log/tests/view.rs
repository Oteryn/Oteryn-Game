use super::*;
use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};

#[test]
fn quest_panel_lists_missions_description_completion_and_tracking() {
    let mut panel = QuestPanel {
        state: QuestState {
            selected: true,
            ..QuestState::default()
        },
        ..QuestPanel::default()
    };
    panel.state.log.view = Some(QuestLogView::List(vec![QuestListEntry {
        quest_index: 7,
        completed: true,
    }]));
    assert!(panel_text(&panel).contains("Quest 7 (completed)"));
    let line = QuestLine {
        quest_index: 7,
        completed: true,
        missions: vec![QuestMission {
            mission_index: 2,
            name: "The tower".into(),
            text: "Bring back the key.".into(),
            done: true,
        }],
    };
    panel.state.log.view = Some(QuestLogView::Line(line.clone()));
    panel.mission = Some(2);
    panel.state.log.tracked.push(line);
    let text = panel_text(&panel);
    for expected in [
        "The tower (completed)",
        "Bring back the key.",
        "Tracked quests (1/10)",
    ] {
        assert!(text.contains(expected));
    }
    panel.state.refusal = Some(QuestRefusal::Rejected);
    assert!(panel_text(&panel).contains("Quest request refused"));
}

#[test]
fn quest_open_binding_is_ctrl_j_without_repeat() -> Result<(), oteryn_input_actions::InputError> {
    let key = KeyCode::new(13)?;
    let event = |state, modifiers, repeat| NormalizedInputEvent::Key {
        code: key,
        state,
        modifiers,
        repeat,
    };
    let ctrl = Modifiers::one(Modifier::Control);
    assert!(open_key(&[event(ButtonState::Pressed, ctrl, false)]));
    for input in [
        event(ButtonState::Released, ctrl, false),
        event(ButtonState::Pressed, ctrl, true),
        event(ButtonState::Pressed, Modifiers::NONE, false),
    ] {
        assert!(!open_key(&[input]));
    }
    Ok(())
}

#[test]
fn quest_tracking_toggle_preserves_order_and_obeys_the_wire_limit() -> Result<(), QuestRefusal> {
    let mut panel = QuestPanel {
        state: QuestState {
            selected: true,
            ..QuestState::default()
        },
        ..QuestPanel::default()
    };
    panel.state.list = (0..=10)
        .map(|quest_index| QuestListEntry {
            quest_index,
            completed: false,
        })
        .collect();
    let line = |quest_index| QuestLine {
        quest_index,
        completed: false,
        missions: vec![],
    };
    panel.state.log.tracked = (0..10).map(line).collect();
    panel.state.log.view = Some(QuestLogView::Line(line(10)));
    let query = tracking_query(&panel).ok_or(QuestRefusal::Malformed)?;
    assert_eq!(
        request(&mut panel, &query),
        Err(QuestRefusal::LimitExceeded)
    );
    assert_eq!(panel.state.log.tracked.len(), 10);
    panel.state.log.view = Some(QuestLogView::Line(line(3)));
    let query = tracking_query(&panel).ok_or(QuestRefusal::Malformed)?;
    assert_eq!(
        query,
        QuestLogQuery::Track {
            quest_indexes: vec![0, 1, 2, 4, 5, 6, 7, 8, 9]
        }
    );
    assert!(request(&mut panel, &query).is_ok());
    assert_eq!(panel.state.log.tracked.len(), 10); // no optimistic tracking mutation
    Ok(())
}
