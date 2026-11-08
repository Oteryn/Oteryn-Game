use super::{QuestListEntry, QuestLogQuery, QuestLogView, QuestPanel};
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum QuestSort {
    #[default]
    NumberAscending,
    NumberDescending,
    ActiveFirst,
    CompletedFirst,
}
pub const QUEST_SORT_LABELS: [&str; 4] = [
    "Quest number ascending",
    "Quest number descending",
    "Active first",
    "Completed first",
];
impl QuestSort {
    pub const fn from_selection(selection: isize) -> Self {
        match selection {
            1 => Self::NumberDescending,
            2 => Self::ActiveFirst,
            3 => Self::CompletedFirst,
            _ => Self::NumberAscending,
        }
    }
    pub const fn selection(self) -> usize {
        match self {
            Self::NumberAscending => 0,
            Self::NumberDescending => 1,
            Self::ActiveFirst => 2,
            Self::CompletedFirst => 3,
        }
    }
}
/// Search and sort only confirmed list facts; the server cursor and tracker are unchanged.
pub fn quest_rows(panel: &QuestPanel) -> Vec<&QuestListEntry> {
    let search = panel.list_search.trim().to_lowercase();
    let visible = match &panel.state.log.view {
        Some(QuestLogView::Line(line)) => Some(line),
        _ => None,
    };
    let known: Vec<_> = panel
        .state
        .log
        .tracked
        .iter()
        .chain(visible)
        .filter(|line| {
            line.missions.iter().any(|mission| {
                mission.name.to_lowercase().contains(&search)
                    || mission.text.to_lowercase().contains(&search)
            })
        })
        .map(|line| line.quest_index)
        .collect();
    let mut rows: Vec<_> = panel
        .state
        .list
        .iter()
        .filter(|q| {
            format!(
                "quest {} {}",
                q.quest_index,
                if q.completed { "completed" } else { "active" }
            )
            .contains(&search)
                || known.contains(&q.quest_index)
        })
        .collect();
    rows.sort_by(|a, b| match panel.list_sort {
        QuestSort::NumberAscending => a.quest_index.cmp(&b.quest_index),
        QuestSort::NumberDescending => b.quest_index.cmp(&a.quest_index),
        QuestSort::ActiveFirst => (a.completed, a.quest_index).cmp(&(b.completed, b.quest_index)),
        QuestSort::CompletedFirst => {
            (!a.completed, a.quest_index).cmp(&(!b.completed, b.quest_index))
        }
    });
    rows
}
/// A click resolves through the last painted ids, then checks the latest server list.
pub fn quest_row_query(panel: &QuestPanel, painted: &[u32], row: usize) -> Option<QuestLogQuery> {
    if !panel.state.selected {
        return None;
    }
    let id = *painted.get(row)?;
    panel
        .state
        .list
        .iter()
        .any(|q| q.quest_index == id)
        .then_some(QuestLogQuery::Quest { quest_index: id })
}
