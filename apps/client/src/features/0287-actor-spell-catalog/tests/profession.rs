use crate::play::play_channel;
use oteryn_session::spell_catalog::{SpellCatalogEntry, encode_catalog};
#[test]
fn o059_profession_search_filters_confirmed_catalog_through_real_caller()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, _commands, outbox) = play_channel();
    let healing = SpellCatalogEntry {
        cast_index: 1,
        name: "Light Healing".into(),
        words: "exura".into(),
        level: 8,
        mana: 20,
        mana_percent: false,
        soul: 0,
        learned: true,
        available: false,
        cooldown_groups: vec![1],
        rune_item: None,
    };
    let strike = SpellCatalogEntry {
        cast_index: 2,
        name: "Brutal Strike".into(),
        words: "exori ico".into(),
        level: 16,
        mana: 30,
        mana_percent: false,
        soul: 0,
        learned: true,
        available: true,
        cooldown_groups: vec![2],
        rune_item: None,
    };
    outbox.spell_catalog_initialize(
        true,
        Some(encode_catalog(&[healing, strike]).map_err(|e| format!("{e:?}"))?),
    )?;
    assert!(link.spell_catalog_search("exura", "", "1", false, "druid"));
    let panel = link.spell_catalog_panel();
    let selected = panel.selected_spell().ok_or("missing healing")?;
    assert_eq!(selected.cast_index, 1);
    assert!(!selected.available);
    assert!(oteryn_client_ui::spell_catalog::panel_text(&panel).contains("Spells: 1"));
    assert!(link.spell_catalog_search("", "", "", false, "knight"));
    assert_eq!(
        link.spell_catalog_panel()
            .selected_spell()
            .map(|spell| spell.cast_index),
        Some(2)
    );
    assert!(!link.spell_catalog_search("", "", "", false, "made_up"));
    assert!(link.spell_catalog_panel().filter_error);
    Ok(())
}
