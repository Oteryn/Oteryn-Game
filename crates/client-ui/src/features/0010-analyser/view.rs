use super::Panel;
#[must_use]
pub fn panel_text(panel: &Panel) -> String {
    if !panel.selected {
        return "Analysers unavailable in this session".into();
    }
    match panel.tool {
        super::Tool::Overview => overview(panel),
        super::Tool::Xp => format!(
            "XP Analyser\r\nExperience: {} raw / {} gained",
            panel.raw_xp, panel.gained_xp
        ),
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
