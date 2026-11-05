use super::{Panel, panel_text, protocol::apply};
use oteryn_client_runtime::spell_catalog::{SpellCatalog, SpellCatalogEntry};
#[test]
fn confirmed_state_is_visible() {
    let mut panel = Panel {
        selected: true,
        ..Panel::default()
    };
    apply(
        &mut panel,
        SpellCatalog {
            revision: 1,
            entries: vec![SpellCatalogEntry {
                cast_index: 71,
                name: "Healing".into(),
                words: "exura".into(),
                level: 8,
                mana: 20,
                mana_percent: false,
                soul: 0,
                learned: true,
                available: false,
                cooldown_groups: vec![1],
                rune_item: None,
            }],
        },
    );
    assert!(panel_text(&panel).contains("71: Healing"));
    assert!(panel_text(&panel).contains("Available false"));
    panel.filter = "missing".into();
    assert!(!panel_text(&panel).contains("71: Healing"));
    apply(
        &mut panel,
        SpellCatalog {
            revision: 2,
            entries: vec![],
        },
    );
    assert!(panel.catalog.entries().is_empty());
}

#[test]
fn filtered_paging_stops_at_the_last_matching_page() {
    use super::protocol::navigate;
    let mut panel = Panel {
        selected: true,
        ..Default::default()
    };
    let entries = (1..=35)
        .map(|cast_index| SpellCatalogEntry {
            cast_index,
            name: if cast_index <= 17 { "Healing" } else { "Light" }.into(),
            words: "".into(),
            level: 8,
            mana: 20,
            mana_percent: false,
            soul: 0,
            learned: true,
            available: true,
            cooldown_groups: vec![],
            rune_item: Some(123),
        })
        .collect();
    apply(
        &mut panel,
        SpellCatalog {
            revision: 1,
            entries,
        },
    );
    navigate(&mut panel, "healing", 2);
    navigate(&mut panel, "healing", 2);
    assert_eq!(panel.page, 1);
    assert!(panel_text(&panel).contains("Page 2/2"));
    navigate(&mut panel, "healing", 1);
    assert_eq!(panel.page, 0);
    navigate(&mut panel, "missing", 2);
    assert_eq!(panel.page, 0);
}
