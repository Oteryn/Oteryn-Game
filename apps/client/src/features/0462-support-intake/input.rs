#[cfg(feature = "bevy-shell")]
pub(crate) mod evidence_owner;
use crate::play::PlayLink;
impl PlayLink {
    /// Queue only evidence previously approved in the native consent dialogs.
    pub fn queue_support_at(&self, directory: &std::path::Path) {
        #[cfg(feature = "bevy-shell")]
        evidence_owner::with_owner(self.support_evidence_key(), &self.commands, |owner| {
            for evidence in evidence_owner::evidence_at(directory) {
                owner.observe(evidence);
            }
        });
        crate::support_delivery::queue_support_at(&self.commands, directory);
    }
}
#[cfg(all(test, feature = "bevy-shell"))]
#[path = "tests.rs"]
mod tests;

#[cfg(feature = "bevy-shell")]
impl PlayLink {
    fn support_evidence_key(&self) -> usize {
        std::sync::Arc::as_ptr(&self.mailbox) as usize
    }
    pub(crate) fn support_evidence_watch(&self) -> std::sync::Arc<std::sync::atomic::AtomicBool> {
        evidence_owner::with_owner(self.support_evidence_key(), &self.commands, |owner| {
            owner.watch()
        })
    }
    pub(crate) fn support_evidence_snapshot(&self) -> Vec<evidence_owner::Evidence> {
        let _mailbox = crate::play::mailbox::lock(&self.mailbox);
        evidence_owner::with_owner(self.support_evidence_key(), &self.commands, |owner| {
            owner.rows().to_vec()
        })
    }
    pub(crate) fn support_evidence_rows(&self) -> Vec<evidence_owner::Evidence> {
        let _mailbox = crate::play::mailbox::lock(&self.mailbox);
        evidence_owner::with_owner(self.support_evidence_key(), &self.commands, |owner| {
            if let Some(directory) = crate::support_delivery::directory() {
                owner.refresh(&directory);
            }
            owner.rows().to_vec()
        })
    }
    pub(crate) fn support_evidence_submit(
        &self,
        expected: &evidence_owner::Evidence,
        consent: bool,
        blocked: bool,
    ) -> bool {
        let mailbox = crate::play::mailbox::lock(&self.mailbox);
        if mailbox.custom_hotkeys_ended
            || mailbox.server_modal.is_active()
            || self.commands.is_closed()
            || !evidence_owner::matches_file(expected)
        {
            return false;
        }
        evidence_owner::with_owner(self.support_evidence_key(), &self.commands, |owner| {
            owner
                .submit(expected, consent, blocked, |command, path| {
                    self.commands
                        .send(crate::combat_input::PlayCommand::SupportDelivery(
                            command, path,
                        ))
                        .is_ok()
                })
                .is_some()
        })
    }
}
#[cfg(feature = "bevy-shell")]
impl crate::play::Outbox {
    pub(in crate::play) fn support_evidence_reply(
        &self,
        path: &std::path::Path,
        command: &oteryn_client_runtime::raw_feature::FeatureCommand,
        result: &oteryn_session::GatedCommandResult,
    ) {
        let mailbox = crate::play::mailbox::lock(&self.0);
        if mailbox.custom_hotkeys_ended {
            return;
        }
        evidence_owner::retain_reply(
            std::sync::Arc::as_ptr(&self.0) as usize,
            path,
            command,
            result,
        );
    }
}
