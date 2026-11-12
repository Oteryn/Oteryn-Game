use crate::{
    combat_input::PlayCommand,
    play::{
        PlayLink,
        mailbox::{Mailbox, lock},
    },
};
use tokio::sync::mpsc::UnboundedSender;
impl PlayLink {
    pub fn coin_market_control(&self, action: usize, fields: [String; 3]) -> bool {
        let mut mailbox = lock(&self.mailbox);
        submit(&mut mailbox, &self.commands, action, fields)
    }
    pub fn open_coin_market(&self) {
        let fields = lock(&self.mailbox).market.coins.fields.clone();
        self.coin_market_control(0, fields);
        #[cfg(windows)]
        if !self.uses_shared_frontend() {
            open(self);
        }
        if cfg!(not(windows)) || self.uses_shared_frontend() {
            lock(&self.mailbox).market.coins.open = true;
        }
    }
}
fn operation() -> Option<[u8; 16]> {
    use sha2::{Digest, Sha256};
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?;
    let mut hash = Sha256::new();
    hash.update(now.as_nanos().to_be_bytes());
    hash.update(std::process::id().to_be_bytes());
    hash.update(SEQUENCE.fetch_add(1, Ordering::Relaxed).to_be_bytes());
    let mut id = [0; 16];
    id.copy_from_slice(&hash.finalize()[..16]);
    id[..6].copy_from_slice(&now.as_millis().to_be_bytes()[10..]);
    id[6] = (id[6] & 15) | 0x70;
    id[8] = (id[8] & 63) | 0x80;
    Some(id)
}
fn submit(
    m: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
    action: usize,
    fields: [String; 3],
) -> bool {
    let id = if action == 6 {
        let Some(id) = operation() else {
            return false;
        };
        id
    } else {
        [0; 16]
    };
    let Some(command) = oteryn_client_ui::market::coin_control(
        &mut m.market.coins,
        m.market.selected,
        action,
        fields,
        id,
    ) else {
        return false;
    };
    if commands.send(PlayCommand::ServerMechanic(command)).is_err() {
        m.market.coins.pending = None;
        m.market.coins.feedback =
            "The session has ended; server retains unresolved confirmations".into();
        return false;
    }
    true
}
#[cfg(windows)]
fn open(link: &PlayLink) {
    use crate::play::feature_0115_mail::view::native::{Form, open};
    open(
        link,
        Form {
            title: "Coin market — Ctrl+Alt+F8",
            labels: &[
                "Coin amount",
                "Gold per coin",
                "Offer or transaction row number",
            ],
            buttons: &[
                "Book",
                "My Offers",
                "Review sell",
                "Review buy",
                "Review take",
                "Review cancel",
                "Confirm reviewed terms",
                "Retry confirmation",
                "Coin history",
                "Next history page",
                "Transaction details",
                "Coin balances",
                "Refresh last operation",
                "Dismiss review",
                "Next offers",
            ],
            snapshot: |m| {
                (
                    m.market.coins.open && !m.market_shutdown,
                    m.market.coins.pending.is_some(),
                    m.market.coins.fields.to_vec(),
                    oteryn_client_ui::market::coin_panel_text(&m.market),
                )
            },
            set_open: |m, v| m.market.coins.open = v,
            submit: |m, commands, action, fields| {
                let mut values = [String::new(), String::new(), String::new()];
                for (slot, value) in values.iter_mut().zip(fields) {
                    *slot = value;
                }
                submit(m, commands, action, values);
            },
        },
    );
}
#[cfg(test)]
#[path="coins/tests.rs"]
mod tests;
