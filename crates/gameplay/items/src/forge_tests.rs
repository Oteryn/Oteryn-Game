use super::*;
use oteryn_gameplay_domain::domain::forge::{ForgeDustLimit, ForgeDustResources};
use oteryn_gameplay_domain::domain::forge_dust::DustLimit;
use oteryn_gameplay_domain::domain::item_definition::{ItemStackClass, TypedDefinitionRef};

pub(super) fn definition(key: &str) -> ItemDefinitionFacts {
    ItemDefinitionFacts {
        definition: TypedDefinitionRef {
            family: "Item".into(),
            production_key: key.into(),
            revision_ref: "r1".into(),
        },
        weight: None,
        stack: ItemStackClass::NonStackable,
        container_capacity: None,
        container_slot_equip_pattern: false,
    }
}

pub(super) fn request(
    item: &ItemDefinitionFacts,
    class: u8,
    tier: u8,
    operation: ForgeOperation,
) -> Result<ForgeRequest<'_>, ForgeRuleError> {
    let target = ForgeItem {
        instance: [1; 16],
        definition: item,
        profile: ProjectV2ItemForgeProfile {
            classification: class,
            max_tier: FORGE_CLASS_CAPS[usize::from(class)],
        },
        tier: ItemTier::new(tier).ok_or(ForgeRuleError::InvalidClassCap)?,
    };
    Ok(ForgeRequest {
        operation,
        target,
        source: ForgeItem {
            instance: [2; 16],
            ..target
        },
    })
}

pub(super) fn resources() -> Result<ForgeResources, ForgeRuleError> {
    Ok(ForgeResources {
        gold: u64::MAX,
        dust: ForgeDustBalance::new(100, DustLimit::INITIAL)
            .map_err(|_| ForgeRuleError::InsufficientDust)?,
        slivers: u64::MAX,
        cores: u64::MAX,
    })
}

const FUSION: ForgeOperation = ForgeOperation::Fusion {
    boost: false,
    protect: false,
};

#[test]
fn forge_reference_prices_and_transfer_indices() -> Result<(), ForgeRuleError> {
    // Each row is one task-card parity row, transcribed from Canary item_tiers.lua.
    let rows = [
        (1, 1, 25_000, 1),
        (2, 1, 750_000, 1),
        (2, 2, 5_000_000, 1),
        (3, 1, 4_000_000, 1),
        (3, 2, 10_000_000, 2),
        (3, 3, 20_000_000, 3),
        (4, 1, 8_000_000, 1),
        (4, 2, 20_000_000, 2),
        (4, 3, 40_000_000, 5),
        (4, 4, 65_000_000, 10),
        (4, 5, 100_000_000, 15),
        (4, 6, 250_000_000, 25),
        (4, 7, 750_000_000, 35),
        (4, 8, 2_500_000_000, 50),
        (4, 9, 8_000_000_000, 60),
        (4, 10, 15_000_000_000, 85),
    ];
    let item = definition("forge:a");
    for (class, tier, gold, cores) in rows {
        let quote = ForgeQuote::try_from(request(&item, class, tier - 1, FUSION)?)?;
        assert_eq!(
            quote.cost,
            ForgeCost {
                gold,
                dust: 100,
                cores: 0
            }
        );
        assert_eq!(price(class, tier)?.transfer_cores, cores);
        if tier >= 2 {
            let mut req = request(&item, class, 0, ForgeOperation::Transfer)?;
            req.source.tier = ItemTier::new(tier).ok_or(ForgeRuleError::InvalidClassCap)?;
            let transfer = ForgeQuote::try_from(req)?;
            assert_eq!(transfer.cost.gold, gold);
            assert_eq!(transfer.cost.cores, price(class, tier - 1)?.transfer_cores);
            let plan = ForgePlan::try_from(ForgeAttempt {
                quote: transfer,
                resources: resources()?,
                success_roll: 99,
                protection_roll: 99,
            })?;
            assert!(plan.success);
            assert_eq!(plan.target_tier.get(), tier - 1);
            assert_eq!(plan.source_tier, None);
        }
    }
    Ok(())
}

#[test]
fn forge_reference_class_caps_and_shape_refusals() -> Result<(), ForgeRuleError> {
    let item = definition("forge:a");
    for (class, cap) in [(1, 1), (2, 2), (3, 3), (4, 10)] {
        assert!(ForgeQuote::try_from(request(&item, class, cap - 1, FUSION)?).is_ok());
        assert_eq!(
            ForgeQuote::try_from(request(&item, class, cap, FUSION)?),
            Err(ForgeRuleError::Shape(ForgeShapeError::TierAboveCap))
        );
        let mut req = request(&item, class, 0, FUSION)?;
        req.target.profile.max_tier = cap + 1;
        assert_eq!(
            ForgeQuote::try_from(req),
            Err(ForgeRuleError::InvalidClassCap)
        );
    }
    let base = request(&item, 4, 1, FUSION)?;
    let mut req = base;
    req.source.instance = req.target.instance;
    assert_eq!(ForgeQuote::try_from(req), Err(ForgeRuleError::SameInstance));
    let mut revised = item.clone();
    revised.definition.revision_ref = "r2".into();
    req = base;
    req.source.definition = &revised;
    assert_eq!(
        ForgeQuote::try_from(req),
        Err(ForgeRuleError::Shape(ForgeShapeError::DifferentDefinition))
    );
    req = base;
    req.source.tier = ItemTier::ZERO;
    assert_eq!(
        ForgeQuote::try_from(req),
        Err(ForgeRuleError::Shape(ForgeShapeError::DifferentTier))
    );
    for class in [0, 5, 255] {
        req = base;
        req.target.profile.classification = class;
        assert_eq!(
            ForgeQuote::try_from(req),
            Err(ForgeRuleError::InvalidClassCap)
        );
    }
    let mut stack = item.clone();
    stack.stack = ItemStackClass::Stackable {
        proven_maximum: Some(100),
    };
    assert_eq!(
        ForgeQuote::try_from(request(&stack, 4, 0, FUSION)?),
        Err(ForgeRuleError::Shape(ForgeShapeError::NotNonStackable))
    );
    Ok(())
}

#[test]
fn forge_reference_success_protection_and_failure_costs() -> Result<(), ForgeRuleError> {
    let item = definition("forge:a");
    for (boost, protect, chance, cores) in [
        (false, false, 50, 0),
        (true, false, 65, 1),
        (false, true, 50, 1),
        (true, true, 65, 2),
    ] {
        for tier in [0, 1, 9] {
            let quote = ForgeQuote::try_from(request(
                &item,
                4,
                tier,
                ForgeOperation::Fusion { boost, protect },
            )?)?;
            assert_eq!((quote.success_percent, quote.cost.cores), (chance, cores));
            for (roll, success) in [(0, true), (chance - 1, true), (chance, false), (99, false)] {
                for protection_roll in [0, 49, 50, 99] {
                    let before = resources()?;
                    let attempt = ForgeAttempt {
                        quote,
                        resources: before,
                        success_roll: roll,
                        protection_roll,
                    };
                    let plan = ForgePlan::try_from(attempt)?;
                    assert_eq!(plan, ForgePlan::try_from(attempt)?); // supplied draws never reroll
                    assert_eq!(plan.success, success);
                    assert_eq!(
                        plan.target_tier.get(),
                        if success { tier + 1 } else { tier }
                    );
                    assert_eq!(
                        plan.source_tier.map(ItemTier::get),
                        if success {
                            None
                        } else if protect && protection_roll < 50 {
                            Some(tier)
                        } else {
                            tier.checked_sub(1)
                        }
                    );
                    assert_eq!(before.gold - plan.resources_after.gold, quote.cost.gold);
                    assert_eq!(plan.resources_after.dust.balance(), 0);
                    assert_eq!(before.cores - plan.resources_after.cores, u64::from(cores));
                    assert_eq!(plan.resources_after.slivers, before.slivers);
                }
            }
        }
    }
    Ok(())
}

#[test]
fn forge_missing_resources_invalid_draws_and_transfer_prerequisites() -> Result<(), ForgeRuleError>
{
    let item = definition("forge:a");
    let other = definition("forge:b");
    let quote = ForgeQuote::try_from(request(
        &item,
        4,
        0,
        ForgeOperation::Fusion {
            boost: true,
            protect: true,
        },
    )?)?;
    let attempt = ForgeAttempt {
        quote,
        resources: resources()?,
        success_roll: 0,
        protection_roll: 0,
    };
    for (resources, error) in [
        (
            ForgeResources {
                gold: quote.cost.gold - 1,
                ..resources()?
            },
            ForgeRuleError::InsufficientGold,
        ),
        (
            ForgeResources {
                dust: ForgeDustBalance::default(),
                ..resources()?
            },
            ForgeRuleError::InsufficientDust,
        ),
        (
            ForgeResources {
                cores: 1,
                ..resources()?
            },
            ForgeRuleError::InsufficientCores,
        ),
    ] {
        assert_eq!(
            ForgePlan::try_from(ForgeAttempt {
                resources,
                ..attempt
            }),
            Err(error)
        );
    }
    for draw in [100, 255] {
        assert_eq!(
            ForgePlan::try_from(ForgeAttempt {
                success_roll: draw,
                ..attempt
            }),
            Err(ForgeRuleError::InvalidRoll)
        );
        assert_eq!(
            ForgePlan::try_from(ForgeAttempt {
                protection_roll: draw,
                ..attempt
            }),
            Err(ForgeRuleError::InvalidRoll)
        );
    }
    let mut req = request(&item, 4, 0, ForgeOperation::Transfer)?;
    req.source.definition = &other;
    for tier in [0, 1] {
        req.source.tier = ItemTier::new(tier).ok_or(ForgeRuleError::InvalidClassCap)?;
        assert_eq!(
            ForgeQuote::try_from(req),
            Err(ForgeRuleError::Shape(ForgeShapeError::DonorTierTooLow))
        );
    }
    req.source.tier = ItemTier::new(2).ok_or(ForgeRuleError::InvalidClassCap)?;
    assert!(ForgeQuote::try_from(req).is_ok());
    req.target.tier = req.source.tier;
    assert_eq!(
        ForgeQuote::try_from(req),
        Err(ForgeRuleError::Shape(ForgeShapeError::TargetAlreadyTiered))
    );
    req.target.tier = ItemTier::ZERO;
    req.source.profile.classification = 3;
    req.source.profile.max_tier = 3;
    assert_eq!(
        ForgeQuote::try_from(req),
        Err(ForgeRuleError::Shape(
            ForgeShapeError::DifferentClassification
        ))
    );
    Ok(())
}

#[test]
fn forge_reference_conversions_and_d1230_dust_cap() -> Result<(), String> {
    let rows = [(60, 0, 0, 0, 3, 0), (100, 7, 9, 40, 10, 9)];
    for (dust, slivers, cores, after_dust, after_slivers, after_cores) in rows {
        let after = ForgeDustResources {
            dust,
            slivers,
            cores,
        }
        .dust_to_slivers()
        .map_err(|e| format!("{e:?}"))?;
        assert_eq!(
            after,
            ForgeDustResources {
                dust: after_dust,
                slivers: after_slivers,
                cores: after_cores
            }
        );
    }
    for (slivers, cores, after_slivers, after_cores) in [(50, 0, 0, 1), (99, 7, 49, 8)] {
        let after = ForgeDustResources {
            dust: 0,
            slivers,
            cores,
        }
        .slivers_to_core()
        .map_err(|e| format!("{e:?}"))?;
        assert_eq!((after.slivers, after.cores), (after_slivers, after_cores));
    }
    // D1230 overrides the old Canary max225: retain the accepted +20 progression.
    for (stage, limit, cost) in [
        (0, 100, Some(25)),
        (1, 120, Some(26)),
        (224, 4580, Some(249)),
        (225, 4600, None),
    ] {
        let policy = ForgeDustLimit::new(stage).map_err(|e| format!("{e:?}"))?;
        assert_eq!(policy.current_limit(), Ok(limit));
        assert_eq!(policy.next_upgrade().ok().map(|(cost, _)| cost), cost);
    }
    assert!(DustLimit::new(4601).is_err());
    Ok(())
}
