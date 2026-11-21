//! The scope barrier a session error crosses: reset the source scope and publish the destination.
use super::{Outbox, Stepper, publish};

pub(super) fn publish_scope_transition<T: Stepper>(
    error: &oteryn_session::SessionError,
    session: &mut T,
    outbox: &Outbox,
    held: &mut Vec<oteryn_session::WorldSpatialEntity>,
    modal: &mut crate::play::feature_0450_server_modal::protocol::Driver,
) -> bool {
    if !matches!(
        error,
        oteryn_session::SessionError::ScopeChanged
            | oteryn_session::SessionError::ActivityScopeChanged
    ) {
        return false;
    }
    // Publish the destination and discard unsent source actions through the
    // same scope barrier, whether a command or an automatic query saw it.
    // Dropping the source driver closes its answer channel, including native dialogs.
    modal.reset_scope(session.feature_selected(450));
    *modal = std::mem::replace(
        modal,
        crate::play::feature_0450_server_modal::protocol::Driver::new(false),
    )
    .with_defaults(session.feature_selected(505))
    .with_input(session.feature_selected(
        oteryn_session::property_services::server_modal::input::CAPABILITY_SERVER_MODAL_INPUT_V1,
    ))
    .with_outbox(outbox);
    {
        let mut mailbox = crate::play::mailbox::lock(&outbox.0);
        mailbox.cancel_activity_requests();
        mailbox.reset_server_message_namespace();
        mailbox.activity_reset_commands = true;
    }
    outbox.hud_stamina_initialize(session.feature_selected(199));
    publish(session, outbox, held);
    true
}
