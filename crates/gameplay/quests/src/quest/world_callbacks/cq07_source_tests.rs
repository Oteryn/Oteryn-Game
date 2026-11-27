use super::{Callback, Position, Trigger, load_for};
use crate::quest::loader::load_embedded_quest_state;

fn realm() -> Result<Vec<Callback>, Box<dyn std::error::Error>> {
    let state = load_embedded_quest_state("content-1").map_err(|error| format!("{error:?}"))?;
    Ok(load_for(state.catalogue())?
        .into_iter()
        .filter(|callback| callback.quest == "oteryn:quest.chayenne_realm")
        .collect())
}

#[test]
fn realm_loader_binds_source_portals_and_retains_entry_key()
-> Result<(), Box<dyn std::error::Error>> {
    let callbacks = realm()?;
    let entry = callbacks
        .iter()
        .find(|c| c.selector.action == Some(55022))
        .ok_or("AID 55022")?;
    assert_eq!(entry.trigger, Trigger::StepIn);
    assert_eq!(entry.minimum_level, Some(40));
    assert_eq!(entry.present[0].item, "oteryn:item.tibia.i14682");
    assert!(entry.item_on_target.is_none());
    assert!(entry.return_on_failure);
    assert_eq!(
        entry.teleport,
        Some(Position {
            x: 32829,
            y: 31451,
            floor: -8
        })
    );
    let exit = callbacks
        .iter()
        .find(|c| c.selector.action == Some(55024))
        .ok_or("AID 55024")?;
    assert_eq!(exit.trigger, Trigger::StepIn);
    assert!(!exit.requires_durable_commit());
    assert_eq!(
        exit.teleport,
        Some(Position {
            x: 33117,
            y: 32604,
            floor: -6
        })
    );
    Ok(())
}

#[test]
fn realm_loader_admits_draconia_book_and_complete_beach_backpack()
-> Result<(), Box<dyn std::error::Error>> {
    let callbacks = realm()?;
    let book = callbacks
        .iter()
        .find(|c| c.key.ends_with("/realm-draconia-book"))
        .ok_or("Draconia book")?;
    let book_reward = book.item_reward().as_ref().ok_or("book reward")?;
    assert_eq!(book_reward.item, "oteryn:item.tibia.i6533");
    assert!(
        book_reward
            .text
            .as_ref()
            .and_then(|text| text.root())
            .is_some()
    );
    assert_eq!(book.storage[0].maximum, 0);
    let corpse = callbacks
        .iter()
        .find(|c| c.selector.action == Some(55023))
        .ok_or("AID 55023")?;
    let reward = corpse.item_reward().as_ref().ok_or("Beach Backpack")?;
    assert_eq!(reward.item, "oteryn:item.tibia.i5949");
    let children: Vec<_> = reward
        .contents
        .iter()
        .map(|child| (child.item.as_str(), child.count))
        .collect();
    assert_eq!(
        children,
        [16244, 3659, 9034, 3027, 5882, 5791, 2995, 6570]
            .map(|item| (format!("oteryn:item.tibia.i{item}"), 1))
            .each_ref()
            .map(|(key, count)| (key.as_str(), *count))
    );
    assert_eq!(corpse.storage[0].maximum, 0);
    assert!(corpse.effects.completes_quest);
    Ok(())
}

#[test]
fn realm_loader_binds_lever_to_independent_wall_and_switch_deadlines()
-> Result<(), Box<dyn std::error::Error>> {
    let callbacks = realm()?;
    let lever = callbacks
        .iter()
        .find(|c| c.selector.action == Some(55021))
        .ok_or("AID 55021")?;
    assert_eq!(lever.trigger, Trigger::Use);
    assert_eq!(lever.present[0].item, "oteryn:item.tibia.i14682");
    assert_eq!(lever.lever, Some([2772, 2773]));
    assert!(
        lever.reversible,
        "active source lever must accept its return use"
    );
    assert_eq!(lever.effects.passages_reset_after_ms, Some(60_000));
    assert_eq!(lever.effects.reset_after_ms, Some(300_000));
    assert_eq!(lever.passages[0].appearance, 2129);
    Ok(())
}

#[test]
fn children_loader_connects_zalamon_to_native_mission_sources()
-> Result<(), Box<dyn std::error::Error>> {
    let state = load_embedded_quest_state("content-1").map_err(|error| format!("{error:?}"))?;
    use crate::quest::reward_callbacks;
    let npc = reward_callbacks::load_for(state.catalogue())?;
    let offered = reward_callbacks::offered_for(state.catalogue())?;
    let quest = "oteryn:quest.children_of_the_revolution_quest";
    for (name, from, to) in [
        ("accept", 0, 1),
        ("map", 2, 3),
        ("temple-report", 5, 6),
        ("spy-report", 7, 8),
        ("poison-report", 11, 12),
        ("phantom-accept", 18, 19),
        ("finish", 20, 21),
    ] {
        let key = format!("oteryn:quest-reward/l2-zalamon-{name}");
        let callback = npc
            .iter()
            .find(|c| c.key == key && c.quest == quest)
            .ok_or(key)?;
        let route = callback
            .storage
            .iter()
            .find(|s| s.track.ends_with("/questline"))
            .ok_or("mission phase")?;
        assert_eq!((route.minimum, route.maximum, route.set), (from, from, to));
        if matches!(name, "phantom-accept" | "finish") {
            assert!(!offered.iter().any(|row| row.key == callback.key));
            assert!(state.catalogue().transition(&callback.key).is_none());
        }
        if name == "finish" {
            assert_eq!(callback.experience, Some(10_000));
            assert_eq!(
                callback.give.as_ref().ok_or("Serpent Crest")?.item,
                "oteryn:item.tibia.i10199"
            );
            assert!(callback.completes);
        }
    }
    let world = load_for(state.catalogue())?;
    for uid in [3162, 3163, 3164] {
        assert!(
            world
                .iter()
                .any(|c| c.quest == quest && c.selector.unique == Some(uid)),
            "UID {uid}"
        );
    }
    for aid in 8009..=8013 {
        assert!(
            world
                .iter()
                .any(|c| c.quest == quest && c.selector.action == Some(aid)),
            "AID {aid}"
        );
    }
    Ok(())
}

#[test]
fn crystal_wand_loader_binds_twinkiller_parchment() -> Result<(), Box<dyn std::error::Error>> {
    let callbacks = super::load()?;
    let book = callbacks
        .iter()
        .find(|c| c.key == "oteryn:quest-callback/crystal-wand-twinkiller")
        .ok_or("Twinkiller Rune")?;
    let reward = book.item_reward().as_ref().ok_or("written parchment")?;
    assert_eq!(reward.item, "oteryn:item.tibia.i2835");
    assert!(
        reward
            .text
            .as_ref()
            .and_then(|text| text.root())
            .is_some_and(|text| text.contains("Twinkiller"))
    );
    assert_eq!(
        book.selector.position,
        Some(Position {
            x: 32478,
            y: 31611,
            floor: -15
        })
    );
    assert_eq!(book.storage[0].maximum, 0);
    Ok(())
}

#[test]
fn children_loader_requires_the_selected_poison_and_grease() -> Result<(), String> {
    let callbacks = super::load()?;
    for (suffix, item) in [("grease", 10189), ("poison-water", 10183)] {
        let key = format!("oteryn:quest-callback/children-{suffix}");
        let callback = callbacks.iter().find(|c| c.key == key).ok_or(&key)?;
        let tool = callback.item_on_target.as_ref().ok_or("mission item")?;
        assert_eq!(tool.item, format!("oteryn:item.tibia.i{item}"));
        assert!(tool.selected_instance, "{key} accepts bare USE");
    }
    Ok(())
}

#[test]
fn cave_explorer_loader_admits_donor_level_thresholds_and_completion_achievement()
-> Result<(), String> {
    let callbacks = crate::quest::reward_callbacks::load()?;
    for (suffix, level) in [("base", 25), ("first", 50), ("second", 80)] {
        let key = format!("oteryn:quest-reward/l12-cave-explorer-{suffix}");
        let callback = callbacks.iter().find(|c| c.key == key).ok_or(key)?;
        assert_eq!(callback.minimum_level, Some(level));
        assert_eq!(
            callback.participants[0].state_ops[0].track,
            "oteryn:quest-progress/spike/fame"
        );
        assert_eq!(
            callback.participants[0].state_ops[0].from,
            crate::quest::reward_callbacks::StateCondition::Between {
                minimum: 100,
                maximum: 2_147_483_647,
            }
        );
        assert_eq!(
            callback.participants[0].state_ops[0].effect,
            crate::quest::reward_callbacks::StateMutation::Add { value: -100 }
        );
        if suffix != "base" {
            assert_eq!(
                callback.achievement_progress[0].key,
                "oteryn:achievement/cave_completionist"
            );
            assert_eq!(callback.achievement_progress[0].target, 2);
        }
    }
    Ok(())
}

#[test]
fn cave_explorer_loader_can_settle_a_pre_upgrade_completed_trial() -> Result<(), String> {
    let state = load_embedded_quest_state("content-1").map_err(|error| format!("{error:?}"))?;
    let catalogue = state.catalogue();
    let callbacks = crate::quest::reward_callbacks::offered_for(catalogue)?;
    let legacy = callbacks
        .iter()
        .find(|callback| {
            callback.npc == "oteryn:npc.gnommander"
                && callback
                    .keywords
                    .iter()
                    .any(|word| word == "cave explorer report")
        })
        .ok_or("the published Cave Explorer report has no callback")?;
    let transition = catalogue.transition(&legacy.key).ok_or("legacy report")?;
    let progress = std::collections::BTreeMap::from([
        (
            "oteryn:quest-progress/chosen-trial/cave_explorer/phase".into(),
            1,
        ),
        (
            "oteryn:quest-progress/chosen-trial/cave_explorer/kills".into(),
            20,
        ),
    ]);
    assert!(catalogue.evaluate(transition, &progress, 0).is_ok());
    assert_eq!(legacy.outfit.as_ref().ok_or("base outfit")?.addons, 0);
    Ok(())
}
