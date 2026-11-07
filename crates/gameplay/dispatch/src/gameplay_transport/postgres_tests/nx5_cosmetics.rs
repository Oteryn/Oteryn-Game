//! NX-5 uses the authored production manifest through the real ordinary-USE route.
#[test]
fn nx5_outfit_items_unlock_catalogue_addons_and_replay_once() -> super::TestResult {
    super::item_cosmetics::nx5_production_case(0)
}

#[test]
fn nx5_mount_parts_unlock_the_production_appearance_catalogue_once() -> super::TestResult {
    super::item_cosmetics::nx5_production_case(1)
}

#[test]
fn nx5_ot_only_addon_doll_is_refused_with_populated_cosmetics() -> super::TestResult {
    super::item_cosmetics::nx5_production_case(2)
}

#[test]
fn nx5_mount_token_unlocks_the_production_catalogue_and_replays_once() -> super::TestResult {
    super::item_cosmetics::nx5_production_case(3)
}

#[test]
fn nx5_every_global_item_rule_qualifies_against_source_bound_production_cosmetics()
-> super::TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| -> Result<(), String> {
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../../content/spells.manifest.json");
            let input = crate::content::native_gameplay::NativeGameplayInput::from_manifest(&path)
                .map_err(|e| format!("{e:?}"))?;
            let cosmetics =
                crate::content::native_gameplay::test_support::cosmetics_fixture::decode_cosmetics(
                    &input.cosmetics.bytes,
                )
                .map_err(|e| format!("{e:?}"))?;
            let catalogue =
                oteryn_gameplay_items::item_cosmetics::catalogue().ok_or("item rules")?;
            let mut count = 0;
            for rule in &catalogue.rules {
                if matches!(
                    rule.mode,
                    oteryn_gameplay_items::item_cosmetics::Mode::RandomOutfit
                        | oteryn_gameplay_items::item_cosmetics::Mode::RandomMount
                ) {
                    continue;
                }
                oteryn_gameplay_items::item_cosmetics::qualify_rule(catalogue, rule, &cosmetics)
                    .map_err(|e| format!("item {}: {e:?}", rule.item_id))?;
                count += 1;
            }
            assert_eq!(count, 46);
            use crate::domain::appearance::{EarnedCosmetics, Sex};
            let facts = vec![
                (
                    "oteryn:appearance-grant/quest/soul-war/revenant-base".to_owned(),
                    "crystal-201189f-soul-war-revenant-v1".to_owned(),
                ),
                (
                    "oteryn:appearance-grant/quest/rotten-blood/decaying-defender-base".to_owned(),
                    "owner-pr2784-decaying-defender-base-v1".to_owned(),
                ),
            ];
            let owned = EarnedCosmetics {
                content: &cosmetics,
                facts: &facts,
            };
            for key in [
                "oteryn:item-grant.outfit.1322-1323.base",
                "oteryn:item-grant.outfit.1662-1663.base",
            ] {
                let grant =
                    oteryn_gameplay_items::item_cosmetics::resolve(catalogue, &cosmetics, key)
                        .map_err(|e| format!("{e:?}"))?;
                assert!(
                    grant.owned(&cosmetics, &owned, Sex::Female),
                    "quest base must satisfy an item's prerequisite"
                );
                assert!(grant.owned(&cosmetics, &owned, Sex::Male));
                let incompatible = vec![
                    (facts[0].0.clone(), "incompatible-provenance".into()),
                    (facts[1].0.clone(), "incompatible-provenance".into()),
                ];
                assert!(!grant.owned(
                    &cosmetics,
                    &EarnedCosmetics {
                        content: &cosmetics,
                        facts: &incompatible
                    },
                    Sex::Female
                ));
            }
            Ok(())
        })?
        .join()
        .map_err(|_| "production cosmetics qualification thread")??;
    Ok(())
}
