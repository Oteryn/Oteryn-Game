use oteryn_gameplay_quests::quest::{
    loader::load_embedded_quest_state, reward_callbacks, world_callbacks,
};

#[test]
fn tibia_tales_appease_registers_both_envoys_and_the_earned_reward() -> Result<(), String> {
    let loaded = load_embedded_quest_state("tibia-tales").map_err(|error| format!("{error:?}"))?;
    let callbacks = reward_callbacks::offered_for(loaded.catalogue())?;
    for (slug, npc, before, after) in [
        ("accept", "kazzan", 0, 1),
        ("marid", "umar", 1, 2),
        ("efreet", "ubaid", 2, 3),
        ("report", "kazzan", 3, 4),
    ] {
        let key = format!("oteryn:quest-reward/tibia-tales/appease-{slug}");
        let callback = callbacks.iter().find(|row| row.key == key).ok_or(key)?;
        assert_eq!(callback.npc, format!("oteryn:npc.{npc}"));
        let state = callback.storage.first().ok_or("appease stage")?;
        assert_eq!(
            (state.minimum, state.maximum, state.set),
            (before, before, after)
        );
        if slug == "report" {
            let reward = callback.give.as_ref().ok_or("appease reward")?;
            assert_eq!(
                (reward.item.as_str(), reward.count),
                ("oteryn:item.tibia.i3035", 20)
            );
        }
        assert!(
            !loaded
                .catalogue()
                .transition(&callback.key)
                .ok_or("boot transition")?
                .completes
        );
    }
    Ok(())
}

#[test]
fn tibia_tales_ultimate_booze_uses_the_brewery_cask_and_rewards_the_sample() -> Result<(), String> {
    let loaded = load_embedded_quest_state("tibia-tales").map_err(|error| format!("{error:?}"))?;
    let world = world_callbacks::load_for(loaded.catalogue())?;
    let brew = world
        .iter()
        .find(|row| row.key.ends_with("/tibia-tales/fill-ale-bottle"))
        .ok_or("brewery cask callback")?;
    assert_eq!(brew.selector.appearance, Some(5538));
    let tool = brew.item_on_target.as_ref().ok_or("beer bottle")?;
    assert_eq!(tool.item, "oteryn:item.tibia.i138");
    assert_eq!(tool.replacement.as_deref(), Some("oteryn:item.tibia.i136"));
    assert_eq!((brew.storage[0].minimum, brew.storage[0].set), (1, 2));
    let rewards = reward_callbacks::offered_for(loaded.catalogue())?;
    let report = rewards
        .iter()
        .find(|row| row.key.ends_with("/tibia-tales/booze-report"))
        .ok_or("Boozer mission report")?;
    assert_eq!((report.storage[0].minimum, report.storage[0].set), (2, 3));
    assert_eq!(report.take[0].item, "oteryn:item.tibia.i136");
    let reward = report.give.as_ref().ok_or("Boozer reward")?;
    assert_eq!(
        (reward.item.as_str(), reward.count),
        ("oteryn:item.tibia.i5710", 1)
    );
    assert_eq!(
        (
            reward.additional[0].item.as_str(),
            reward.additional[0].count
        ),
        ("oteryn:item.tibia.i3035", 10)
    );
    assert_eq!(report.experience, Some(100));
    Ok(())
}

#[test]
fn tibia_tales_outfox_claim_requires_acceptance_and_report_exchanges_the_helmet()
-> Result<(), String> {
    let loaded = load_embedded_quest_state("tibia-tales").map_err(|error| format!("{error:?}"))?;
    let world = world_callbacks::load_for(loaded.catalogue())?;
    let chest = world
        .iter()
        .find(|row| row.key.ends_with("/tibia-tales/outfox-helmet"))
        .ok_or("helmet chest")?;
    assert_eq!(
        chest.selector.position,
        Some(world_callbacks::Position {
            x: 32467,
            y: 31970,
            floor: -5
        })
    );
    assert_eq!(chest.selector.unique, Some(6257));
    assert_eq!(
        chest.item_reward().as_ref().ok_or("helmet")?.item,
        "oteryn:item.tibia.i139"
    );
    let rewards = reward_callbacks::offered_for(loaded.catalogue())?;
    let report = rewards
        .iter()
        .find(|row| row.key.ends_with("/outfox-helmet"))
        .ok_or("Budrik report")?;
    assert_eq!((report.storage[0].minimum, report.storage[0].set), (1, 2));
    assert_eq!(report.take[0].item, "oteryn:item.tibia.i139");
    assert_eq!(
        report.give.as_ref().ok_or("spare helmet")?.item,
        "oteryn:item.tibia.i875"
    );
    Ok(())
}

#[test]
fn tibia_tales_hallowed_graves_preserve_distinct_credit_and_empty_the_final_flask()
-> Result<(), String> {
    let loaded = load_embedded_quest_state("tibia-tales").map_err(|error| format!("{error:?}"))?;
    let world = world_callbacks::load_for(loaded.catalogue())?;
    for action in 4208..=4223 {
        let key = format!("oteryn:quest-callback/tibia-tales/sanctify-grave-{action}");
        let ordinary = world.iter().find(|row| row.key == key).ok_or(key.clone())?;
        let final_grave = world
            .iter()
            .find(|row| row.key == format!("{key}-finish"))
            .ok_or("last grave phase")?;
        assert_eq!(ordinary.selector, final_grave.selector);
        assert_eq!(ordinary.selector.action, Some(action));
        assert!(
            ordinary
                .item_on_target
                .as_ref()
                .ok_or("holy water")?
                .reusable
        );
        assert_eq!(
            final_grave
                .item_on_target
                .as_ref()
                .ok_or("empty flask")?
                .replacement
                .as_deref(),
            Some("oteryn:item.tibia.i2874")
        );
        assert_eq!(
            (ordinary.storage[0].minimum, ordinary.storage[0].set),
            (0, 1)
        );
        assert!(
            ordinary.storage[0]
                .track
                .ends_with(&format!("/grave{}", action - 4207))
        );
        assert_eq!(
            (final_grave.storage[2].minimum, final_grave.storage[2].set),
            (3, 4)
        );
    }
    let rewards = reward_callbacks::offered_for(loaded.catalogue())?;
    let report = rewards
        .iter()
        .find(|row| row.key.ends_with("/hallowed-report"))
        .ok_or("Amanda reward")?;
    assert_eq!((report.storage[0].minimum, report.storage[0].set), (4, 5));
    let reward = report.give.as_ref().ok_or("potions")?;
    assert_eq!(
        (reward.item.as_str(), reward.count),
        ("oteryn:item.tibia.i268", 5)
    );
    assert_eq!(
        (
            reward.additional[0].item.as_str(),
            reward.additional[0].count
        ),
        ("oteryn:item.tibia.i266", 5)
    );
    Ok(())
}

#[test]
fn tibia_tales_exterminator_binds_both_source_pool_targets_and_four_gems() -> Result<(), String> {
    let loaded = load_embedded_quest_state("tibia-tales").map_err(|error| format!("{error:?}"))?;
    let world = world_callbacks::load_for(loaded.catalogue())?;
    for y in 31821..=31822 {
        let key = format!("oteryn:quest-callback/tibia-tales/poison-slime-pool-{y}");
        let pool = world.iter().find(|row| row.key == key).ok_or(key)?;
        assert_eq!(
            pool.selector.position,
            Some(world_callbacks::Position {
                x: 32306,
                y,
                floor: -9
            })
        );
        assert_eq!(pool.selector.action, Some(4207));
        let flask = pool.item_on_target.as_ref().ok_or("vermin poison")?;
        assert_eq!(flask.item, "oteryn:item.tibia.i135");
        assert_eq!(
            flask.replacement.as_deref(),
            Some("oteryn:item.tibia.i2874")
        );
        assert_eq!((pool.storage[0].minimum, pool.storage[0].set), (1, 2));
    }
    let rewards = reward_callbacks::offered_for(loaded.catalogue())?;
    let report = rewards
        .iter()
        .find(|row| row.key.ends_with("/exterminator-report"))
        .ok_or("Padreia reward")?;
    assert_eq!((report.storage[0].minimum, report.storage[0].set), (2, 3));
    let reward = report.give.as_ref().ok_or("gems")?;
    assert_eq!(reward.item, "oteryn:item.tibia.i3033");
    let gems: Vec<_> = reward
        .additional
        .iter()
        .map(|item| (item.item.as_str(), item.count))
        .collect();
    assert_eq!(
        gems,
        [
            ("oteryn:item.tibia.i3032", 1),
            ("oteryn:item.tibia.i3030", 1),
            ("oteryn:item.tibia.i3029", 1)
        ]
    );
    Ok(())
}

#[test]
fn tibia_tales_cursed_crystal_loads_all_four_source_stairs() -> Result<(), String> {
    let loaded = load_embedded_quest_state("tibia-tales").map_err(|error| format!("{error:?}"))?;
    let world = world_callbacks::load_for(loaded.catalogue())?;
    for (index, source, destination) in [
        (29, (31973, 32905, -11), (31971, 32904, -10)),
        (31, (31973, 32905, -10), (31975, 32911, -9)),
        (33, (32009, 32928, -9), (32009, 32929, -10)),
        (35, (32009, 32928, -10), (32009, 32929, -9)),
    ] {
        let key = format!("oteryn:quest-callback/batch3/the_cursed_crystal/crystal-stairs-{index}");
        let callback = world.iter().find(|row| row.key == key).ok_or(key)?;
        let cell = |(x, y, floor)| Some(world_callbacks::Position { x, y, floor });
        assert_eq!(callback.selector.position, cell(source));
        assert_eq!(callback.selector.action, Some(25018));
        assert_eq!(callback.teleport, cell(destination));
    }
    Ok(())
}

#[test]
fn tibia_tales_lion_signs_require_the_scroll_and_each_credit_once() -> Result<(), String> {
    let loaded = load_embedded_quest_state("tibia-tales").map_err(|error| format!("{error:?}"))?;
    let world = world_callbacks::load_for(loaded.catalogue())?;
    for (animal, x, y) in [
        ("snake", 33095, 32244),
        ("lizard", 33130, 32300),
        ("scorpion", 33109, 32329),
        ("hyena", 33127, 32340),
    ] {
        let key = format!("oteryn:quest-callback/tibia-tales/lion-sign-{animal}");
        let sign = world.iter().find(|row| row.key == key).ok_or(key)?;
        assert_eq!(sign.trigger, world_callbacks::Trigger::StepIn);
        assert_eq!(
            sign.selector.position,
            Some(world_callbacks::Position { x, y, floor: -9 })
        );
        assert_eq!(sign.present[0].item, "oteryn:item.tibia.i21467");
        assert_eq!(sign.present[0].count, 1);
        assert_eq!(sign.present.len(), 1);
        let state = &sign.storage[0];
        assert!(state.track.ends_with(&format!("/{animal}_sign")));
        assert_eq!((state.minimum, state.maximum, state.set), (0, 0, 1));
        assert_eq!(sign.storage[1].set, 1);
        assert!(sign.storage[1].increment);
    }
    Ok(())
}
