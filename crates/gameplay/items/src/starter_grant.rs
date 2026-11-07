//! Starter template and grant outcomes.

use oteryn_gameplay_domain::domain::item_definition::TypedDefinitionRef;

#[path = "starting_loadout.rs"]
pub mod starting_loadout;

/// A native definition resolved from the dedicated donor input, never from a client payload.
#[derive(Debug, Clone)]
pub struct StartingLoadout {
    pub vocation: String,
    pub backpack: TypedDefinitionRef,
    pub items: Vec<(TypedDefinitionRef, u32, u8)>,
}

impl StartingLoadout {
    pub fn resolve(
        vocation: &str,
        mut definition: impl FnMut(u32) -> Option<TypedDefinitionRef>,
    ) -> Option<Self> {
        Some(Self {
            vocation: vocation.into(),
            backpack: definition(2854)?,
            items: starting_loadout::items(vocation)?
                .iter()
                .map(|&(id, quantity, slot)| Some((definition(id)?, quantity, slot)))
                .collect::<Option<_>>()?,
        })
    }

    /// Re-check mutable public input at the durable boundary.
    pub fn matches_source(&self) -> bool {
        let Some(source) = starting_loadout::items(&self.vocation) else {
            return false;
        };
        self.backpack.family == "Item"
            && self.backpack.production_key == "oteryn:item.tibia.i2854"
            && source.len() == self.items.len()
            && source.iter().zip(&self.items).all(
                |(&(id, quantity, slot), (definition, count, destination))| {
                    definition.family == "Item"
                        && definition.production_key == format!("oteryn:item.tibia.i{id}")
                        && *count == quantity
                        && *destination == slot
                },
            )
    }
}

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

/// Immutable starter interpretation selected only for newly created Characters.
pub const NEWHAVEN_STARTER_TEMPLATE_REVISION: &str = "newhaven-starter-r1";

/// A Newhaven kit destination, using the existing durable equipment slot codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NewhavenStarterDestination {
    Equipment(u8),
    Backpack,
    Quiver,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NewhavenStarterItem {
    pub tibia_id: u32,
    pub quantity: u32,
    pub destination: NewhavenStarterDestination,
}

/// CrystalServer 9f5a72c6 `data-global/lib/others/newhaven.lua:3-95,126-129`.
/// The task admits this kit at first entry as well as the donor's Newhaven portal.
pub fn newhaven_starter_items(vocation: &str) -> Option<Vec<NewhavenStarterItem>> {
    use NewhavenStarterDestination::{Backpack, Equipment, Quiver};
    let (weapon, shield, mana, health, rune) = match vocation {
        "sorcerer" | "master_sorcerer" => (21348, Some(21400), 10, 2, true),
        "druid" | "elder_druid" => (21350, Some(21400), 10, 2, true),
        "paladin" | "royal_paladin" => (3350, Some(35562), 7, 5, false),
        "knight" | "elite_knight" => (3272, Some(3412), 2, 10, false),
        "monk" | "exalted_monk" => (50166, None, 5, 7, false),
        _ => return None,
    };
    let mut items: Vec<_> = [
        (3355, 1),
        (3561, 2),
        (3559, 3),
        (3552, 4),
        (34017, 10),
        (weapon, 5),
    ]
    .into_iter()
    .map(|(tibia_id, slot)| NewhavenStarterItem {
        tibia_id,
        quantity: 1,
        destination: Equipment(slot),
    })
    .collect();
    if let Some(tibia_id) = shield {
        items.push(NewhavenStarterItem {
            tibia_id,
            quantity: 1,
            destination: Equipment(6),
        });
    }
    for (tibia_id, quantity) in [(268, mana), (266, health), (3031, 50), (3578, 1)] {
        items.push(NewhavenStarterItem {
            tibia_id,
            quantity,
            destination: Backpack,
        });
    }
    if rune {
        items.push(NewhavenStarterItem {
            tibia_id: 21351,
            quantity: 1,
            destination: Backpack,
        });
    }
    if shield == Some(35562) {
        for _ in 0..2 {
            items.push(NewhavenStarterItem {
                tibia_id: 3447,
                quantity: 100,
                destination: Quiver,
            });
        }
    }
    Some(items)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NewhavenStarterOutcome {
    NotApplicable,
    Granted,
    AlreadyGranted,
    NotAdmitted,
}

#[cfg(test)]
mod newhaven_tests {
    use super::*;

    #[test]
    fn newhaven_profession_kits_match_source_equipment_supplies_and_promotions()
    -> Result<(), Box<dyn std::error::Error>> {
        use NewhavenStarterDestination::{Backpack, Equipment, Quiver};
        // Crystal newhaven.lua:3-95. Quantities differ from the excluded Dawnport kits.
        for (vocation, promoted, weapon, shield, mana, health, rune) in [
            (
                "sorcerer",
                "master_sorcerer",
                21348,
                Some(21400),
                10,
                2,
                true,
            ),
            ("druid", "elder_druid", 21350, Some(21400), 10, 2, true),
            ("paladin", "royal_paladin", 3350, Some(35562), 7, 5, false),
            ("knight", "elite_knight", 3272, Some(3412), 2, 10, false),
            ("monk", "exalted_monk", 50166, None, 5, 7, false),
        ] {
            let kit = newhaven_starter_items(vocation).ok_or("missing kit")?;
            assert_eq!(newhaven_starter_items(promoted).as_ref(), Some(&kit));
            let equipment: Vec<_> = kit
                .iter()
                .filter_map(|item| match item.destination {
                    Equipment(slot) => Some((item.tibia_id, item.quantity, slot)),
                    _ => None,
                })
                .collect();
            let mut expected = vec![
                (3355, 1, 1),
                (3561, 1, 2),
                (3559, 1, 3),
                (3552, 1, 4),
                (34017, 1, 10),
                (weapon, 1, 5),
            ];
            if let Some(shield) = shield {
                expected.push((shield, 1, 6));
            }
            assert_eq!(equipment, expected, "{vocation}");
            let mut supplies = vec![(268, mana), (266, health), (3031, 50), (3578, 1)];
            if rune {
                supplies.push((21351, 1));
            }
            assert_eq!(
                kit.iter()
                    .filter(|item| item.destination == Backpack)
                    .map(|item| (item.tibia_id, item.quantity))
                    .collect::<Vec<_>>(),
                supplies,
                "{vocation}"
            );
            let arrows: Vec<_> = kit
                .iter()
                .filter(|item| item.destination == Quiver)
                .map(|item| (item.tibia_id, item.quantity))
                .collect();
            assert_eq!(
                arrows,
                if vocation == "paladin" {
                    vec![(3447, 100), (3447, 100)]
                } else {
                    vec![]
                }
            );
        }
        assert!(newhaven_starter_items("none").is_none());
        assert!(newhaven_starter_items("invalid").is_none());
        Ok(())
    }
}
