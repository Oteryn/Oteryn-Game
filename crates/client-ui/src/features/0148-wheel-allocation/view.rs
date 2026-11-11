use super::*;
#[path = "view/catalogue.rs"]
mod catalogue;

/// Gem Vessels are numbered slots; they carry no domain.
pub fn vessel_name(slot: u8) -> String {
    format!("Gem Vessel {}", u32::from(slot) + 1)
}

/// Text of a Gem Vessel perk node; each node grants a flat bonus and one vessel slot.
pub fn vessel_node_perk(bonus: u32) -> String {
    format!(
        "Each node grants +{} Damage / Healing and unlocks a vessel in the Gem Atelier.",
        bonus
    )
}

/// Gem Vessel perk nodes are recognised by their name; the flat bonus is +1 per node.
pub fn is_vessel_node(name: &str) -> bool {
    name.to_ascii_lowercase().contains("gem vessel")
}

/// Mod Reshaping prices per mod position (1-3): gold, fragments, fragment kind.
pub fn reshape_cost(position: usize) -> (u64, u32, &'static str) {
    match position {
        1 => (125_000, 1, "lesser"),
        2 => (1_000_000, 2, "lesser"),
        _ => (5_000_000, 3, "greater"),
    }
}

/// 1-based position of the selected Workshop modifier inside the selected gem.
pub fn reshape_position(panel: &WheelPanel, gem: &atelier::GemView) -> Option<usize> {
    let (kind, id) = panel.modifier?;
    gem.modifiers
        .iter()
        .enumerate()
        .position(|(index, mod_id)| *mod_id == id && u8::from(index == 2) == kind)
        .map(|index| index + 1)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReshapeBlock {
    NotUnlocked,
    NoModifier,
    NotPremium,
    NotInTemple,
    Gold,
    Fragments,
}

impl ReshapeBlock {
    pub fn reason(self) -> &'static str {
        match self {
            Self::NotUnlocked => "The gem must be unlocked first",
            Self::NoModifier => "Select a modifier of the gem first",
            Self::NotPremium => "Your account must be Premium to reshape mods",
            Self::NotInTemple => "You must be inside a temple to reshape mods",
            Self::Gold => "You do not have enough gold at this position",
            Self::Fragments => "You do not have enough fragments at this position",
        }
    }
}

/// Why Mod Reshaping is disabled for the selected gem, or `None` when it may be requested.
pub fn reshape_block(panel: &WheelPanel, gem: &atelier::GemView) -> Option<ReshapeBlock> {
    if gem.modifiers.is_empty() {
        return Some(ReshapeBlock::NotUnlocked);
    }
    let position = reshape_position(panel, gem)?;
    if panel.premium != Some(true) {
        return Some(ReshapeBlock::NotPremium);
    }
    if panel.in_temple != Some(true) {
        return Some(ReshapeBlock::NotInTemple);
    }
    let (gold, fragments, kind) = reshape_cost(position);
    let resources = panel.confirmed_resources.as_ref()?;
    if resources.carried_gold.saturating_add(resources.bank_gold) < gold {
        return Some(ReshapeBlock::Gold);
    }
    let owned = if kind == "lesser" {
        resources.basic_fragments
    } else {
        resources.supreme_fragments
    };
    (owned < u64::from(fragments)).then_some(ReshapeBlock::Fragments)
}

/// Fragment Workshop totals: +5 Health, Mana and Capacity per unlocked mod grade, one
/// Promotion Point per fully enhanced mod (Grade IV), and Revelation Mastery mods.
pub fn workshop_lines(panel: &WheelPanel) -> Vec<String> {
    let Some(atelier) = &panel.confirmed_atelier else {
        return Vec::new();
    };
    let unlocked: u32 = atelier.grades.iter().map(|g| u32::from(g.grade)).sum();
    let promotion = atelier.grades.iter().filter(|g| g.grade >= 3).count();
    let mut lines = vec![
        format!(
            "Fragment Workshop: +{} Health, Mana and Capacity (+5 for each mod grade unlocked).",
            5 * unlocked
        ),
        format!(
            "Promotion Points: {} (one promotion point for each fully enhanced mod).",
            promotion
        ),
    ];
    for grade in atelier.grades.iter().filter(|g| {
        g.kind == 1 && catalogue::modifier_name(g.kind, g.mod_id).starts_with("Revelation Mastery")
    }) {
        lines.push(format!(
            "{}: gain additional promotion points towards unlocking the related Revelation perk.",
            modifier_label(panel, grade.kind, grade.mod_id)
        ));
    }
    lines
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
    format!(
        "Slot {:02} · {} / {}",
        slot + 1,
        panel.points[slot],
        capacity
    )
}

pub fn gem_label(gem: &atelier::GemView) -> String {
    format!(
        "{} gem {:02X}{:02X}{}{}",
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
    if let Some(board) = panel.vessel_board.as_ref().filter(|_| panel.atelier_v2) {
        let name_of = |id: [u8; 16]| {
            panel
                .confirmed_atelier
                .as_ref()
                .and_then(|atelier| atelier.gems.iter().find(|gem| gem.id == id))
                .map_or_else(|| tr!("Empty").into(), gem_label)
        };
        lines.extend(crate::gem_atelier::vessel_lines(board, name_of));
        lines.push(format!(
            "Selected: {}. Replace Vessel puts the selected gem there when its quality matches.",
            vessel_name(panel.vessel_slot)
        ));
    }
    for (slot, gem) in panel
        .vessels
        .iter()
        .enumerate()
        .take(if panel.atelier_v2 { 0 } else { 4 })
    {
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
        lines.push(format!("{}: {name}", vessel_name(slot as u8)));
    }
    if !panel.atelier_v2 && panel.vessels.get(usize::from(panel.vessel_slot)) == Some(&None) {
        lines.push("Select a Vessel to Modify Gems".into());
    }
    if let Some(atelier) = &panel.confirmed_atelier
        && !panel.atelier_v2
        && panel.vessels.iter().any(Option::is_none)
        && atelier
            .gems
            .iter()
            .any(|gem| !panel.vessels.contains(&Some(gem.id)))
    {
        lines.push("Empty vessel slots are available in the Gem Atelier.".into());
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
            lines.push(format!(
                "Selected: {}. Dismantle: {}–{} {} fragments.",
                gem_label(gem),
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
        if let Some(gem) = panel.selected_gem()
            && let Some(position) = reshape_position(panel, gem)
        {
            let (gold, fragments, kind) = reshape_cost(position);
            lines.push(format!("Reshaping a gem's mods lets you randomly re-roll each mod of the gem. Reshape position {}: {} gold and {} {} fragments.",
                position,
                gold,
                fragments,
                kind
            ));
        }
        if let Some(gem) = panel.selected_gem()
            && let Some(block) = reshape_block(panel, gem)
        {
            lines.push(format!("Mod Reshaping unavailable: {}.", block.reason()));
        }
        lines.extend(workshop_lines(panel));
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
