use crate::play::{Outbox, Pushed};
use oteryn_client_runtime::quest_log::{QuestLog, QuestState};
use oteryn_session::{AppliedDelta, Session, SessionError, SessionStream};

pub(in crate::play) fn quest_log(pushed: &mut Pushed, delta: AppliedDelta<QuestLog>) {
    pushed.quests.push(delta);
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
    deltas: Vec<AppliedDelta<QuestLog>>,
) {
    for delta in deltas {
        if let Err(refusal) = panel.state.apply(&delta) {
            panel.state.refusal = Some(refusal);
        }
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
