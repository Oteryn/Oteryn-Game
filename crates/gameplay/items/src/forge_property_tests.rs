use super::tests::{definition, request, resources};
use super::*;
use oteryn_test_support::property::for_each_case;

#[test]
fn forge_property_plans_conserve_costs_items_and_bounds() {
    for_each_case("forge standard outcomes", |rng, _| {
        let item = definition("forge:property");
        let class = 1 + rng.below(4) as u8;
        let tier = rng.below(u64::from(FORGE_CLASS_CAPS[usize::from(class)])) as u8;
        let operation = ForgeOperation::Fusion {
            boost: rng.below(2) == 1,
            protect: rng.below(2) == 1,
        };
        let quote = ForgeQuote::try_from(
            request(&item, class, tier, operation).map_err(|e| format!("{e:?}"))?,
        )
        .map_err(|e| format!("{e:?}"))?;
        let before = ForgeResources {
            gold: quote.cost.gold.saturating_add(rng.u64()),
            ..resources().map_err(|e| format!("{e:?}"))?
        };
        let plan = ForgePlan::try_from(ForgeAttempt {
            quote,
            resources: before,
            success_roll: rng.below(100) as u8,
            protection_roll: rng.below(100) as u8,
        })
        .map_err(|e| format!("{e:?}"))?;
        assert_eq!(
            u128::from(plan.resources_after.gold) + u128::from(quote.cost.gold),
            u128::from(before.gold)
        );
        assert_eq!(
            plan.resources_after.cores + u64::from(quote.cost.cores),
            before.cores
        );
        assert_eq!(
            u32::from(plan.resources_after.dust.balance()) + quote.cost.dust,
            u32::from(before.dust.balance())
        );
        assert_eq!(plan.resources_after.slivers, before.slivers);
        assert!(plan.target_tier.get() <= FORGE_CLASS_CAPS[usize::from(class)]);
        if let Some(source) = plan.source_tier {
            assert!(!plan.success);
            assert!(source.get() <= tier);
        }
        let remaining_items = 1 + usize::from(plan.source_tier.is_some());
        let burned = usize::from(plan.source_tier.is_none());
        assert_eq!(remaining_items + burned, 2);
        Ok(())
    });
}

#[test]
fn forge_property_conversion_arithmetic_never_wraps() {
    use oteryn_gameplay_domain::domain::forge::ForgeDustResources;
    for_each_case("forge conversion conservation", |rng, _| {
        for value in [0, 1, 49, 50, 59, 60, rng.u64(), u64::MAX] {
            let before = ForgeDustResources {
                dust: value,
                slivers: value,
                cores: value,
            };
            assert_eq!(
                before.dust_to_slivers().is_ok(),
                (60..=u64::MAX - 3).contains(&value)
            );
            if let Ok(after) = before.dust_to_slivers() {
                assert_eq!(after.dust + 60, before.dust);
                assert_eq!(after.slivers - 3, before.slivers);
                assert_eq!(after.cores, before.cores);
            }
            assert_eq!(
                before.slivers_to_core().is_ok(),
                (50..u64::MAX).contains(&value)
            );
            if let Ok(after) = before.slivers_to_core() {
                assert_eq!(after.slivers + 50, before.slivers);
                assert_eq!(after.cores - 1, before.cores);
                assert_eq!(after.dust, before.dust);
            }
        }
        Ok(())
    });
}

#[test]
fn forge_property_transfers_preserve_unspent_assets_and_dust_limit() {
    use oteryn_gameplay_domain::domain::forge_dust::DustLimit;
    for_each_case("forge transfer conservation", |rng, _| {
        let item = definition("forge:transfer");
        let class = 2 + rng.below(3) as u8;
        let tier = 2 + rng.below(u64::from(FORGE_CLASS_CAPS[usize::from(class)] - 1)) as u8;
        let mut request =
            request(&item, class, 0, ForgeOperation::Transfer).map_err(|e| format!("{e:?}"))?;
        request.source.tier = ItemTier::new(tier).ok_or("invalid generated tier")?;
        let quote = ForgeQuote::try_from(request).map_err(|e| format!("{e:?}"))?;
        let limit =
            DustLimit::from_upgrade_count(rng.below(226) as u16).map_err(|e| format!("{e:?}"))?;
        let before = ForgeResources {
            gold: quote.cost.gold.saturating_add(rng.u64()),
            dust: ForgeDustBalance::new(limit.get(), limit).map_err(|e| format!("{e:?}"))?,
            ..resources().map_err(|e| format!("{e:?}"))?
        };
        let plan = ForgePlan::try_from(ForgeAttempt {
            quote,
            resources: before,
            success_roll: 99,
            protection_roll: 99,
        })
        .map_err(|e| format!("{e:?}"))?;
        assert!(plan.success);
        assert_eq!(plan.source_tier, None);
        assert_eq!(plan.target_tier.get(), tier - 1);
        assert_eq!(plan.resources_after.gold + quote.cost.gold, before.gold);
        assert_eq!(plan.resources_after.dust.limit(), before.dust.limit());
        assert_eq!(
            u32::from(plan.resources_after.dust.balance()) + quote.cost.dust,
            u32::from(before.dust.balance())
        );
        assert_eq!(
            plan.resources_after.cores + u64::from(quote.cost.cores),
            before.cores
        );
        assert_eq!(plan.resources_after.slivers, before.slivers);
        Ok(())
    });
}
