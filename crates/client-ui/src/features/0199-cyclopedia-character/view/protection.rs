use oteryn_client_runtime::cyclopedia_character::protection::{Consumption, Forecast, Scenario};
fn xp(value: u64, total: u64) -> String {
    let percent = if total == 0 {
        0
    } else {
        u128::from(value) * 10_000 / u128::from(total)
    };
    format!(
        "{value} XP · {}.{:02}% of current experience",
        percent / 100,
        percent % 100
    )
}
fn risk(s: Scenario) -> String {
    let pct = |v: u16| format!("{}.{:01}%", v / 10, v % 10);
    format!(
        "Per occupied equipment slot: {}\r\nBackpack, including its contents: {}",
        pct(s.equipment_per_mille),
        pct(s.container_per_mille)
    )
}
fn consumed(s: Scenario, count: u8) -> String {
    let blessings = match s.consumption {
        Consumption::Keep => "All blessings kept".into(),
        Consumption::Regular => format!("{count} regular blessings consumed"),
        Consumption::Twist => "PvP safeguard consumed; regular blessings kept".into(),
    };
    format!(
        "{blessings}\r\nAmulet of Loss: {}",
        if s.amulet_consumed {
            "consumed"
        } else {
            "not consumed"
        }
    )
}
pub fn protection_cards(f: &Forecast) -> Vec<(&'static str, String)> {
    let unavailable = || "Not available in this scope".to_owned();
    vec![
        (
            "Creature death · XP",
            xp(f.pve.experience_lost, f.experience),
        ),
        (
            "Player death · XP",
            f.pvp
                .map_or_else(unavailable, |s| xp(s.experience_lost, f.experience)),
        ),
        ("Creature death · Item chance", risk(f.pve)),
        (
            "Player death · Item chance",
            f.pvp.map_or_else(unavailable, risk),
        ),
        (
            "Creature death · Consumption",
            consumed(f.pve, f.regular_blessings),
        ),
        (
            "Player death · Consumption",
            f.pvp
                .map_or_else(unavailable, |s| consumed(s, f.regular_blessings)),
        ),
    ]
}
pub fn protection_heading(f: &Forecast) -> String {
    let state = |active| if active { "active" } else { "inactive" };
    let seconds = f.observed_at_unix_ms / 1000;
    format!(
        "Level {} · {} regular blessings · PvP safeguard: {}\r\nPromotion benefit: {} · Red/black skull: {} · Adventurer PvP protection: {}\r\nObserved at {:02}:{:02}:{:02} UTC",
        f.level,
        f.regular_blessings,
        if f.twist { "owned" } else { "not owned" },
        state(f.promotion_benefit),
        state(f.red_or_black),
        state(f.adventurer),
        seconds / 3600 % 24,
        seconds / 60 % 60,
        seconds % 60
    )
}
pub const PROTECTION_NOTE: &str = "Normal-death baseline before creature-specific Bless reductions. Item chances apply separately to occupied slots; a lost backpack takes its contents. The server recalculates all facts at death.";
