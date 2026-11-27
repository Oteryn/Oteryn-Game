use std::collections::BTreeMap;

use oteryn_gameplay_quests::quest::{
    QuestRefusal,
    loader::load_embedded_quest_state,
    reward_callbacks,
    world_callbacks::{self, Position, Trigger},
};

#[test]
fn paradox_carvings_and_return_portals_use_crystal_landings() -> Result<(), String> {
    let callbacks = world_callbacks::load()?;
    for (uid, arrival) in [
        (25031, (32478, 31908, -7)),
        (25032, (32478, 31908, -7)),
        (25034, (32566, 31959, -1)),
        (25035, (32566, 31959, -1)),
        (25036, (32566, 31959, -1)),
        (25037, (32566, 31959, -1)),
        (38018, (32476, 31904, -6)),
        (38019, (32481, 31905, -5)),
        (38020, (32476, 31904, -4)),
        (38021, (32479, 31904, -3)),
        (38022, (32480, 31905, -2)),
    ] {
        let route = callbacks
            .iter()
            .find(|callback| {
                callback.quest == "oteryn:quest.the_paradox_tower_quest"
                    && callback.selector.unique == Some(uid)
            })
            .ok_or_else(|| format!("missing Paradox route {uid}"))?;
        assert_eq!(route.trigger, Trigger::StepIn);
        assert_eq!(
            route.teleport,
            Some(Position {
                x: arrival.0,
                y: arrival.1,
                floor: arrival.2,
            })
        );
        assert!(route.storage.is_empty(), "return routes are repeatable");
    }
    Ok(())
}

#[test]
fn banshees_true_path_refuses_early_and_unlocks_the_fifth_seal() -> Result<(), String> {
    let loaded = load_embedded_quest_state("banshees-true-path-test")
        .map_err(|error| format!("{error:?}"))?;
    let catalogue = loaded.catalogue();
    let path = "oteryn:quest-progress/quest/u7_2/the_queen_of_the_banshees/fifth_seal_tile";
    let reset = catalogue
        .transition("oteryn:quest-callback/banshees-true-path-reset")
        .ok_or("path reset")?;
    let advance = catalogue
        .transition("oteryn:quest-callback/banshees-true-path-advance")
        .ok_or("path advance")?;
    assert_eq!(
        catalogue.evaluate(advance, &BTreeMap::new(), 0),
        Err(QuestRefusal::StageMismatch)
    );
    let mut path_state = BTreeMap::new();
    for transition in std::iter::once(reset).chain(std::iter::repeat_n(advance, 8)) {
        for change in catalogue
            .evaluate(transition, &path_state, 0)
            .map_err(|error| format!("{error:?}"))?
        {
            path_state.insert(change.track, change.after);
        }
    }
    assert_eq!(path_state.get(path), Some(&9));
    let flame = catalogue
        .transition("oteryn:quest-callback/banshees-fifth-seal")
        .ok_or("fifth flame")?;
    assert_eq!(
        catalogue.evaluate(flame, &BTreeMap::from([(path.into(), 8)]), 0),
        Err(QuestRefusal::StageMismatch)
    );
    let result = catalogue
        .evaluate(flame, &path_state, 0)
        .map_err(|error| format!("{error:?}"))?;
    for suffix in ["fifth_seal", "fifth_seal_door", "fifth_seal_tile"] {
        let key = format!("oteryn:quest-progress/quest/u7_2/the_queen_of_the_banshees/{suffix}");
        assert!(
            result
                .iter()
                .any(|change| change.track == key && change.after == 1)
        );
    }
    Ok(())
}

#[test]
fn banshees_logic_flame_requires_the_crystal_campfire_pattern() -> Result<(), String> {
    let callbacks = world_callbacks::load()?;
    let flame = callbacks
        .iter()
        .find(|callback| callback.key == "oteryn:quest-callback/banshees-sixth-seal")
        .ok_or("sixth flame")?;
    assert_eq!(flame.selector.unique, Some(35018));
    assert_eq!(
        flame
            .world_requirements
            .iter()
            .map(|entry| (entry.position.x, entry.position.y, entry.appearance))
            .collect::<Vec<_>>(),
        [
            (32309, 31975, 1996),
            (32311, 31975, 1996),
            (32313, 31975, 1998),
            (32309, 31976, 1996),
            (32311, 31976, 1996),
            (32313, 31976, 1998),
        ]
    );
    Ok(())
}

#[test]
fn paradox_contacts_publish_the_state_that_the_riddler_reads() -> Result<(), String> {
    let callbacks = reward_callbacks::load()?;
    for (key, track, value) in [
        ("l6-paradox-lubo", "the_feared_hugo", 4),
        ("l6-paradox-prisoner-lesson", "mathemagics", 1),
    ] {
        let callback = callbacks
            .iter()
            .find(|callback| callback.key == format!("oteryn:quest-reward/{key}"))
            .ok_or_else(|| format!("missing {key}"))?;
        let track = format!("oteryn:quest-progress/quest/u7_24/the_paradox_tower/{track}");
        assert!(
            callback
                .storage
                .iter()
                .any(|effect| effect.track == track && effect.set == value),
            "{key} leaves the Riddler guard unsatisfied"
        );
    }
    Ok(())
}

#[test]
fn paradox_claims_require_the_riddler_and_refuse_a_forfeited_reward() -> Result<(), String> {
    let loaded =
        load_embedded_quest_state("paradox-claim-test").map_err(|error| format!("{error:?}"))?;
    let catalogue = loaded.catalogue();
    let prefix = "oteryn:quest-progress/quest/u7_24/the_paradox_tower/";
    for reward in ["egg", "gold", "talon", "wand"] {
        let math = format!("{prefix}mathemagics");
        let available = format!("{prefix}reward/{reward}");
        let claim = catalogue
            .transition(&format!("oteryn:quest-callback/paradox-claim-{reward}"))
            .ok_or_else(|| format!("unbound Paradox chest {reward}"))?;
        assert_eq!(
            catalogue.evaluate(claim, &BTreeMap::from([(math.clone(), 1)]), 0),
            Err(QuestRefusal::StageMismatch)
        );
        let forfeit = catalogue
            .transition(&format!("oteryn:quest-callback/paradox-forfeit-{reward}"))
            .ok_or("reward plate")?;
        let mut state = BTreeMap::from([(math.clone(), 2)]);
        for change in catalogue
            .evaluate(forfeit, &state, 0)
            .map_err(|error| format!("{error:?}"))?
        {
            state.insert(change.track, change.after);
        }
        assert_eq!(
            catalogue.evaluate(claim, &state, 0),
            Err(QuestRefusal::StageMismatch)
        );
        let awarded = catalogue
            .evaluate(claim, &BTreeMap::from([(math, 2)]), 0)
            .map_err(|error| format!("{error:?}"))?;
        assert!(
            awarded
                .iter()
                .any(|change| change.track == available && change.after == 1)
        );
        assert!(claim.completes);
        let callbacks = world_callbacks::load()?;
        let source = callbacks
            .iter()
            .find(|callback| callback.key == claim.key)
            .ok_or("native Paradox cache")?;
        let identity = world_callbacks::legacy_world_claim(source)?.ok_or("once claim identity")?;
        assert_eq!(
            identity.key,
            format!("oteryn:reward-claim.quest.u7_24.the_paradox_tower.reward.{reward}")
        );
    }
    Ok(())
}

#[test]
fn spirit_wills_use_the_souleater_mountain_pickups() -> Result<(), String> {
    let callbacks = world_callbacks::load()?;
    for (number, x, y, floor, appearance) in [
        (1, 33177, 31021, -5, 2473),
        (2, 33161, 31056, -4, 2469),
        (3, 33166, 31056, -6, 2472),
    ] {
        let callback = callbacks
            .iter()
            .find(|row| row.key == format!("oteryn:quest-callback/l6/spirit-will-{number}"))
            .ok_or("Spirit will pickup")?;
        assert_eq!(callback.selector.position, Some(Position { x, y, floor }));
        assert_eq!(callback.selector.appearance, Some(appearance));
        assert_eq!(
            callback.item_reward().as_ref().ok_or("will reward")?.item,
            "oteryn:item.tibia.i11544"
        );
    }
    Ok(())
}

#[test]
fn spirit_completion_enables_both_wayfarer_hand_ins() -> Result<(), String> {
    let callbacks = reward_callbacks::load()?;
    for addon in ["cape", "hat"] {
        let callback = callbacks
            .iter()
            .find(|row| row.key == format!("oteryn:quest-reward/l1-wayfarer-{addon}"))
            .ok_or("Wayfarer hand-in")?;
        assert_eq!(
            callback.requires_completed_quests,
            ["oteryn:quest.the_spirit_will_get_you_quest"]
        );
        assert_eq!(callback.take.len(), 1);
        assert!(callback.outfit.is_some());
    }
    Ok(())
}

#[test]
fn classic_quest_routes_keep_exact_base_save_predecessors() -> Result<(), String> {
    use oteryn_gameplay_quests::quest::npc_rules::legacy_defaults;

    let loaded = load_embedded_quest_state("classic-quest-predecessors")
        .map_err(|error| format!("{error:?}"))?;
    let active = loaded.catalogue();
    for (slug, previous) in [
        (
            "the_paradox_tower_quest",
            "4fc6ffa2b309728ed9ee0e76381c1e6d20d618389b2e74bdd5b773cf108bdec5",
        ),
        (
            "the_queen_of_the_banshees_quest",
            "678a96577e5535714661dd42a36a1fe44de1eafa146415a9d644d9aa96ede1b3",
        ),
    ] {
        let key = format!("oteryn:quest.{slug}");
        let mut hash = [0; 32];
        for (index, byte) in hash.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&previous[index * 2..index * 2 + 2], 16)
                .map_err(|error| error.to_string())?;
        }
        assert!(legacy_defaults(active, &key, &hash).is_some(), "{key}");
        hash[0] ^= 1;
        assert!(legacy_defaults(active, &key, &hash).is_none());
    }
    Ok(())
}
