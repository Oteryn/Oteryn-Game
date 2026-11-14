//! Atomic Bevy presentation adapter over the retained cast owner.
use crate::play::{
    PlayLink,
    mailbox::{SharedMailbox, lock},
};
use oteryn_client_ui::cast_control::Panel;

#[derive(Clone)]
pub(crate) struct CastControlSnapshot {
    pub(crate) panel: Panel,
    session: SharedMailbox,
}
impl CastControlSnapshot {
    pub(crate) fn same(&self, other: &Self) -> bool {
        std::sync::Arc::ptr_eq(&self.session, &other.session)
            && same_panel(&self.panel, &other.panel)
    }
}
fn same_panel(a: &Panel, b: &Panel) -> bool {
    (
        a.selected,
        a.consent_selected,
        a.consent_known,
        a.consent_revision,
        a.allow_shown,
        a.open,
        a.pending,
        a.active,
        a.revision,
        a.epoch,
    ) == (
        b.selected,
        b.consent_selected,
        b.consent_known,
        b.consent_revision,
        b.allow_shown,
        b.open,
        b.pending,
        b.active,
        b.revision,
        b.epoch,
    ) && (
        a.delay,
        a.cap,
        a.password,
        a.access_revision,
        a.viewers,
        &a.feedback,
    ) == (
        b.delay,
        b.cap,
        b.password,
        b.access_revision,
        b.viewers,
        &b.feedback,
    )
}
impl PlayLink {
    pub(crate) fn cast_control_snapshot(&self) -> CastControlSnapshot {
        CastControlSnapshot {
            panel: lock(&self.mailbox).cast_control.clone(),
            session: self.mailbox.clone(),
        }
    }
    pub(crate) fn cast_control_toggle_bound(&self) -> bool {
        let mut shared = lock(&self.mailbox);
        if self.commands.is_closed()
            || shared.world_input_blocked()
            || shared.lens.active
            || shared.server_modal.is_active()
            || !(shared.cast_control.selected || shared.cast_control.consent_selected)
        {
            return false;
        }
        shared.cast_control.open = !shared.cast_control.open;
        if shared.cast_control.open && !shared.cast_control.pending {
            let action = if shared.cast_control.selected { 5 } else { 2 };
            super::super::input::submit(&mut shared, &self.commands, action);
        }
        true
    }
    pub(crate) fn cast_control_close_bound(&self, expected: &CastControlSnapshot) -> bool {
        let mut shared = lock(&self.mailbox);
        if !std::sync::Arc::ptr_eq(&self.mailbox, &expected.session)
            || !same_panel(&shared.cast_control, &expected.panel)
            || shared.server_modal.is_active()
        {
            return false;
        }
        shared.cast_control.open = false;
        true
    }
    /// Draft settings only build the request; the retained panel changes on the server reply.
    pub(crate) fn cast_control_dispatch_bound(
        &self,
        expected: &CastControlSnapshot,
        action: usize,
        settings: &[String; 4],
        confirmed: bool,
    ) -> bool {
        let mut shared = lock(&self.mailbox);
        if !std::sync::Arc::ptr_eq(&self.mailbox, &expected.session)
            || !same_panel(&shared.cast_control, &expected.panel)
            || !shared.cast_control.open
            || shared.cast_control.pending
            || shared.world_input_blocked()
            || shared.lens.active
            || shared.server_modal.is_active()
            || self.commands.is_closed()
            || action > 5
            || (matches!(action, 0 | 1 | 3 | 4) && !confirmed)
        {
            return false;
        }
        let mut draft = shared.cast_control.clone();
        if action == 0 {
            let parsed = (|| {
                Some((
                    settings[0].parse().ok()?,
                    settings[1].parse().ok()?,
                    match settings[2].as_str() {
                        "0" => false,
                        "1" => true,
                        _ => return None,
                    },
                    settings[3].parse().ok()?,
                ))
            })();
            let Some((delay, cap, password, access_revision)) = parsed else {
                return false;
            };
            draft.delay = delay;
            draft.cap = cap;
            draft.password = password;
            draft.access_revision = access_revision;
        }
        let message = if (2..=4).contains(&action) {
            oteryn_client_ui::cast_consent::protocol::request(
                &draft,
                match action {
                    2 => None,
                    3 => Some(true),
                    _ => Some(false),
                },
            )
        } else {
            oteryn_client_ui::cast_control::command(&draft, action)
        };
        let Ok(payload) =
            message.and_then(|message| message.encode().map_err(|_| "Invalid settings"))
        else {
            return false;
        };
        let command = oteryn_client_runtime::server_mechanic::ServerMechanicCommand {
            capability: if (2..=4).contains(&action) { 292 } else { 290 },
            command_type: if (2..=4).contains(&action) { 503 } else { 502 },
            payload,
        };
        if self
            .commands
            .send(crate::combat_input::PlayCommand::ServerMechanic(command))
            .is_err()
        {
            return false;
        }
        shared.cast_control.pending = true;
        true
    }
}
#[cfg(test)]
impl crate::play::Outbox {
    pub(crate) fn bevy_cast_end(&self) {
        self.logged_out();
    }
    pub(crate) fn bevy_cast_initialize(&self) {
        self.cast_control_initialize(true);
        self.cast_consent_initialize(true);
    }
    pub(crate) fn bevy_cast_result(
        &self,
        message: oteryn_client_runtime::cast::Message,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.cast_control_result(
            &oteryn_client_runtime::server_mechanic::ServerMechanicResponse {
                command_id: 1,
                server_sequence: 1,
                status: oteryn_session::CommandStatus::Accepted,
                payload: message.encode().map_err(|e| format!("{e:?}"))?,
            },
        );
        Ok(())
    }
    pub(crate) fn bevy_cast_consent_result(
        &self,
        result: &oteryn_client_runtime::server_mechanic::ServerMechanicResponse,
    ) {
        self.cast_consent_result(result);
    }
}
