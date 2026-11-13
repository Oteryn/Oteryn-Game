//! Snapshot-bound frontend requests. The immutable READY pair remains server-owned.
use crate::{
    combat_input::PlayCommand,
    play::{PlayLink, mailbox::lock},
};
use oteryn_client_ui::player_trade::{Intent, PlayerTradePanel, picker, request};
use oteryn_session::ItemHandle;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TradePhase {
    Inactive,
    Offered,
    Ready,
    Transferring,
    Unknown,
}
impl TradePhase {
    #[must_use]
    pub fn active(self) -> bool {
        matches!(self, Self::Offered | Self::Ready | Self::Transferring)
    }
}
#[must_use]
pub fn trade_phase(panel: &PlayerTradePanel) -> TradePhase {
    match panel.state.as_ref().map(|state| state.phase) {
        None | Some(4) => TradePhase::Inactive,
        Some(1) => TradePhase::Offered,
        Some(2) => TradePhase::Ready,
        Some(3) => TradePhase::Transferring,
        Some(_) => TradePhase::Unknown,
    }
}
#[must_use]
pub fn trade_picker_available(panel: &PlayerTradePanel) -> bool {
    let available = match trade_phase(panel) {
        TradePhase::Inactive => true,
        TradePhase::Offered => panel
            .state
            .as_ref()
            .is_some_and(|state| state.own.is_none()),
        _ => false,
    };
    available
        && panel
            .partner
            .or_else(|| panel.state.as_ref()?.partner)
            .is_some()
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TradeAction {
    Request(Intent),
    Select(ItemHandle),
    Offer,
    Open(ItemHandle),
    Close,
}
#[must_use]
pub fn same_presentation(a: &PlayerTradePanel, b: &PlayerTradePanel) -> bool {
    a.selected == b.selected
        && a.open == b.open
        && a.show_requested == b.show_requested
        && a.pending == b.pending
        && a.state == b.state
        && a.feedback == b.feedback
        && a.offer == b.offer
        && a.partner == b.partner
        && a.items == b.items
}
#[must_use]
pub fn trade_action_enabled(panel: &PlayerTradePanel, action: TradeAction) -> bool {
    let phase = trade_phase(panel);
    if !panel.selected || phase == TradePhase::Unknown {
        return false;
    }
    if action == TradeAction::Close {
        return phase == TradePhase::Inactive;
    }
    if panel.pending || panel.items.pending {
        return false;
    }
    let picker = trade_picker_available(panel);
    match action {
        TradeAction::Request(Intent::Accept) => {
            phase == TradePhase::Ready
                && panel.state.as_ref().is_some_and(|state| {
                    state.trade_id.is_some()
                        && state.own.is_some()
                        && state.counter.is_some()
                        && !state.accepted[0]
                })
        }
        TradeAction::Request(Intent::Cancel) => {
            matches!(phase, TradePhase::Offered | TradePhase::Ready)
        }
        TradeAction::Request(Intent::LookOwn | Intent::LookCounter) => {
            phase == TradePhase::Ready
                && panel.state.as_ref().is_some_and(|state| {
                    state.trade_id.is_some()
                        && if action == TradeAction::Request(Intent::LookOwn) {
                            state.own.is_some()
                        } else {
                            state.counter.is_some()
                        }
                })
        }
        TradeAction::Request(Intent::Offer { .. }) => false,
        TradeAction::Select(handle) => {
            picker
                && picker::rows(&panel.items)
                    .iter()
                    .any(|(_, item)| item.handle == handle)
        }
        TradeAction::Offer => picker && panel.offer.is_some(),
        TradeAction::Open(handle) => {
            picker
                && panel.items.can_open
                && picker::rows(&panel.items)
                    .iter()
                    .any(|(_, item)| item.handle == handle)
        }
        TradeAction::Close => false,
    }
}
impl PlayLink {
    /// Compare and dispatch under one mailbox lock. This fences stale frontend controls;
    /// the existing wire pair identity and server custody checks fence the exchange.
    pub fn player_trade_dispatch_bound(
        &self,
        expected: &PlayerTradePanel,
        action: TradeAction,
    ) -> bool {
        let mut mailbox = lock(&self.mailbox);
        let panel = &mut mailbox.player_trade;
        if !same_presentation(panel, expected) || !trade_action_enabled(panel, action) {
            return false;
        }
        let command = match action {
            TradeAction::Close => {
                panel.open = false;
                panel.show_requested = false;
                return true;
            }
            TradeAction::Select(handle) | TradeAction::Open(handle) => {
                let Some(row) = picker::rows(&panel.items)
                    .iter()
                    .position(|(_, item)| item.handle == handle)
                else {
                    return false;
                };
                if matches!(action, TradeAction::Select(_)) {
                    return picker::select_item(panel, row);
                }
                super::super::input::open_container(panel, row, &self.commands);
                return panel.items.pending;
            }
            TradeAction::Request(intent) => request(panel, intent),
            TradeAction::Offer => picker::offer_selected(panel),
        };
        let Some(command) = command else {
            return false;
        };
        if self
            .commands
            .send(PlayCommand::PlayerTrade(command))
            .is_err()
        {
            panel.pending = false;
            panel.feedback = Some(tr!("The session has ended"));
            return false;
        }
        true
    }
}
