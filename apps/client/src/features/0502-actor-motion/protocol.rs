#[cfg(bevy_shell)]
use crate::play::PlayLink;
use crate::play::{Outbox, mailbox::lock};

impl Outbox {
    pub(in crate::play) fn actor_motion_initialize(&self, selected: bool) {
        let mut mailbox = lock(&self.0);
        mailbox.motion_selected = selected;
        mailbox.mount_pending = false;
    }
    pub(in crate::play) fn mount_result(&self) {
        lock(&self.0).mount_pending = false;
    }
}

#[cfg(bevy_shell)]
impl PlayLink {
    pub(crate) fn riding(&self) -> Option<bool> {
        let mailbox = lock(&self.mailbox);
        Self::riding_locked(&mailbox)
    }

    fn riding_locked(mailbox: &crate::play::mailbox::Mailbox) -> Option<bool> {
        if !mailbox.motion_selected {
            return None;
        }
        let own = mailbox.bar_actions.own_actor?;
        Some(own.motion?.mount_look_type != 0)
    }

    pub(crate) fn mount_request(&self) {
        let mut mailbox = lock(&self.mailbox);
        if mailbox.mount_pending || mailbox.world_input_blocked() {
            return;
        }
        let Some(riding) = Self::riding_locked(&mailbox) else {
            return;
        };
        if let Some(command) =
            oteryn_client_ui::actor_motion::mount_command(mailbox.motion_selected, !riding)
        {
            mailbox.mount_pending = self
                .commands
                .send(crate::combat_input::PlayCommand::Feature(command))
                .is_ok();
        }
    }

    pub(crate) fn mount_input(
        &self,
        events: &[oteryn_input_actions::NormalizedInputEvent],
    ) -> bool {
        if !oteryn_client_ui::actor_motion::mount_key(events) {
            return false;
        }
        self.mount_request();
        true
    }
}

#[cfg(all(test, bevy_shell))]
impl Outbox {
    pub(crate) fn bevy_mount_test_actor(&self, actor: oteryn_session::WorldSpatialEntity) {
        self.actor_motion_initialize(true);
        self.hotkey_own_actor(Some(actor));
    }
}
