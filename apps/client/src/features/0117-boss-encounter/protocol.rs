use crate::play::{Outbox, mailbox::lock};
use oteryn_client_runtime::raw_feature::GatedCommandResult;
impl Outbox {
    pub(in crate::play) fn boss_encounter_initialize(&self, selected: bool) {
        let mut mailbox = lock(&self.0);
        mailbox.boss_encounter = oteryn_client_ui::boss_encounter::BossEncounterPanel {
            selected,
            ..Default::default()
        };
        mailbox.boss_encounter_shutdown = false;
    }
    pub(in crate::play) fn boss_encounter_result(&self, response: &GatedCommandResult) {
        oteryn_client_ui::boss_encounter::apply_result(&mut lock(&self.0).boss_encounter, response);
    }
}

impl crate::play::mailbox::Mailbox {
    pub(in crate::play) fn boss_encounter_end(&mut self) {
        self.boss_encounter = oteryn_client_ui::boss_encounter::BossEncounterPanel {
            feedback: Some(tr!("The session has ended")),
            ..Default::default()
        };
        self.boss_encounter_shutdown = true;
    }
}

/// Decode this feature's committed scope receipt before the shared mailbox invalidates
/// origin controls and applies the fresh destination's native world presentation.
pub(in crate::play) fn activity_scope_receipt(
    events: &[oteryn_session::SessionEvent],
) -> Option<oteryn_session::activity_scope::ActivityScopeSnapshot> {
    use oteryn_session::SessionEvent;
    use oteryn_session::activity_scope::{STATE_DOMAIN_ACTIVITY_SCOPE, decode_activity_scope};
    events
        .iter()
        .filter_map(|event| match event {
            SessionEvent::GatedSnapshotReplaced(delta)
                if delta.value.domain_id == STATE_DOMAIN_ACTIVITY_SCOPE =>
            {
                decode_activity_scope(&delta.value.payload).ok()
            }
            _ => None,
        })
        .next_back()
}
