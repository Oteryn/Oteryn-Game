use crate::combat_input::PlayCommand;
use crate::play::{Outbox, Pushed, Stepper, mailbox::lock};
use oteryn_client_runtime::tutorial_notices::{self, ServerSaveCountdown, TutorialNotice};
use oteryn_session::AppliedDelta;
/// Initialize transient notices before entering the shared session loop.
pub async fn run_session<T: Stepper>(
    session: T,
    commands: tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    outbox: Outbox,
) {
    outbox.notices_initialize(session.feature_selected(31), session.feature_selected(75));
    crate::play::session_loop::run_session(session, commands, outbox).await;
}
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
