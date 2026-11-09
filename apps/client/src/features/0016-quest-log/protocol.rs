use crate::play::{Outbox, Pushed};
use oteryn_client_runtime::quest_log::{QuestLog, QuestState, QuestUpdate};
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
        crate::play::mailbox::lock(&self.0).quest.state = state;
    }

    pub(in crate::play) fn quest_result(&self, result: &oteryn_session::GatedCommandResult) {
        let mut mailbox = crate::play::mailbox::lock(&self.0);
        mailbox.quest_pending = false;
        if result.status != oteryn_client_runtime::CommandStatus::Accepted {
            mailbox.quest.tracker.command_failed();
        }
        if mailbox.quest.pending_query.take()
            == Some(oteryn_client_runtime::quest_log::QuestLogQuery::ListOnly)
        {
            if result.status == oteryn_client_runtime::CommandStatus::Accepted {
                match oteryn_client_runtime::quest_log::decode_quest_log(&result.payload) {
                    Ok(QuestLog {
                        view: Some(oteryn_client_runtime::quest_log::QuestLogView::List(list)),
                        tracked,
                    }) if tracked.is_empty() => {
                        let panel = &mut mailbox.quest;
                        panel.tracker.observe(&list);
                        panel.state.list = list;
                        panel.tracker.confirm(&panel.state.log.tracked);
                        panel.state.refusal = None;
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

pub(in crate::play) fn reduce(
    panel: &mut oteryn_client_ui::quest_log::QuestPanel,
    deltas: Vec<QuestUpdate>,
) {
    for delta in deltas {
        if let Err(refusal) = panel.state.apply_update(&delta) {
            panel.state.refusal = Some(refusal);
            continue;
        }
        if matches!(&delta, QuestUpdate::Snapshot(_)) {
            panel.tracker.reconcile();
        }
        if matches!(
            &panel.state.log.view,
            Some(oteryn_client_runtime::quest_log::QuestLogView::List(_))
        ) {
            panel.tracker.observe(&panel.state.list);
        }
        panel.tracker.confirm(&panel.state.log.tracked);
        let missions = match &panel.state.log.view {
            Some(oteryn_client_runtime::quest_log::QuestLogView::Line(line)) => &line.missions[..],
            _ => &[],
        };
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
