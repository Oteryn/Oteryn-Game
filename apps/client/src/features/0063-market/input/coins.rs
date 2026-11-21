use crate::{
    combat_input::PlayCommand,
    play::{
        PlayLink,
        mailbox::{Mailbox, lock},
    },
};
use tokio::sync::mpsc::UnboundedSender;
#[cfg(bevy_shell)]
#[path = "coins/bound.rs"]
mod bound;
impl PlayLink {
    pub fn coin_market_control(&self, action: usize, fields: [String; 3]) -> bool {
        let mut mailbox = lock(&self.mailbox);
        submit(&mut mailbox, &self.commands, action, fields)
    }
    pub fn open_coin_market(&self) {
        let fields = lock(&self.mailbox).market.coins.fields.clone();
        self.coin_market_control(0, fields);
        lock(&self.mailbox).market.coins.open = true;
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

#[cfg(test)]
#[path="coins/tests.rs"]
mod tests;
