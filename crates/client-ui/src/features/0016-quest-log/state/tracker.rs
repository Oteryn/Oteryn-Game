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
    needs_fresh_list: bool,
}
impl QuestTracker {
    /// Refresh command membership after resync, retaining the discovery baseline.
    pub fn reconcile(&mut self) {
        self.awaiting = None;
        self.attempted = None;
        self.next_poll = None;
        self.needs_fresh_list = true;
    }

    /// Consume every authoritative observation, even when several arrive between UI frames.
    pub fn confirm(&mut self, tracked: &[QuestLine]) {
        let confirmed: Vec<_> = tracked.iter().map(|q| q.quest_index).collect();
        self.added.retain(|id| !confirmed.contains(id));
        if self.awaiting.as_ref().is_some_and(|expected| {
            *expected == confirmed
                || self.seen.as_ref().is_some_and(|seen| {
                    expected
                        .iter()
                        .filter(|id| seen.contains(id))
                        .copied()
                        .collect::<Vec<_>>()
                        == confirmed
                })
        }) {
            self.awaiting = None;
        }
    }
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
        self.needs_fresh_list = false;
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
    if t.needs_fresh_list {
        if (t.auto_add || t.auto_remove) && t.next_poll.is_none_or(|at| now >= at) {
            t.next_poll = Some(now + Duration::from_secs(5));
            return Some(QuestLogQuery::ListOnly);
        }
        return None;
    }
    t.confirm(&panel.state.log.tracked);
    let confirmed: Vec<_> = panel
        .state
        .log
        .tracked
        .iter()
        .map(|q| q.quest_index)
        .collect();
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
    let attempt = (t.generation, desired.clone());
    if t.awaiting.is_none()
        && desired != confirmed
        && t.attempted.as_ref() != Some(&attempt)
        && t.next_change.is_none_or(|at| now >= at)
    {
        t.awaiting = Some(desired.clone());
        t.attempted = Some(attempt);
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
    let search = panel.tracker.search.trim().to_lowercase();
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

/// Synchronize two tracker windows without treating another window's update as typing.
pub fn sync_tracker_search(
    panel: &mut QuestPanel,
    previous: &mut Option<String>,
    typed: String,
) -> Option<String> {
    if previous.as_deref().is_some_and(|last| last != typed) {
        panel.tracker.search.clone_from(&typed);
    }
    let repaint = (typed != panel.tracker.search).then(|| panel.tracker.search.clone());
    *previous = Some(panel.tracker.search.clone());
    repaint
}
