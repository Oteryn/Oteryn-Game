//! Continue the user's bulk sale only after a successful acknowledgement.
use super::*;

pub(super) fn advance(
    state: &mut State,
    owner: &HostOwner,
    link: &crate::play::PlayLink,
    panel: &oteryn_client_ui::npc_shop::NpcShopPanel,
) {
    if state.sales.is_empty() {
        return;
    }
    if owner.modal_active() || !link.npc_shop_is_open() || state.side != Some(NpcTradeSide::Sell) {
        state.sales.clear();
        return;
    }
    if panel.pending {
        return;
    }
    if panel.feedback.is_some()
        || !panel.result.as_ref().is_some_and(|r| {
            r.disposition == oteryn_session::NpcIntentDisposition::Ok && r.sold_units > 0
        })
    {
        state.sales.clear();
        return;
    }
    if state
        .sale_inventory
        .is_some_and(|(item, before)| panel.live.owned(item) >= before)
    {
        return;
    }
    while let Some(quote) = state.sales.pop_front() {
        if panel.live.look(quote).is_none() {
            state.sales.clear();
            return;
        }
        if panel.live.owned(quote.item) > 0 {
            state.sale_inventory = Some((quote.item, panel.live.owned(quote.item)));
            if !link.npc_sell_all_request(quote) {
                state.sales.clear();
            }
            return;
        }
    }
}
