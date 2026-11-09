use super::PotionDurationsPanel;
use std::time::Instant;
pub fn panel_text(panel: &PotionDurationsPanel) -> String {
    panel_text_at(panel, Instant::now())
}
pub fn panel_text_at(panel: &PotionDurationsPanel, now: Instant) -> String {
    if !panel.selected {
        return tr!("Potion durations unavailable in this session").into();
    }
    let Some(snapshot) = &panel.snapshot else {
        return panel
            .error
            .unwrap_or(tr!("Loading potion durations…"))
            .into();
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
            3 => lfmt!("Store XP boost +{}%", entry.percent),
            4 => lfmt!("Daily XP boost +{}%", entry.percent),
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
        text.push_str(tr!("No active potion effects"));
    }
    text
}
fn potion_name(item: u32) -> &'static str {
    match item {
        36723 => tr!("Kooldown Aid"),
        36724 => tr!("Strike Enhancement"),
        36725 => tr!("Stamina Extension"),
        36726 => tr!("Charm Upgrade"),
        36727 => tr!("Wealth Duplex"),
        36728 => tr!("Bestiary Betterment"),
        36729 => tr!("Fire Resilience"),
        36730 => tr!("Ice Resilience"),
        36731 => tr!("Earth Resilience"),
        36732 => tr!("Energy Resilience"),
        36733 => tr!("Holy Resilience"),
        36734 => tr!("Death Resilience"),
        36735 => tr!("Physical Resilience"),
        36736 => tr!("Fire Amplification"),
        36737 => tr!("Ice Amplification"),
        36738 => tr!("Earth Amplification"),
        36739 => tr!("Energy Amplification"),
        36740 => tr!("Holy Amplification"),
        36741 => tr!("Death Amplification"),
        36742 => tr!("Physical Amplification"),
        7439 => tr!("Berserk potion"),
        7440 => tr!("Mastermind potion"),
        7443 => tr!("Bullseye potion"),
        49271 => tr!("Transcendence potion"),
        35563 => tr!("Magic shield potion"),
        _ => tr!("Unknown potion"),
    }
}
