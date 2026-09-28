//! Pure Reference equipment-slot rules (tibia.com manual §3.4.1, owner answer 4 in #162
//! 5879169470; GAME-ITEM-01 §6.2).
//!
//! Item categories come from content, and vocation or level requirements are typed item
//! requirements checked elsewhere; the move itself is DUR-03-owned.

use std::collections::BTreeMap;

use super::death::EquipmentSlot;

/// What an equippable item is, as far as slot occupancy is concerned.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EquipCategory {
    Head,
    Armor,
    Legs,
    Feet,
    Necklace,
    Ring,
    /// Bags and backpacks.
    Container,
    OneHandedWeapon,
    TwoHandedWeapon,
    /// Two-handed bows and crossbows: the shield slot may still hold a quiver.
    TwoHandedDistanceWeapon,
    Shield,
    Spellbook,
    Quiver,
    /// Free-form items kept close at hand, such as a torch or ammunition.
    Extra,
}

impl EquipCategory {
    /// The slot the item occupies.
    #[must_use]
    pub const fn slot(self) -> EquipmentSlot {
        match self {
            Self::Head => EquipmentSlot::Head,
            Self::Armor => EquipmentSlot::Armor,
            Self::Legs => EquipmentSlot::Legs,
            Self::Feet => EquipmentSlot::Feet,
            Self::Necklace => EquipmentSlot::Necklace,
            Self::Ring => EquipmentSlot::Ring,
            Self::Container => EquipmentSlot::Container,
            Self::OneHandedWeapon | Self::TwoHandedWeapon | Self::TwoHandedDistanceWeapon => {
                EquipmentSlot::RightHand
            }
            Self::Shield | Self::Spellbook | Self::Quiver => EquipmentSlot::LeftHand,
            Self::Extra => EquipmentSlot::Ammo,
        }
    }

    const fn is_two_handed(self) -> bool {
        matches!(self, Self::TwoHandedWeapon | Self::TwoHandedDistanceWeapon)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EquipRejection {
    /// The item's slot already holds an item.
    SlotOccupied(EquipmentSlot),
    /// A two-handed weapon and the shield-slot item cannot be worn together.
    HandsConflict,
}

/// Whether an item of `category` may be equipped against the currently equipped items. A swap
/// is checked against the equipment without the item it displaces.
///
/// A two-handed weapon blocks the shield slot, except that a two-handed distance weapon may be
/// worn with a quiver (manual §3.4.1).
///
/// # Errors
///
/// Returns the occupied slot or the two-handed conflict.
pub fn check_equip(
    category: EquipCategory,
    equipped: &BTreeMap<EquipmentSlot, EquipCategory>,
) -> Result<(), EquipRejection> {
    let slot = category.slot();
    if equipped.contains_key(&slot) {
        return Err(EquipRejection::SlotOccupied(slot));
    }
    let compatible = match slot {
        EquipmentSlot::RightHand => match equipped.get(&EquipmentSlot::LeftHand) {
            Some(left) if category.is_two_handed() => hands_compatible(category, *left),
            _ => true,
        },
        EquipmentSlot::LeftHand => match equipped.get(&EquipmentSlot::RightHand) {
            Some(right) if right.is_two_handed() => hands_compatible(*right, category),
            _ => true,
        },
        _ => true,
    };
    if compatible {
        Ok(())
    } else {
        Err(EquipRejection::HandsConflict)
    }
}

const fn hands_compatible(two_handed: EquipCategory, left: EquipCategory) -> bool {
    matches!(
        (two_handed, left),
        (
            EquipCategory::TwoHandedDistanceWeapon,
            EquipCategory::Quiver
        )
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wearing(items: &[EquipCategory]) -> BTreeMap<EquipmentSlot, EquipCategory> {
        items.iter().map(|item| (item.slot(), *item)).collect()
    }

    #[test]
    fn every_category_goes_to_its_manual_slot() {
        let expected = [
            (EquipCategory::Head, EquipmentSlot::Head),
            (EquipCategory::Armor, EquipmentSlot::Armor),
            (EquipCategory::Legs, EquipmentSlot::Legs),
            (EquipCategory::Feet, EquipmentSlot::Feet),
            (EquipCategory::Necklace, EquipmentSlot::Necklace),
            (EquipCategory::Ring, EquipmentSlot::Ring),
            (EquipCategory::Container, EquipmentSlot::Container),
            (EquipCategory::OneHandedWeapon, EquipmentSlot::RightHand),
            (EquipCategory::TwoHandedWeapon, EquipmentSlot::RightHand),
            (
                EquipCategory::TwoHandedDistanceWeapon,
                EquipmentSlot::RightHand,
            ),
            (EquipCategory::Shield, EquipmentSlot::LeftHand),
            (EquipCategory::Spellbook, EquipmentSlot::LeftHand),
            (EquipCategory::Quiver, EquipmentSlot::LeftHand),
            (EquipCategory::Extra, EquipmentSlot::Ammo),
        ];
        for (category, slot) in expected {
            assert_eq!(category.slot(), slot);
            assert_eq!(check_equip(category, &BTreeMap::new()), Ok(()));
        }
    }

    #[test]
    fn an_occupied_slot_rejects() {
        let equipped = wearing(&[EquipCategory::Ring, EquipCategory::Shield]);
        assert_eq!(
            check_equip(EquipCategory::Ring, &equipped),
            Err(EquipRejection::SlotOccupied(EquipmentSlot::Ring))
        );
        assert_eq!(
            check_equip(EquipCategory::Quiver, &equipped),
            Err(EquipRejection::SlotOccupied(EquipmentSlot::LeftHand))
        );
    }

    #[test]
    fn a_one_handed_weapon_goes_with_any_shield_slot_item() {
        for left in [
            EquipCategory::Shield,
            EquipCategory::Spellbook,
            EquipCategory::Quiver,
        ] {
            assert_eq!(
                check_equip(EquipCategory::OneHandedWeapon, &wearing(&[left])),
                Ok(())
            );
            assert_eq!(
                check_equip(left, &wearing(&[EquipCategory::OneHandedWeapon])),
                Ok(())
            );
        }
    }

    #[test]
    fn a_two_handed_weapon_blocks_the_shield_slot_in_both_orders() {
        for left in [
            EquipCategory::Shield,
            EquipCategory::Spellbook,
            EquipCategory::Quiver,
        ] {
            assert_eq!(
                check_equip(EquipCategory::TwoHandedWeapon, &wearing(&[left])),
                Err(EquipRejection::HandsConflict)
            );
            assert_eq!(
                check_equip(left, &wearing(&[EquipCategory::TwoHandedWeapon])),
                Err(EquipRejection::HandsConflict)
            );
        }
    }

    #[test]
    fn a_two_handed_distance_weapon_allows_only_a_quiver() {
        let bow = EquipCategory::TwoHandedDistanceWeapon;
        assert_eq!(check_equip(bow, &wearing(&[EquipCategory::Quiver])), Ok(()));
        assert_eq!(check_equip(EquipCategory::Quiver, &wearing(&[bow])), Ok(()));
        for left in [EquipCategory::Shield, EquipCategory::Spellbook] {
            assert_eq!(
                check_equip(bow, &wearing(&[left])),
                Err(EquipRejection::HandsConflict)
            );
            assert_eq!(
                check_equip(left, &wearing(&[bow])),
                Err(EquipRejection::HandsConflict)
            );
        }
    }

    #[test]
    fn other_slots_ignore_the_hands() {
        let equipped = wearing(&[EquipCategory::TwoHandedWeapon]);
        for category in [
            EquipCategory::Head,
            EquipCategory::Container,
            EquipCategory::Extra,
        ] {
            assert_eq!(check_equip(category, &equipped), Ok(()));
        }
    }
}
