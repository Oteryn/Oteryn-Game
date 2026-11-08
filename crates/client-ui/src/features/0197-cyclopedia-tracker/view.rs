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
    tracker_rows(panel, false)
}

pub fn tracker_rows(
    panel: &CyclopediaTrackerPanel,
    boss: bool,
) -> Vec<&oteryn_client_runtime::cyclopedia_tracker::TrackerEntry> {
    use super::TrackerSort;
    let (snapshot, search, sort, hide_completed) = if boss {
        (
            &panel.bosstiary_snapshot,
            &panel.boss_search,
            panel.boss_sort,
            panel.boss_hide_completed,
        )
    } else {
        (
            &panel.bestiary_snapshot,
            &panel.search,
            panel.sort,
            panel.hide_completed,
        )
    };
    let search = search.to_lowercase();
    let mut rows: Vec<_> = snapshot
        .iter()
        .flat_map(|s| &s.entries)
        .filter(|e| {
            (!hide_completed || !e.completed) && e.creature_key.to_lowercase().contains(&search)
        })
        .collect();
    rows.sort_by(|a, b| {
        let order = match sort {
            TrackerSort::Name => std::cmp::Ordering::Equal,
            TrackerSort::Kills => b.kill_count.cmp(&a.kill_count),
            TrackerSort::Remaining => remaining_to_stage(a).cmp(&remaining_to_stage(b)),
        };
        order.then_with(|| a.creature_key.cmp(&b.creature_key))
    });
    rows
}

fn remaining_to_stage(entry: &oteryn_client_runtime::cyclopedia_tracker::TrackerEntry) -> u32 {
    entry
        .thresholds
        .iter()
        .copied()
        .find(|goal| *goal > entry.kill_count)
        .unwrap_or(entry.kill_count)
        .saturating_sub(entry.kill_count)
}
