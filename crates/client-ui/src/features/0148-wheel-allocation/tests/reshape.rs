use super::*;
use atelier::resources::ResourceBalances;

fn reshape_panel() -> WheelPanel {
    let mut panel = panel();
    panel.premium = Some(true);
    panel.in_temple = Some(true);
    panel.confirmed_resources = Some(ResourceBalances {
        carried_gold: 10_000_000,
        basic_fragments: 5,
        supreme_fragments: 5,
        ..ResourceBalances::default()
    });
    // Gem modifiers are [31, 0, 5]: positions 1, 2 and 3.
    panel.modifier = Some((0, 31));
    panel
}

#[test]
fn reshape_position_costs_match_the_announced_prices() {
    assert_eq!(reshape_cost(1), (125_000, 1, "lesser"));
    assert_eq!(reshape_cost(2), (1_000_000, 2, "lesser"));
    assert_eq!(reshape_cost(3), (5_000_000, 3, "greater"));
}

#[test]
fn reshape_is_disabled_with_a_reason_outside_a_temple_without_premium_or_for_a_locked_gem()
-> Result<(), Error> {
    let panel = reshape_panel();
    let gem = panel.selected_gem().ok_or("gem")?.clone();
    assert_eq!(reshape_block(&panel, &gem), None);

    let mut no_temple = panel.clone();
    no_temple.in_temple = Some(false);
    assert_eq!(
        reshape_block(&no_temple, &gem),
        Some(ReshapeBlock::NotInTemple)
    );
    no_temple.in_temple = None;
    assert_eq!(
        reshape_block(&no_temple, &gem),
        Some(ReshapeBlock::NotInTemple)
    );

    let mut free = panel.clone();
    free.premium = Some(false);
    assert_eq!(reshape_block(&free, &gem), Some(ReshapeBlock::NotPremium));

    let mut unrevealed = gem.clone();
    unrevealed.modifiers.clear();
    assert_eq!(
        reshape_block(&panel, &unrevealed),
        Some(ReshapeBlock::NotUnlocked)
    );

    let mut poor = panel.clone();
    poor.modifier = Some((1, 5));
    poor.confirmed_resources = Some(ResourceBalances {
        carried_gold: 4_999_999,
        supreme_fragments: 3,
        ..ResourceBalances::default()
    });
    assert_eq!(reshape_block(&poor, &gem), Some(ReshapeBlock::Gold));
    poor.confirmed_resources = Some(ResourceBalances {
        carried_gold: 3_000_000,
        bank_gold: 2_000_000,
        supreme_fragments: 2,
        ..ResourceBalances::default()
    });
    assert_eq!(reshape_block(&poor, &gem), Some(ReshapeBlock::Fragments));
    Ok(())
}

#[test]
fn reshape_request_reports_the_block_reason_and_sends_nothing() {
    let mut panel = reshape_panel();
    panel.in_temple = Some(false);
    assert!(request(&mut panel, Intent::Reshape, occurrence(3)).is_none());
    assert_eq!(
        panel.feedback,
        Some("You must be inside a temple to reshape mods")
    );
    assert!(panel.pending.is_none());
}

#[test]
fn panel_text_shows_reshape_price_and_disabled_reason() {
    let mut panel = reshape_panel();
    let text = panel_text(&panel);
    assert!(text.contains("Reshape position 1: 125000 gold and 1 lesser fragments."));
    assert!(!text.contains("Mod Reshaping unavailable"));
    panel.premium = Some(false);
    assert!(
        panel_text(&panel)
            .contains("Mod Reshaping unavailable: Your account must be Premium to reshape mods.")
    );
}

#[test]
fn replace_vessel_moves_a_gem_into_the_chosen_slot_without_duplicates() -> Result<(), Error> {
    let mut panel = panel();
    panel.vessel_slot = 0;
    assert!(panel.socket_selected());
    assert_eq!(panel.vessels, [Some([17; 16]), None, None, None]);
    panel.vessel_slot = 2;
    assert!(panel.socket_selected());
    assert_eq!(panel.vessels, [None, None, Some([17; 16]), None]);
    panel.vessel_slot = 4;
    assert!(!panel.socket_selected());
    Ok(())
}

#[test]
fn gems_and_vessels_show_no_domain_or_resonance() -> Result<(), Error> {
    let mut panel = panel();
    let label = gem_label(panel.selected_gem().ok_or("gem")?);
    assert!(label.starts_with("Greater gem"));
    for domain in [
        "Green",
        "Red",
        "Purple",
        "Blue",
        "Affinity",
        "Resonance",
        "VR",
    ] {
        assert!(!label.contains(domain));
    }
    panel.vessels[1] = Some([17; 16]);
    let text = panel_text(&panel);
    assert!(text.contains("Gem Vessel 2: Greater gem 1111"));
    assert!(text.contains("Select a Vessel to Modify Gems"));
    assert!(!text.contains("Switch domain"));
    assert!(!text.contains("Resonance"));
    assert!(!slot_label(&panel, 14).contains("Green"));
    assert_eq!(vessel_name(0), "Gem Vessel 1");
    Ok(())
}

#[test]
fn vessel_node_perk_text_matches_the_announced_wording() {
    assert_eq!(
        vessel_node_perk(1),
        "Each node grants +1 Damage / Healing and unlocks a vessel in the Gem Atelier."
    );
    assert!(is_vessel_node("Gem Vessel"));
    assert!(!is_vessel_node("Vessel Resonance"));
}

#[test]
fn workshop_lines_show_health_mana_capacity_and_promotion_points() -> Result<(), Error> {
    let mut panel = panel();
    let atelier = panel.confirmed_atelier.as_mut().ok_or("atelier")?;
    atelier.grades = vec![
        atelier::GradeView {
            kind: 1,
            mod_id: 5,
            grade: 3,
        },
        atelier::GradeView {
            kind: 0,
            mod_id: 31,
            grade: 2,
        },
        atelier::GradeView {
            kind: 1,
            mod_id: 0,
            grade: 0,
        },
    ];
    let lines = workshop_lines(&panel);
    assert_eq!(
        lines[0],
        "Fragment Workshop: +25 Health, Mana and Capacity (+5 for each mod grade unlocked)."
    );
    assert_eq!(
        lines[1],
        "Promotion Points: 1 (one promotion point for each fully enhanced mod)."
    );
    assert!(lines[2].contains("Revelation Mastery Gift of Life"));
    assert!(lines[2].contains("towards unlocking the related Revelation perk"));
    Ok(())
}

#[test]
fn importing_an_older_wheel_configuration_shows_the_changed_values_notice() -> Result<(), Error> {
    let mut panel = panel();
    panel.import_build(&format!("OW1:{}", ["0"; 36].join(",")))?;
    assert!(panel.feedback.is_some_and(|text| text.starts_with(
        "You have imported an older Wheel of Destiny configuration. Some values may have changed."
    )));
    Ok(())
}
