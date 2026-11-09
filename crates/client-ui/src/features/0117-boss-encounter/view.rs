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
    text.push_str(&super::browser_text(panel));
    if let Some((unlocked, selected)) = panel.hazard {
        let gauge: String = (1..=12)
            .map(|level| {
                if level <= selected {
                    '■'
                } else if level <= unlocked {
                    '□'
                } else {
                    '×'
                }
            })
            .collect();
        let _ = writeln!(
            text,
            "\nHazard [{gauge}]\nSelected level {selected} of {unlocked} unlocked"
        );
        if panel.fields[0].is_empty() {
            text.push_str("Choose a boss from the browser before entering.\n");
        } else {
            let _ = writeln!(text, "Enter {} at hazard {selected}", panel.fields[0]);
        }
        text.push_str("Use Lower/Raise hazard to choose an unlocked level, then Enter for the selected boss.\nEntry and cooldown eligibility are confirmed by the server.\n");
    } else {
        text.push_str("Query hazard to load unlocked levels before choosing an encounter.\n");
    }
    if let Some(slots) = &panel.slots {
        for (index, boss) in slots.bosses.iter().enumerate() {
            let state = if slots.unlocked & (1 << index) == 0 {
                "Locked"
            } else if boss.is_empty() {
                "Empty"
            } else {
                boss
            };
            let _ = writeln!(text, "Slot {}: {state}", index + 1);
        }
        let _ = writeln!(
            text,
            "Confirmed bonus: {} | Points: {} | Removal fee: {} gold | Ready at: {}",
            slots.bonus, slots.points, slots.remove_fee, slots.ready_at_ms
        );
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

/// A countdown is presentation only; refreshing confirms availability with the server.
pub fn cooldown_text(panel: &BossEncounterPanel, elapsed_ms: u64) -> String {
    let mut out = String::from("Boss cooldown tracker — Ctrl+Alt+Shift+J\r\n");
    if !panel.selected {
        out.push_str("Feature unavailable in this session\r\n");
        return out;
    }
    if let Some(snapshot) = &panel.snapshot {
        if snapshot.cooldowns.is_empty() {
            out.push_str("No cooldown records on this page\r\n");
        }
        let mut cooldowns: Vec<_> = snapshot.cooldowns.iter().collect();
        cooldowns.sort_by(|(a_key, a_deadline), (b_key, b_deadline)| {
            a_deadline.cmp(b_deadline).then_with(|| a_key.cmp(b_key))
        });
        for (key, deadline) in cooldowns {
            let remaining = deadline
                .saturating_sub(snapshot.now)
                .saturating_sub(elapsed_ms);
            if remaining == 0 {
                let _ = write!(out, "{key}: Ready — refresh to confirm\r\n");
            } else {
                let _ = write!(out, "{key}: {remaining} ms remaining\r\n");
            }
        }
        if snapshot.cooldown_next.is_some() || snapshot.progress_next.is_some() {
            out.push_str("More records available: Next page\r\n");
        }
    } else {
        out.push_str("Refresh to load server cooldowns\r\n");
    }
    if panel.pending {
        out.push_str("Waiting for the server\r\n");
    }
    if let Some(feedback) = panel.feedback {
        out.push_str(feedback);
    }
    out
}
