use super::CyclopediaTrackerPanel;
use std::fmt::Write;
#[must_use]
pub fn panel_text(panel: &CyclopediaTrackerPanel) -> String {
    let mut out = String::from(if panel.boss {
        "Bosstiary tracker\n"
    } else {
        "Bestiary tracker\n"
    });
    if let Some(snapshot) = &panel.snapshot {
        for e in &snapshot.entries {
            let _ = writeln!(
                out,
                "{}: {} kills ({} / {} / {}){}",
                e.creature_key,
                e.kill_count,
                e.thresholds[0],
                e.thresholds[1],
                e.thresholds[2],
                if e.completed { " complete" } else { "" }
            );
        }
    } else {
        out.push_str("No server snapshot yet\n")
    }
    if let Some(f) = panel.feedback {
        let _ = writeln!(out, "{f}");
    }
    out
}
