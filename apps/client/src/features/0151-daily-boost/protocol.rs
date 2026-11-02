use crate::play::{Outbox, mailbox::lock};
use oteryn_client_runtime::raw_feature::GatedCommandResult;
impl Outbox {
    pub(in crate::play) fn daily_boost_initialize(&self, selected: bool) {
        lock(&self.0).daily_boost.selected = selected;
    }
    pub(in crate::play) fn daily_boost_result(&self, result: &GatedCommandResult) {
        oteryn_client_ui::daily_boost::apply_result(&mut lock(&self.0).daily_boost, result);
    }
}
