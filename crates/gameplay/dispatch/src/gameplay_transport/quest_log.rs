//! Domain 16 `QUEST_LOG` and command type 22 `QUEST_LOG_QUERY` behind capability 16
//! `QUEST_LOG_V1` (QUEST-LOG-WIRE-1; QUEST-GATE-0 §7).
//!
//! - **Projection.** The domain is computed from the session's quest copy by `quest::log` and
//!   never stored: the last requested list or quest line, and the tracked quests with their
//!   visible missions. Only content whose every quest's worst-case line fits
//!   `QUESTGATE0-RL-08-BYTES` is used ([`QuestLogLines`]), so a projection always encodes.
//! - **State.** Per GameSession, in the session continuity ([`QuestLogContinuity`]): the domain
//!   revision, the tracked set and the `QUESTGATE0-RL-09` query window, so a reconnect, resume or
//!   transfer resets none of them. Per connection ([`QuestLogState`]): the requested view, which
//!   closes at every reconnect and transfer, and the last payload sent.
//! - **Query.** Over the rate, malformed, with the copy not loaded, or naming a quest that is not
//!   listed, a query is `REJECTED` with an empty payload and changes nothing. Otherwise it is
//!   `ACCEPTED` with an empty payload and the whole domain follows as a delta. ListOnly instead
//!   returns a bounded list-only result without changing the view, tracked set or revision.
//! - **Refresh.** Every [`QUEST_LOG_REFRESH`] the connection asks the owner for a copy changed
//!   since the one it projected; a committed receipt that changes what the domain shows sends a
//!   delta.
//!
//! Boot composes the journal content and the admitted-session copy source.

use std::sync::Arc;

use crate::durability::quest_tracker::QuestTrackerEntry;

use oteryn_gameplay_quests::quest::log::{ProjectedLine, QuestLogContent, QuestLogFacts};
use oteryn_protocol_oteryn::quest_log::{
    MAX_QUEST_LINE_BYTES, MAX_QUEST_LOG_QUERIES_PER_SECOND, MAX_TRACKED_MISSIONS,
    MAX_TRACKED_QUESTS, QuestLine, QuestListEntry, QuestLog, QuestLogQuery, QuestLogView,
    QuestMission, TrackedMission, encode_quest_log, encode_quest_log_missions,
    worst_case_line_bytes,
};
use tokio::time::{Duration, Instant};

/// `QUESTGATE0-RL-09`: queries per [`QUEST_LOG_QUERY_WINDOW`].
pub const QUEST_LOG_QUERY_WINDOW: Duration = Duration::from_secs(1);
/// How often a connection looks for a changed copy.
pub const QUEST_LOG_REFRESH: Duration = Duration::from_secs(1);

/// The sliding window of `QUESTGATE0-RL-09`: the instants of the last queries. `Copy` so it
/// travels with the session continuity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueryWindow([Option<Instant>; MAX_QUEST_LOG_QUERIES_PER_SECOND]);

impl QueryWindow {
    pub const EMPTY: Self = Self([None; MAX_QUEST_LOG_QUERIES_PER_SECOND]);

    /// Whether one more query fits the window ending at `now`; if so it is recorded.
    pub fn admit(&mut self, now: Instant) -> bool {
        let free = self.0.iter_mut().find(|slot| {
            !matches!(slot, Some(at) if now.saturating_duration_since(*at) < QUEST_LOG_QUERY_WINDOW)
        });
        let Some(slot) = free else {
            return false;
        };
        *slot = Some(now);
        true
    }
}

/// The tracked quest indexes in the order the client tracked them: at most `QUESTGATE0-RL-06`,
/// distinct.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrackedQuests {
    indexes: [u32; MAX_TRACKED_QUESTS],
    len: usize,
}

impl TrackedQuests {
    pub const NONE: Self = Self {
        indexes: [0; MAX_TRACKED_QUESTS],
        len: 0,
    };

    /// `None` over the bound; the query codec already refuses it and repeated indexes.
    fn new(indexes: &[u32]) -> Option<Self> {
        let mut tracked = Self::NONE;
        tracked
            .indexes
            .get_mut(..indexes.len())?
            .copy_from_slice(indexes);
        tracked.len = indexes.len();
        Some(tracked)
    }

    pub fn as_slice(&self) -> &[u32] {
        &self.indexes[..self.len]
    }
}

/// The mission selection survives connection replacement like the legacy tracker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TrackedMissions {
    pairs: [TrackedMission; MAX_TRACKED_MISSIONS],
    len: usize,
}

impl TrackedMissions {
    const NONE: Self = Self {
        pairs: [TrackedMission {
            quest_index: 0,
            mission_index: 0,
        }; MAX_TRACKED_MISSIONS],
        len: 0,
    };

    fn new(pairs: &[TrackedMission]) -> Option<Self> {
        let mut tracked = Self::NONE;
        tracked.pairs.get_mut(..pairs.len())?.copy_from_slice(pairs);
        tracked.len = pairs.len();
        Some(tracked)
    }

    fn project(self, source: &QuestLogSource) -> Vec<QuestLine> {
        self.pairs[..self.len]
            .iter()
            .filter_map(|pair| {
                let mut line = line(source.content.0.line(&*source.copy, pair.quest_index)?);
                line.missions
                    .retain(|mission| mission.mission_index == pair.mission_index);
                (!line.missions.is_empty()).then_some(line)
            })
            .collect()
    }
}

/// What one GameSession keeps of its quest log across connections.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuestLogContinuity {
    /// The highest domain 16 revision the session may have seen.
    pub revision: u64,
    pub tracked: TrackedQuests,
    missions: TrackedMissions,
    pub queries: QueryWindow,
}

impl QuestLogContinuity {
    pub const FRESH: Self = Self {
        revision: 0,
        tracked: TrackedQuests::NONE,
        missions: TrackedMissions::NONE,
        queries: QueryWindow::EMPTY,
    };

    pub(super) fn restore(
        &mut self,
        source: &QuestLogSource,
        mission_tracking: bool,
        saved: Vec<QuestTrackerEntry>,
    ) {
        let catalogue = source.content.0.active().quests();
        let resolved = saved.into_iter().filter_map(|entry| {
            let index = catalogue
                .binary_search_by(|quest| quest.key.cmp(&entry.quest_key))
                .ok()?;
            let quest_index = u32::try_from(index).ok()?;
            Some((quest_index, entry.mission))
        });
        if mission_tracking {
            let pairs: Vec<_> = resolved
                .filter_map(|(quest_index, mission)| {
                    let (revision, mission_index) = mission?;
                    (source.mission_revision(quest_index)? == revision).then_some(TrackedMission {
                        quest_index,
                        mission_index,
                    })
                })
                .collect();
            self.missions = TrackedMissions::new(&pairs).unwrap_or(TrackedMissions::NONE);
        } else {
            let indexes: Vec<_> = resolved
                .filter_map(|(index, mission)| mission.is_none().then_some(index))
                .collect();
            self.tracked = TrackedQuests::new(&indexes).unwrap_or(TrackedQuests::NONE);
        }
    }
}

/// Quest log content in which every quest of every revision has a worst-case line within
/// `QUESTGATE0-RL-08-BYTES`.
#[derive(Debug, Clone)]
pub struct QuestLogLines(Arc<QuestLogContent>);

impl QuestLogSource {
    fn mission_revision(&self, quest_index: u32) -> Option<&str> {
        let active = self.content.0.active();
        let key = &active.quests().get(usize::try_from(quest_index).ok()?)?.key;
        Some(
            self.copy
                .quest_state(key)
                .filter(|state| !state.completed)
                .map_or(active.content_revision(), |state| {
                    state.pinned_content_revision
                }),
        )
    }

    pub(super) fn saved_quest(&self, quest_index: u32) -> Option<QuestTrackerEntry> {
        let quest = self
            .content
            .0
            .active()
            .quests()
            .get(usize::try_from(quest_index).ok()?)?;
        Some(QuestTrackerEntry {
            quest_key: quest.key.clone(),
            mission: None,
        })
    }

    pub(super) fn saved_mission(&self, pair: TrackedMission) -> Option<QuestTrackerEntry> {
        let mut entry = self.saved_quest(pair.quest_index)?;
        entry.mission = Some((
            self.mission_revision(pair.quest_index)?.to_owned(),
            pair.mission_index,
        ));
        Some(entry)
    }

    pub(in crate::gameplay_transport) fn titles(&self) -> Vec<(u32, String)> {
        self.content.0.titles(&*self.copy)
    }
}

impl QuestLogLines {
    /// `None` (the content does not load) when any quest's worst-case line is over the bound.
    pub fn new(content: QuestLogContent) -> Option<Self> {
        let fits = content.catalogues().all(|catalogue| {
            catalogue.quests().iter().all(|quest| {
                worst_case_line_bytes(quest.worst_case_missions()) <= MAX_QUEST_LINE_BYTES
            })
        });
        fits.then(|| Self(Arc::new(content)))
    }
}

/// The session copy as the Channel owner holds it, with the content it is projected against.
#[derive(Clone)]
pub struct QuestLogSource {
    pub map_marks: Vec<oteryn_protocol_oteryn::quest_log::QuestMapMark>,
    pub content: QuestLogLines,
    pub copy: Arc<dyn QuestLogFacts + Send + Sync>,
    /// Changes whenever the copy changes (each applied receipt and each reload).
    pub version: u64,
}

impl std::fmt::Debug for QuestLogSource {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("QuestLogSource")
            .field("version", &self.version)
            .finish_non_exhaustive()
    }
}

/// The owner's answer to a copy read.
#[derive(Debug, Clone)]
pub enum QuestLogObservation {
    /// The copy is not loaded (the load failed, a bound was exceeded, or no quest content is
    /// loaded): queries fail closed and the domain shows nothing new.
    Unavailable,
    /// The copy has the version the caller named.
    Unchanged,
    Changed(QuestLogSource),
}

/// The view a client requested on this connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RequestedView {
    List,
    Quest(u32),
}

/// One domain 16 payload to send: a snapshot at `to`, or a delta from `to - 1` to `to`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestLogDomain {
    pub to: u64,
    pub payload: Vec<u8>,
}

/// The domain could not be built: the revision would wrap or a projection did not encode. The
/// connection fails closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuestLogFault;

/// One connection's quest log.
#[derive(Debug, Default, Clone)]
pub struct QuestLogState {
    view: Option<RequestedView>,
    /// The copy version last projected.
    version: Option<u64>,
    /// The payload last sent.
    sent: Vec<u8>,
    include_map_marks: bool,
    mission_tracking: bool,
    pub(super) tracker_loaded: bool,
}

fn line(projected: ProjectedLine) -> QuestLine {
    QuestLine {
        quest_index: projected.index,
        completed: projected.completed,
        missions: projected
            .missions
            .into_iter()
            .map(|mission| QuestMission {
                mission_index: mission.index,
                name: mission.name,
                text: mission.text,
                done: mission.done,
            })
            .collect(),
    }
}

/// The domain for `view` and `tracked` over `source`; without a source, nothing is shown.
fn project(
    source: Option<&QuestLogSource>,
    view: Option<RequestedView>,
    tracked: &[u32],
    include_map_marks: bool,
) -> QuestLog {
    let Some(source) = source else {
        return QuestLog::default();
    };
    let (content, copy) = (&*source.content.0, &*source.copy);
    QuestLog {
        map_marks: if include_map_marks {
            source.map_marks.clone()
        } else {
            Vec::new()
        },
        view: match view {
            None => None,
            Some(RequestedView::List) => Some(QuestLogView::List(
                content
                    .list(copy)
                    .into_iter()
                    .map(|quest| QuestListEntry {
                        quest_index: quest.index,
                        completed: quest.completed,
                    })
                    .collect(),
            )),
            Some(RequestedView::Quest(index)) => content
                .line(copy, index)
                .map(|projected| QuestLogView::Line(line(projected))),
        },
        tracked: tracked
            .iter()
            .filter_map(|index| content.line(copy, *index).map(line))
            .collect(),
    }
}

fn advance(revision: &mut u64) -> Result<u64, QuestLogFault> {
    *revision = revision.checked_add(1).ok_or(QuestLogFault)?;
    Ok(*revision)
}

impl QuestLogState {
    /// Independent tracker polling must not steal the visible quest line.
    pub fn list_only(observation: QuestLogObservation, include_map_marks: bool) -> Option<Vec<u8>> {
        let QuestLogObservation::Changed(source) = observation else {
            return None;
        };
        encode_quest_log(&project(
            Some(&source),
            Some(RequestedView::List),
            &[],
            include_map_marks,
        ))
        .ok()
    }

    /// The domain 16 snapshot of a new connection: no view, the carried tracked quests, above
    /// every revision the session has seen.
    pub fn snapshot(
        continuity: &mut QuestLogContinuity,
        observation: QuestLogObservation,
        include_map_marks: bool,
        mission_tracking: bool,
    ) -> Result<(Self, QuestLogDomain), QuestLogFault> {
        let source = match observation {
            QuestLogObservation::Changed(source) => Some(source),
            QuestLogObservation::Unavailable | QuestLogObservation::Unchanged => None,
        };
        let mut state = Self {
            version: source.as_ref().map(|source| source.version),
            include_map_marks,
            mission_tracking,
            ..Self::default()
        };
        let payload = state.encode(source.as_ref(), None, continuity)?;
        let to = advance(&mut continuity.revision)?;
        state.sent.clone_from(&payload);
        Ok((state, QuestLogDomain { to, payload }))
    }

    fn encode(
        &self,
        source: Option<&QuestLogSource>,
        view: Option<RequestedView>,
        continuity: &QuestLogContinuity,
    ) -> Result<Vec<u8>, QuestLogFault> {
        let tracked = if self.mission_tracking {
            &[][..]
        } else {
            continuity.tracked.as_slice()
        };
        let mut log = project(source, view, tracked, self.include_map_marks);
        if self.mission_tracking {
            if let Some(source) = source {
                log.tracked = continuity.missions.project(source);
            }
            encode_quest_log_missions(&log).map_err(|_| QuestLogFault)
        } else {
            encode_quest_log(&log).map_err(|_| QuestLogFault)
        }
    }

    pub fn track_missions(
        &mut self,
        continuity: &mut QuestLogContinuity,
        pairs: &[TrackedMission],
        limit: usize,
        observation: QuestLogObservation,
    ) -> Option<Result<QuestLogDomain, QuestLogFault>> {
        let QuestLogObservation::Changed(source) = observation else {
            return None;
        };
        if !self.mission_tracking || pairs.len() > limit {
            return None;
        }
        let missions = TrackedMissions::new(pairs)?;
        if missions.project(&source).len() != pairs.len() {
            return None;
        }
        let payload = self
            .encode(
                Some(&source),
                self.view,
                &QuestLogContinuity {
                    missions,
                    ..*continuity
                },
            )
            .ok()?;
        self.version = Some(source.version);
        continuity.missions = missions;
        Some(self.delta(continuity, payload))
    }

    /// The copy version this connection last projected, for the owner's change check.
    pub const fn version(&self) -> Option<u64> {
        self.version
    }

    /// Command type 22 after its rate check: `None` (`REJECTED`, nothing changes) when the copy
    /// is not loaded, a named quest is not listed, or the domain does not encode; otherwise the
    /// whole-domain delta.
    pub fn query(
        &mut self,
        continuity: &mut QuestLogContinuity,
        query: &QuestLogQuery,
        observation: QuestLogObservation,
    ) -> Option<Result<QuestLogDomain, QuestLogFault>> {
        let QuestLogObservation::Changed(source) = observation else {
            return None;
        };
        let content = &*source.content.0;
        let (view, tracked) = match query {
            QuestLogQuery::List => (Some(RequestedView::List), continuity.tracked),
            QuestLogQuery::ListOnly => return None,
            QuestLogQuery::Quest { quest_index } => {
                if !content.is_listed(&*source.copy, *quest_index) {
                    return None;
                }
                (Some(RequestedView::Quest(*quest_index)), continuity.tracked)
            }
            QuestLogQuery::Track { quest_indexes } => {
                if self.mission_tracking
                    || !quest_indexes
                        .iter()
                        .all(|index| content.is_listed(&*source.copy, *index))
                {
                    return None;
                }
                (self.view, TrackedQuests::new(quest_indexes)?)
            }
        };
        let payload = self
            .encode(
                Some(&source),
                view,
                &QuestLogContinuity {
                    tracked,
                    ..*continuity
                },
            )
            .ok()?;
        self.view = view;
        self.version = Some(source.version);
        continuity.tracked = tracked;
        Some(self.delta(continuity, payload))
    }

    /// The refresh cadence: a delta when the changed copy changes what the domain shows.
    pub fn refresh(
        &mut self,
        continuity: &mut QuestLogContinuity,
        observation: QuestLogObservation,
    ) -> Result<Option<QuestLogDomain>, QuestLogFault> {
        let QuestLogObservation::Changed(source) = observation else {
            return Ok(None);
        };
        self.version = Some(source.version);
        let payload = self.encode(Some(&source), self.view, continuity)?;
        if payload == self.sent {
            return Ok(None);
        }
        self.delta(continuity, payload).map(Some)
    }

    fn delta(
        &mut self,
        continuity: &mut QuestLogContinuity,
        payload: Vec<u8>,
    ) -> Result<QuestLogDomain, QuestLogFault> {
        let to = advance(&mut continuity.revision)?;
        self.sent.clone_from(&payload);
        Ok(QuestLogDomain { to, payload })
    }
}

#[cfg(test)]
#[path = "quest_log_tests.rs"]
mod tests;
