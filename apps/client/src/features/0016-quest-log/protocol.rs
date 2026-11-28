use crate::play::{Outbox, Pushed};
use oteryn_client_runtime::{
    CommandStatus,
    quest_log::{MAX_FREE_TRACKED_MISSIONS, QuestLog, QuestRefusal, QuestState, QuestUpdate},
};
use oteryn_session::{AppliedDelta, Session, SessionError, SessionStream};

pub(in crate::play) fn quest_log(pushed: &mut Pushed, delta: AppliedDelta<QuestLog>) {
    pushed.quests.push(QuestUpdate::Delta(delta));
}

pub(in crate::play) fn initial<S: SessionStream>(session: &Session<S>) -> QuestState {
    // Session admission has already validated the selected domain's snapshot.
    QuestState::from_snapshot(session.selected_capabilities(), session.gated_snapshot(16))
        .unwrap_or_else(|refusal| QuestState {
            refusal: Some(refusal),
            ..QuestState::default()
        })
}

pub(in crate::play) async fn send<S: SessionStream>(
    session: &mut Session<S>,
    payload: &[u8],
) -> Result<oteryn_session::GatedCommandResult, SessionError> {
    oteryn_client_runtime::quest_log::send(session, payload).await
}

impl Outbox {
    pub(in crate::play) fn quest_initialize(&self, state: QuestState) {
        let mut mailbox = crate::play::mailbox::lock(&self.0);
        let panel = &mut mailbox.quest;
        panel.pinned.clear();
        panel.hidden.clear();
        panel.show_hidden = false;
        panel.tracker_selection = None;
        panel.pending_missions = None;
        if matches!(
            &state.log.view,
            Some(oteryn_client_runtime::quest_log::QuestLogView::List(_))
        ) {
            panel.tracker.observe(&state.list);
        }
        panel.state = state;
    }

    pub(in crate::play) fn quest_result(&self, result: &oteryn_session::GatedCommandResult) {
        let mut mailbox = crate::play::mailbox::lock(&self.0);
        mailbox.quest_pending = false;
        let panel = &mut mailbox.quest;
        if let Some(requested) = &panel.pending_missions {
            if let Err(refusal) = panel.state.record_result(result.status, &result.payload) {
                panel.state.refusal = Some(refusal);
            } else if result.status == CommandStatus::Rejected
                && requested.len() > MAX_FREE_TRACKED_MISSIONS
            {
                panel.state.refusal = Some(QuestRefusal::MissionTrackingRejected);
            }
            if panel.state.refusal.is_some() {
                panel.pending_missions = None;
            }
            return;
        }
        if result.status != oteryn_client_runtime::CommandStatus::Accepted {
            mailbox.quest.tracker.command_failed();
        }
        if mailbox.quest.pending_query.take()
            == Some(oteryn_client_runtime::quest_log::QuestLogQuery::ListOnly)
        {
            if result.status == oteryn_client_runtime::CommandStatus::Accepted {
                match oteryn_client_runtime::quest_log::decode_quest_log(&result.payload) {
                    Ok(QuestLog {
                        map_marks: _,
                        view: Some(oteryn_client_runtime::quest_log::QuestLogView::List(list)),
                        tracked,
                    }) if tracked.is_empty() => {
                        let panel = &mut mailbox.quest;
                        #[cfg(bevy_shell)]
                        let prior = before_list(panel);
                        panel.tracker.observe(&list);
                        panel.state.list = list;
                        panel.tracker.confirm(&panel.state.log.tracked);
                        panel.state.refusal = None;
                        #[cfg(bevy_shell)]
                        if let Some(prior) = prior {
                            let changes = list_changes(panel, &prior);
                            mailbox.achievement_banners.extend(changes);
                        }
                    }
                    _ => {
                        mailbox.quest.state.refusal =
                            Some(oteryn_client_runtime::quest_log::QuestRefusal::Malformed)
                    }
                }
            } else if let Err(refusal) = mailbox
                .quest
                .state
                .record_result(result.status, &result.payload)
            {
                mailbox.quest.state.refusal = Some(refusal);
            }
            return;
        }
        if let Err(refusal) = mailbox
            .quest
            .state
            .record_result(result.status, &result.payload)
        {
            mailbox.quest.state.refusal = Some(refusal);
        }
    }
}

#[cfg(bevy_shell)]
fn before_list(
    panel: &oteryn_client_ui::quest_log::QuestPanel,
) -> Option<std::collections::BTreeMap<u32, bool>> {
    (panel.tracker.observed_list()
        || !panel.state.list.is_empty()
        || matches!(
            panel.state.log.view,
            Some(oteryn_client_runtime::quest_log::QuestLogView::List(_))
        ))
    .then(|| {
        panel
            .state
            .list
            .iter()
            .map(|q| (q.quest_index, q.completed))
            .collect()
    })
}

#[cfg(bevy_shell)]
fn list_changes(
    panel: &oteryn_client_ui::quest_log::QuestPanel,
    prior: &std::collections::BTreeMap<u32, bool>,
) -> Vec<String> {
    panel
        .state
        .list
        .iter()
        .filter_map(|entry| {
            let label = match prior.get(&entry.quest_index) {
                Some(false) if entry.completed => "Quest completed",
                None if !entry.completed => "Quest started",
                _ => return None,
            };
            Some(format!("{label}: {}", panel.title(entry.quest_index)))
        })
        .collect()
}

pub(in crate::play) fn reduce(
    mailbox: &mut crate::play::mailbox::Mailbox,
    deltas: Vec<QuestUpdate>,
) {
    let panel = &mut mailbox.quest;
    for delta in deltas {
        #[cfg(bevy_shell)]
        let prior = matches!(&delta, QuestUpdate::Delta(_))
            .then(|| before_list(panel))
            .flatten();
        if let Err(refusal) = panel.state.apply_update(&delta) {
            panel.state.refusal = Some(refusal);
            continue;
        }
        #[cfg(bevy_shell)]
        if let Some(prior) = prior {
            mailbox
                .achievement_banners
                .extend(list_changes(panel, &prior));
        }
        if matches!(&delta, QuestUpdate::Snapshot(_)) {
            panel.tracker.reconcile();
            panel.pending_missions = None;
        }
        if panel.pending_missions.as_ref().is_some_and(|expected| {
            panel.confirmed_missions().eq(expected.iter().copied())
        }) {
            panel.pending_missions = None;
        }
        if matches!(
            &panel.state.log.view,
            Some(oteryn_client_runtime::quest_log::QuestLogView::List(_))
        ) {
            panel.tracker.observe(&panel.state.list);
        }
        panel.tracker.confirm(&panel.state.log.tracked);
        if panel.tracker_selection.is_some() && panel.tracker_line().is_none() {
            panel.tracker_selection = None;
        }
        let missions = panel
            .journal_line()
            .map_or(&[][..], |line| line.missions.as_slice());
        if !missions
            .iter()
            .any(|m| Some(m.mission_index) == panel.mission)
        {
            panel.mission = missions.first().map(|m| m.mission_index);
        }
    }
}

/// Other gated features can subscribe to the same session notification when they cache a domain.
pub(in crate::play) fn snapshot_replaced(
    pushed: &mut Pushed,
    snapshot: AppliedDelta<oteryn_session::GatedSnapshot>,
) {
    if snapshot.value.domain_id == 16 {
        pushed.quests.push(QuestUpdate::Snapshot(snapshot.value));
    }
}
