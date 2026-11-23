use crate::play::{Outbox, mailbox::lock};
use oteryn_client_runtime::raw_feature::GatedCommandResult;
impl Outbox {
    pub(in crate::play) fn daily_boost_initialize(&self, selected: bool) {
        lock(&self.0).daily_boost.selected = selected;
    }
    pub(in crate::play) fn daily_boost_result(&self, result: &GatedCommandResult) {
        let mut mailbox = lock(&self.0);
        #[cfg(bevy_shell)]
        let previous = mailbox.daily_boost.boost.as_ref().map(|boost| boost.reset_epoch);
        oteryn_client_ui::daily_boost::apply_result(&mut mailbox.daily_boost, result);
        #[cfg(bevy_shell)]
        if let Some(card) = oteryn_client_ui::daily_boost::spotlight_cards(&mailbox.daily_boost).first()
            && Some(card.reset_epoch) != previous {
            mailbox.window_notice(crate::play::feature_local_text_messages::view::NoticeKind::Boosted, format!("{}: {}", card.kind, card.name));
        }
    }
}
