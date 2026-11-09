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

#[cfg(windows)]
fn open_gift(link: &PlayLink) {
    use crate::play::feature_0115_mail::view::native::{Form, open};
    open(
        link,
        Form {
            title: "In-game coin gift — preview",
            labels: &["Recipient name", "Transferable coins"],
            buttons: &[
                "Review recipient and amount",
                "Confirm gift (disabled)",
                "Cancel",
            ],
            snapshot: |mailbox| {
                let p = &mailbox.store;
                (
                    p.gift_open,
                    p.pending,
                    p.gift_fields.to_vec(),
                    lfmt!(
                        "Coin transfers are unavailable in this session.\r\n{}",
                        p.feedback.as_deref().unwrap_or(tr!(
                            "Review a recipient and amount. No coins are moved by a preview."
                        ))
                    ),
                )
            },
            set_open: |mailbox, open| {
                mailbox.store.gift_open = open;
                if !open {
                    mailbox.store.gift_reviewed = None;
                }
            },
            submit: |mailbox, _commands, action, fields| {
                super::input::gift_submit(&mut mailbox.store, action, &fields);
            },
        },
    );
}
