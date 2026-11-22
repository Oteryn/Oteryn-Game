#[cfg(bevy_shell)]
use crate::play::PlayLink;
use crate::play::{
    Outbox,
    mailbox::{Mailbox, lock},
};
use oteryn_session::{CommandStatus, SessionEvent};

#[derive(Debug, Clone, Copy, Default)]
pub(in crate::play) struct PendingMount {
    sent: bool,
    accepted: bool,
    motion_seen: bool,
}

impl Outbox {
    pub(in crate::play) fn actor_motion_initialize(&self, selected: bool) {
        let mut mailbox = lock(&self.0);
        mailbox.motion_selected = selected;
        mailbox.mount_pending = None;
    }

    pub(in crate::play) fn mount_sent(&self) {
        if let Some(pending) = &mut lock(&self.0).mount_pending {
            pending.sent = true;
        }
    }

    pub(in crate::play) fn mount_result(&self, status: CommandStatus) {
        let mut mailbox = lock(&self.0);
        if let Some(pending) = mailbox.mount_pending {
            mailbox.mount_pending = (status == CommandStatus::Accepted && !pending.motion_seen)
                .then_some(PendingMount {
                    accepted: true,
                    ..pending
                });
        }
    }
}

impl Mailbox {
    #[cfg(bevy_shell)]
    fn riding(&self) -> Option<bool> {
        if !self.motion_selected {
            return None;
        }
        Some(self.bar_actions.own_actor?.motion?.mount_look_type != 0)
    }

    pub(in crate::play) fn mount_actor_published(&mut self, events: &[SessionEvent]) {
        let Some(pending) = self.mount_pending.as_mut().filter(|pending| pending.sent) else {
            return;
        };
        let Some(own) = self.bar_actions.own_actor else {
            return;
        };
        // Publish also projects the cached actor on idle turns. Only fresh own-actor
        // motion carried by the visibility stream can finish the command.
        if events.iter().any(|event| match event {
            SessionEvent::WorldSpatialEntities(delta) => delta
                .value
                .enter
                .iter()
                .chain(&delta.value.update)
                .any(|actor| actor.entity == own.entity && actor.motion.is_some()),
            SessionEvent::GatedSnapshotReplaced(snapshot) => {
                snapshot.value.domain_id == 1 && own.motion.is_some()
            }
            _ => false,
        }) {
            pending.motion_seen = true;
            if pending.accepted {
                self.mount_pending = None;
            }
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
                .then(PendingMount::default);
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
