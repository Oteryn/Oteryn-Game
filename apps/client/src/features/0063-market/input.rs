use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::market::{Intent, request};
impl PlayLink {
    pub fn market_request(&self, intent: Intent) {
        let mut mailbox = lock(&self.mailbox);
        if let Some(command) = request(&mut mailbox.market, intent)
            && self.commands.send(PlayCommand::Market(command)).is_err()
        {
            mailbox.market.pending = None;
            mailbox.market.feedback = Some("The session has ended");
        }
    }
    pub fn open_market(&self) {
        self.market_request(Intent::Refresh);
        #[cfg(windows)]
        super::view::open_window(self);
        #[cfg(not(windows))]
        {
            lock(&self.mailbox).market.open = true;
        }
    }
}

#[cfg(windows)]
pub(in crate::play) fn enqueue(
    mailbox: &mut crate::play::mailbox::Mailbox,
    commands: &tokio::sync::mpsc::UnboundedSender<PlayCommand>,
    intent: Intent,
) {
    if let Some(command) = request(&mut mailbox.market, intent)
        && commands.send(PlayCommand::Market(command)).is_err()
    {
        mailbox.market.pending = None;
        mailbox.market.feedback = Some("The session has ended");
    }
}
