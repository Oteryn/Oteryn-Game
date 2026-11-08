use super::Panel;
#[must_use]
pub fn panel_text(panel: &Panel) -> String {
    if !panel.selected {
        return "Analysers unavailable in this session".into();
    }
    match panel.tool {
        super::Tool::Overview => overview(panel),
        super::Tool::Xp => xp_text(panel, panel.clock.seconds()),
        super::Tool::Loot => format!(
            "Loot Analyser\r\nLoot: {} items / {} gold",
            panel.loot_items, panel.gold
        ),
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
