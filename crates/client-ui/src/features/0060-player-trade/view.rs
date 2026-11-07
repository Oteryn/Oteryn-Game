use super::{PlayerTradePanel, picker, player_trade};

#[must_use]
pub fn panel_text(panel: &PlayerTradePanel) -> String {
    let mut lines = vec![status_text(panel)];
    lines.extend(picker::picker_text(panel));
    lines.join("\r\n")
}

#[must_use]
pub fn status_text(panel: &PlayerTradePanel) -> String {
    if !panel.selected {
        return "Player trade unavailable in this session".into();
    }
    let Some(state) = &panel.state else {
        return panel.feedback.unwrap_or("No active trade").into();
    };
    let offer = |name: &str, value: Option<player_trade::Offer>| {
        value.map_or_else(
            || format!("{name}: none"),
            |value| {
                format!(
                    "{name}: item {} x{} (subtype {})",
                    value.definition, value.count, value.sub_type
                )
            },
        )
    };
    let phase = match state.phase {
        1 => "offered",
        2 => "ready",
        3 => "transferring",
        _ => "closed",
    };
    let mut lines = vec![
        format!("Trade: {phase}"),
        offer("Your offer", state.own),
        offer("Their offer", state.counter),
        format!(
            "Accepted: you={} partner={}",
            state.accepted[0], state.accepted[1]
        ),
    ];
    if !state.message.is_empty() {
        lines.push(state.message.clone());
    }
    if !state.look.is_empty() {
        lines.push(state.look.clone());
    }
    if let Some(feedback) = panel.feedback {
        lines.push(feedback.into());
    }
    lines.join("\r\n")
}
