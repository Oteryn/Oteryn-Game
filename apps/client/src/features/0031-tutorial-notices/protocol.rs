use crate::play::{Outbox, Pushed, mailbox::lock};
use oteryn_client_runtime::tutorial_notices::{self, ServerSaveCountdown, TutorialNotice};
use oteryn_session::AppliedDelta;
impl Outbox {
    pub(in crate::play) fn notices_initialize(&self, tutorial: bool, save: bool) {
        let mut mailbox = lock(&self.0);
        mailbox.notices.tutorial_selected = tutorial;
        mailbox.notices.save_selected = save;
    }
}
pub(in crate::play) fn tutorial_notice(pushed: &mut Pushed, delta: AppliedDelta<TutorialNotice>) {
    oteryn_client_ui::tutorial_notices::apply(
        &mut pushed.notices,
        tutorial_notices::tutorial(delta.value),
    );
}
pub(in crate::play) fn server_save(pushed: &mut Pushed, delta: AppliedDelta<ServerSaveCountdown>) {
    oteryn_client_ui::server_save_countdown::apply(&mut pushed.notices, delta.value);
}
