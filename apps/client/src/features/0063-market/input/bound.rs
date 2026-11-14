//! Atomic presentation dispatch; the retained owners still prepare every protocol command.
use crate::{
    combat_input::PlayCommand,
    play::{
        PlayLink,
        mailbox::{Mailbox, lock},
    },
};
use oteryn_client_ui::market::{Intent, MarketPanel, MarketView, request};
impl PlayLink {
    /// Identifies the retained session, in addition to the shell's generation fence.
    pub(crate) fn market_session_binding(&self) -> usize {
        std::sync::Arc::as_ptr(&self.mailbox) as usize
    }
    pub(crate) fn market_request_bound(
        &self,
        binding: usize,
        expected: &MarketPanel,
        intent: Intent,
    ) -> bool {
        if !matches!(
            intent,
            Intent::OwnOffers
                | Intent::History
                | Intent::Refresh
                | Intent::NextPage
                | Intent::PreviousPage
        ) {
            return false;
        }
        let mut m = lock(&self.mailbox);
        if !self.market_operations_allowed(&m, binding, expected) {
            return false;
        }
        let mut prepared = expected.clone();
        self.market_dispatch_prepared(&mut m, &mut prepared, intent)
    }
    pub(crate) fn market_create_bound(
        &self,
        binding: usize,
        expected: &MarketPanel,
        fields: [&str; 2],
        sell: bool,
        anonymous: bool,
    ) -> bool {
        let mut m = lock(&self.mailbox);
        if !self.market_operations_allowed(&m, binding, expected) || expected.definition == 0 {
            return false;
        }
        let mut prepared = expected.clone();
        if !oteryn_client_ui::market::edit_details(
            &mut prepared,
            &expected.definition.to_string(),
            fields[0],
            fields[1],
            sell,
            anonymous,
        ) {
            return false;
        }
        self.market_dispatch_prepared(&mut m, &mut prepared, Intent::CreateOffer)
    }
    pub(crate) fn market_cancel_bound(
        &self,
        binding: usize,
        expected: &MarketPanel,
        offer: &oteryn_client_runtime::market::market::MarketRow,
    ) -> bool {
        let mut m = lock(&self.mailbox);
        if !self.market_operations_allowed(&m, binding, expected)
            || expected.view != MarketView::Own
            || offer.state != 1
            || offer.amount == 0
            || !expected.rows.contains(offer)
        {
            return false;
        }
        let mut prepared = expected.clone();
        prepared.selected_offer = Some(offer.offer);
        self.market_dispatch_prepared(&mut m, &mut prepared, Intent::CancelOffer)
    }
    fn market_operations_allowed(
        &self,
        m: &Mailbox,
        binding: usize,
        expected: &MarketPanel,
    ) -> bool {
        binding == self.market_session_binding()
            && !m.market_shutdown
            && !m.depot_shutdown
            && !m.server_modal.is_active()
            && !m.world_input_blocked()
            && !self.commands.is_closed()
            && m.market == *expected
            && expected.open
            && expected.selected
            && expected.pending.is_none()
            && expected.details.pending.is_none()
            && expected.coins.pending.is_none()
    }
    fn market_dispatch_prepared(
        &self,
        m: &mut Mailbox,
        prepared: &mut MarketPanel,
        intent: Intent,
    ) -> bool {
        let Some(command) = request(prepared, intent) else {
            return false;
        };
        if self.commands.send(PlayCommand::Market(command)).is_err() {
            return false;
        }
        m.market.clone_from(prepared);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::play::play_channel;
    #[test]
    fn market_bound_operations_reject_shutdown_closed_stale_and_duplicate_without_mutation()
    -> Result<(), Box<dyn std::error::Error>> {
        for reason in [
            "market shutdown",
            "depot shutdown",
            "closed",
            "stale",
            "pending",
            "ended",
        ] {
            let (link, commands, outbox) = play_channel();
            outbox.market_initialize(true);
            link.market_set_open(true);
            assert!(link.market_details("42", "2", "100", true, false));
            let mut expected = link.market_panel();
            let binding = link.market_session_binding();
            match reason {
                "market shutdown" => lock(&outbox.0).market_shutdown = true,
                "depot shutdown" => lock(&outbox.0).depot_shutdown = true,
                "closed" => link.market_set_open(false),
                "stale" => expected.price += 1,
                "pending" => link.market_request(Intent::OwnOffers),
                "ended" => drop(commands),
                _ => return Err("unknown case".into()),
            }
            let before = link.market_panel();
            assert!(
                !link.market_create_bound(binding, &expected, ["3", "100"], true, true),
                "{reason}"
            );
            for intent in [
                Intent::OwnOffers,
                Intent::History,
                Intent::NextPage,
                Intent::PreviousPage,
                Intent::Refresh,
            ] {
                assert!(
                    !link.market_request_bound(binding, &expected, intent),
                    "{reason}"
                );
            }
            assert_eq!(link.market_panel(), before, "{reason}");
        }
        Ok(())
    }
    #[test]
    fn coin_market_bound_rejects_every_action_from_another_or_closed_session() {
        let (link, mut commands, outbox) = play_channel();
        outbox.market_initialize(true);
        let binding = link.market_session_binding();
        assert!(link.coin_market_open_bound(binding));
        assert!(commands.try_recv().is_ok());
        lock(&outbox.0).market.coins.pending = None;
        let expected = link.market_panel().coins;
        for action in 0..=14 {
            let before = link.market_panel();
            assert!(!link.coin_market_bound(
                binding.wrapping_add(1),
                &expected,
                action,
                Default::default()
            ));
            assert_eq!(link.market_panel(), before);
        }
        assert!(link.coin_market_close_bound(binding, &expected));
        let before = link.market_panel();
        for action in 0..=14 {
            assert!(!link.coin_market_bound(binding, &expected, action, Default::default()));
            assert_eq!(link.market_panel(), before);
        }
        assert!(commands.try_recv().is_err());
    }
}
