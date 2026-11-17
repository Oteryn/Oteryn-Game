use super::WeaponProficiencyPanel;
#[must_use]
pub fn panel_text(p: &WeaponProficiencyPanel) -> String {
    let mut state = p
        .result
        .clone()
        .unwrap_or_else(|| tr!("No server result yet").to_owned());
    if let Some(page) = &p.catalogue {
        use super::{mastery_target, progress_level, progress_percent};
        let mastered = page
            .entries
            .iter()
            .filter(|entry| entry.experience >= mastery_target(entry))
            .count();
        state.push_str(&lfmt!(
            "\nSummary: {} weapons, {} mastered\n",
            page.entries.len(),
            mastered
        ));
        for definition in &p.advancements {
            if let Some(entry) = page
                .entries
                .iter()
                .find(|entry| entry.definition == *definition)
            {
                state.push_str(&lfmt!(
                    "Advancement: {} — level {}\n",
                    entry.name,
                    progress_level(entry)
                ));
            }
        }
        if let Some(entry) = p.weapon_entry().or_else(|| p.progress_entry()) {
            state.push_str(&lfmt!(
                "Mastery: {} — {}% ({} / {} XP)\nPerk tree\n",
                entry.name,
                progress_percent(entry),
                entry.experience,
                mastery_target(entry)
            ));
            for (text, _) in p.perk_tree_rows() {
                state.push_str(&text);
                state.push('\n');
            }
        }

        use std::fmt::Write;
        let _ = lwriteln!(
            state,
            "\nWeapons {}-{} of {}",
            page.offset.saturating_add(1).min(page.total),
            page.offset.saturating_add(page.entries.len() as u32),
            page.total
        );
        for entry in &page.entries {
            let _ = lwriteln!(state, "{}: {} XP", entry.name, entry.experience);
            let _ = lwriteln!(state, "Level thresholds: {:?}", entry.thresholds);
            for (level, data) in entry.levels.iter().enumerate() {
                let _ = lwriteln!(state, "  Level {}", level + 1);
                for (index, perk) in data.perks.iter().enumerate() {
                    let mark = if data.selected == Some(index as u8) {
                        "☑"
                    } else {
                        "☐"
                    };
                    let _ = writeln!(state, "    {mark} {perk}");
                }
            }
        }
    }
    if let Some(shaping) = &p.shaping {
        use std::fmt::Write;
        let _ = lwriteln!(
            state,
            "\nShaping costs: modify {} / {} dust; reshape {} dust; clear {} dust; orb rank {} orb(s).",
            shaping.modify_dust[0],
            shaping.modify_dust[1],
            shaping.reshape_dust,
            shaping.clear_dust,
            shaping.orbs
        );
        for (index, slot) in shaping.slots.iter().enumerate() {
            if let Some(slot) = slot {
                if usize::from(p.slot) == index + 1 {
                    state.push_str("╔════════════════════════════════════╗\n");
                    state.push_str(tr!("║ Selected modified slot\n"));
                }
                let cost = shaping
                    .rank_dust
                    .get(usize::from(slot.rank))
                    .copied()
                    .unwrap_or(0);
                let _ = lwriteln!(
                    state,
                    "Slot {}: rank {} — {}",
                    index + 1,
                    slot.rank,
                    slot.current
                );
                if usize::from(p.slot) == index + 1 {
                    state.push_str("╚════════════════════════════════════╝\n");
                }
                if slot.rank < 10 {
                    let _ = lwriteln!(state, "Next rank ({} dust): {}", cost, slot.next);
                } else {
                    let _ = lwriteln!(state, "Maximum rank reached");
                }
                let _ = lwriteln!(state, "Maximum: {}", slot.maximum);
                for (choice, offer) in slot.offers.iter().enumerate() {
                    let _ = lwriteln!(state, "Offer {}: {}", choice + 1, offer);
                }
            }
        }
        if shaping.revision == 0 {
            state.push_str(tr!(
                "No committed proficiency track yet; shaping changes are unavailable.\n"
            ));
        }
    }
    let mut catalogue = String::new();
    if let Some(page) = &p.page {
        catalogue.push_str(&lfmt!(
            "Weapons {}–{} / {}\n",
            if page.rows.is_empty() {
                0
            } else {
                page.offset + 1
            },
            page.offset + page.rows.len() as u32,
            page.total
        ));
        for row in &page.rows {
            catalogue.push_str(&lfmt!(
                "{}: {} XP, level {}\n",
                row.name,
                row.experience,
                row.level
            ));
        }
    }
    catalogue.push_str(&state);
    let state = catalogue.as_str();
    match p.feedback {
        Some(f) => lfmt!("Weapon proficiency\n{state}\n{f}", f = f, state = state),
        None => lfmt!("Weapon proficiency\n{state}", state = state),
    }
}

/// A small per-frame snapshot; the HUD never clones the complete perk catalogue.
pub struct HudView {
    pub definition: u32,
    pub name: String,
    pub percent: u32,
    pub pending: bool,
    pub advancement: Option<(u32, String)>,
}
pub fn hud_view(p: &WeaponProficiencyPanel) -> Option<HudView> {
    if !p.selected {
        return None;
    }
    let entry = p.progress_entry()?;
    let advancement = p.advancements.first().and_then(|definition| {
        let entry = p
            .catalogue
            .as_ref()?
            .entries
            .iter()
            .find(|entry| entry.definition == *definition)?;
        Some((*definition, entry.name.clone()))
    });
    Some(HudView {
        definition: entry.definition,
        name: entry.name.clone(),
        percent: super::progress_percent(entry),
        pending: p.pending,
        advancement,
    })
}
