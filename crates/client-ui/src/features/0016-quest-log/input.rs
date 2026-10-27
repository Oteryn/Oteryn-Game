use oteryn_input_actions::{ButtonState, Modifier, NormalizedInputEvent};
/// Ctrl+J opens the quest log, ignoring key repeats and releases.
pub fn open_key(events: &[NormalizedInputEvent]) -> bool {
    events.iter().any(|event| matches!(event, NormalizedInputEvent::Key { code, state: ButtonState::Pressed, modifiers, repeat: false } if code.get() == 13 && modifiers.contains(Modifier::Control) && !modifiers.contains(Modifier::Alt) && !modifiers.contains(Modifier::Shift) && !modifiers.contains(Modifier::Super)))
}

/// Replaces the complete tracked set; the server confirms the change in domain 16.
pub fn tracking_query(panel: &super::QuestPanel) -> Option<super::QuestLogQuery> {
    let Some(super::QuestLogView::Line(line)) = &panel.state.log.view else {
        return None;
    };
    let mut indexes: Vec<_> = panel
        .state
        .log
        .tracked
        .iter()
        .map(|q| q.quest_index)
        .collect();
    if indexes.contains(&line.quest_index) {
        indexes.retain(|q| *q != line.quest_index);
    } else {
        indexes.push(line.quest_index);
    }
    Some(super::QuestLogQuery::Track {
        quest_indexes: indexes,
    })
}
