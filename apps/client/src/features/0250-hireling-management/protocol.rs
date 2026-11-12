use crate::play::{Outbox, mailbox::lock};
use oteryn_client_runtime::raw_feature::GatedCommandResult;
impl Outbox {
    pub(in crate::play) fn hireling_management_initialize(&self, selected: bool) {
        lock(&self.0).hireling_management.selected = selected;
    }
    pub(in crate::play) fn hireling_management_result(&self, result: &GatedCommandResult) {
        let mut m = lock(&self.0);
        let refreshing = matches!(
            m.hireling_management.pending,
            Some(oteryn_client_ui::hireling_management::wire::Request::List)
        );
        let state = matches!(
            oteryn_client_ui::hireling_management::wire::Reply::decode(&result.payload),
            Some(oteryn_client_ui::hireling_management::wire::Reply::State(_))
        );
        if oteryn_client_ui::hireling_management::apply_result(&mut m.hireling_management, result)
            && refreshing
            && state
        {
            m.store.purchase = None;
        }
    }
}
