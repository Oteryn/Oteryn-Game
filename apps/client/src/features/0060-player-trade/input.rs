use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::player_trade::{Intent, request};
use oteryn_input_actions::NormalizedInputEvent;
impl PlayLink {
    pub fn player_trade_request(&self, intent: Intent) {
        let mut mailbox = lock(&self.mailbox);
        if let Some(command) = request(&mut mailbox.player_trade, intent)
            && self
                .commands
                .send(PlayCommand::PlayerTrade(command))
                .is_err()
        {
            mailbox.player_trade.pending = false;
            mailbox.player_trade.feedback = Some("The session has ended");
        }
    }
    pub fn player_trade_input(&self, events: &[NormalizedInputEvent]) -> bool {
        if !oteryn_client_ui::player_trade::open_key(events) {
            return false;
        }
        #[cfg(windows)]
        super::view::open_window(self);
        #[cfg(not(windows))]
        {
            lock(&self.mailbox).player_trade.open = true;
        }
        true
    }
}
