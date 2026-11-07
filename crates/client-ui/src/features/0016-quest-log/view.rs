use super::*;
/// Text shared by the native description control and recorded-frame view tests.
pub fn panel_text(panel: &QuestPanel) -> String {
    let mut lines = vec!["Quest Log".to_owned()];
    if !panel.state.selected {
        lines.push("Quest log unavailable in this session".into());
    }
    if let Some(refusal) = panel.state.refusal {
        lines.push(
            match refusal {
                QuestRefusal::CapabilityNotSelected => "Quest log unavailable in this session",
                QuestRefusal::UnknownQuest => "Unknown quest; refresh the list",
                QuestRefusal::LimitExceeded => "At most ten quests can be tracked",
                QuestRefusal::Rejected => "Quest request refused; refresh the list",
                QuestRefusal::ReconciliationRequired => {
                    "Quest request outcome unavailable; refresh the list"
                }
                QuestRefusal::Malformed | QuestRefusal::RevisionMismatch => {
                    "Quest log could not be updated"
                }
            }
            .into(),
        );
    }
    match &panel.state.log.view {
        Some(QuestLogView::List(entries)) => {
            if entries.is_empty() {
                lines.push("No quests started".into());
            }
            lines.extend(entries.iter().map(|q| {
                format!(
                    "Quest {}{}",
                    q.quest_index,
                    if q.completed { " (completed)" } else { "" }
                )
            }));
        }
        Some(QuestLogView::Line(line)) => {
            lines.push(format!(
                "Quest {}{}",
                line.quest_index,
                if line.completed { " (completed)" } else { "" }
            ));
            lines.extend(
                line.missions
                    .iter()
                    .map(|m| format!("{}{}", m.name, if m.done { " (completed)" } else { "" })),
            );
            if let Some(mission) = line
                .missions
                .iter()
                .find(|m| Some(m.mission_index) == panel.mission)
            {
                lines.push(mission.text.clone());
            }
        }
        None => lines.push("Open the quest list".into()),
    }
    lines.push(format!(
        "Tracked quests ({}/10)",
        panel.state.log.tracked.len()
    ));
    for line in &panel.state.log.tracked {
        lines.push(format!("Quest {}", line.quest_index));
        for mission in &line.missions {
            lines.push(format!("{}: {}", mission.name, mission.text));
        }
    }
    lines.join("\r\n")
}
