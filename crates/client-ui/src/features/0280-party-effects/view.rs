use super::PartyEffectsPanel;
pub fn panel_text(panel: &PartyEffectsPanel) -> String {
    if !panel.selected {
        return tr!("Party effects unavailable in this session").into();
    }
    if let Some(error) = panel.error {
        return error.into();
    }
    let Some(value) = panel.snapshot else {
        return tr!("Loading party effects…").into();
    };
    let status = if !value.in_party {
        tr!("No party")
    } else if !value.shared_enabled {
        tr!("Disabled")
    } else if value.shared_active {
        tr!("Active")
    } else {
        tr!("Enabled, waiting for eligibility")
    };
    let eligible = if !value.shared_enabled {
        tr!("Not evaluated while sharing is disabled")
    } else if value.eligible {
        tr!("Yes")
    } else {
        tr!("No")
    };
    let mut text = lfmt!(
        "Shared experience: {status}\r\nEligible: {eligible}\r\n",
        eligible = eligible,
        status = status
    );
    for (label, bonus) in [
        tr!("Damage reduction"),
        tr!("Auto attack"),
        tr!("Spell damage"),
        tr!("Spell healing"),
    ]
    .into_iter()
    .zip(value.bonuses[..4].iter())
    {
        text.push_str(&format!("{label}: {bonus}%\r\n"));
    }
    text.push_str(&lfmt!("Mantra: +{}\r\n", value.bonuses[4]));
    for (label, remaining) in [
        tr!("Train Party"),
        tr!("Protect Party"),
        tr!("Enchant Party"),
        tr!("Heal Party"),
        tr!("Enlighten Party"),
    ]
    .into_iter()
    .zip(value.party_spell_remaining_ms)
    {
        if remaining > 0 {
            text.push_str(&lfmt!(
                "{label}: {}s\r\n",
                remaining.div_ceil(1000),
                label = label
            ));
        }
    }
    text
}
