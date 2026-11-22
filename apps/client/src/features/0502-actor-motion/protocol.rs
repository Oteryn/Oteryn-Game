#[cfg(bevy_shell)]
use crate::play::PlayLink;
use crate::play::{
    Outbox,
    mailbox::{Mailbox, lock},
};

#[derive(Debug, Clone, Copy)]
pub(in crate::play) struct PendingMount {
    riding: bool,
    accepted: bool,
}

impl Outbox {
    pub(in crate::play) fn actor_motion_initialize(&self, selected: bool) {
        let mut mailbox = lock(&self.0);
        mailbox.motion_selected = selected;
        mailbox.mount_pending = None;
    }

    pub(in crate::play) fn mount_result(&self, status: oteryn_session::CommandStatus) {
        let mut mailbox = lock(&self.0);
        if let Some(pending) = mailbox.mount_pending {
            mailbox.mount_pending =
                (status == oteryn_session::CommandStatus::Accepted).then_some(PendingMount {
                    accepted: true,
                    ..pending
                });
        }
    }
}

impl Mailbox {
    fn riding(&self) -> Option<bool> {
        if !self.motion_selected {
            return None;
        }
        Some(self.bar_actions.own_actor?.motion?.mount_look_type != 0)
    }

    pub(in crate::play) fn mount_actor_published(&mut self) {
        // The command reply can arrive before the corresponding actor delta.
        if let Some(PendingMount {
            riding,
            accepted: true,
        }) = self.mount_pending
            && self.riding() == Some(riding)
        {
            self.mount_pending = None;
        }
    }
}

#[cfg(bevy_shell)]
impl PlayLink {
    pub(crate) fn riding(&self) -> Option<bool> {
        lock(&self.mailbox).riding()
    }

    pub(crate) fn mount_request(&self) {
        let mut mailbox = lock(&self.mailbox);
        if mailbox.mount_pending.is_some() || mailbox.world_input_blocked() {
            return;
        }
        let Some(riding) = mailbox.riding() else {
            return;
        };
        let riding = !riding;
        if let Some(command) =
            oteryn_client_ui::actor_motion::mount_command(mailbox.motion_selected, riding)
        {
            mailbox.mount_pending = self
                .commands
                .send(crate::combat_input::PlayCommand::Feature(command))
                .is_ok()
                .then_some(PendingMount {
                    riding,
                    accepted: false,
                });
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

#[cfg(all(test, bevy_shell))]
#[path = "tests.rs"]
mod tests;
