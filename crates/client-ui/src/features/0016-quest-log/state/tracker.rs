use super::*;
use std::collections::BTreeSet;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TrackerFilter {
    #[default]
    All,
    Active,
    Completed,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct QuestTracker {
    pub auto_add: bool,
    pub auto_remove: bool,
    pub filter: TrackerFilter,
    pub search: String,
    seen: Option<BTreeSet<u32>>,
    added: BTreeSet<u32>,
    next_poll: Option<Instant>,
    next_change: Option<Instant>,
    attempted: Option<(u64, Vec<u32>)>,
    generation: u64,
    awaiting: Option<Vec<u32>>,
}
impl QuestTracker {
    pub fn command_failed(&mut self) {
        self.awaiting = None;
    }
    /// First observation is a baseline; enabling automation never adds every old quest.
    pub fn observe(&mut self, list: &[QuestListEntry]) {
        let current: BTreeSet<_> = list.iter().map(|q| q.quest_index).collect();
        if self.auto_add
            && let Some(seen) = &self.seen
        {
            self.added.extend(
                list.iter()
                    .filter(|q| !q.completed && !seen.contains(&q.quest_index))
                    .map(|q| q.quest_index),
            );
        }
        self.added
            .retain(|id| list.iter().any(|q| q.quest_index == *id && !q.completed));
        self.seen = Some(current);
        self.generation = self.generation.saturating_add(1);
    }
}
/// Automation returns commands; the confirmed tracked lines are never edited locally.
pub fn tracker_query(panel: &mut QuestPanel, now: Instant) -> Option<QuestLogQuery> {
    if !panel.state.selected {
        return None;
    }
    let t = &mut panel.tracker;
    if !t.auto_add {
        t.added.clear();
    }
    let mut desired: Vec<_> = panel
        .state
        .log
        .tracked
        .iter()
        .filter(|q| !t.auto_remove || !q.completed)
        .map(|q| q.quest_index)
        .collect();
    for id in &t.added {
        if desired.len() == MAX_TRACKED_QUESTS {
            break;
        }
        if !desired.contains(id) {
            desired.push(*id);
        }
    }
    let confirmed: Vec<_> = panel
        .state
        .log
        .tracked
        .iter()
        .map(|q| q.quest_index)
        .collect();
    if t.awaiting.as_ref().is_some_and(|expected| {
        *expected == confirmed
            || t.seen.as_ref().is_some_and(|seen| {
                expected
                    .iter()
                    .filter(|id| seen.contains(id))
                    .copied()
                    .collect::<Vec<_>>()
                    == confirmed
            })
    }) {
        t.awaiting = None;
    }
    let attempt = (t.generation, desired.clone());
    if t.awaiting.is_none()
        && desired != confirmed
        && t.attempted.as_ref() != Some(&attempt)
        && t.next_change.is_none_or(|at| now >= at)
    {
        t.awaiting = Some(desired.clone());
        t.attempted = Some(attempt);
        t.added.retain(|id| !desired.contains(id));
        t.next_change = Some(now + Duration::from_secs(1));
        return Some(QuestLogQuery::Track {
            quest_indexes: desired,
        });
    }
    if (t.auto_add || t.auto_remove) && t.next_poll.is_none_or(|at| now >= at) {
        t.next_poll = Some(now + Duration::from_secs(5));
        return Some(QuestLogQuery::ListOnly);
    }
    None
}
pub fn tracker_lines(panel: &QuestPanel) -> Vec<&QuestLine> {
    let search = panel.tracker.search.to_lowercase();
    panel
        .state
        .log
        .tracked
        .iter()
        .filter(|line| match panel.tracker.filter {
            TrackerFilter::All => true,
            TrackerFilter::Active => !line.completed,
            TrackerFilter::Completed => line.completed,
        })
        .filter(|line| {
            format!("Quest {}", line.quest_index)
                .to_lowercase()
                .contains(&search)
                || line.missions.iter().any(|m| {
                    m.name.to_lowercase().contains(&search)
                        || m.text.to_lowercase().contains(&search)
                })
        })
        .collect()
}
