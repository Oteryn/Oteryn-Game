//! Read-only catalogue/history request; it cannot issue a claim or alter reward choices.
use super::*;
use oteryn_client_runtime::raw_feature::{FeatureCommand, GatedCommandResult};
pub fn request_details(panel: &mut DailyRewardPanel) -> Option<FeatureCommand> {
    if !panel.selected || panel.pending {
        return None;
    }
    panel.pending = true;
    panel.details_pending = true;
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
        return "Refresh reward details to load the server catalogue and account history.\n".into();
    };
    let mut text = String::from("30-day reward catalogue (Free / Premium)\n");
    for (index, day) in details.days.iter().enumerate() {
        text.push_str(&format!(
            "Day {:02}: Free [{}] | Premium [{}]\n",
            index + 1,
            tier_text(day.free),
            tier_text(day.premium)
        ));
    }
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
fn tier_text(t: daily_reward::details::Tier) -> String {
    format!(
        "cosmetic {}; prey {}; temple scroll {}; XP {}%/{}m; bounty {}",
        t.cosmetic_tokens,
        t.prey_wildcards,
        t.temple_scrolls,
        t.xp_percent,
        t.xp_minutes,
        t.bounty_tokens
    )
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
