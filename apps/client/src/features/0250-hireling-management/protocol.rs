use crate::play::{Outbox, mailbox::lock};
use oteryn_client_runtime::raw_feature::GatedCommandResult;
impl Outbox {
    pub(in crate::play) fn hireling_management_initialize(&self, selected: bool) {
        lock(&self.0).hireling_management.selected = selected;
    }
    pub(in crate::play) fn hireling_management_result(&self, result: &GatedCommandResult) {
        oteryn_client_ui::hireling_management::apply_result(
            &mut lock(&self.0).hireling_management,
            result,
        );
    }
}
