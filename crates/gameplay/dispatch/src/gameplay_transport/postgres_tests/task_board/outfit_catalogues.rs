use super::TestResult;

#[test]
fn coutfit_wayland_loads_verified_scrap_exchanges() -> TestResult {
    use oteryn_gameplay_quests::quest::{loader, reward_callbacks};
    let loaded = loader::load_embedded_quest_state("content-1").map_err(super::debug)?;
    let catalogue = loaded.catalogue();
    let callbacks = reward_callbacks::offered_for(catalogue).map_err(super::debug)?;
    let outfit = "oteryn:outfit.illuminated_warrior";
    for (keyword, amount, addon) in [("basic", 2500, 0), ("insignia", 1250, 1), ("halo", 1250, 2)] {
        let exchange = callbacks
            .iter()
            .find(|row| row.npc == "oteryn:npc.wayland_smythers" && row.key.ends_with(keyword))
            .ok_or(keyword)?;
        assert_eq!(exchange.keywords, ["yes"]);
        assert_eq!(exchange.topic, 101 + u16::from(addon));
        assert!(catalogue.transition(&exchange.key).is_some());
        assert_eq!(exchange.take.len(), 1);
        assert_eq!(exchange.take[0].item, "oteryn:item.tibia.i54640");
        assert_eq!(exchange.take[0].count, amount);
        let reward = exchange.outfit.as_ref().ok_or("outfit reward")?;
        assert_eq!((reward.outfit.as_str(), reward.addons), (outfit, addon));
        let requirements = exchange.requirements.as_ref().ok_or("Wayland requirements")?;
        assert_eq!(requirements.min_level, 1000);
        assert!(requirements.premium);
        assert!(requirements.completed_quests.is_empty());
        if addon > 0 {
            assert_eq!(exchange.requires_outfits, [outfit]);
            assert_eq!(exchange.forbids_addons.as_slice(), std::slice::from_ref(reward));
        }
    }
    Ok(())
}

#[test]
fn coutfit_source_items_qualify_for_their_exact_earned_outfit() -> TestResult {
    use crate::content::native_gameplay::{NativeGameplayInput, test_support::cosmetics_fixture};
    use oteryn_gameplay_items::item_cosmetics::{self, Mode};

    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../content/spells.manifest.json");
    let input = NativeGameplayInput::from_manifest(&manifest).map_err(super::debug)?;
    let cosmetics = cosmetics_fixture::decode_cosmetics(&input.cosmetics.bytes)
        .map_err(super::debug)?;
    let catalogue = item_cosmetics::catalogue().ok_or("item cosmetics catalogue")?;
    for (item, outfit, addon) in [
        (40578, "ancient_aucar", 0),
        (44753, "draccoon_herald", 2),
        (44754, "draccoon_herald", 1),
        (48514, "rootwalker", 1),
        (48515, "rootwalker", 2),
    ] {
        let key = format!("oteryn:item.tibia.i{item}");
        let outfit = format!("oteryn:outfit.{outfit}");
        let rule = item_cosmetics::rule(&key).ok_or(key.clone())?;
        assert!(item_cosmetics::source_policy_allows(rule, Some(&cosmetics), &key));
        assert_eq!(rule.mode, Mode::Direct);
        assert!(rule.premium);
        item_cosmetics::qualify_rule(catalogue, rule, &cosmetics).map_err(super::debug)?;
        let [choice] = rule.choices.as_slice() else {
            return Err("item must have a single deterministic choice".into());
        };
        assert_eq!(choice.grants, [choice.owned.as_str()]);
        let grant = item_cosmetics::resolve(catalogue, &cosmetics, &choice.owned)
            .map_err(super::debug)?;
        assert_eq!((grant.cosmetic_key.as_str(), grant.addon), (outfit.as_str(), addon));
        if addon == 0 {
            assert!(choice.required.is_empty());
        } else {
            let [base] = choice.required.as_slice() else {
                return Err("addon must require its exact base outfit".into());
            };
            let base = item_cosmetics::resolve(catalogue, &cosmetics, base)
                .map_err(super::debug)?;
            assert_eq!((base.cosmetic_key, base.addon), (outfit, 0));
        }
    }
    Ok(())
}
