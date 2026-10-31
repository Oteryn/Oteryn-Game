use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::{Mailbox, lock}};
use oteryn_client_ui::wheel_allocation::{Intent, request};
use oteryn_input_actions::NormalizedInputEvent;
use tokio::sync::mpsc::UnboundedSender;

impl PlayLink {
    pub fn wheel_request(&self, intent: Intent) {
        enqueue(&mut lock(&self.mailbox), &self.commands, intent);
    }

    pub fn wheel_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::wheel_allocation::open_key(events) { return false; }
        self.wheel_request(Intent::Refresh);
        #[cfg(windows)]
        super::view::open_window(self);
        #[cfg(not(windows))]
        { lock(&self.mailbox).wheel.open = true; }
        true
    }
}

/// The prepared bytes, including occurrence, remain in the panel until the outcome settles.
pub(in crate::play) fn enqueue(mailbox: &mut Mailbox, commands: &UnboundedSender<PlayCommand>, intent: Intent) {
    if mailbox.wheel.pending.is_some() { return; }
    let occurrence = if matches!(intent, Intent::Refresh | Intent::Retry) { [0; 16] }
        else if let Some(id) = occurrence() { id }
        else {
            mailbox.wheel.feedback = Some("A request identity could not be created; try again");
            return;
        };
    if let Some(command) = request(&mut mailbox.wheel, intent, occurrence)
        && commands.send(PlayCommand::Wheel(command)).is_err()
    {
        mailbox.wheel.pending = None;
        mailbox.wheel.feedback = Some("The session has ended");
    }
}

fn occurrence() -> Option<[u8; 16]> {
    let mut id = [0; 16];
    oteryn_platform_client::native_login::secure_random(&mut id).ok()?;
    let milliseconds = u64::try_from(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).ok()?.as_millis()).ok()?;
    if milliseconds >= 1_u64 << 48 { return None; }
    id[..6].copy_from_slice(&milliseconds.to_be_bytes()[2..]);
    id[6] = 0x70 | (id[6] & 0x0f);
    id[8] = 0x80 | (id[8] & 0x3f);
    Some(id)
}
