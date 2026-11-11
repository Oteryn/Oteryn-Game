//! Compare qualified loot with every pinned source entry and its admitted facts.
use crate::content::creature_reward::{SectionItemFacts, SectionLootEntry};
use oteryn_gameplay_domain::domain::loot_data::LootTableEntry;

pub(super) fn entries(
    actual: &[LootTableEntry],
    source: &[SectionLootEntry],
    facts: &[SectionItemFacts],
) {
    assert_eq!(actual.len(), source.len());
    for (actual, source) in actual.iter().zip(source) {
        assert_eq!(
            (
                &actual.item.family,
                &actual.item.production_key,
                &actual.item.revision_ref
            ),
            (&source.item.family, &source.item.key, &source.item.revision)
        );
        assert_eq!(
            (actual.min_count, actual.max_count, actual.probability_ppm),
            (source.min_count, source.max_count, source.probability_ppm)
        );
        let item_facts = facts
            .iter()
            .find(|f| f.item == source.item)
            .expect("source Item facts");
        assert!(item_facts.materializable);
        assert_eq!(
            actual.details.container_capacity,
            item_facts.container_capacity
        );
        assert_eq!(actual.details.charges, item_facts.charges);
        assert_eq!(actual.details.stackable, item_facts.stackable);
        assert_eq!(actual.details.creature_product, item_facts.creature_product);
        assert_eq!(actual.details.subtype, source.subtype);
        assert_eq!(actual.details.unique, source.unique);
        assert_eq!(actual.details.gut_bonus_hundredths, 0);
        assert_eq!(actual.details.rated_probability_ppm, None);
        entries(&actual.details.children, &source.children, facts);
    }
}
