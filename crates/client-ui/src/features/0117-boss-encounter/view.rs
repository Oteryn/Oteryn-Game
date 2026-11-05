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
        if page.cooldowns.is_empty() && page.progress.is_empty() {
            text.push_str("No boss records on this page\n");
        }
        if page.cooldown_next.is_some() || page.progress_next.is_some() {
            text.push_str("More records: Next page\n");
        }
    } else {
        text.push_str("No server snapshot loaded\n");
    }
    if panel.pending {
        text.push_str("Waiting for the server\n");
    }
    if let Some(feedback) = panel.feedback {
        let _ = writeln!(text, "{feedback}");
    }
    text
}
