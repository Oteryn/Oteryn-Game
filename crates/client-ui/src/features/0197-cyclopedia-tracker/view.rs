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

/// The widget keeps its own snapshot even while the command editor displays bosses.
pub fn bestiary_rows(
    panel: &CyclopediaTrackerPanel,
) -> Vec<&oteryn_client_runtime::cyclopedia_tracker::TrackerEntry> {
    use super::TrackerSort;
    let search = panel.search.to_lowercase();
    let mut rows: Vec<_> = panel
        .bestiary_snapshot
        .iter()
        .flat_map(|s| &s.entries)
        .filter(|e| {
            (!panel.hide_completed || !e.completed)
                && e.creature_key.to_lowercase().contains(&search)
        })
        .collect();
    rows.sort_by(|a, b| {
        let order = match panel.sort {
            TrackerSort::Name => std::cmp::Ordering::Equal,
            TrackerSort::Kills => b.kill_count.cmp(&a.kill_count),
            TrackerSort::Remaining => a.thresholds[2]
                .saturating_sub(a.kill_count)
                .cmp(&b.thresholds[2].saturating_sub(b.kill_count)),
        };
        order.then_with(|| a.creature_key.cmp(&b.creature_key))
    });
    rows
}
