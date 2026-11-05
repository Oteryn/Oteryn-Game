use super::*;
#[must_use]
pub fn panel_text(panel: &DailyRewardPanel) -> String {
    let mut text =
        String::from("Daily Reward (Ctrl+D)\nRefresh near a reward shrine to enable claiming.\n");
    if let Some(w) = panel.wall {
        text.push_str(&format!(
            "Day {} of 30\nStreak: {}\nJokers: {}\nStatus: {}\n",
            w.lane_day,
            w.streak,
            w.jokers,
            if w.claimed {
                "claimed"
            } else if w.claim_enabled {
                "ready to claim"
            } else {
                "not available"
            }
        ));
    } else {
        text.push_str("No server state loaded\n");
    }
    if let Some(feedback) = panel.feedback {
        text.push_str(feedback);
        text.push('\n');
    }
    text
}
