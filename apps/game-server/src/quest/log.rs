//! The quest log catalogue and its projection over the session copy (QUEST-GATE-0 §7;
//! QUEST-LOG-WIRE-1).
//!
//! [`QuestLogCatalogue`] holds, for one content revision, what the log shows of each quest: its
//! kind, whether it is shown in the log, its `start` track and value, `hide_when_completed`, and its
//! missions (one track each, `[start_value, end_value]`, a name and a journal). It is checked
//! against that revision's [`QuestStateCatalogue`]: every quest is declared, every track it names
//! is declared, and each mission's track belongs to its quest. Tests build it in code; the journal
//! text arrives with QUEST-CONTENT-2.
//!
//! [`QuestLogContent`] is the active catalogue with the earlier revisions still loadable. The
//! projection is computed from the session copy ([`QuestLogFacts`]) and never stored:
//! - a storyline quest shown in the log is listed once its `start` track reaches its value;
//!   `reward_only` and `script_only` quests are never listed;
//! - a quest is completed when its state has its completed receipt; one flagged
//!   `hide_when_completed` then leaves the list;
//! - a mission is shown while its track lies in `[start_value, end_value]`, and done at
//!   `end_value`, with the journal entry for the current value;
//! - an in-progress quest (a state with no completed receipt) resolves against its pinned
//!   revision; a quest with no state, and a completed one, against the active revision. A pinned
//!   revision that is not loadable hides the quest (fail closed).
//!
//! Quests are canonical indexes per content generation: the position in the active catalogue's
//! ascending Oteryn keys. Missions are indexed by their position in their quest's revision.
//!
//! Like the rest of this module it depends on `std` only.

use std::collections::BTreeMap;

use super::{QuestStateCatalogue, valid_quest_key};

/// `QUESTGATE0-RL-07`: quests the log can list.
pub const QUESTGATE0_RL_07: usize = 1024;
/// `QUESTGATE0-RL-08`: missions per quest.
pub const QUESTGATE0_RL_08: usize = 128;
/// `QUESTGATE0-RL-08-NAME`: UTF-8 bytes of a mission name.
pub const QUESTGATE0_RL_08_NAME: usize = 128;
/// `QUESTGATE0-RL-08-TEXT`: UTF-8 bytes of a journal text, each counter counted at its longest.
pub const QUESTGATE0_RL_08_TEXT: usize = 1024;
/// The longest decimal `i64`, the bytes a template counter takes at most.
pub const COUNTER_BYTES: usize = 20;

/// The quest kinds of the authoring format; only storyline quests have missions in the log.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestLogKind {
    Storyline,
    RewardOnly,
    ScriptOnly,
}

/// One piece of a journal template.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JournalPiece {
    Text(String),
    /// A counter: the track's value in decimal.
    Track(String),
}

/// One journal entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JournalText {
    Text(String),
    Template(Vec<JournalPiece>),
}

/// A mission's journal (format §3.2): none, one text for every value, or one per stage value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuestJournal {
    None,
    Fixed(JournalText),
    PerStage(BTreeMap<i64, JournalText>),
}

/// One mission: its track and the range in which it shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestLogMission {
    pub name: String,
    pub track: String,
    pub start_value: i64,
    pub end_value: i64,
    pub journal: QuestJournal,
}

/// What the log shows of one quest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestLogQuest {
    pub key: String,
    pub kind: QuestLogKind,
    pub shown_in_quest_log: bool,
    /// The track and the value it must reach for the quest to be listed.
    pub start: Option<(String, i64)>,
    /// Outfit and addon quests leave the list once completed.
    pub hide_when_completed: bool,
    pub missions: Vec<QuestLogMission>,
}

impl QuestLogQuest {
    /// A storyline quest shown in the log with a start: the only quests ever listed.
    fn listable(&self) -> bool {
        self.kind == QuestLogKind::Storyline && self.shown_in_quest_log && self.start.is_some()
    }

    /// Each mission's name bytes and its longest journal text, the worst case of its line
    /// (`QUESTGATE0-RL-08-BYTES`, checked where the line is encoded).
    pub fn worst_case_missions(&self) -> impl Iterator<Item = (usize, usize)> + '_ {
        self.missions.iter().map(|mission| {
            let text = match &mission.journal {
                QuestJournal::None => 0,
                QuestJournal::Fixed(text) => worst_case_text_bytes(text),
                QuestJournal::PerStage(stages) => stages
                    .values()
                    .map(worst_case_text_bytes)
                    .max()
                    .unwrap_or(0),
            };
            (mission.name.len(), text)
        })
    }
}

/// A catalogue that breaks a §7 rule or a bound; the whole catalogue does not load.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuestLogCatalogueError {
    /// A quest the state catalogue of the same revision does not declare.
    UnknownQuest(String),
    DuplicateQuest(String),
    /// A track the state catalogue does not declare, or a mission or start track of another
    /// quest.
    UnknownTrack(String),
    /// An empty or over-long name, an inverted range, or a journal text over its bound.
    InvalidMission(String),
    /// More than `QUESTGATE0-RL-08` missions, or more than `QUESTGATE0-RL-07` listable quests.
    CapacityExceeded,
}

/// The log of one content revision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestLogCatalogue {
    content_revision: String,
    /// In ascending key order: the position is the canonical quest index.
    quests: Vec<QuestLogQuest>,
    /// The declared initial value of every track a quest of the log reads.
    initial: BTreeMap<String, i64>,
}

impl QuestLogCatalogue {
    /// Check every quest and track against `state`, the same revision's state catalogue.
    pub fn new(
        state: &QuestStateCatalogue,
        quests: Vec<QuestLogQuest>,
    ) -> Result<Self, QuestLogCatalogueError> {
        use QuestLogCatalogueError as E;
        let mut by_key = BTreeMap::new();
        let mut initial = BTreeMap::new();
        for quest in quests {
            if !valid_quest_key(&quest.key) || state.definition_hash(&quest.key).is_none() {
                return Err(E::UnknownQuest(quest.key));
            }
            if quest.missions.len() > QUESTGATE0_RL_08 {
                return Err(E::CapacityExceeded);
            }
            let mut own = |track: &str| match state.track(track) {
                Some(declared) if declared.quest == quest.key => {
                    initial.insert(track.to_owned(), declared.initial);
                    Ok(())
                }
                _ => Err(E::UnknownTrack(track.to_owned())),
            };
            if let Some((track, _)) = &quest.start {
                own(track)?;
            }
            for mission in &quest.missions {
                own(&mission.track)?;
            }
            for mission in &quest.missions {
                if mission.name.is_empty()
                    || mission.name.len() > QUESTGATE0_RL_08_NAME
                    || mission.start_value > mission.end_value
                {
                    return Err(E::InvalidMission(mission.name.clone()));
                }
                let texts: Vec<&JournalText> = match &mission.journal {
                    QuestJournal::None => Vec::new(),
                    QuestJournal::Fixed(text) => vec![text],
                    QuestJournal::PerStage(stages) => stages.values().collect(),
                };
                for text in texts {
                    if worst_case_text_bytes(text) > QUESTGATE0_RL_08_TEXT {
                        return Err(E::InvalidMission(mission.name.clone()));
                    }
                    if let JournalText::Template(pieces) = text {
                        for piece in pieces {
                            if let JournalPiece::Track(track) = piece {
                                let declared = state
                                    .track(track)
                                    .ok_or_else(|| E::UnknownTrack(track.clone()))?;
                                initial.insert(track.clone(), declared.initial);
                            }
                        }
                    }
                }
            }
            if by_key.contains_key(&quest.key) {
                return Err(E::DuplicateQuest(quest.key));
            }
            by_key.insert(quest.key.clone(), quest);
        }
        if by_key.values().filter(|quest| quest.listable()).count() > QUESTGATE0_RL_07
            || by_key.len() > u32::MAX as usize
        {
            return Err(E::CapacityExceeded);
        }
        Ok(Self {
            content_revision: state.content_revision().to_owned(),
            quests: by_key.into_values().collect(),
            initial,
        })
    }

    #[must_use]
    pub fn content_revision(&self) -> &str {
        &self.content_revision
    }

    /// Every quest in canonical index order.
    #[must_use]
    pub fn quests(&self) -> &[QuestLogQuest] {
        &self.quests
    }

    fn index_of(&self, key: &str) -> Option<usize> {
        self.quests
            .binary_search_by(|quest| quest.key.as_str().cmp(key))
            .ok()
    }

    /// The copy's value of `track`, or the declared initial value when it has no row. `None`
    /// for a track this revision does not declare: it never reads as an initial value.
    fn value(&self, facts: &(impl QuestLogFacts + ?Sized), track: &str) -> Option<i64> {
        let initial = *self.initial.get(track)?;
        Some(facts.track_value(track).unwrap_or(initial))
    }
}

/// The longest a journal text renders to: its text pieces and each counter at its longest.
#[must_use]
pub fn worst_case_text_bytes(text: &JournalText) -> usize {
    match text {
        JournalText::Text(text) => text.len(),
        JournalText::Template(pieces) => pieces
            .iter()
            .map(|piece| match piece {
                JournalPiece::Text(text) => text.len(),
                JournalPiece::Track(_) => COUNTER_BYTES,
            })
            .sum(),
    }
}

/// The active catalogue and the earlier revisions in-progress quests may be pinned to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestLogContent {
    active: QuestLogCatalogue,
    pinned: BTreeMap<String, QuestLogCatalogue>,
}

impl QuestLogContent {
    /// `pinned` are the loadable earlier revisions; one with the active revision is ignored.
    #[must_use]
    pub fn new(active: QuestLogCatalogue, pinned: Vec<QuestLogCatalogue>) -> Self {
        let pinned = pinned
            .into_iter()
            .filter(|catalogue| catalogue.content_revision != active.content_revision)
            .map(|catalogue| (catalogue.content_revision.clone(), catalogue))
            .collect();
        Self { active, pinned }
    }

    #[must_use]
    pub fn active(&self) -> &QuestLogCatalogue {
        &self.active
    }

    /// Every catalogue: the active one first.
    pub fn catalogues(&self) -> impl Iterator<Item = &QuestLogCatalogue> {
        std::iter::once(&self.active).chain(self.pinned.values())
    }

    fn revision(&self, content_revision: &str) -> Option<&QuestLogCatalogue> {
        if content_revision == self.active.content_revision {
            Some(&self.active)
        } else {
            self.pinned.get(content_revision)
        }
    }
}

/// One quest's state in the session copy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuestLogState<'a> {
    pub pinned_content_revision: &'a str,
    pub completed: bool,
}

/// What the projection reads: the session copy.
pub trait QuestLogFacts {
    /// The copy's stored value of `track`; `None` when it has no row.
    fn track_value(&self, track: &str) -> Option<i64>;
    /// The Character's state for `quest`; `None` when it has none.
    fn quest_state(&self, quest: &str) -> Option<QuestLogState<'_>>;
}

/// One listed quest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ListedQuest {
    pub index: u32,
    pub completed: bool,
}

/// One visible mission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisibleMission {
    pub index: u32,
    pub name: String,
    pub text: String,
    pub done: bool,
}

/// One listed quest's visible missions in ascending mission index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectedLine {
    pub index: u32,
    pub completed: bool,
    pub missions: Vec<VisibleMission>,
}

impl QuestLogContent {
    /// The revision and record quest `index` resolves against, with its completion, when it is
    /// listed (§7); `None` otherwise.
    fn listed(
        &self,
        facts: &(impl QuestLogFacts + ?Sized),
        index: u32,
    ) -> Option<(&QuestLogCatalogue, &QuestLogQuest, bool)> {
        let active = self.active.quests.get(usize::try_from(index).ok()?)?;
        let (catalogue, completed) = match facts.quest_state(&active.key) {
            None => (&self.active, false),
            Some(state) if state.completed => (&self.active, true),
            Some(state) => (self.revision(state.pinned_content_revision)?, false),
        };
        let quest = &catalogue.quests[catalogue.index_of(&active.key)?];
        if !quest.listable() || (completed && quest.hide_when_completed) {
            return None;
        }
        let (track, at_least) = quest.start.as_ref()?;
        (catalogue.value(facts, track)? >= *at_least).then_some((catalogue, quest, completed))
    }

    /// The quest list: every listed quest in ascending index.
    #[must_use]
    pub fn list(&self, facts: &(impl QuestLogFacts + ?Sized)) -> Vec<ListedQuest> {
        (0..self.active.quests.len())
            .filter_map(|index| {
                let index = u32::try_from(index).ok()?;
                let (_, _, completed) = self.listed(facts, index)?;
                Some(ListedQuest { index, completed })
            })
            .collect()
    }

    /// Quest `index`'s line when it is listed; `None` for an unknown or unlisted index.
    #[must_use]
    pub fn line(&self, facts: &(impl QuestLogFacts + ?Sized), index: u32) -> Option<ProjectedLine> {
        let (catalogue, quest, completed) = self.listed(facts, index)?;
        let mut missions = Vec::new();
        for (position, mission) in quest.missions.iter().enumerate() {
            let value = catalogue.value(facts, &mission.track)?;
            if !(mission.start_value..=mission.end_value).contains(&value) {
                continue;
            }
            let entry = match &mission.journal {
                QuestJournal::None => None,
                QuestJournal::Fixed(text) => Some(text),
                QuestJournal::PerStage(stages) => stages.get(&value),
            };
            let text = match entry {
                None => String::new(),
                Some(JournalText::Text(text)) => text.clone(),
                Some(JournalText::Template(pieces)) => {
                    let mut text = String::new();
                    for piece in pieces {
                        match piece {
                            JournalPiece::Text(part) => text.push_str(part),
                            JournalPiece::Track(track) => {
                                text.push_str(&catalogue.value(facts, track)?.to_string());
                            }
                        }
                    }
                    text
                }
            };
            missions.push(VisibleMission {
                index: u32::try_from(position).ok()?,
                name: mission.name.clone(),
                text,
                done: value == mission.end_value,
            });
        }
        Some(ProjectedLine {
            index,
            completed,
            missions,
        })
    }

    /// Whether quest `index` is listed.
    #[must_use]
    pub fn is_listed(&self, facts: &(impl QuestLogFacts + ?Sized), index: u32) -> bool {
        self.listed(facts, index).is_some()
    }
}

#[cfg(test)]
#[path = "log_tests.rs"]
mod tests;
