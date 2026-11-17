use crate::{
    combat_input::PlayCommand,
    play::{
        PlayLink,
        mailbox::{Mailbox, lock},
    },
};
use tokio::sync::mpsc::UnboundedSender;
impl PlayLink {
    pub fn coin_gift_control(&self, action: usize, fields: [String; 2]) -> bool {
        submit(&mut lock(&self.mailbox), &self.commands, action, fields)
    }
}
pub(in crate::play) fn submit(
    mailbox: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
    action: usize,
    fields: [String; 2],
) -> bool {
    let op = if action == 1 {
        let Some(id) = operation() else {
            return false;
        };
        id
    } else {
        [0; 16]
    };
    let Some(command) =
        oteryn_client_ui::store::gift_control(&mut mailbox.store, action, fields, op)
    else {
        return false;
    };
    if commands.send(PlayCommand::ServerMechanic(command)).is_err() {
        mailbox.store.gift.pending = None;
        mailbox.store.feedback =
            Some("Session ended; unresolved gift confirmation retained".into());
        return false;
    }
    true
}
fn operation() -> Option<[u8; 16]> {
    use sha2::{Digest, Sha256};
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?;
    let mut hash = Sha256::new();
    hash.update(b"oteryn:coin-gift:client:v1");
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

#[cfg(test)]
#[path = "gift/tests.rs"]
mod tests;
