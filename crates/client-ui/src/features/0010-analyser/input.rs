// oteryn-choice: CTRL+SHIFT+Y opens this local panel; text focus is handled by the shell.
use oteryn_input_actions::{ButtonState, Modifier, NormalizedInputEvent};
pub fn open_key(events: &[NormalizedInputEvent]) -> bool {
    events.iter().any(|event| matches!(event, NormalizedInputEvent::Key { code, state: ButtonState::Pressed, modifiers, repeat: false } if code.get() == 28 && modifiers.contains(Modifier::Control) && modifiers.contains(Modifier::Shift) && !modifiers.contains(Modifier::Alt) && !modifiers.contains(Modifier::Super)))
}

/// Select a tool without resetting confirmed session measurements.
pub fn select_tool(panel: &mut super::Panel, key: &str) -> bool {
    if let Some(tool) = super::Tool::ALL
        .into_iter()
        .find(|tool| tool.key() == key.trim())
    {
        panel.tool = tool;
        true
    } else {
        false
    }
}

/// Set the XP still needed for the player's chosen level goal.
pub fn set_xp_goal(panel: &mut super::Panel, value: &str) -> bool {
    let value = value.trim();
    let goal = if value.is_empty() {
        None
    } else {
        let Some(goal) = value
            .parse::<u64>()
            .ok()
            .and_then(|remaining| panel.gained_xp.checked_add(remaining))
        else {
            panel.feedback = Some("Enter the XP still needed for your level goal");
            return false;
        };
        Some(goal)
    };
    panel.goal_xp = goal;
    panel.goal_input = value.into();
    panel.feedback = None;
    true
}

/// Custom item valuations affect all confirmed counts and chart buckets immediately.
pub fn set_item_price(panel: &mut super::Panel, item: &str, price: &str) -> bool {
    let parsed = item
        .trim()
        .parse::<u32>()
        .ok()
        .filter(|item| *item != 0)
        .zip(price.trim().parse::<u64>().ok());
    let Some((item, price)) = parsed else {
        panel.feedback = Some("Enter an item number from the analyser and its unit value");
        return false;
    };
    let prices = if panel.tool == super::Tool::Supply {
        &mut panel.supply_prices
    } else {
        &mut panel.loot_prices
    };
    if !prices.contains_key(&item) && prices.len() >= super::state::items::MAX_ITEM_DETAILS {
        panel.feedback = Some("The item price list is full");
        return false;
    }
    prices.insert(item, price);
    panel.party.clear();
    panel.party_revision = panel.party_revision.saturating_add(1);
    panel.party_feedback = Some("Prices changed; refresh party accounting");
    panel.price_input = [item.to_string(), price.to_string()];
    panel.feedback = None;
    true
}
