//! P11: the counted Achievement a committed item use advances.
//!
//! Crystal `201189f8` counts one per use and grants at the total
//! (`data/scripts/lib/register_achievements.lua:696`):
//! - Petrologist (TibiaWiki id 190, spoiler Shiny Stone): one per shiny stone refined with a
//!   pick, total 100 (`data-global/scripts/lib/register_actions.lua:628`).
//! - Potion Addict (TibiaWiki id 74, spoiler potions): one per health or mana potion drunk,
//!   total 100000 (`data/scripts/actions/items/potions.lua:100`).

/// The grant request's source kind; the event id is the item use's event id.
pub const SOURCE_KIND: &str = "oteryn:item-use";

/// The counted Achievement and the count that earns it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counted {
    pub key: &'static str,
    pub total: u32,
}

pub const PETROLOGIST: Counted = Counted {
    key: "oteryn:achievement/petrologist",
    total: 100,
};

pub const POTION_ADDICT: Counted = Counted {
    key: "oteryn:achievement/potion_addict",
    total: 100_000,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_counted_key_is_earnable_in_the_embedded_catalogue()
    -> Result<(), crate::achievement_catalogue::AchievementCatalogueError> {
        let catalogue =
            crate::achievement_catalogue::parse_shards(&crate::achievement_catalogue::SHARDS)?;
        for counted in [PETROLOGIST, POTION_ADDICT] {
            assert!(
                catalogue
                    .get(counted.key)
                    .is_some_and(|entry| !entry.retired),
                "{}",
                counted.key
            );
        }
        Ok(())
    }
}
