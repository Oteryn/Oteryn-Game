use super::*;
use oteryn_client_ui::market::CoinPanel;

impl PlayLink {
    fn coin_market_binding(&self) -> usize {
        std::sync::Arc::as_ptr(&self.mailbox) as usize
    }

    pub(crate) fn coin_market_snapshot(&self) -> Option<(usize, CoinPanel)> {
        let mailbox = lock(&self.mailbox);
        (mailbox.market.selected
            && mailbox.market.coins.open
            && !mailbox.market_shutdown
            && !mailbox.depot_shutdown)
            .then(|| (self.coin_market_binding(), mailbox.market.coins.clone()))
    }

    pub(crate) fn coin_market_open_bound(&self) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if !self.coin_market_available(&mailbox) {
            return false;
        }
        if mailbox.market.coins.open {
            return true;
        }
        if mailbox.market.coins.pending.is_some() {
            mailbox.market.coins.open = true;
            return true;
        }
        let mut prepared = mailbox.market.coins.clone();
        let fields = prepared.fields.clone();
        let Some(command) =
            oteryn_client_ui::market::coin_control(&mut prepared, true, 0, fields, [0; 16])
        else {
            return false;
        };
        if self
            .commands
            .send(PlayCommand::ServerMechanic(command))
            .is_err()
        {
            return false;
        }
        prepared.open = true;
        mailbox.market.coins = prepared;
        true
    }

    pub(crate) fn coin_market_close_bound(&self, binding: usize, expected: &CoinPanel) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if !self.coin_market_matches(&mailbox, binding, expected) {
            return false;
        }
        mailbox.market.coins.open = false;
        true
    }

    fn coin_market_available(&self, mailbox: &Mailbox) -> bool {
        !mailbox.market_shutdown
            && !mailbox.depot_shutdown
            && !mailbox.activity_reset_commands
            && !mailbox.server_modal.is_active()
            && !mailbox.world_input_blocked()
            && !self.commands.is_closed()
            && mailbox.market.selected
    }

    fn coin_market_matches(&self, mailbox: &Mailbox, binding: usize, expected: &CoinPanel) -> bool {
        binding == self.coin_market_binding()
            && self.coin_market_available(mailbox)
            && expected.open
            && mailbox.market.coins == *expected
    }

    pub(crate) fn coin_market_bound(
        &self,
        binding: usize,
        expected: &CoinPanel,
        action: usize,
        fields: [String; 3],
    ) -> bool {
        let mut mailbox = lock(&self.mailbox);
        if !self.coin_market_matches(&mailbox, binding, expected) || expected.pending.is_some() {
            return false;
        }
        let id = if action == 6 {
            let Some(id) = operation() else {
                return false;
            };
            id
        } else {
            [0; 16]
        };
        let mut prepared = expected.clone();
        let command =
            oteryn_client_ui::market::coin_control(&mut prepared, true, action, fields, id);
        if let Some(command) = command {
            if self
                .commands
                .send(PlayCommand::ServerMechanic(command))
                .is_err()
            {
                return false;
            }
        } else if prepared.feedback == expected.feedback && prepared.reviewed == expected.reviewed {
            return false;
        }
        mailbox.market.coins = prepared;
        true
    }
}
#[cfg(test)]
impl crate::play::Outbox {
    pub(crate) fn bevy_coin_market_panel(&self, panel: CoinPanel) {
        lock(&self.0).market.coins = panel;
    }

    pub(crate) fn bevy_coin_market_response(&self, response: &oteryn_session::GatedCommandResult) {
        self.coin_market_result(response);
    }
}
