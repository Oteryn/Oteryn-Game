use super::*;
use crate::quest::{QuestComparison, reward_callbacks};

#[test]
fn dread_lord_lever_creates_the_crystal_output_on_the_offering_tile() -> Result<(), String> {
    let loaded = crate::quest::loader::load_embedded_quest_state("dread-lord-lever")
        .map_err(|error| format!("{error:?}"))?;
    let callbacks = load_for(loaded.catalogue())?;
    let lever = callbacks
        .iter()
        .find(|callback| callback.key == "oteryn:quest-callback/dreamer-dread-lord-scroll")
        .ok_or("Dread Lord lever")?;
    let [offering] = lever.offerings.as_slice() else {
        return Err("diamond offering".into());
    };
    assert_eq!(offering.count, 1);
    let scene = lever.effects.scene.as_ref().ok_or("lever scene")?;
    let [output] = scene.open.as_slice() else {
        return Err("one ground output".into());
    };
    assert_eq!(
        (output.position, output.create, output.passable),
        (offering.position, Some(7845), Some(true))
    );
    assert!(lever.item_reward().is_none());
    assert!(lever.effects.item_presence.is_none());
    assert_eq!(lever.effects.reset_after_ms, Some(4_000));
    assert!(loaded.catalogue().transition(&lever.key).is_some());
    Ok(())
}

#[test]
fn ajax_axe_reward_loads_with_brutal_politeness() -> Result<(), String> {
    let rewards = reward_callbacks::load()?;
    let axe = rewards
        .iter()
        .find(|callback| callback.key == "oteryn:quest-reward/qb-barbarian-ajax-axe")
        .ok_or("Ajax axe reward")?;
    assert_eq!(
        axe.achievement.as_deref(),
        Some("oteryn:achievement/brutal_politeness")
    );
    assert_eq!(
        axe.outfit
            .as_ref()
            .map(|outfit| (outfit.outfit.as_str(), outfit.addons)),
        Some(("oteryn:outfit.barbarian", 1))
    );
    Ok(())
}

#[test]
fn kroox_report_loads_both_donor_keywords() -> Result<(), String> {
    let rewards = reward_callbacks::load()?;
    let report = rewards
        .iter()
        .find(|callback| callback.key == "oteryn:quest-reward/qd-sams-old-backpack-kroox")
        .ok_or("Kroox report")?;
    assert_eq!(report.keywords, ["sam sent me", "sam send me"]);
    Ok(())
}

#[test]
fn hunter_glove_contacts_share_the_addon_and_allow_later_sales() -> Result<(), String> {
    let rewards = reward_callbacks::load()?;
    for (npc, addon, sale) in [
        (
            "elane",
            "l8-hunter-elane-gloves",
            "l8-hunter-elane-gloves-sale",
        ),
        ("legola", "l8-hunter-gloves", "l8-hunter-gloves-sale"),
    ] {
        let find = |suffix: &str| {
            rewards
                .iter()
                .find(|row| row.key.strip_prefix("oteryn:quest-reward/") == Some(suffix))
                .ok_or_else(|| suffix.to_owned())
        };
        let addon = find(addon)?;
        let sale = find(sale)?;
        assert_eq!(addon.npc, format!("oteryn:npc.{npc}"));
        assert_eq!(sale.npc, addon.npc);
        assert_eq!(sale.topic, addon.topic);
        assert!(addon.requires_premium);
        assert_eq!(addon.take[0].item, "oteryn:item.tibia.i5875");
        assert_eq!(sale.take, addon.take);
        assert_eq!(addon.outfits.len(), 2);
        assert_eq!((addon.outfits[0].addons, addon.outfits[1].addons), (2, 1));
        assert_eq!(sale.storage[0].track, addon.storage[0].track);
        assert_eq!((sale.storage[0].minimum, sale.storage[0].maximum), (1, 1));
        assert!(sale.storage[0].preserve && sale.repeatable);
        let payment = sale.give.as_ref().ok_or("glove payment")?;
        assert_eq!(
            (payment.item.as_str(), payment.count),
            ("oteryn:item.tibia.i3035", 20)
        );
    }
    Ok(())
}

#[test]
fn hunter_contacts_load_a_free_account_sale_before_the_addon() -> Result<(), String> {
    let loaded = crate::quest::loader::load_embedded_quest_state("content-1")
        .map_err(|error| format!("{error:?}"))?;
    let catalogue = loaded.catalogue();
    let rewards = reward_callbacks::offered_for(catalogue)?;
    for npc in ["oteryn:npc.elane", "oteryn:npc.legola"] {
        let callback = rewards
            .iter()
            .find(|row| {
                row.npc == npc
                    && !row.requires_premium
                    && row.repeatable
                    && row.keywords.iter().any(|word| word == "yes")
                    && row
                        .take
                        .iter()
                        .any(|item| item.item == "oteryn:item.tibia.i5875")
                    && row.storage.iter().all(|guard| {
                        catalogue.track(&guard.track).is_some_and(|track| {
                            (guard.minimum..=guard.maximum).contains(&track.initial)
                        })
                    })
            })
            .ok_or_else(|| format!("{npc} has no free-account sale before the addon"))?;
        let payment = callback.give.as_ref().ok_or("glove payment")?;
        assert_eq!(
            (payment.item.as_str(), payment.count),
            ("oteryn:item.tibia.i3035", 20)
        );
    }
    Ok(())
}

fn assert_native_source(callback: &Callback, position: Position, ordinal: u8, appearance: u16) {
    assert_eq!(callback.selector.position, None);
    assert_eq!(callback.selector.appearance, None);
    assert_eq!(
        callback.selector.native,
        Some(NativeSelector {
            position,
            ordinal,
            appearance,
        })
    );
}

#[test]
fn druid_waterskin_and_wolf_tooth_chain_load_as_exact_world_actions() -> Result<(), String> {
    let lowered = crate::quest::loader::load_embedded_quest_state("content-1")
        .map_err(|error| format!("{error:?}"))?;
    let catalogue = lowered.catalogue();
    let worlds = load_for(catalogue)?;
    let water = worlds
        .iter()
        .find(|c| c.key == "oteryn:quest-callback/druid-ceiron-waterskin")
        .ok_or("Ceiron's waterskin")?;
    assert_native_source(
        water,
        Position {
            x: 33024,
            y: 32672,
            floor: -6,
        },
        2,
        5662,
    );
    let input = water.item_on_target.as_ref().ok_or("selected waterskin")?;
    assert!(input.selected_instance);
    assert_eq!(input.item, "oteryn:item.tibia.i5938");
    assert_eq!(
        input.replacement.as_deref(),
        Some("oteryn:item.tibia.i5939")
    );
    let progress = catalogue
        .transition(&water.key)
        .ok_or("waterskin transition")?;
    assert_eq!(progress.effects.len(), 1);
    assert_eq!(progress.effects[0].from, QuestComparison::Between(-1, 0));

    let chain = worlds
        .iter()
        .find(|c| c.key == "oteryn:quest-callback/druid-ceiron-chain-chest")
        .ok_or("Ceiron's wolf tooth chain chest")?;
    assert_native_source(
        chain,
        Position {
            x: 32939,
            y: 31776,
            floor: -9,
        },
        1,
        2480,
    );
    let reward = chain
        .effects
        .item_reward
        .as_ref()
        .ok_or("wolf tooth chain reward")?;
    assert_eq!(reward.item, "oteryn:item.tibia.i5940");
    assert_eq!(reward.count, 1);
    assert_eq!(
        catalogue
            .transition(&chain.key)
            .ok_or("chain transition")?
            .effects[0]
            .from,
        QuestComparison::Between(-1, 0)
    );
    Ok(())
}

#[test]
fn hunter_hymn_items_piano_and_crossbow_load_as_source_actions() -> Result<(), String> {
    let lowered = crate::quest::loader::load_embedded_quest_state("content-1")
        .map_err(|error| format!("{error:?}"))?;
    let catalogue = lowered.catalogue();
    let rewards = reward_callbacks::load_for(catalogue)?;
    let worlds = load_for(catalogue)?;
    let piano = worlds
        .iter()
        .find(|c| c.key == "oteryn:quest-callback/hunter-hymn-piano")
        .ok_or("Hunter hymn piano")?;
    assert_native_source(
        piano,
        Position {
            x: 32406,
            y: 32794,
            floor: -9,
        },
        1,
        6123,
    );
    assert_eq!(piano.storage.len(), 4);
    assert_eq!(
        piano.teleport,
        Some(Position {
            x: 32402,
            y: 32794,
            floor: -9
        })
    );
    let authored = worlds
        .iter()
        .find(|c| c.key == "oteryn:quest-callback/authored-hunter-hymn-piano")
        .ok_or("authored Hunter hymn piano")?;
    assert_eq!(authored.teleport, piano.teleport);
    for (verse, item) in (6087..=6090).enumerate() {
        let key = format!("oteryn:item.tibia.i{item}");
        for (prefix, allowed, route) in [
            ("hunter-hymn-verse-", 0, piano),
            ("chosen/hunter-outfits/verse-", -1, authored),
        ] {
            let callback_key = format!("oteryn:quest-reward/{prefix}{}", verse + 1);
            let use_ = rewards
                .iter()
                .find(|c| c.key == callback_key)
                .ok_or(callback_key)?;
            assert_eq!(use_.carried_item.as_deref(), Some(key.as_str()));
            assert_eq!(use_.take.len(), 1);
            assert_eq!(use_.take[0].item, key);
            assert_eq!(use_.take[0].count, 1);
            assert_eq!(use_.storage.len(), 1);
            let learned = &use_.storage[0];
            assert_eq!(
                (learned.minimum, learned.maximum, learned.set),
                (-1, allowed, 1)
            );
            let required = &route.storage[verse];
            assert_eq!(required.track, learned.track);
            assert_eq!((required.minimum, required.maximum), (1, 1));
            assert!(required.read_only);
        }
    }
    let chest = worlds
        .iter()
        .find(|c| c.key == "oteryn:quest-callback/hunter-elane-crossbow-chest")
        .ok_or("Elane's crossbow chest")?;
    assert_native_source(
        chest,
        Position {
            x: 32369,
            y: 32795,
            floor: -10,
        },
        1,
        2472,
    );
    let reward = chest
        .effects
        .item_reward
        .as_ref()
        .ok_or("crossbow reward")?;
    assert_eq!(reward.item, "oteryn:item.tibia.i5947");
    assert_eq!(reward.count, 1);
    Ok(())
}
