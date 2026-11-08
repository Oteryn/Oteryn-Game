use crate::play::{Outbox, mailbox::lock};
use oteryn_client_runtime::raw_feature::GatedCommandResult;
impl Outbox {
    pub(in crate::play) fn weapon_proficiency_state_initialize(&self, selected: bool) {
        lock(&self.0).weapon_proficiency.state_selected = selected;
    }
    pub(in crate::play) fn weapon_proficiency_initialize(&self, selected: bool) {
        lock(&self.0).weapon_proficiency =
            oteryn_client_ui::weapon_proficiency::WeaponProficiencyPanel {
                selected,
                ..Default::default()
            };
    }
    pub(in crate::play) fn weapon_proficiency_result(&self, result: &GatedCommandResult) {
        oteryn_client_ui::weapon_proficiency::apply_result(
            &mut lock(&self.0).weapon_proficiency,
            result,
        );
    }
}
