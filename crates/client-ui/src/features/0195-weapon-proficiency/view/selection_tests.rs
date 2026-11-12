use super::*;
use oteryn_client_runtime::weapon_proficiency::{
    ProficiencyCatalogueEntry, ProficiencyCatalogueLevel, ProficiencyCataloguePage,
    ProficiencyPage, ProficiencyRow,
};

fn entry(definition: u32, name: &str, experience: u32) -> ProficiencyCatalogueEntry {
    ProficiencyCatalogueEntry {
        definition,
        weapon: format!("oteryn:weapon.{definition}"),
        name: name.into(),
        experience,
        thresholds: [0, 100, 300, 600, 1000, 1500, 2100, 2800, 3600],
        levels: vec![
            ProficiencyCatalogueLevel {
                selected: Some(1),
                perks: vec!["+2% damage".into(), "+1% critical".into()],
            },
            ProficiencyCatalogueLevel {
                selected: None,
                perks: vec!["+5 mana".into(), "+3 life".into()],
            },
        ],
    }
}
fn panel() -> WeaponProficiencyPanel {
    WeaponProficiencyPanel {
        selected: true,
        catalogue: Some(ProficiencyCataloguePage {
            offset: 0,
            total: 2,
            entries: vec![entry(41, "Crimson blade", 150), entry(42, "Ash bow", 0)],
        }),
        ..WeaponProficiencyPanel::default()
    }
}

#[test]
fn t123_visual_selection_picks_weapon_level_and_perk_by_row_without_typing_ids() {
    let mut panel = panel();
    assert!(panel.select_weapon(1));
    assert_eq!(panel.weapon, 42);
    assert!(panel.select_weapon(0));
    assert_eq!((panel.weapon, panel.perk), (41, 1));
    assert!(panel.choose_level(1));
    assert_eq!((panel.level, panel.perk), (1, 0));
    assert!(!panel.choose_level(2));
    let rows = panel.perk_tree_rows();
    assert!(rows[0].0.contains("Unlocked"));
    assert_eq!(rows[1].1, Some((0, 0)));
    assert_eq!(rows[2].1, Some((0, 1)));
    assert!(rows[2].0.contains("☑ +1% critical"));
    assert!(rows[3].0.contains("Unlocked"));
    assert!(panel.select_weapon(1));
    let rows = panel.perk_tree_rows();
    assert_eq!(rows[1].1, Some((0, 0)));
    assert!(rows[3].0.contains("Locked"));
    assert_eq!(
        (rows[4].1, rows[5].1),
        (None, None),
        "locked perks are not selectable"
    );
    assert!(panel.select_weapon(0));
    panel.pending = true;
    assert!(!panel.select_weapon(1));
    assert!(!panel.choose_level(0));
}

#[test]
fn t123_catalogue_text_names_weapons_and_marks_the_selected_perk_instead_of_showing_ids() {
    let mut panel = panel();
    assert!(panel.select_weapon(0));
    panel.page = Some(ProficiencyPage {
        offset: 0,
        total: 1,
        rows: vec![ProficiencyRow {
            definition: std::num::NonZeroU32::new(41).unwrap_or(std::num::NonZeroU32::MIN),
            name: "Crimson blade".into(),
            experience: 150,
            level: 1,
            selections: vec![Some(1)],
        }],
    });
    let text = panel_text(&panel);
    assert!(text.contains("Crimson blade: 150 XP"));
    assert!(text.contains("Crimson blade: 150 XP, level 1"));
    assert!(text.contains("    ☑ +1% critical"));
    assert!(text.contains("    ☐ +2% damage"));
    assert!(!text.contains("weapon id"));
    assert!(!text.contains("[41]"));
    assert!(!text.contains("selected Some"));
}
