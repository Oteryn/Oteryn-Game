use super::Panel;
#[must_use]
pub fn panel_text(panel: &Panel) -> String {
    if !panel.selected {
        return "Analysers unavailable in this session".into();
    }
    match panel.tool {
        super::Tool::Overview => overview(panel),
        super::Tool::Xp => xp_text(panel, panel.clock.seconds()),
        super::Tool::Loot => loot_text(panel, panel.clock.seconds()),
        super::Tool::Supply => format!("Supply Analyser\r\nSupplies used: {}", panel.supplies),
        super::Tool::Impact => format!(
            "Impact Analyser\r\nDamage: {} | Healing: {}",
            panel.damage, panel.healing
        ),
        super::Tool::Input => format!("Input Analyser\r\nDamage received: {}", panel.received),
        super::Tool::Session => format!("Hunting Session\r\n{}", overview(panel)),
        super::Tool::Party => "Party Hunt\r\nNo party accounting received".into(),
        super::Tool::LootTracker => format!("Loot Tracker\r\nLoot: {} items", panel.loot_items),
        super::Tool::Kills => format!("Kill Tracker\r\nKills: {}", panel.kills),
    }
}
fn overview(panel: &Panel) -> String {
    format!(
        "Kills: {}\r\nExperience: {} raw / {} gained\r\nLoot: {} items / {} gold\r\nSupplies used: {}\r\nDamage dealt: {} | Healing: {} | Damage received: {}\r\nDropped facts: {} (totals may be incomplete)",
        panel.kills,
        panel.raw_xp,
        panel.gained_xp,
        panel.loot_items,
        panel.gold,
        panel.supplies,
        panel.damage,
        panel.healing,
        panel.received,
        panel.dropped
    )
}

pub(super) fn xp_text(panel: &Panel, seconds: u64) -> String {
    use super::state::{eta_seconds, per_hour};
    let remaining = panel
        .goal_xp
        .map(|goal| goal.saturating_sub(panel.gained_xp));
    let eta = remaining.and_then(|value| eta_seconds(value, panel.gained_xp, seconds));
    let goal = remaining.map_or_else(
        || "Set the XP still needed for your level goal".into(),
        |value| format!("Level goal remaining: {value} XP"),
    );
    let eta = eta.map_or_else(
        || "Collect experience and set a level goal to estimate time".into(),
        |value| format!("Estimated time to level goal: {value} seconds"),
    );
    format!(
        "XP Analyser\r\nElapsed: {seconds} seconds\r\nExperience: {} raw / {} gained\r\nXP/h: {}\r\nRaw XP/h: {}\r\n{goal}\r\n{eta}\r\nGained XP\r\n{}\r\nRaw XP\r\n{}\r\n{}",
        panel.raw_xp,
        panel.gained_xp,
        per_hour(panel.gained_xp, seconds),
        per_hour(panel.raw_xp, seconds),
        panel.xp_chart.chart(seconds),
        panel.raw_xp_chart.chart(seconds),
        panel.feedback.unwrap_or("")
    )
}

pub(super) fn loot_text(panel: &Panel, seconds: u64) -> String {
    use super::state::{chart_values, per_hour};
    use std::fmt::Write;
    let unpriced = panel
        .loot
        .iter()
        .filter(|(item, _)| {
            panel
                .item_info
                .get(item)
                .is_none_or(|info| info.coin_worth == 0 && !panel.loot_prices.contains_key(item))
        })
        .fold(0u64, |total, (_, row)| total.saturating_add(row.count));
    let mut text = format!(
        "Loot Analyser\r\nGold: {} | Looted items: {}\r\nValued loot: {} (gold + priced items)\r\nLoot value/h: {}\r\nUnpriced items: {unpriced} | Unlisted items: {}\r\n{}\r\n",
        panel.gold,
        panel.loot_items,
        panel.loot_value(),
        per_hour(panel.loot_value(), seconds),
        panel.omitted_items,
        chart_values(&panel.loot_values(seconds))
    );
    for (item, row) in &panel.loot {
        let price = panel.item_info.get(item).map_or_else(
            || "identity unresolved; resolve item info".into(),
            |info| {
                if info.coin_worth != 0 {
                    "currency (already included in gold)".into()
                } else {
                    panel.loot_prices.get(item).map_or_else(
                        || "unpriced".into(),
                        |price| {
                            format!("unit {price} | value {}", row.count.saturating_mul(*price))
                        },
                    )
                }
            },
        );
        let _ = writeln!(
            text,
            "Item {item}: {} | {}/h | {price} | {}\r",
            row.count,
            per_hour(row.count, seconds),
            panel
                .item_info
                .get(item)
                .map_or("", |info| info.name.as_str())
        );
    }
    if let Some(feedback) = panel.feedback {
        text.push_str(feedback);
    }
    text
}
