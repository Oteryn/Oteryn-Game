use super::BossEncounterPanel;
use std::fmt::Write;
#[must_use]
pub fn panel_text(panel: &BossEncounterPanel) -> String {
    let mut text = String::from("Boss encounter / Bosstiary (Ctrl+Shift+J)\n");
    if let Some(page) = &panel.snapshot {
        let _ = writeln!(
            text,
            "Boss points: {}\nServer time: {}",
            page.points, page.now
        );
        for (key, at) in &page.cooldowns {
            let _ = writeln!(text, "{key}: {} ms remaining", at.saturating_sub(page.now));
        }
        for (key, kills) in &page.progress {
            let _ = writeln!(text, "{key}: {kills} kills");
        }
        if let Some(taints) = page.taints {
            let _ = writeln!(text, "Soul War taints: {}", taints.soul_war);
            let _ = writeln!(
                text,
                "Rotten Blood taints: {}; final taint: {}",
                taints.rotten_blood,
                if taints.rotten_blood_final {
                    "yes"
                } else {
                    "no"
                }
            );
            text.push_str("Rotten Blood bosses defeated:");
            for (index, name) in ["Ichgahal", "Murcion", "Vemiath", "Chagorz"]
                .iter()
                .enumerate()
            {
                if taints.rotten_blood_flags & (1 << index) != 0 {
                    let _ = write!(text, " {name}");
                }
            }
            if taints.rotten_blood_flags == 0 {
                text.push_str(" none");
            }
            text.push('\n');
        }
        if page.cooldowns.is_empty() && page.progress.is_empty() {
            text.push_str("No boss records on this page\n");
        }
        if page.cooldown_next.is_some() || page.progress_next.is_some() {
            text.push_str("More records: Next page\n");
        }
    } else {
        text.push_str("No server snapshot loaded\n");
    }
    let _ = writeln!(text, "{}", panel.summary);
    if panel.pending {
        text.push_str("Waiting for the server\n");
    }
    if let Some(feedback) = panel.feedback {
        let _ = writeln!(text, "{feedback}");
    }
    text
}
