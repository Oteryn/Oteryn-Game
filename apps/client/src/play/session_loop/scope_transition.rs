//! The scope barrier a session error crosses: reset the source scope and publish the destination.
use super::{Outbox, Stepper, publish};

pub(super) fn initialize<T: Stepper>(
    session: &T,
    outbox: &Outbox,
) -> Result<(), oteryn_session::SessionError> {
    super::super::session_initialize::initialize(session, outbox)?;
    outbox.cast_control_initialize(session.server_mechanic_selected(290));
    outbox.cast_consent_initialize(session.server_mechanic_selected(292));
    outbox.cast_chat_initialize(
        session.server_mechanic_selected(290) && session.server_mechanic_selected(291),
    );
    Ok(())
}

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
    if !publish(session, outbox, held) {
        outbox.reset_unbound_scope();
    }
    outbox.take_scope_modal_reset();
    reset_modal(session, outbox, modal);
    outbox.hud_stamina_initialize(session.feature_selected(199));
    true
}

pub(super) fn reset_modal<T: Stepper>(
    session: &T,
    outbox: &Outbox,
    modal: &mut crate::play::feature_0450_server_modal::protocol::Driver,
) {
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
}

/// Returns true when the Returner query crossed a scope barrier.
pub(super) async fn query_initial_returner<T: Stepper>(
    session: &mut T,
    outbox: &Outbox,
    held: &mut Vec<oteryn_session::WorldSpatialEntity>,
    modal: &mut crate::play::feature_0450_server_modal::protocol::Driver,
    armed: &mut bool,
) -> bool {
    if !*armed {
        return false;
    }
    let Some(command) = outbox.returner_initial_request() else {
        return false;
    };
    match session.feature(&command).await {
        Ok(result) => outbox.daily_reward_returner_result(command.kind, &result),
        Err(error) => {
            if publish_scope_transition(&error, session, outbox, held, modal) {
                return true;
            }
            outbox.returner_unavailable();
        }
    }
    *armed = false;
    false
}
