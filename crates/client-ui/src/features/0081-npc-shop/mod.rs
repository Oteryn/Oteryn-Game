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
        panel.feedback = Some("NPC shop request refused by the command gate");
        return;
    };
    panel.feedback = match result.disposition {
        NpcIntentDisposition::Ok => None,
        NpcIntentDisposition::NotInRange => Some("NPC shop refused: not in range"),
        NpcIntentDisposition::NoConversation => Some("NPC shop refused: no open conversation"),
        NpcIntentDisposition::RateLimited => Some("NPC shop refused: rate limited"),
        NpcIntentDisposition::Stale => Some("NPC shop refused: stale catalogue or price"),
        NpcIntentDisposition::Rejected => Some("NPC shop request rejected"),
    };
    panel.result = Some(result);
}

pub fn panel_text(panel: &NpcShopPanel) -> String {
    if !panel.selected {
        return "NPC shop is unavailable in this session".into();
    }
    if panel.pending {
        return "NPC shop request pending".into();
    }
    if let Some(feedback) = panel.feedback {
        return feedback.into();
    }
    match panel.result.as_ref() {
        Some(result) if !result.look_text.is_empty() => result.look_text.clone(),
        Some(result) if result.sold_units != 0 => format!(
            "Sold {} units for {} gold",
            result.sold_units, result.total_gold_units
        ),
        Some(_) => "NPC shop ready".into(),
        None => "Choose Look or Sell all for the selected NPC trade offer".into(),
    }
}

#[cfg(test)]
#[path = "tests/state.rs"]
mod tests;
