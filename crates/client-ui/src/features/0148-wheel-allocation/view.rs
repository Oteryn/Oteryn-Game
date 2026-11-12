use super::*;
#[path = "view/catalogue.rs"]
mod catalogue;

pub fn domain_name(domain: u8) -> &'static str {
    [tr!("Green"), tr!("Red"), tr!("Purple"), tr!("Blue")]
        .get(usize::from(domain))
        .copied()
        .unwrap_or(tr!("Unknown"))
}

fn quality_name(quality: u8) -> &'static str {
    [tr!("Lesser"), tr!("Regular"), tr!("Greater")]
        .get(usize::from(quality))
        .copied()
        .unwrap_or(tr!("Unknown"))
}

fn fragment_name(kind: u8) -> &'static str {
    if kind == 0 { "basic" } else { "supreme" }
}

pub fn slot_label(panel: &WheelPanel, slot: usize) -> String {
    let Some(capacity) = SLOT_CAPACITIES.get(slot) else {
        return String::new();
    };
    lfmt!(
        "Slot {:02} · {} · {} / {}",
        slot + 1,
        domain_name(SLOT_DOMAINS[slot]),
        panel.points[slot],
        capacity
    )
}

pub fn gem_label(gem: &atelier::GemView) -> String {
    format!(
        "{} {} gem {:02X}{:02X}{}{}",
        domain_name(gem.domain),
        quality_name(gem.quality),
        gem.id[14],
        gem.id[15],
        if gem.locked { tr!(" · locked") } else { "" },
        if gem.vessel { tr!(" · in vessel") } else { "" }
    )
}

pub fn modifier_label(panel: &WheelPanel, kind: u8, mod_id: u16) -> String {
    let grade = ["I", "II", "III", "IV"]
        .get(usize::from(panel.grade(kind, mod_id)))
        .copied()
        .unwrap_or("?");
    lfmt!(
        "{} · Grade {grade}",
        catalogue::modifier_name(kind, mod_id),
        grade = grade
    )
}

/// Plain text also feeds the native accessible description control.
pub fn panel_text(panel: &WheelPanel) -> String {
    let mut lines = Vec::new();
    if !panel.selected {
        lines.push(tr!("Wheel and Atelier are unavailable in this session").to_owned());
    }
    if panel.pending.is_some() {
        lines.push(tr!("Waiting for the server…").into());
    }
    if panel.retry.is_some() {
        lines.push(tr!("A previous change needs confirmation. Retry checks the same change without another payment.").into());
    }
    if let Some(feedback) = panel.feedback {
        lines.push(feedback.to_owned());
    }
    let committed: u32 = panel
        .confirmed_wheel
        .as_ref()
        .map_or(0, |state| state.points.iter().map(|n| u32::from(*n)).sum());
    let draft: u32 = panel.points.iter().map(|n| u32::from(*n)).sum();
    lines.push(lfmt!(
        "Allocated points: {committed}. Edited points: {draft}{}.",
        if panel.dirty() {
            tr!(" (not saved)")
        } else {
            ""
        },
        committed = committed,
        draft = draft
    ));
    lines.push(tr!("Save Wheel applies point and vessel edits together. Refresh discards unsaved edits. Removing points or resetting is blocked while you are in a fight (PZ lock).").into());
    for (domain, gem) in panel.vessels.iter().enumerate() {
        let name = gem
            .and_then(|id| {
                panel
                    .confirmed_atelier
                    .as_ref()?
                    .gems
                    .iter()
                    .find(|gem| gem.id == id)
            })
            .map_or_else(|| tr!("Empty").into(), gem_label);
        lines.push(format!("{} vessel: {name}", domain_name(domain as u8)));
    }
    if let Some(atelier) = &panel.confirmed_atelier {
        if let Some(resources) = &panel.confirmed_resources {
            lines.push(lfmt!(
                "Gold: {} carried; {} in bank.",
                resources.carried_gold,
                resources.bank_gold
            ));
            lines.push(lfmt!(
                "Unrevealed gems carried: {} lesser; {} regular; {} greater.",
                resources.lesser_gems,
                resources.regular_gems,
                resources.greater_gems
            ));
            lines.push(lfmt!(
                "Fragments carried and in Supply Stash: {} basic; {} supreme.",
                resources.basic_fragments,
                resources.supreme_fragments
            ));
        }
        if let Some(quote) = atelier
            .quality_quotes
            .iter()
            .find(|quote| quote.quality == panel.quality)
        {
            lines.push(lfmt!(
                "Reveal {} gem: {} gold and one matching unrevealed gem.",
                quality_name(quote.quality),
                quote.reveal_gold
            ));
        }
        if let Some(gem) = panel.selected_gem()
            && let Some(quote) = atelier
                .quality_quotes
                .iter()
                .find(|quote| quote.quality == gem.quality)
        {
            lines.push(lfmt!(
                "Selected: {}. Switch domain: {} gold. Dismantle: {}–{} {} fragments.",
                gem_label(gem),
                quote.switch_gold,
                quote.fragment_min,
                quote.fragment_max,
                fragment_name(quote.fragment_kind)
            ));
        }
        if let Some(gem) = panel.selected_gem()
            && !gem.modifiers.is_empty()
        {
            let modifiers = gem
                .modifiers
                .iter()
                .enumerate()
                .map(|(index, id)| modifier_label(panel, u8::from(index == 2), *id))
                .collect::<Vec<_>>();
            lines.push(lfmt!("Selected gem modifiers: {}.", modifiers.join("; ")));
        }
        if let Some((kind, mod_id)) = panel.modifier {
            let grade = panel.grade(kind, mod_id);
            if let Some(quote) = atelier
                .grade_quotes
                .iter()
                .find(|quote| quote.kind == kind && quote.grade == grade + 1)
            {
                lines.push(lfmt!("Upgrade {}: {} gold and {} {} fragments. The grade applies to all matching modifiers.",
                    modifier_label(panel, kind, mod_id), quote.gold, quote.fragments, fragment_name(kind)));
            }
        }
    }
    lines.join("\r\n")
}
