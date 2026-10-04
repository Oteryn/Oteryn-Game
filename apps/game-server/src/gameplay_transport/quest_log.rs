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
//!   `ACCEPTED` with an empty payload and the whole domain follows as a delta.
//! - **Refresh.** Every [`QUEST_LOG_REFRESH`] the connection asks the owner for a copy changed
//!   since the one it projected; a committed receipt that changes what the domain shows sends a
//!   delta.
//!
//! Nothing here writes. Capability 16 stays `offered: false` until the quest log content and the
//! production copy source are composed, so only the tests select it.

#![cfg_attr(not(test), allow(dead_code))]

use std::sync::Arc;

use crate::durability::quest_state::QuestStateCopy;
use crate::durability::quest_state::quest::log::{
    ProjectedLine, QuestLogContent, QuestLogFacts, QuestLogState as CopyQuestState,
};
use oteryn_protocol_oteryn::quest_log::{
    MAX_QUEST_LINE_BYTES, MAX_QUEST_LOG_QUERIES_PER_SECOND, MAX_TRACKED_QUESTS, QuestLine,
    QuestListEntry, QuestLog, QuestLogQuery, QuestLogView, QuestMission, encode_quest_log,
    worst_case_line_bytes,
};
use tokio::time::{Duration, Instant};

/// `QUESTGATE0-RL-09`: queries per [`QUEST_LOG_QUERY_WINDOW`].
pub(crate) const QUEST_LOG_QUERY_WINDOW: Duration = Duration::from_secs(1);
/// How often a connection looks for a changed copy.
pub(crate) const QUEST_LOG_REFRESH: Duration = Duration::from_secs(1);

/// The sliding window of `QUESTGATE0-RL-09`: the instants of the last queries. `Copy` so it
/// travels with the session continuity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct QueryWindow([Option<Instant>; MAX_QUEST_LOG_QUERIES_PER_SECOND]);

impl QueryWindow {
    pub(crate) const EMPTY: Self = Self([None; MAX_QUEST_LOG_QUERIES_PER_SECOND]);

    /// Whether one more query fits the window ending at `now`; if so it is recorded.
    pub(crate) fn admit(&mut self, now: Instant) -> bool {
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
pub(crate) struct TrackedQuests {
    indexes: [u32; MAX_TRACKED_QUESTS],
    len: usize,
}

impl TrackedQuests {
    pub(crate) const NONE: Self = Self {
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

    pub(crate) fn as_slice(&self) -> &[u32] {
        &self.indexes[..self.len]
    }
}

/// What one GameSession keeps of its quest log across connections.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct QuestLogContinuity {
    /// The highest domain 16 revision the session may have seen.
    pub(crate) revision: u64,
    pub(crate) tracked: TrackedQuests,
    pub(crate) queries: QueryWindow,
}

impl QuestLogContinuity {
    pub(crate) const FRESH: Self = Self {
        revision: 0,
        tracked: TrackedQuests::NONE,
        queries: QueryWindow::EMPTY,
    };
}

/// Quest log content in which every quest of every revision has a worst-case line within
/// `QUESTGATE0-RL-08-BYTES`.
#[derive(Debug, Clone)]
pub(crate) struct QuestLogLines(Arc<QuestLogContent>);

impl QuestLogLines {
    /// `None` (the content does not load) when any quest's worst-case line is over the bound.
    pub(crate) fn new(content: QuestLogContent) -> Option<Self> {
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
pub(crate) struct QuestLogSource {
    pub(crate) content: QuestLogLines,
    pub(crate) copy: Arc<dyn QuestLogFacts + Send + Sync>,
    /// Changes whenever the copy changes (each applied receipt and each reload).
    pub(crate) version: u64,
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
pub(crate) enum QuestLogObservation {
    /// The copy is not loaded (the load failed, a bound was exceeded, or no quest content is
    /// loaded): queries fail closed and the domain shows nothing new.
    Unavailable,
    /// The copy has the version the caller named.
    Unchanged,
    Changed(QuestLogSource),
}

/// The production session copy (QUEST-STATE-0 §7).
impl QuestLogFacts for QuestStateCopy {
    fn track_value(&self, track: &str) -> Option<i64> {
        self.tracks().get(track).copied()
    }

    fn quest_state(&self, quest: &str) -> Option<CopyQuestState<'_>> {
        self.states().get(quest).map(|state| CopyQuestState {
            pinned_content_revision: &state.pinned_content_revision,
            completed: state.completed,
        })
    }
}

/// The view a client requested on this connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RequestedView {
    List,
    Quest(u32),
}

/// One domain 16 payload to send: a snapshot at `to`, or a delta from `to - 1` to `to`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct QuestLogDomain {
    pub(crate) to: u64,
    pub(crate) payload: Vec<u8>,
}

/// The domain could not be built: the revision would wrap or a projection did not encode. The
/// connection fails closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct QuestLogFault;

/// One connection's quest log.
#[derive(Debug, Default)]
pub(crate) struct QuestLogState {
    view: Option<RequestedView>,
    /// The copy version last projected.
    version: Option<u64>,
    /// The payload last sent.
    sent: Vec<u8>,
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
) -> QuestLog {
    let Some(source) = source else {
        return QuestLog::default();
    };
    let (content, copy) = (&*source.content.0, &*source.copy);
    QuestLog {
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
    /// The domain 16 snapshot of a new connection: no view, the carried tracked quests, above
    /// every revision the session has seen.
    pub(crate) fn snapshot(
        continuity: &mut QuestLogContinuity,
        observation: QuestLogObservation,
    ) -> Result<(Self, QuestLogDomain), QuestLogFault> {
        let source = match observation {
            QuestLogObservation::Changed(source) => Some(source),
            QuestLogObservation::Unavailable | QuestLogObservation::Unchanged => None,
        };
        let payload = encode_quest_log(&project(
            source.as_ref(),
            None,
            continuity.tracked.as_slice(),
        ))
        .map_err(|_| QuestLogFault)?;
        let to = advance(&mut continuity.revision)?;
        let state = Self {
            view: None,
            version: source.map(|source| source.version),
            sent: payload.clone(),
        };
        Ok((state, QuestLogDomain { to, payload }))
    }

    /// The copy version this connection last projected, for the owner's change check.
    pub(crate) const fn version(&self) -> Option<u64> {
        self.version
    }

    /// Command type 22 after its rate check: `None` (`REJECTED`, nothing changes) when the copy
    /// is not loaded, a named quest is not listed, or the domain does not encode; otherwise the
    /// whole-domain delta.
    pub(crate) fn query(
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
            QuestLogQuery::Quest { quest_index } => {
                if !content.is_listed(&*source.copy, *quest_index) {
                    return None;
                }
                (Some(RequestedView::Quest(*quest_index)), continuity.tracked)
            }
            QuestLogQuery::Track { quest_indexes } => {
                if !quest_indexes
                    .iter()
                    .all(|index| content.is_listed(&*source.copy, *index))
                {
                    return None;
                }
                (self.view, TrackedQuests::new(quest_indexes)?)
            }
        };
        let payload = encode_quest_log(&project(Some(&source), view, tracked.as_slice())).ok()?;
        self.view = view;
        self.version = Some(source.version);
        continuity.tracked = tracked;
        Some(self.delta(continuity, payload))
    }

    /// The refresh cadence: a delta when the changed copy changes what the domain shows.
    pub(crate) fn refresh(
        &mut self,
        continuity: &mut QuestLogContinuity,
        observation: QuestLogObservation,
    ) -> Result<Option<QuestLogDomain>, QuestLogFault> {
        let QuestLogObservation::Changed(source) = observation else {
            return Ok(None);
        };
        self.version = Some(source.version);
        let payload = encode_quest_log(&project(
            Some(&source),
            self.view,
            continuity.tracked.as_slice(),
        ))
        .map_err(|_| QuestLogFault)?;
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
