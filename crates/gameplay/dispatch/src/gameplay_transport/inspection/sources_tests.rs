use super::*;
use crate::domain::loot_data::{LootDefinitionRef, LootTableEntry};
#[test]
fn catalogue_loot_sources_page_creatures_without_skipping_sparse_matches() {
    let page = sources::project_sources(40000, "Sword", 0, |cursor| {
        Some((cursor % 2 == 0).then_some("Dragon"))
    });
    assert_eq!(page.items[0].attributes.len(), 10);
    assert_eq!(page.items[0].definition_ref, 40000);
    assert_eq!(page.catalogue.and_then(|page| page.next), Some(20));
    let end = sources::project_sources(40000, "Sword", u32::MAX, |_| None);
    assert!(end.items[0].attributes.is_empty());
    assert_eq!(end.catalogue.and_then(|page| page.next), None);
}
#[test]
fn catalogue_loot_sources_require_current_item_revision_and_nonzero_probability() {
    let item = LootTableEntry {
        item: LootDefinitionRef::new("Item", "sword", "r1"),
        min_count: 1,
        max_count: 1,
        probability_ppm: Some(100),
        details: Default::default(),
    };
    assert!(sources::contains(
        std::slice::from_ref(&item),
        "sword",
        "r1"
    ));
    assert!(!sources::contains(
        std::slice::from_ref(&item),
        "sword",
        "r2"
    ));
    let mut group = item.clone();
    group.item.production_key = "bag".into();
    group.details.children.push(item);
    assert!(sources::contains(
        std::slice::from_ref(&group),
        "sword",
        "r1"
    ));
    group.probability_ppm = Some(0);
    assert!(!sources::contains(&[group], "sword", "r1"));
}
