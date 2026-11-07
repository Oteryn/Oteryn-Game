use super::PartyEffectsPanel;
pub fn panel_text(panel: &PartyEffectsPanel) -> String {
    if !panel.selected {
        return "Party effects unavailable in this session".into();
    }
    if let Some(error) = panel.error {
        return error.into();
    }
    let Some(value) = panel.snapshot else {
        return "Loading party effects…".into();
    };
    let status = if !value.in_party {
        "No party"
    } else if !value.shared_enabled {
        "Disabled"
    } else if value.shared_active {
        "Active"
    } else {
        "Enabled, waiting for eligibility"
    };
    let eligible = if !value.shared_enabled {
        "Not evaluated while sharing is disabled"
    } else if value.eligible {
        "Yes"
    } else {
        "No"
    };
    let mut text = format!("Shared experience: {status}\r\nEligible: {eligible}\r\n");
    for (label, bonus) in [
        "Damage reduction",
        "Auto attack",
        "Spell damage",
        "Spell healing",
    ]
    .into_iter()
    .zip(value.bonuses[..4].iter())
    {
        text.push_str(&format!("{label}: {bonus}%\r\n"));
    }
    text.push_str(&format!("Mantra: +{}\r\n", value.bonuses[4]));
    for (label, remaining) in [
        "Train Party",
        "Protect Party",
        "Enchant Party",
        "Heal Party",
        "Enlighten Party",
    ]
    .into_iter()
    .zip(value.party_spell_remaining_ms)
    {
        if remaining > 0 {
            text.push_str(&format!("{label}: {}s\r\n", remaining.div_ceil(1000)));
        }
    }
    text
}
