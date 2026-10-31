//! Starter template and grant outcomes.

use oteryn_gameplay_domain::domain::item_definition::TypedDefinitionRef;

/// One starter template: its registered label and revision, and the one item it grants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StarterTemplate {
    pub key: String,
    pub revision: String,
    pub definition: TypedDefinitionRef,
    pub quantity: u32,
}

impl StarterTemplate {
    /// `oteryn:starter-template.main` (record `oteryn:starter.main_backpack`): one backpack
    /// `oteryn:item.tibia.i2854` into the `container` slot.
    pub fn main() -> Self {
        Self {
            key: "oteryn:starter-template.main".to_owned(),
            revision: "starter-kit-r1".to_owned(),
            definition: TypedDefinitionRef {
                family: "Item".to_owned(),
                production_key: "oteryn:item.tibia.i2854".to_owned(),
                revision_ref: "definition-r1".to_owned(),
            },
            quantity: 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StarterGrantOutcome {
    /// The fresh main backpack.
    Minted([u8; 16]),
    Skipped,
    AlreadyGranted,
    NotAdmitted,
}

/// A source-backed Newhaven kit destination. Main backpack creation is conditional.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StarterDestination {
    Equipment(u8),
    Backpack,
    Quiver,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StarterItem {
    pub tibia_id: u32,
    pub quantity: u32,
    pub destination: StarterDestination,
}

/// Crystal 201189f data-global/lib/others/newhaven.lua:3-99,125-127.
/// Slot numbers are the native equipment ABI, rather than Crystal slot constants.
pub fn newhaven_starter_items(vocation: &str) -> Option<Vec<StarterItem>> {
    use StarterDestination::{Backpack, Equipment, Quiver};
    let (hands, mana, health, rune): (&[(u32, u8)], u32, u32, bool) = match vocation {
        "sorcerer" => (&[(21400, 6), (21348, 5)], 10, 2, true),
        "druid" => (&[(21400, 6), (21350, 5)], 10, 2, true),
        "paladin" => (&[(3350, 5), (35562, 6)], 7, 5, false),
        "knight" => (&[(3272, 5), (3412, 6)], 2, 10, false),
        "monk" => (&[(50166, 5)], 5, 7, false),
        _ => return None,
    };
    let mut items: Vec<_> = [(3355, 1), (3561, 4), (3559, 7), (3552, 8), (34017, 10)]
        .into_iter()
        .chain(hands.iter().copied())
        .map(|(tibia_id, slot)| StarterItem {
            tibia_id,
            quantity: 1,
            destination: Equipment(slot),
        })
        .collect();
    for (tibia_id, quantity) in [(268, mana), (266, health), (3031, 50), (3578, 1)] {
        items.push(StarterItem {
            tibia_id,
            quantity,
            destination: Backpack,
        });
    }
    if rune {
        items.push(StarterItem {
            tibia_id: 21351,
            quantity: 1,
            destination: Backpack,
        });
    }
    if vocation == "paladin" {
        for _ in 0..2 {
            items.push(StarterItem {
                tibia_id: 3447,
                quantity: 100,
                destination: Quiver,
            });
        }
    }
    Some(items)
}

#[cfg(test)]
#[path = "starter_grant_tests.rs"]
mod tests;
