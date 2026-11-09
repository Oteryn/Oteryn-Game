use super::{PreyPanel, prey};
use std::fmt::Write;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridLayout {
    pub columns: usize,
    pub rows: usize,
}

impl GridLayout {
    pub fn for_panel(panel: &PreyPanel) -> Self {
        let count = panel
            .confirmed
            .as_ref()
            .map_or(9, |reply| usize::from(reply.list_size));
        Self {
            columns: 3,
            rows: count.div_ceil(3),
        }
    }
}

/// Label, checked, enabled. A retained tier after level loss remains checked and disabled.
pub fn tier_choices(panel: &PreyPanel, slot: u8) -> [(String, bool, bool); 5] {
    std::array::from_fn(|index| {
        let tier = index as u8 + 1;
        let state = panel.slot(slot);
        (
            format!("{tier}★"),
            state.is_some_and(|state| state.chosen_tier == tier),
            panel.pending.is_none()
                && panel.unlocked(slot)
                && state.is_some_and(|state| tier <= state.allowed_tier),
        )
    })
}

pub fn creature_label(key: &str) -> String {
    key.strip_prefix("oteryn:creature.")
        .unwrap_or(key)
        .replace('_', " ")
}

pub fn slot_text(slot: &prey::Slot) -> String {
    if slot.state == 0 {
        return lfmt!("Slot {} · Locked", slot.slot);
    }
    if slot.selected.is_empty() {
        return lfmt!("Slot {} · Choose a creature", slot.slot);
    }
    let bonus = [
        tr!("None"),
        tr!("Damage"),
        tr!("Defense"),
        tr!("Experience"),
        tr!("Loot"),
    ]
    .get(usize::from(slot.bonus))
    .copied()
    .unwrap_or(tr!("Unknown"));
    lfmt!(
        "Slot {} · {}\r\n{} +{}% · grade {} · {} min",
        slot.slot,
        creature_label(&slot.selected),
        bonus,
        slot.percent,
        slot.grade,
        slot.hunting_seconds.div_ceil(60)
    )
}

pub fn panel_text(panel: &PreyPanel) -> String {
    if !panel.selected {
        return tr!("Prey is unavailable in this session.").into();
    }
    if let Some(feedback) = panel.feedback {
        return feedback.into();
    }
    if panel.pending.is_some() {
        return tr!("Waiting for the server…").into();
    }
    let Some(reply) = &panel.confirmed else {
        return tr!("Loading Prey…").into();
    };
    let mut text = lfmt!(
        "Wildcards: {} · {} creatures per list{}",
        reply.wildcard_balance,
        reply.list_size,
        if reply.expansion_owned {
            tr!(" · Prey List Expansion")
        } else {
            ""
        }
    );
    for slot in &reply.slots {
        let _ = write!(text, "\n{}", slot_text(slot));
        if !slot.creatures.is_empty() {
            let _ = lwrite!(text, "\n  Candidates: {}", slot.creatures.join(", "));
        }
    }
    text
}
