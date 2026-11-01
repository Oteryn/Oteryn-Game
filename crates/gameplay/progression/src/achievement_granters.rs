//! P11 counters for successful donor gathering and completed NPC travel.
//! Crystal `201189f`: `data-global/scripts/actions/tools/skinning.lua:317-321`,
//! `data-global/scripts/actions/other/fishing.lua:290-314`,
//! `data-global/scripts/actions/other/mechanical_fishing.lua:348-372`, and
//! `data/npclib/npc_system/modules.lua:243-245`;
//! rust success is `data/scripts/actions/items/rust_remover.lua:104-107`.
use crate::item_use_achievement::Counted;

pub const CRAFT_SOURCE_KIND: &str = "oteryn:item-craft";
pub const TRAVEL_SOURCE_KIND: &str = "oteryn:npc-travel";
pub const TRANSFORM_SOURCE_KIND: &str = "oteryn:consume-transform";

pub const HERE_FISHY_FISHY: Counted = Counted {
    key: "oteryn:achievement/here_fishy_fishy",
    total: 250,
};
pub const EXQUISITE_TASTE: Counted = Counted {
    key: "oteryn:achievement/exquisite_taste",
    total: 250,
};
pub const SKIN_DEEP: Counted = Counted {
    key: "oteryn:achievement/skin_deep",
    total: 500,
};
pub const ASHES_TO_DUST: Counted = Counted {
    key: "oteryn:achievement/ashes_to_dust",
    total: 500,
};
pub const SHIPS_KOBOLD: Counted = Counted {
    key: "oteryn:achievement/ship_s_kobold",
    total: 1250,
};
pub const POLISHER: Counted = Counted {
    key: "oteryn:achievement/polisher",
    total: 1000,
};

/// Classify a successful, owner-validated gathering outcome. The existing gathering
/// rules own the permitted tool/target pairs and their yield; this is not a second
/// admission table. An unsuccessful corpse attempt and an unfinished ice sculpture
/// have no inventory yield and therefore advance no Achievement.
pub fn gathering(tool_key: &str, yield_item: Option<u16>) -> Option<Counted> {
    match (tool_key, yield_item?) {
        ("oteryn:item.tibia.i3483" | "oteryn:item.tibia.i9306", 3578) => Some(HERE_FISHY_FISHY),
        ("oteryn:item.tibia.i3483" | "oteryn:item.tibia.i9306", 7158 | 3580 | 7159) => {
            Some(EXQUISITE_TASTE)
        }
        ("oteryn:item.tibia.i5908" | "oteryn:item.tibia.i5942", 5905 | 5906) => Some(ASHES_TO_DUST),
        ("oteryn:item.tibia.i5908" | "oteryn:item.tibia.i5942", _) => Some(SKIN_DEEP),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn p11_both_rods_distinguish_common_and_all_three_rare_fish() {
        for tool in ["oteryn:item.tibia.i3483", "oteryn:item.tibia.i9306"] {
            assert_eq!(gathering(tool, Some(3578)), Some(HERE_FISHY_FISHY));
            for fish in [7158, 3580, 7159] {
                assert_eq!(gathering(tool, Some(fish)), Some(EXQUISITE_TASTE));
            }
            assert_eq!(gathering(tool, None), None);
            assert_eq!(gathering(tool, Some(5878)), None);
        }
    }

    #[test]
    fn p11_skinning_counts_only_successful_inventory_yields_and_separates_dust() {
        for tool in ["oteryn:item.tibia.i5908", "oteryn:item.tibia.i5942"] {
            for skin in [5878, 5876, 5877, 5948, 5893, 5925, 24842, 6017] {
                assert_eq!(gathering(tool, Some(skin)), Some(SKIN_DEEP));
            }
            for dust in [5905, 5906] {
                assert_eq!(gathering(tool, Some(dust)), Some(ASHES_TO_DUST));
            }
            assert_eq!(gathering(tool, None), None);
        }
        assert_eq!(gathering("oteryn:item.tibia.i3604", Some(3578)), None);
    }

    #[test]
    fn p11_every_counted_key_and_threshold_matches_the_donor_catalogue()
    -> Result<(), crate::achievement_catalogue::AchievementCatalogueError> {
        let catalogue =
            crate::achievement_catalogue::parse_shards(&crate::achievement_catalogue::SHARDS)?;
        for (rule, name, total) in [
            (HERE_FISHY_FISHY, "Here, Fishy Fishy!", 250),
            (EXQUISITE_TASTE, "Exquisite Taste", 250),
            (SKIN_DEEP, "Skin-Deep", 500),
            (ASHES_TO_DUST, "Ashes to Dust", 500),
            (SHIPS_KOBOLD, "Ship's Kobold", 1250),
            (POLISHER, "Polisher", 1000),
        ] {
            assert_eq!(rule.total, total);
            assert!(
                catalogue
                    .get(rule.key)
                    .is_some_and(|entry| !entry.retired && entry.name == name)
            );
        }
        Ok(())
    }
}
