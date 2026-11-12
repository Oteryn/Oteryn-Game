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
/// Promotion facts the server confirms through the forecast flag; the benefits follow Tibia 15.33
/// (promoted title, faster regeneration, soul cap 200 with a 15 s soul tick, reduced death loss).
/// `title` is the character's exact vocation title once capability 487 reported its id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PromotionView {
    pub promoted: bool,
    pub soul_cap: u16,
    pub soul_tick_seconds: u16,
    pub title: Option<&'static str>,
}
pub const NORMAL_SOUL_CAP: u16 = 100;
pub const PROMOTED_SOUL_CAP: u16 = 200;
pub fn promotion_view(f: &Forecast, vocation_id: Option<u8>) -> PromotionView {
    PromotionView {
        promoted: f.promotion_benefit,
        soul_cap: if f.promotion_benefit {
            PROMOTED_SOUL_CAP
        } else {
            NORMAL_SOUL_CAP
        },
        soul_tick_seconds: if f.promotion_benefit { 15 } else { 120 },
        title: vocation_id.and_then(crate::cyclopedia_character_vocation::vocation_title),
    }
}
pub fn promotion_text(f: &Forecast, vocation_id: Option<u8>) -> String {
    let v = promotion_view(f, vocation_id);
    match (v.promoted, v.title) {
        (true, Some(title)) => format!(
            "Promotion: promoted rank ({title}) · faster health/mana regeneration · soul cap {} with a {} s soul tick · reduced death loss",
            v.soul_cap, v.soul_tick_seconds
        ),
        (true, None) => format!(
            "Promotion: promoted rank (promoted vocation title) · faster health/mana regeneration · soul cap {} with a {} s soul tick · reduced death loss",
            v.soul_cap, v.soul_tick_seconds
        ),
        (false, Some(title)) => format!(
            "Promotion: not promoted · base vocation title ({title}) · soul cap {} with a {} s soul tick",
            v.soul_cap, v.soul_tick_seconds
        ),
        (false, None) => format!(
            "Promotion: not promoted · base vocation title · soul cap {} with a {} s soul tick",
            v.soul_cap, v.soul_tick_seconds
        ),
    }
}
pub fn protection_heading(f: &Forecast, vocation_id: Option<u8>) -> String {
    let state = |active| if active { "active" } else { "inactive" };
    let seconds = f.observed_at_unix_ms / 1000;
    format!(
        "Level {} · {} regular blessings · PvP safeguard: {}\r\n{}\r\nRed/black skull: {} · Adventurer PvP protection: {}\r\nObserved at {:02}:{:02}:{:02} UTC",
        f.level,
        f.regular_blessings,
        if f.twist { "owned" } else { "not owned" },
        promotion_text(f, vocation_id),
        state(f.red_or_black),
        state(f.adventurer),
        seconds / 3600 % 24,
        seconds / 60 % 60,
        seconds % 60
    )
}
pub const PROTECTION_NOTE: &str = "Normal-death baseline before creature-specific Bless reductions. Item chances apply separately to occupied slots; a lost backpack takes its contents. The server recalculates all facts at death.";
