use super::PotionDurationsPanel;
use std::time::Instant;
pub fn panel_text(panel: &PotionDurationsPanel) -> String {
    panel_text_at(panel, Instant::now())
}
pub fn panel_text_at(panel: &PotionDurationsPanel, now: Instant) -> String {
    if !panel.selected {
        return "Potion durations unavailable in this session".into();
    }
    let Some(snapshot) = &panel.snapshot else {
        return panel.error.unwrap_or("Loading potion durations…").into();
    };
    let elapsed_ms = panel.received_at.map_or(0, |received| {
        u64::try_from(now.saturating_duration_since(received).as_millis()).unwrap_or(u64::MAX)
    });
    let mut text = String::new();
    if let Some(error) = panel.error {
        text.push_str(error);
        text.push_str("\r\n");
    }
    let mut shown = 0;
    for entry in &snapshot.active {
        let remaining = entry.remaining_ms.saturating_sub(elapsed_ms);
        if remaining == 0 {
            continue;
        }
        let label = match entry.source {
            3 => format!("Store XP boost +{}%", entry.percent),
            4 => format!("Daily XP boost +{}%", entry.percent),
            _ => potion_name(entry.item_id).to_owned(),
        };
        let seconds = remaining.div_ceil(1000);
        text.push_str(&format!(
            "{label}: {:02}:{:02}\r\n",
            seconds / 60,
            seconds % 60
        ));
        shown += 1;
    }
    if shown == 0 {
        text.push_str("No active potion effects");
    }
    text
}
fn potion_name(item: u32) -> &'static str {
    match item {
        36723 => "Kooldown Aid",
        36724 => "Strike Enhancement",
        36725 => "Stamina Extension",
        36726 => "Charm Upgrade",
        36727 => "Wealth Duplex",
        36728 => "Bestiary Betterment",
        36729 => "Fire Resilience",
        36730 => "Ice Resilience",
        36731 => "Earth Resilience",
        36732 => "Energy Resilience",
        36733 => "Holy Resilience",
        36734 => "Death Resilience",
        36735 => "Physical Resilience",
        36736 => "Fire Amplification",
        36737 => "Ice Amplification",
        36738 => "Earth Amplification",
        36739 => "Energy Amplification",
        36740 => "Holy Amplification",
        36741 => "Death Amplification",
        36742 => "Physical Amplification",
        7439 => "Berserk potion",
        7440 => "Mastermind potion",
        7443 => "Bullseye potion",
        49271 => "Transcendence potion",
        35563 => "Magic shield potion",
        _ => "Unknown potion",
    }
}
