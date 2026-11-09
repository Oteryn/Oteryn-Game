use super::*;
/// Text shared by the native description control and recorded-frame view tests.
pub fn panel_text(panel: &QuestPanel) -> String {
    let mut lines = vec![tr!("Quest Log").to_owned()];
    if !panel.state.selected {
        lines.push(tr!("Quest log unavailable in this session").into());
    }
    if let Some(refusal) = panel.state.refusal {
        lines.push(
            match refusal {
                QuestRefusal::CapabilityNotSelected => tr!("Quest log unavailable in this session"),
                QuestRefusal::UnknownQuest => tr!("Unknown quest; refresh the list"),
                QuestRefusal::LimitExceeded => tr!("At most ten quests can be tracked"),
                QuestRefusal::Rejected => tr!("Quest request refused; refresh the list"),
                QuestRefusal::ReconciliationRequired => {
                    tr!("Quest request outcome unavailable; refresh the list")
                }
                QuestRefusal::Malformed | QuestRefusal::RevisionMismatch => {
                    tr!("Quest log could not be updated")
                }
            }
            .into(),
        );
    }
    match &panel.state.log.view {
        Some(QuestLogView::List(entries)) => {
            if entries.is_empty() {
                lines.push(tr!("No quests started").into());
            }
            lines.extend(entries.iter().map(|q| {
                lfmt!(
                    "Quest {}{}",
                    q.quest_index,
                    if q.completed { " (completed)" } else { "" }
                )
            }));
        }
        Some(QuestLogView::Line(line)) => {
            lines.push(lfmt!(
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
        None => lines.push(tr!("Open the quest list").into()),
    }
    lines.push(lfmt!(
        "Tracked quests ({}/10)",
        panel.state.log.tracked.len()
    ));
    for line in &panel.state.log.tracked {
        lines.push(lfmt!("Quest {}", line.quest_index));
        for mission in &line.missions {
            lines.push(format!("{}: {}", mission.name, mission.text));
        }
    }
    lines.join("\r\n")
}

/// Independent tracker view, using confirmed lines and local visibility filters only.
pub fn tracker_text(panel: &QuestPanel) -> String {
    let rows = tracker_lines(panel);
    let mut text = lfmt!(
        "Quest tracker — {} / {} tracked quests\r\n",
        rows.len(),
        panel.state.log.tracked.len()
    );
    if !panel.state.selected {
        text.push_str(tr!("Quest tracking unavailable in this session\r\n"));
    }
    for line in &rows {
        text.push_str(&lfmt!(
            "\r\nQuest {}{}\r\n",
            line.quest_index,
            if line.completed { " (completed)" } else { "" }
        ));
        for mission in &line.missions {
            text.push_str(&format!(
                "{}{}: {}\r\n",
                mission.name,
                if mission.done { " (completed)" } else { "" },
                mission.text
            ));
        }
    }
    if rows.is_empty() {
        text.push_str(tr!("No tracked quests match these filters\r\n"));
    }
    if panel.pending_query.is_some() {
        text.push_str(tr!("Waiting for server confirmation\r\n"));
    }
    if panel.state.refusal.is_some() {
        text.push_str(tr!(
            "Quest request refused; refresh the quest log to reconcile\r\n"
        ));
    }
    text
}
