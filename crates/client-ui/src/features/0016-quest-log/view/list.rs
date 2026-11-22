use super::{QuestListEntry, QuestLogQuery, QuestLogView, QuestPanel};
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum QuestSort {
    #[default]
    NumberAscending,
    NumberDescending,
    ActiveFirst,
    CompletedFirst,
    Alphabetical,
    ReverseAlphabetical,
}
pub const QUEST_SORT_LABELS: [&str; 6] = [
    "Quest number ascending",
    "Quest number descending",
    "Active first",
    "Completed first",
    "Quest name A–Z",
    "Quest name Z–A",
];
impl QuestSort {
    pub const fn from_selection(selection: isize) -> Self {
        match selection {
            1 => Self::NumberDescending,
            2 => Self::ActiveFirst,
            3 => Self::CompletedFirst,
            4 => Self::Alphabetical,
            5 => Self::ReverseAlphabetical,
            _ => Self::NumberAscending,
        }
    }
    pub const fn selection(self) -> usize {
        match self {
            Self::NumberAscending => 0,
            Self::NumberDescending => 1,
            Self::ActiveFirst => 2,
            Self::CompletedFirst => 3,
            Self::Alphabetical => 4,
            Self::ReverseAlphabetical => 5,
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
        .filter(|q| panel.show_hidden == panel.hidden.contains(&q.quest_index))
        .filter(|q| {
            format!(
                "quest {} {}",
                q.quest_index,
                if q.completed { "completed" } else { "active" }
            )
            .contains(&search)
                || panel.title(q.quest_index).to_lowercase().contains(&search)
                || known.contains(&q.quest_index)
        })
        .collect();
    match panel.list_sort {
        QuestSort::Alphabetical | QuestSort::ReverseAlphabetical => {
            rows.sort_by_cached_key(|q| (panel.title(q.quest_index).to_lowercase(), q.quest_index));
            if panel.list_sort == QuestSort::ReverseAlphabetical {
                rows.reverse();
            }
        }
        QuestSort::NumberAscending => rows.sort_by_key(|q| q.quest_index),
        QuestSort::NumberDescending => rows.sort_by_key(|q| std::cmp::Reverse(q.quest_index)),
        QuestSort::ActiveFirst => rows.sort_by_key(|q| (q.completed, q.quest_index)),
        QuestSort::CompletedFirst => rows.sort_by_key(|q| (!q.completed, q.quest_index)),
    }
    if !panel.pinned.is_empty() {
        let ranks: std::collections::BTreeMap<_, _> = panel
            .pinned
            .iter()
            .copied()
            .enumerate()
            .map(|(rank, id)| (id, rank))
            .collect();
        rows.sort_by_key(|q| ranks.get(&q.quest_index).copied().unwrap_or(usize::MAX));
    }
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
