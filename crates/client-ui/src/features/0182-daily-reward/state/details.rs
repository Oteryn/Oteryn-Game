//! Read-only catalogue/history request; it cannot issue a claim or alter reward choices.
use super::*;
use oteryn_client_runtime::raw_feature::{FeatureCommand, GatedCommandResult};
pub fn request_details(panel: &mut DailyRewardPanel) -> Option<FeatureCommand> {
    if !panel.selected || panel.pending {
        return None;
    }
    panel.pending = true;
    panel.details_pending = true;
    panel.details_attempted = true;
    panel.feedback = None;
    Some(FeatureCommand {
        capability: 182,
        kind: daily_reward::details::COMMAND_TYPE,
        payload: vec![],
    })
}
pub fn apply_details(panel: &mut DailyRewardPanel, result: &GatedCommandResult) -> bool {
    if !panel.details_pending {
        return false;
    }
    panel.pending = false;
    panel.details_pending = false;
    if !matches!(
        result.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    ) {
        panel.feedback = Some("The server refused reward details");
        return false;
    }
    match daily_reward::details::Details::decode(&result.payload) {
        Ok(details) => {
            panel.details = Some(details);
            panel.feedback = None;
            true
        }
        Err(_) => {
            panel.feedback = Some("Invalid reward catalogue/history reply");
            false
        }
    }
}
pub fn details_text(panel: &DailyRewardPanel) -> String {
    let Some(details) = &panel.details else {
        return String::new();
    };
    let mut text = String::new();
    text.push_str("Recent account/World history (latest 30 durable receipts)\n");
    if details.history[0].is_none() {
        text.push_str("No recorded claims\n");
    }
    for receipt in details.history.iter().flatten() {
        text.push_str(&format!(
            "{}: day {}, streak {}, jokers spent {} — {}\n",
            date(receipt.reset_day),
            receipt.lane_day,
            receipt.streak,
            receipt.jokers_spent,
            if receipt.own_character {
                "this character"
            } else {
                "another account character"
            }
        ));
    }
    text
}
/// Seven-day Reward Wall rows from the server catalogue; `today` is the 1-based cycle day.
pub(super) fn cycle_text(panel: &DailyRewardPanel, today: u8, claimed: bool) -> String {
    let Some(details) = &panel.details else {
        return "Rewards are loading from the server\n".into();
    };
    let mut text = String::new();
    for (index, day) in details.days.iter().take(7).enumerate() {
        let number = index as u8 + 1;
        let marker = if number == today {
            if claimed { "claimed" } else { "today" }
        } else {
            ""
        };
        text.push_str(&format!(
            "Day {number} {marker}\n  Free: {}\n  Premium: {}\n",
            tier_text(day.free),
            tier_text(day.premium)
        ));
    }
    text
}

fn tier_text(t: daily_reward::details::Tier) -> String {
    let mut parts = Vec::new();
    if t.cosmetic_tokens > 0 {
        parts.push(format!("{} Cosmetic Tokens", t.cosmetic_tokens));
    }
    if t.prey_wildcards > 0 {
        parts.push(format!("{}x Prey Wildcards", t.prey_wildcards));
    }
    match t.temple_scrolls {
        0 => {}
        1 => parts.push("1 Temple Teleport Scroll".into()),
        n => parts.push(format!("{n} Temple Teleport Scrolls")),
    }
    if t.xp_percent > 0 {
        parts.push(format!(
            "{} minutes XP Boost {}%",
            t.xp_minutes, t.xp_percent
        ));
    }
    match t.bounty_tokens {
        0 => {}
        1 => parts.push("Bounty Reroll Token".into()),
        n => parts.push(format!("{n}x Bounty Reroll Token")),
    }
    if parts.is_empty() {
        return "no reward".into();
    }
    parts.join(", ")
}

fn date(day: i64) -> String {
    if !(-719162..=2932896).contains(&day) {
        return "Unknown date".into();
    }
    let z = day + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let mut year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}
