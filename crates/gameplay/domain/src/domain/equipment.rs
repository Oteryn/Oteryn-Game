//! Pure Reference equipment-slot rules (tibia.com manual §3.4.1, owner answer 4 in #162
//! 5879169470; GAME-ITEM-01 §6.2).
//!
//! Item categories come from content, and vocation or level requirements are typed item
//! requirements checked elsewhere; the move itself is DUR-03-owned.

use std::collections::{BTreeMap, BTreeSet};

use super::death::EquipmentSlot;
use super::{DomainError, EquipPattern};

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
    /// The item's complete occupancy claim (GAME-ITEM-01 §6.2): its primary slot plus every
    /// further resource it reserves.
    ///
    /// A two-handed weapon reserves both hands. A two-handed distance weapon reserves the right
    /// hand and the non-quiver left-hand group, so only a quiver may share the left hand with it
    /// (manual §3.4.1). Shields and spellbooks reserve that same group with the left hand.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::MissingPrimarySlot`] only if a claim omitted its primary slot,
    /// which the fixed patterns below never do.
    pub fn pattern(self) -> Result<EquipPattern<EquipResource>, DomainError> {
        use EquipResource::{NonQuiverLeftHand, Slot};
        let primary = Slot(self.slot());
        match self {
            Self::TwoHandedWeapon => EquipPattern::new(
                primary,
                [
                    Slot(EquipmentSlot::RightHand),
                    Slot(EquipmentSlot::LeftHand),
                ],
            ),
            Self::TwoHandedDistanceWeapon => {
                EquipPattern::new(primary, [Slot(EquipmentSlot::RightHand), NonQuiverLeftHand])
            }
            Self::Shield | Self::Spellbook => {
                EquipPattern::new(primary, [Slot(EquipmentSlot::LeftHand), NonQuiverLeftHand])
            }
            _ => EquipPattern::new(primary, [primary]),
        }
    }
}

/// A resource an equipped item can reserve: a slot, or an occupancy group shared by several
/// slot-holders.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EquipResource {
    Slot(EquipmentSlot),
    /// Reserved by everything in the left hand except a quiver, and by a distance weapon so that
    /// only a quiver may join it there.
    NonQuiverLeftHand,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EquipRejection {
    /// The item's slot already holds an item.
    SlotOccupied(EquipmentSlot),
    /// The item's occupancy claim overlaps another equipped item's claim, such as a two-handed
    /// weapon against the shield slot.
    HandsConflict,
    /// An item category produced a pattern without its primary slot.
    MalformedPattern,
}

/// Whether an item of `category` may be equipped against the currently equipped items. A swap
/// is checked against the equipment without the item it displaces.
///
/// The item's complete claim must be disjoint from the union of the equipped items' claims.
///
/// # Errors
///
/// Returns the occupied slot, the claim conflict, or a malformed pattern.
pub fn check_equip(
    category: EquipCategory,
    equipped: &BTreeMap<EquipmentSlot, EquipCategory>,
) -> Result<(), EquipRejection> {
    let slot = category.slot();
    if equipped.contains_key(&slot) {
        return Err(EquipRejection::SlotOccupied(slot));
    }
    let mut occupied = BTreeSet::new();
    for item in equipped.values() {
        let pattern = item
            .pattern()
            .map_err(|_| EquipRejection::MalformedPattern)?;
        occupied.extend(pattern.claims().copied());
    }
    let pattern = category
        .pattern()
        .map_err(|_| EquipRejection::MalformedPattern)?;
    if pattern.is_legal_against(&occupied) {
        Ok(())
    } else {
        Err(EquipRejection::HandsConflict)
    }
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

    #[test]
    fn a_two_handed_claim_occupies_both_hands() -> Result<(), DomainError> {
        let claim: BTreeSet<_> = EquipCategory::TwoHandedWeapon
            .pattern()?
            .claims()
            .copied()
            .collect();
        assert_eq!(
            claim,
            BTreeSet::from([
                EquipResource::Slot(EquipmentSlot::RightHand),
                EquipResource::Slot(EquipmentSlot::LeftHand),
            ])
        );
        Ok(())
    }

    #[test]
    fn a_shield_and_a_two_handed_weapon_conflict_through_the_claim() -> Result<(), DomainError> {
        let shield = EquipCategory::Shield.pattern()?;
        let two_handed = EquipCategory::TwoHandedWeapon.pattern()?;
        let occupied: BTreeSet<_> = shield.claims().copied().collect();
        assert!(!two_handed.is_legal_against(&occupied));
        let occupied: BTreeSet<_> = two_handed.claims().copied().collect();
        assert!(!shield.is_legal_against(&occupied));
        Ok(())
    }

    #[test]
    fn a_distance_weapon_claim_leaves_the_left_hand_to_a_quiver_only() -> Result<(), DomainError> {
        let bow: BTreeSet<_> = EquipCategory::TwoHandedDistanceWeapon
            .pattern()?
            .claims()
            .copied()
            .collect();
        assert!(EquipCategory::Quiver.pattern()?.is_legal_against(&bow));
        assert!(!EquipCategory::Shield.pattern()?.is_legal_against(&bow));
        assert!(!EquipCategory::Spellbook.pattern()?.is_legal_against(&bow));
        Ok(())
    }
}
