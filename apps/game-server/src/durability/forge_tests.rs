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

#[test]
fn accepted_dust_limit_and_next_cost_follow_separately_classified_evidence() {
    use super::ForgeDustLimit;
    let initial = ForgeDustLimit::INITIAL;
    assert_eq!(initial.current_limit(), Ok(100));
    let (cost, second) = initial.next_upgrade().unwrap();
    assert_eq!(cost, 25); // GUI/CIPSOFT initial cost.
    assert_eq!(second.current_limit(), Ok(120));
    assert_eq!(second.next_upgrade().unwrap().0, 26); // PARITY_WIKI general cost.
    for upgrades in 0..225 {
        let current = ForgeDustLimit::new(upgrades).unwrap();
        let (cost, next) = current.next_upgrade().unwrap();
        assert_eq!(current.current_limit(), Ok(100 + 20 * u32::from(upgrades)));
        assert_eq!(cost, 25 + u32::from(upgrades));
        assert_eq!(
            next.current_limit(),
            Ok(current.current_limit().unwrap() + 20)
        );
    }
}

#[test]
fn wiki_dust_ceiling_refuses_further_upgrades_and_invalid_states() {
    use super::{ForgeDustError, ForgeDustLimit};
    let penultimate = ForgeDustLimit::new(224).unwrap();
    let (last_cost, maximum) = penultimate.next_upgrade().unwrap();
    assert_eq!(last_cost, 249);
    assert_eq!(maximum.current_limit(), Ok(4600)); // PARITY_WIKI, not a GUI display.
    assert_eq!(
        maximum.next_upgrade(),
        Err(ForgeDustError::MaximumLimitReached)
    );
    for upgrades in [226, u16::MAX] {
        assert_eq!(
            ForgeDustLimit::new(upgrades),
            Err(ForgeDustError::InvalidUpgradeCount)
        );
    }
}

#[test]
fn accepted_conversions_charge_exact_inputs_and_preserve_other_resources() {
    use super::ForgeDustResources;
    let before = ForgeDustResources {
        dust: 120,
        slivers: 47,
        cores: 7,
    };
    let slivers = before.dust_to_slivers().unwrap();
    assert_eq!(
        slivers,
        ForgeDustResources {
            dust: 60,
            slivers: 50,
            cores: 7
        }
    );
    assert_eq!(
        slivers.slivers_to_core(),
        Ok(ForgeDustResources {
            dust: 60,
            slivers: 0,
            cores: 8
        })
    );
    assert_eq!(
        ForgeDustResources {
            dust: 60,
            slivers: 0,
            cores: 0
        }
        .dust_to_slivers(),
        Ok(ForgeDustResources {
            dust: 0,
            slivers: 3,
            cores: 0
        })
    );
}

#[test]
fn conversions_refuse_underfunded_and_overflowing_projections() {
    use super::{ForgeDustError, ForgeDustResources};
    let zero = ForgeDustResources {
        dust: 0,
        slivers: 0,
        cores: 0,
    };
    for dust in [0, 59] {
        assert_eq!(
            ForgeDustResources { dust, ..zero }.dust_to_slivers(),
            Err(ForgeDustError::InsufficientDust)
        );
    }
    for slivers in [0, 49] {
        assert_eq!(
            ForgeDustResources { slivers, ..zero }.slivers_to_core(),
            Err(ForgeDustError::InsufficientSlivers)
        );
    }
    assert_eq!(
        ForgeDustResources {
            dust: 60,
            slivers: u64::MAX - 2,
            ..zero
        }
        .dust_to_slivers(),
        Err(ForgeDustError::ArithmeticOverflow)
    );
    assert_eq!(
        ForgeDustResources {
            slivers: 50,
            cores: u64::MAX,
            ..zero
        }
        .slivers_to_core(),
        Err(ForgeDustError::ArithmeticOverflow)
    );
}
