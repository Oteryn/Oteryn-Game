use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::daily_boost::DailyBoostPanel;
impl PlayLink {
    #[must_use]
    pub fn daily_boost_panel(&self) -> DailyBoostPanel {
        lock(&self.mailbox).daily_boost.clone()
    }
}

/// Daily boost and active XP boosts for the shared frontend; Refresh repeats both queries.
pub(in crate::play) fn form(_link: &PlayLink) -> crate::play::feature_0115_mail::view::form::Form {
    crate::play::feature_0115_mail::view::form::Form {
        title: tr!("Daily boost (Ctrl+B)"),
        labels: &[],
        buttons: &["Refresh"],
        snapshot: |mailbox| {
            let mut text = oteryn_client_ui::daily_boost::panel_text(&mailbox.daily_boost);
            let potions = &mailbox.potion_durations;
            if let (Some(snapshot), Some(at)) = (&potions.snapshot, potions.received_at) {
                let elapsed = u64::try_from(at.elapsed().as_millis()).unwrap_or(u64::MAX);
                text.push_str(&oteryn_client_ui::daily_boost::xp_boost_lines(
                    &snapshot.active,
                    elapsed,
                ));
            }
            (
                super::input::window_open(mailbox),
                mailbox.daily_boost.pending,
                Vec::new(),
                text,
            )
        },
        set_open: |mailbox, value| mailbox.daily_boost.open = value,
        submit: |mailbox, commands, _, _| super::input::refresh(mailbox, commands),
    }
}
