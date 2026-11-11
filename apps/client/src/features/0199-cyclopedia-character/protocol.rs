use crate::play::{Outbox, mailbox::lock};
use oteryn_client_runtime::raw_feature::GatedCommandResult;
impl Outbox {
    pub(in crate::play) fn cyclopedia_character_initialize(&self, selected: bool) {
        let mut mailbox = lock(&self.0);
        mailbox.cyclopedia_character.selected = selected;
        mailbox.cyclopedia_character.blessings = Default::default();
    }
    /// Capabilities 487 and 488 append fields to the tab replies of a session that selected them.
    pub(in crate::play) fn cyclopedia_character_extensions(&self, vocation: bool, deaths: bool) {
        lock(&self.0).cyclopedia_character.extensions =
            oteryn_client_ui::cyclopedia_character::TabExtensions {
                vocation,
                death_details: deaths,
            };
    }
    pub(in crate::play) fn cyclopedia_character_result(&self, result: &GatedCommandResult) {
        oteryn_client_ui::cyclopedia_character::apply_result(
            &mut lock(&self.0).cyclopedia_character,
            result,
        );
    }
}
impl Outbox {
    /// Capability 497: the own sanction record window; a new session starts without a record.
    pub(in crate::play) fn sanction_initialize(&self, selected: bool) {
        let mut mailbox = lock(&self.0);
        mailbox.sanctions = Default::default();
        mailbox.sanctions.selected = selected;
    }
    pub(in crate::play) fn sanction_record_result(&self, result: &GatedCommandResult) {
        oteryn_client_ui::sanction_record::apply_result(&mut lock(&self.0).sanctions, result);
    }
}
