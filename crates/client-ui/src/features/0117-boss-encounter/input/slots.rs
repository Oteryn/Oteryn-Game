use super::{BossEncounterPanel, Intent};
use oteryn_client_runtime::boss_encounter::{bosstiary_slots::BossSlotRequest, catalogue::Entry};
/// Progress eligibility is only offered from confirmed canonical records and current slot state.
pub fn slot_candidates(panel: &BossEncounterPanel) -> Vec<&Entry> {
    let Some(slots) = &panel.slots else {
        return vec![];
    };
    let Some(details) = &slots.details else {
        return vec![];
    };
    super::catalogue_cards(panel)
        .into_iter()
        .filter(|e| e.stage() >= 1 && e.key != details.boosted && !slots.bosses.contains(&e.key))
        .collect()
}
impl BossEncounterPanel {
    pub fn select_slot_boss(&mut self, previous: bool) {
        if self.pending {
            return;
        }
        let rows = slot_candidates(self);
        if rows.is_empty() {
            self.feedback = Some("Browse the gallery and refresh slots to load eligible bosses");
            return;
        }
        let i = rows
            .iter()
            .position(|e| e.key == self.fields[0])
            .map_or(0, |i| {
                if previous {
                    (i + rows.len() - 1) % rows.len()
                } else {
                    (i + 1) % rows.len()
                }
            });
        self.fields[0] = rows[i].key.clone();
    }
}
pub(super) fn request(
    panel: &BossEncounterPanel,
    intent: Intent,
) -> Result<BossSlotRequest, &'static str> {
    if intent == Intent::SlotsDetailed {
        return Ok(BossSlotRequest::Detailed(Box::new(BossSlotRequest::Query)));
    }
    let slots = panel.slots.as_ref().ok_or("Refresh boss slots first")?;
    let details = slots
        .details
        .as_ref()
        .ok_or("Refresh detailed boss slots first")?;
    let slot = panel.fields[1]
        .parse::<u8>()
        .ok()
        .filter(|n| (1..=2).contains(n))
        .ok_or("Choose an unlocked slot")?;
    let at = usize::from(slot - 1);
    if slots.unlocked & (1 << at) == 0 {
        return Err("This boss slot is locked");
    }
    if details.now < slots.ready_at_ms {
        return Err("Slot action cooldown: refresh slots to confirm readiness");
    }
    let inner = if intent == Intent::SlotEquipSelected {
        if !slots.bosses[at].is_empty() {
            return Err("Remove the assigned boss before equipping this slot");
        }
        if !slot_candidates(panel)
            .iter()
            .any(|e| e.key == panel.fields[0])
        {
            return Err("Select an unassigned non-boosted boss with Prowess from the gallery");
        }
        BossSlotRequest::Equip {
            slot,
            boss: panel.fields[0].clone(),
        }
    } else {
        if slots.bosses[at].is_empty() {
            return Err("This boss slot is empty");
        }
        BossSlotRequest::Remove { slot }
    };
    Ok(BossSlotRequest::Detailed(Box::new(inner)))
}
pub fn slot_text(panel: &BossEncounterPanel) -> String {
    use std::fmt::Write;
    let Some(slots) = &panel.slots else {
        return "\nBoss slots: refresh to load unlocked slots, equipment bonuses and removal fees.\n".into();
    };
    let mut text = String::from("\nBoss slots — equipment loot\n");
    for (i, key) in slots.bosses.iter().enumerate() {
        let state = if slots.unlocked & (1 << i) == 0 {
            "Locked"
        } else if key.is_empty() {
            "Empty"
        } else {
            panel
                .boss_names
                .get(key)
                .map_or(key.as_str(), String::as_str)
        };
        let _ = write!(
            text,
            "{}Slot {}: {state}",
            if panel.fields[1] == (i + 1).to_string() {
                "> "
            } else {
                ""
            },
            i + 1
        );
        if let Some(details) = &slots.details {
            let _ = write!(text, " | +{}% equipment loot", details.bonuses[i]);
        }
        text.push('\n');
    }
    let base = slots.details.as_ref().map_or(slots.bonus, |d| d.base_bonus);
    let _ = writeln!(
        text,
        "Confirmed bonus: {base}% | Points: {} | Removal fee: {} gold | Ready at: {}",
        slots.points, slots.remove_fee, slots.ready_at_ms
    );
    if let Some(details) = &slots.details {
        if !details.boosted.is_empty() {
            let name = panel
                .boss_names
                .get(&details.boosted)
                .map_or(details.boosted.as_str(), String::as_str);
            let _ = writeln!(
                text,
                "Daily boosted boss: {name} — cannot be assigned to a slot"
            );
        }
        if details.now < slots.ready_at_ms {
            let _ = writeln!(
                text,
                "Slot cooldown: {} ms at snapshot time; refresh to confirm readiness",
                slots.ready_at_ms - details.now
            );
        } else {
            text.push_str("Slot actions ready at this server snapshot\n");
        }
    }
    text.push_str("Bonuses apply to eligible equipment loot, not currency or creature products.\nBrowse/search the gallery for more bosses, then Previous/Next eligible slot boss.\n");
    for boss in slot_candidates(panel) {
        let _ = writeln!(
            text,
            "{} {} — {} — stage {}/3",
            if panel.fields[0] == boss.key {
                ">"
            } else {
                " "
            },
            boss.name,
            boss.category_name(),
            boss.stage()
        );
    }
    text
}
