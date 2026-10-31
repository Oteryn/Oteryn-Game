use crate::play::{Outbox, mailbox::lock};
use oteryn_protocol_oteryn::newhaven_vocation::{
    CAPABILITY_NEWHAVEN_VOCATION_V1, COMMAND_TYPE_NEWHAVEN_VOCATION, NewhavenChoiceResult,
};
use oteryn_protocol_oteryn::tutorial_notices::TutorialNotice;
use oteryn_session::{
    CommandStatus, GatedCommandResult, Session, SessionError, SessionEvent, SessionStream,
};

pub(in crate::play) async fn send<S: SessionStream>(
    session: &mut Session<S>,
    payload: &[u8],
) -> Result<GatedCommandResult, SessionError> {
    session
        .gated_command(
            CAPABILITY_NEWHAVEN_VOCATION_V1,
            COMMAND_TYPE_NEWHAVEN_VOCATION,
            payload,
        )
        .await
}

impl Outbox {
    pub(in crate::play) fn vocation_initialize(&self, selected: bool) {
        lock(&self.0).vocation.selected = selected;
    }

    pub(in crate::play) fn vocation_result(&self, result: &GatedCommandResult) {
        let chosen = result.status == CommandStatus::Accepted
            && matches!(
                NewhavenChoiceResult::decode(&result.payload),
                Some(NewhavenChoiceResult::Chosen | NewhavenChoiceResult::AlreadyChosen)
            );
        lock(&self.0).vocation.result(chosen);
    }
}

pub(in crate::play) fn notices(
    panel: &mut oteryn_client_ui::newhaven_vocation::VocationPanel,
    events: &[SessionEvent],
) {
    for event in events {
        if let SessionEvent::TutorialNotice(delta) = event
            && let TutorialNotice::Hint(id) = &delta.value
        {
            panel.hint(*id);
        }
    }
}
