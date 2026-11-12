use crate::play::{PlayLink, mailbox::lock};
#[path = "coin_shop.rs"]
mod coin_shop;
impl PlayLink {
    pub fn store_gift_open(&self) {
        #[cfg(windows)]
        open_gift(self);
        #[cfg(not(windows))]
        {
            lock(&self.mailbox).store.gift_open = true;
        }
    }

    #[must_use]
    pub fn store_panel(&self) -> oteryn_client_ui::store::StorePanel {
        lock(&self.mailbox).store.clone()
    }
}
#[cfg(any(windows, test))]
#[path = "native.rs"]
pub(in crate::play) mod native;
#[cfg(windows)]
pub(in crate::play) use native::open_window;

#[path = "../local-premium/game_play.rs"]
mod premium_game;
#[cfg(windows)]
#[path = "../local-premium/game_native.rs"]
mod premium_native;

#[cfg(windows)]
fn open_gift(link: &PlayLink) {
    use crate::play::feature_0115_mail::view::native::{Form, open};
    open(
        link,
        Form {
            title: "In-game coin gift",
            labels: &["Recipient name", "Coins"],
            buttons: &[
                "Review recipient and amount",
                "Confirm reviewed gift",
                "Close",
                "Retry confirmation",
                "Refresh receipt",
            ],
            snapshot: |mailbox| {
                let p = &mailbox.store;
                (
                    p.gift_open,
                    p.gift.pending.is_some(),
                    p.gift_fields.to_vec(),
                    p.feedback.clone().unwrap_or_else(|| "Review the exact recipient and amount before confirming. Gifts cannot be recalled.".into()),
                )
            },
            set_open: |mailbox, open| {
                mailbox.store.gift_open = open;
                if !open {
                    mailbox.store.gift_reviewed = None;
                }
            },
            submit: |mailbox, commands, action, fields| {
                if let Ok(fields) = <[String; 2]>::try_from(fields) {
                    super::input::gift::submit(mailbox, commands, action, fields);
                }
            },
        },
    );
}
