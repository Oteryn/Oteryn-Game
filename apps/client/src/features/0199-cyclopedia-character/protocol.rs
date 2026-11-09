use crate::play::{Outbox, mailbox::lock};
use oteryn_client_runtime::raw_feature::GatedCommandResult;
impl Outbox {
    pub(in crate::play) fn cyclopedia_character_initialize(&self, selected: bool) {
        let mut mailbox = lock(&self.0);
        mailbox.cyclopedia_character.selected = selected;
        mailbox.cyclopedia_character.blessings = Default::default();
    }
    pub(in crate::play) fn cyclopedia_character_result(&self, result: &GatedCommandResult) {
        oteryn_client_ui::cyclopedia_character::apply_result(
            &mut lock(&self.0).cyclopedia_character,
            result,
        );
    }
}
