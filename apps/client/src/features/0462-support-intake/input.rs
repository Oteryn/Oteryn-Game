use crate::play::PlayLink;
impl PlayLink {
    /// Queue only evidence previously approved in the native consent dialogs.
    pub fn queue_support_at(&self, directory: &std::path::Path) {
        #[cfg(feature = "bevy-shell")]
        crate::support_delivery::with_owner(self.support_evidence_key(), &self.commands, |owner| {
            for evidence in crate::support_delivery::evidence_at(directory) {
                owner.observe(evidence.clone());
                owner.submit(&evidence, true, false, |_| true);
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
    pub(crate) fn support_evidence_rows(&self) -> Vec<crate::support_delivery::Evidence> {
        let _mailbox = crate::play::mailbox::lock(&self.mailbox);
        crate::support_delivery::with_owner(self.support_evidence_key(), &self.commands, |owner| {
            if let Some(directory) = crate::support_delivery::directory() {
                for evidence in crate::support_delivery::evidence_at(&directory) {
                    owner.observe(evidence);
                }
            }
            owner.rows().to_vec()
        })
    }
    pub(crate) fn support_evidence_submit(
        &self,
        expected: &crate::support_delivery::Evidence,
        consent: bool,
        blocked: bool,
    ) -> bool {
        let mailbox = crate::play::mailbox::lock(&self.mailbox);
        if mailbox.server_modal.is_active()
            || self.commands.is_closed()
            || crate::support_delivery::evidence_at(
                expected.path.parent().unwrap_or(std::path::Path::new("")),
            )
            .iter()
            .all(|row| row.path != expected.path || row.text != expected.text)
        {
            return false;
        }
        crate::support_delivery::with_owner(self.support_evidence_key(), &self.commands, |owner| {
            owner
                .submit(expected, consent, blocked, |command| {
                    self.commands
                        .send(crate::combat_input::PlayCommand::SupportDelivery(
                            command,
                            expected.path.clone(),
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
        crate::support_delivery::retain_reply(
            std::sync::Arc::as_ptr(&self.0) as usize,
            path,
            command,
            result,
        );
    }
}
