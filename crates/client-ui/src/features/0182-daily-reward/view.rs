use super::*;
/// The reward wall repeats every seven days (Tibia 15.33 Reward Wall).
pub const CYCLE_DAYS: u8 = 7;
/// The server lane still counts 1..=30; the wall shows the day inside the seven-day cycle.
#[must_use]
pub fn cycle_day(lane_day: u8) -> u8 {
    lane_day.saturating_sub(1) % CYCLE_DAYS + 1
}
/// Streak bonus chance, one point per consecutive claim and capped at 7%.
#[must_use]
pub fn streak_bonus_percent(streak: u32) -> u8 {
    streak.min(7) as u8
}
fn streak_bonus_text(streak: u32) -> String {
    let mut text = String::from("Reward Streak Bonus\n");
    match streak_bonus_percent(streak) {
        0 => text.push_str("You currently have no Reward Streak Bonus.\n"),
        chance => text.push_str(&format!(
            "Your reward streak currently grants you a {chance}% chance to gain double weapon proficiency XP.\n"
        )),
    }
    text.push_str(
        "Claim your daily reward once between two regular server saves to keep your streak rising. Your streak grants a chance of up to 7% to gain double weapon proficiency XP.\n",
    );
    text
}
#[must_use]
pub fn panel_text(panel: &DailyRewardPanel) -> String {
    let mut text = String::from(tr!(
        "Daily Reward (Ctrl+D)\nRefresh near a reward shrine to enable claiming.\n"
    ));
    if let Some(w) = panel.wall {
        text.push_str(&format!(
            "Day {} of {}\nStreak: {}\nJokers: {}\nStatus: {}\n",
            cycle_day(w.lane_day),
            CYCLE_DAYS,
            w.streak,
            w.jokers,
            if w.claimed {
                "claimed"
            } else if w.claim_enabled {
                tr!("ready to claim")
            } else {
                tr!("not available")
            }
        ));
        text.push_str(&streak_bonus_text(w.streak));
        text.push_str("Seven-day reward cycle (Free / Premium)\n");
        text.push_str(&super::details::cycle_text(
            panel,
            cycle_day(w.lane_day),
            w.claimed,
        ));
        text.push_str(
            "Premium accounts receive larger rewards: more Cosmetic Tokens, more Prey Wildcards and stronger XP boosts.\n",
        );
        for (day, streak) in panel.claim_receipts.iter().flatten() {
            text.push_str(&lfmt!(
                "Confirmed this session: day {day}, streak {streak}\n",
                day = day,
                streak = streak
            ));
        }
    } else {
        text.push_str(tr!("No server state loaded\n"));
    }
    text.push_str(&super::details_text(panel));
    if let Some(feedback) = panel.feedback {
        text.push_str(feedback);
        text.push('\n');
    }
    text
}
