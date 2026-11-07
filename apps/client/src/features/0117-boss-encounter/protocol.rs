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
            feedback: Some("The session has ended"),
            ..Default::default()
        };
        self.boss_encounter_shutdown = true;
    }
}
