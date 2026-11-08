use oteryn_client_runtime::spell_catalog::{SpellCatalog, SpellCatalogEntry};
use oteryn_client_ui::spell_catalog::{Panel, panel_text, protocol};

fn catalogue() -> Panel {
    let mut panel = Panel {
        selected: true,
        ..Default::default()
    };
    protocol::apply(
        &mut panel,
        SpellCatalog {
            revision: 1,
            entries: vec![
                SpellCatalogEntry {
                    cast_index: 71,
                    name: "Healing".into(),
                    words: "exura".into(),
                    level: 8,
                    mana: 20,
                    mana_percent: false,
                    soul: 0,
                    learned: true,
                    available: true,
                    cooldown_groups: vec![1],
                    rune_item: None,
                },
                SpellCatalogEntry {
                    cast_index: 93,
                    name: "Strike".into(),
                    words: "exori".into(),
                    level: 20,
                    mana: 30,
                    mana_percent: false,
                    soul: 1,
                    learned: false,
                    available: false,
                    cooldown_groups: vec![2],
                    rune_item: None,
                },
            ],
        },
    );
    panel
}

#[test]
fn t032_spellbook_filters_confirmed_level_group_and_learning() {
    let mut panel = catalogue();
    assert!(protocol::set_filters(&mut panel, "10", "1", true));
    assert!(panel_text(&panel).contains("71: Healing"));
    assert!(!panel_text(&panel).contains("93: Strike"));
    assert!(!protocol::set_filters(&mut panel, "oops", "", false));
    assert!(panel_text(&panel).contains("Enter"));
    assert!(protocol::set_filters(&mut panel, "", "2", false));
    assert!(!panel_text(&panel).contains("71: Healing"));
    assert!(panel_text(&panel).contains("93: Strike"));
    protocol::navigate(&mut panel, "exori", 2);
    assert!(panel_text(&panel).contains("Page 1/1"));
}

#[test]
fn t033_selection_follows_filtered_catalogue_rows() {
    let mut panel = catalogue();
    assert_eq!(panel.selected_spell().map(|entry| entry.cast_index), Some(71));
    protocol::select_relative(&mut panel, true);
    assert_eq!(panel.selected_spell().map(|entry| entry.cast_index), Some(93));
    protocol::select_relative(&mut panel, true);
    assert_eq!(panel.selected_spell().map(|entry| entry.cast_index), Some(93));
    assert!(protocol::set_filters(&mut panel, "", "1", false));
    assert_eq!(panel.selected_spell().map(|entry| entry.cast_index), Some(71));
}
