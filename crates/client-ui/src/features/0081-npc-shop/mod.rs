//! NPC shop look and sell-all panel state and protocol mapping.
pub use oteryn_client_runtime::npc_shop::{
    CommandStatus, EntityRef, NpcIntentDisposition, NpcShopCommand, NpcShopResponse, NpcTradeSide,
    npc_shop,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Look(npc_shop::NpcShopIntent),
    SellAll(npc_shop::NpcShopIntent),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NpcShopPanel {
    pub live: LiveShop,
    pub selected: bool,
    pub open: bool,
    pub pending: bool,
    pub result: Option<npc_shop::NpcShopResult>,
    pub feedback: Option<&'static str>,
}

pub fn request(panel: &mut NpcShopPanel, intent: Intent) -> Option<NpcShopCommand> {
    if !panel.selected || panel.pending {
        return None;
    }
    let mut value = match intent {
        Intent::Look(value) | Intent::SellAll(value) => value,
    };
    value.action = match intent {
        Intent::Look(_) => npc_shop::NpcShopAction::Look,
        Intent::SellAll(_) => npc_shop::NpcShopAction::SellAll,
    };
    npc_shop::encode_npc_shop_intent(&value).ok()?;
    panel.pending = true;
    panel.feedback = None;
    Some(NpcShopCommand(value))
}

pub fn apply_result(panel: &mut NpcShopPanel, response: &NpcShopResponse) {
    panel.pending = false;
    let Some(result) = response.result.clone() else {
        panel.feedback = Some(tr!("NPC shop request refused by the command gate"));
        return;
    };
    panel.feedback = match result.disposition {
        NpcIntentDisposition::Ok => None,
        NpcIntentDisposition::NotInRange => Some(tr!("NPC shop refused: not in range")),
        NpcIntentDisposition::NoConversation => Some(tr!("NPC shop refused: no open conversation")),
        NpcIntentDisposition::RateLimited => Some(tr!("NPC shop refused: rate limited")),
        NpcIntentDisposition::Stale => Some(tr!("NPC shop refused: stale catalogue or price")),
        NpcIntentDisposition::Rejected => Some(tr!("NPC shop request rejected")),
    };
    panel.result = Some(result);
}

pub fn panel_text(panel: &NpcShopPanel) -> String {
    if !panel.selected {
        return tr!("NPC shop is unavailable in this session").into();
    }
    if panel.pending {
        return tr!("NPC shop request pending").into();
    }
    if let Some(feedback) = panel.feedback {
        return feedback.into();
    }
    match panel.result.as_ref() {
        Some(result) if !result.look_text.is_empty() => result.look_text.clone(),
        Some(result) if result.sold_units != 0 => lfmt!(
            "Sold {} units for {} gold",
            result.sold_units,
            result.total_gold_units
        ),
        Some(_) => tr!("NPC shop ready").into(),
        None => tr!("Select a live NPC trade offer").into(),
    }
}

#[cfg(test)]
#[path = "tests/state.rs"]
mod tests;

/// Production NPC prerequisite reducer. Closed windows retain no stale offers.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NpcServicePanel {
    pub conversation: Option<npc_service::NpcConversation>,
    pub window: Option<npc_service::NpcTradeWindow>,
}

pub use oteryn_client_runtime::npc_shop::npc_service;

pub fn apply_conversation(panel: &mut NpcServicePanel, payload: &[u8]) -> bool {
    let Ok(conversation) = npc_service::decode_npc_conversation(payload) else {
        return false;
    };
    if conversation.open.is_none() {
        panel.window = None;
    }
    panel.conversation = Some(conversation);
    true
}

pub fn apply_window(panel: &mut NpcServicePanel, payload: &[u8]) -> bool {
    let Ok(window) = npc_service::decode_npc_trade_window(payload) else {
        return false;
    };
    panel.window = Some(window);
    true
}

pub fn buy_request(
    panel: &NpcServicePanel,
    offer_index: usize,
) -> Option<npc_service::NpcTradeIntent> {
    let window = panel.window.as_ref()?;
    let (npc_actor, catalogue_revision) = window.open?;
    let offer = window.offers.get(offer_index)?;
    (offer.side == NpcTradeSide::Buy).then_some(npc_service::NpcTradeIntent {
        npc_actor,
        catalogue_revision,
        offer_index: u32::try_from(offer_index).ok()?,
        side: offer.side,
        quantity: 1,
        expected_unit_price: offer.unit_price,
    })
}

#[cfg(test)]
#[path = "tests/live.rs"]
mod live_tests;

mod state;
pub use state::{LiveShop, Quote};

mod input;
pub use input::parse_amount;
