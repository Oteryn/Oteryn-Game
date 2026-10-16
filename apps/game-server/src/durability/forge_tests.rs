//! Pure eligibility tests use the actual Content and native item-facts types.
#![allow(
    clippy::unwrap_used,
    reason = "test fixtures and expected typed outcomes"
)]

use super::{ForgeItemShape, ForgeShapeError, ItemTier};
use crate::content::ProjectV2ItemForgeProfile;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_transfer::{ItemDefinitionFacts, ItemStackClass};

fn item(key: &str) -> ItemDefinitionFacts {
    ItemDefinitionFacts {
        definition: TypedDefinitionRef {
            family: "Item".into(),
            production_key: key.into(),
            revision_ref: "item-definition-1".into(),
        },
        stack: ItemStackClass::NonStackable,
        container_capacity: None,
        container_slot_equip_pattern: false,
    }
}

fn profile(classification: u8, max_tier: u8) -> ProjectV2ItemForgeProfile {
    ProjectV2ItemForgeProfile {
        classification,
        max_tier,
    }
}

fn shape(item: &ItemDefinitionFacts, class: u8, cap: u8, tier: u8) -> ForgeItemShape<'_> {
    ForgeItemShape::from_content(
        item,
        Some(profile(class, cap)),
        ItemTier::new(tier).unwrap(),
    )
    .unwrap()
}

#[test]
fn content_profile_must_be_present_and_within_accepted_bounds() {
    let item = item("oteryn:item.test");
    assert_eq!(
        ForgeItemShape::from_content(&item, None, ItemTier::ZERO).unwrap_err(),
        ForgeShapeError::MissingForgeProfile
    );
    for class in [0, 5, u8::MAX] {
        assert_eq!(
            ForgeItemShape::from_content(&item, Some(profile(class, 1)), ItemTier::ZERO)
                .unwrap_err(),
            ForgeShapeError::InvalidClassification
        );
    }
    for cap in [0, 11, u8::MAX] {
        assert_eq!(
            ForgeItemShape::from_content(&item, Some(profile(1, cap)), ItemTier::ZERO).unwrap_err(),
            ForgeShapeError::InvalidTierCap
        );
    }
    for class in 1..=4 {
        for cap in [1, 10] {
            // Caps come from Content rather than a hardcoded class-to-cap map.
            assert!(
                ForgeItemShape::from_content(
                    &item,
                    Some(profile(class, cap)),
                    ItemTier::new(cap).unwrap()
                )
                .is_ok()
            );
        }
    }
    assert!(ItemTier::new(11).is_none());
}

#[test]
fn unknown_or_stackable_items_and_tiers_above_the_definition_cap_are_refused() {
    let mut item = item("oteryn:item.test");
    for stack in [
        ItemStackClass::Unknown,
        ItemStackClass::Stackable {
            proven_maximum: None,
        },
        ItemStackClass::Stackable {
            proven_maximum: Some(1),
        },
    ] {
        item.stack = stack;
        assert_eq!(
            ForgeItemShape::from_content(&item, Some(profile(2, 3)), ItemTier::ZERO).unwrap_err(),
            ForgeShapeError::NotNonStackable
        );
    }
    item.stack = ItemStackClass::NonStackable;
    assert_eq!(
        ForgeItemShape::from_content(&item, Some(profile(2, 3)), ItemTier::new(4).unwrap())
            .unwrap_err(),
        ForgeShapeError::TierAboveCap
    );
}

#[test]
fn fusion_checks_both_definition_revision_and_tier_then_the_success_cap() {
    let target = item("oteryn:item.target");
    let mut source = target.clone();
    assert_eq!(
        shape(&target, 4, 10, 9)
            .fusion_target_tier(shape(&source, 4, 10, 9))
            .unwrap()
            .get(),
        10
    );
    assert_eq!(
        shape(&target, 4, 10, 10).fusion_target_tier(shape(&source, 4, 10, 10)),
        Err(ForgeShapeError::TierAboveCap)
    );
    assert_eq!(
        shape(&target, 4, 10, 1).fusion_target_tier(shape(&source, 4, 10, 2)),
        Err(ForgeShapeError::DifferentTier)
    );
    assert_eq!(
        shape(&target, 4, 10, 1).fusion_target_tier(shape(&source, 4, 9, 1)),
        Err(ForgeShapeError::InconsistentDefinitionProfile)
    );
    source.definition.revision_ref = "item-definition-2".into();
    assert_eq!(
        shape(&target, 4, 10, 1).fusion_target_tier(shape(&source, 4, 10, 1)),
        Err(ForgeShapeError::DifferentDefinition)
    );
    source = item("oteryn:item.other");
    assert_eq!(
        shape(&target, 4, 10, 1).fusion_target_tier(shape(&source, 4, 10, 1)),
        Err(ForgeShapeError::DifferentDefinition)
    );
    assert_eq!(
        shape(&target, 1, 1, 0)
            .fusion_target_tier(shape(&target, 1, 1, 0))
            .unwrap()
            .get(),
        1
    );
}

#[test]
fn transfer_allows_different_definitions_but_enforces_class_donor_and_receiver_caps() {
    let target = item("oteryn:item.target");
    let donor = item("oteryn:item.donor");
    assert_eq!(
        shape(&target, 4, 9, 0)
            .transfer_target_tier(shape(&donor, 4, 10, 10))
            .unwrap()
            .get(),
        9
    );
    assert_eq!(
        shape(&target, 4, 8, 0).transfer_target_tier(shape(&donor, 4, 10, 10)),
        Err(ForgeShapeError::TierAboveCap)
    );
    assert_eq!(
        shape(&target, 3, 3, 0).transfer_target_tier(shape(&donor, 4, 10, 2)),
        Err(ForgeShapeError::DifferentClassification)
    );
    assert_eq!(
        shape(&target, 4, 10, 1).transfer_target_tier(shape(&donor, 4, 10, 2)),
        Err(ForgeShapeError::TargetAlreadyTiered)
    );
    for donor_tier in [0, 1] {
        assert_eq!(
            shape(&target, 4, 10, 0).transfer_target_tier(shape(&donor, 4, 10, donor_tier)),
            Err(ForgeShapeError::DonorTierTooLow)
        );
    }
    assert_eq!(
        shape(&target, 4, 10, 0)
            .transfer_target_tier(shape(&donor, 4, 10, 2))
            .unwrap()
            .get(),
        1
    );
}

#[test]
fn convergence_transfer_requires_both_class_four_and_keeps_the_donor_tier_within_cap() {
    let target = item("oteryn:item.target");
    let donor = item("oteryn:item.donor");
    for tier in [2, 10] {
        assert_eq!(
            shape(&target, 4, 10, 0)
                .convergence_transfer_target_tier(shape(&donor, 4, 10, tier))
                .unwrap()
                .get(),
            tier
        );
    }
    // Standard transfer fits at cap nine, but convergence keeps tier ten.
    assert_eq!(
        shape(&target, 4, 9, 0)
            .transfer_target_tier(shape(&donor, 4, 10, 10))
            .unwrap()
            .get(),
        9
    );
    assert_eq!(
        shape(&target, 4, 9, 0).convergence_transfer_target_tier(shape(&donor, 4, 10, 10)),
        Err(ForgeShapeError::TierAboveCap)
    );
    for (target_class, donor_class) in [(3, 3), (3, 4), (4, 3)] {
        assert_eq!(
            shape(&target, target_class, 3, 0).convergence_transfer_target_tier(shape(
                &donor,
                donor_class,
                3,
                2
            )),
            Err(ForgeShapeError::ConvergenceRequiresClassFour)
        );
    }
    assert_eq!(
        shape(&target, 4, 10, 1).convergence_transfer_target_tier(shape(&donor, 4, 10, 2)),
        Err(ForgeShapeError::TargetAlreadyTiered)
    );
    for donor_tier in [0, 1] {
        assert_eq!(
            shape(&target, 4, 10, 0)
                .convergence_transfer_target_tier(shape(&donor, 4, 10, donor_tier)),
            Err(ForgeShapeError::DonorTierTooLow)
        );
    }
}
