use super::*;
impl PlayLink {
    pub(crate) fn coin_market_open_bound(&self, binding: usize) -> bool {
        let mut m = lock(&self.mailbox);
        if binding != self.market_session_binding()
            || m.market_shutdown
            || m.depot_shutdown
            || m.server_modal.is_active()
            || m.world_input_blocked()
            || self.commands.is_closed()
            || !m.market.selected
            || m.market.coins.pending.is_some()
        {
            return false;
        }
        let mut prepared = m.market.coins.clone();
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
        m.market.coins = prepared;
        true
    }
    pub(crate) fn coin_market_close_bound(
        &self,
        binding: usize,
        expected: &oteryn_client_ui::market::CoinPanel,
    ) -> bool {
        let mut m = lock(&self.mailbox);
        if !self.coin_market_available(&m, binding, expected) {
            return false;
        }
        m.market.coins.open = false;
        true
    }
    fn coin_market_available(
        &self,
        m: &Mailbox,
        binding: usize,
        expected: &oteryn_client_ui::market::CoinPanel,
    ) -> bool {
        binding == self.market_session_binding()
            && !m.market_shutdown
            && !m.depot_shutdown
            && !m.server_modal.is_active()
            && !m.world_input_blocked()
            && !self.commands.is_closed()
            && m.market.selected
            && expected.open
            && m.market.coins == *expected
    }
    pub(crate) fn coin_market_bound(
        &self,
        binding: usize,
        expected: &oteryn_client_ui::market::CoinPanel,
        action: usize,
        fields: [String; 3],
    ) -> bool {
        let mut m = lock(&self.mailbox);
        if !self.coin_market_available(&m, binding, expected) || expected.pending.is_some() {
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
        } else if !matches!(action, 2..=5 | 13) || prepared == *expected {
            return false;
        }
        m.market.coins = prepared;
        true
    }
}
#[cfg(all(test, feature = "bevy-shell"))]
impl crate::play::Outbox {
    pub(crate) fn bevy_coin_market_response(&self, response: &oteryn_session::GatedCommandResult) {
        self.coin_market_result(response);
    }
}
