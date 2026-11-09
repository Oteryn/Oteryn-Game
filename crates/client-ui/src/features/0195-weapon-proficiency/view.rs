use super::WeaponProficiencyPanel;
#[must_use]
pub fn panel_text(p: &WeaponProficiencyPanel) -> String {
    let mut state = p
        .result
        .clone()
        .unwrap_or_else(|| tr!("No server result yet").to_owned());
    if let Some(page) = &p.catalogue {
        use std::fmt::Write;
        let _ = lwriteln!(
            state,
            "\nWeapons {}-{} of {}",
            page.offset.saturating_add(1).min(page.total),
            page.offset.saturating_add(page.entries.len() as u32),
            page.total
        );
        for entry in &page.entries {
            let _ = lwriteln!(
                state,
                "{} (weapon id {}): {} XP",
                entry.name,
                entry.definition,
                entry.experience
            );
            let _ = lwriteln!(state, "Level thresholds: {:?}", entry.thresholds);
            for (level, data) in entry.levels.iter().enumerate() {
                let _ = lwriteln!(
                    state,
                    "  Level {} (selected {:?})",
                    level + 1,
                    data.selected
                );
                for (index, perk) in data.perks.iter().enumerate() {
                    let _ = writeln!(state, "    {index}: {perk}");
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
                "{} [{}]: {} XP, level {}, perks {:?}\n",
                row.name,
                row.definition,
                row.experience,
                row.level,
                row.selections
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
