use super::*;
#[must_use]
pub fn panel_text(panel: &DailyRewardPanel) -> String {
    let mut text = String::from(tr!(
        "Daily Reward (Ctrl+D)\nRefresh near a reward shrine to enable claiming.\n"
    ));
    if let Some(w) = panel.wall {
        text.push_str(&lfmt!(
            "Day {} of 30\nStreak: {}\nJokers: {}\nStatus: {}\n",
            w.lane_day,
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
        // Presentation of the existing DAILY30B rule; the server alone rolls
        // and awards XP (gameplay/progression/src/daily_reward.rs).
        let chance = match w.streak {
            0..=7 => w.streak as u8,
            8..=13 => 7,
            14..=20 => 8,
            21..=27 => 9,
            _ => 10,
        };
        text.push_str(&lfmt!(
            "Weapon proficiency XP: {}% chance to gain double XP\n",
            chance
        ));
        text.push_str(tr!(
            "Each eligible weapon proficiency XP award can be doubled. Chance follows the last refreshed streak: 1% per streak day up to 7%; 8% at 14, 9% at 21, 10% at 28 (maximum). Refresh after claiming to update the bonus.\n"
        ));
        text.push_str(tr!("30-day calendar\n"));
        for day in 1..=30 {
            let state = if day == w.lane_day {
                if w.claimed {
                    "claimed"
                } else if w.claim_enabled {
                    tr!("available")
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
