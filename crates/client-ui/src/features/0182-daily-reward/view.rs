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
        text.push_str("30-day calendar\n");
        for day in 1..=30 {
            let state = if day == w.lane_day {
                if w.claimed {
                    "claimed"
                } else if w.claim_enabled {
                    "available"
                } else {
                    "current"
                }
            } else {
                "unobserved"
            };
            text.push_str(&format!(
                "{day:02}: {state}{}",
                if day % 5 == 0 { "\n" } else { " | " }
            ));
        }
        for (day, streak) in panel.claim_receipts.iter().flatten() {
            text.push_str(&format!(
                "Confirmed this session: day {day}, streak {streak}\n"
            ));
        }
    } else {
        text.push_str("No server state loaded\n");
    }
    if let Some(feedback) = panel.feedback {
        text.push_str(feedback);
        text.push('\n');
    }
    text
}
