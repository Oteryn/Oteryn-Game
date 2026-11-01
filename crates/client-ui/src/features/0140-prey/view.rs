use super::*;
use std::fmt::Write;

#[must_use]
pub fn slot_text(slot: &prey::Slot) -> String {
    let state = match slot.state {
        0 => "locked",
        1 => "selection",
        2 => "active",
        _ => "unknown",
    };
    format!(
        "Slot {}: {state}; selected {}; bonus {} grade {} ({}%); {}s; option {}; free reroll at {}",
        slot.slot,
        if slot.selected.is_empty() {
            "none"
        } else {
            &slot.selected
        },
        slot.bonus,
        slot.grade,
        slot.percent,
        slot.hunting_seconds,
        slot.option,
        slot.free_reroll_at
    )
}

#[must_use]
pub fn panel_text(panel: &PreyPanel) -> String {
    let mut text = String::from("Prey\n");
    if let Some(reply) = &panel.reply {
        let _ = writeln!(text, "Wildcards: {}", reply.wildcard_balance);
        for slot in &reply.slots {
            let _ = writeln!(text, "{}", slot_text(slot));
            if !slot.creatures.is_empty() {
                let _ = writeln!(text, "  Candidates: {}", slot.creatures.join(", "));
            }
        }
    } else {
        text.push_str("No server state loaded\n");
    }
    if let Some(feedback) = panel.feedback {
        let _ = writeln!(text, "{feedback}");
    }
    text
}
