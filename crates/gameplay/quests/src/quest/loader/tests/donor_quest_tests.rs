use super::*;

#[test]
fn donor_quest_hunter_piano_requires_all_four_independent_verse_receipts() -> Result<(), String> {
    use crate::quest::{reward_callbacks, world_callbacks};

    let loaded = load_embedded_quest_state("hunter-source").map_err(|err| format!("{err:?}"))?;
    let catalogue = loaded.catalogue();
    let rewards = reward_callbacks::load_for(catalogue)?;
    let worlds = world_callbacks::load_for(catalogue)?;
    for (piano_key, verse_prefix) in [
        (
            "oteryn:quest-callback/authored-hunter-hymn-piano",
            "oteryn:quest-reward/chosen/hunter-outfits/verse-",
        ),
        (
            "oteryn:quest-callback/hunter-hymn-piano",
            "oteryn:quest-reward/hunter-hymn-verse-",
        ),
    ] {
        let piano = worlds
            .iter()
            .find(|callback| callback.key == piano_key)
            .ok_or(piano_key)?;
        assert_eq!(piano.trigger, world_callbacks::Trigger::Use);
        let appearance = piano
            .selector
            .native
            .as_ref()
            .map(|native| native.appearance)
            .or(piano.selector.appearance);
        assert_eq!(appearance, Some(6123));
        assert_eq!(
            piano.teleport,
            Some(world_callbacks::Position {
                x: 32402,
                y: 32794,
                floor: -9,
            })
        );
        let mut progress = BTreeMap::new();
        assert!(piano.is_read_only());
        assert_eq!(piano.storage.len(), 4);
        let gate = world_callbacks::primary_transition(piano)?;
        for (source, verse) in [(6089, 3), (6087, 1), (6090, 4), (6088, 2)] {
            assert!(catalogue.evaluate(&gate, &progress, 0).is_err());
            let item = format!("oteryn:item.tibia.i{source}");
            let key = format!("{verse_prefix}{verse}");
            let callback = rewards
                .iter()
                .find(|reward| reward.key == key)
                .ok_or_else(|| format!("missing verse {key}"))?;
            assert_eq!(callback.carried_item.as_deref(), Some(item.as_str()));
            assert_eq!(callback.take.len(), 1);
            assert_eq!(callback.take[0].item, item);
            assert_eq!(callback.take[0].count, 1);
            let transition = catalogue.transition(&key).ok_or("verse transition")?;
            let changes = catalogue
                .evaluate(transition, &progress, 0)
                .map_err(|err| format!("{err:?}"))?;
            assert_eq!(changes.len(), 1);
            progress.insert(changes[0].track.clone(), changes[0].after);
            assert!(catalogue.evaluate(transition, &progress, 0).is_err());
        }
        assert!(
            catalogue.evaluate(&gate, &progress, 0).is_ok(),
            "{piano_key}"
        );
    }
    Ok(())
}

#[test]
fn donor_quest_hunter_elane_and_legola_share_the_sniper_glove_claim() -> Result<(), String> {
    let loaded =
        load_embedded_quest_state("hunter-gloves-source").map_err(|error| format!("{error:?}"))?;
    let catalogue = loaded.catalogue();
    let rewards = crate::quest::reward_callbacks::load_for(catalogue)?;
    let legola = rewards
        .iter()
        .find(|c| c.key.ends_with("l8-hunter-gloves"))
        .ok_or("Legola")?;
    let elane = rewards
        .iter()
        .find(|c| c.key.ends_with("hunter-outfits/elane-gloves"))
        .ok_or("Elane")?;
    assert_eq!(elane.npc, "oteryn:npc.elane");
    assert_eq!(elane.outfits, legola.outfits);
    assert_eq!(elane.take, legola.take);
    assert!(elane.requires_premium && legola.requires_premium);
    let first = catalogue.transition(&elane.key).ok_or("Elane transition")?;
    let other = catalogue
        .transition(&legola.key)
        .ok_or("Legola transition")?;
    for (first, other) in [(first, other), (other, first)] {
        let progress = catalogue
            .evaluate(first, &BTreeMap::new(), 0)
            .map_err(|error| format!("{error:?}"))?
            .into_iter()
            .map(|change| (change.track, change.after))
            .collect();
        assert!(catalogue.evaluate(other, &progress, 0).is_err());
    }
    Ok(())
}

#[test]
fn donor_quest_voodoo_both_box_orders_complete_without_kills() -> Result<(), String> {
    let loaded = load_embedded_quest_state("voodoo-source").map_err(|err| format!("{err:?}"))?;
    let catalogue = loaded.catalogue();
    let callbacks = crate::quest::world_callbacks::load_for(catalogue)?;
    let quest = "oteryn:quest.authored.voodoo_doll_quest";
    let at = |y, progress: &BTreeMap<String, i64>| {
        callbacks
            .iter()
            .filter_map(move |callback| {
                if callback.quest != quest
                    || callback.selector.position.is_none_or(|pos| pos.y != y)
                {
                    return None;
                }
                let transition = catalogue.transition(&callback.key)?;
                let changes = catalogue.evaluate(transition, progress, 0).ok()?;
                Some((callback, changes))
            })
            .collect::<Vec<_>>()
    };
    for [first, second] in [[31953, 31957], [31957, 31953]] {
        let mut progress = BTreeMap::new();
        for (y, completes) in [(first, false), (second, true)] {
            let results = at(y, &progress);
            assert_eq!(results.len(), 1, "one eligible box at {y}");
            let (callback, changes) = results.into_iter().next().ok_or("box binding")?;
            let reward = callback.item_reward().as_ref().ok_or("box reward")?;
            assert_eq!(
                reward.item,
                if y == 31953 {
                    "oteryn:item.tibia.i3209"
                } else {
                    "oteryn:item.tibia.i3046"
                }
            );
            assert_eq!(callback.completes, completes);
            for change in changes {
                progress.insert(change.track, change.after);
            }
            assert!(at(y, &progress).is_empty(), "box at {y} already claimed");
        }
    }
    Ok(())
}

#[test]
fn donor_quest_venore_jobs_have_independent_acceptance_and_payouts() -> Result<(), String> {
    use crate::quest::reward_callbacks;

    let loaded = load_embedded_quest_state("venore-source").map_err(|err| format!("{err:?}"))?;
    let catalogue = loaded.catalogue();
    let rewards = reward_callbacks::load_for(catalogue)?;
    let mut acceptance_tracks = std::collections::BTreeSet::new();
    for job in ["maintenance", "pathfinder", "sabotage"] {
        let suffix = format!("/venore-daily/{job}/accept");
        let callback = rewards
            .iter()
            .find(|r| r.key.ends_with(&suffix))
            .ok_or(suffix)?;
        let transition = catalogue
            .transition(&callback.key)
            .ok_or("job acceptance")?;
        let changes = catalogue
            .evaluate(transition, &BTreeMap::new(), 80000)
            .map_err(|err| format!("{err:?}"))?;
        assert!(!changes.is_empty());
        for change in changes {
            assert!(
                acceptance_tracks.insert(change.track),
                "job must not advance another job's track"
            );
        }
        let suffix = format!("/venore-daily/{job}/report");
        let callback = rewards
            .iter()
            .find(|r| r.key.ends_with(&suffix))
            .ok_or(suffix)?;
        assert_eq!(callback.experience, Some(250));
        assert!(callback.per_occurrence_mint);
    }
    let callbacks = crate::quest::world_callbacks::load_for(catalogue)?;
    let machines = callbacks
        .iter()
        .filter(|c| c.key.contains("/authored-venore-machine-"))
        .collect::<Vec<_>>();
    assert_eq!(machines.len(), 9);
    for machine in machines {
        let tool = machine.item_on_target.as_ref().ok_or("machine tool")?;
        assert!(
            tool.selected_instance && tool.reusable,
            "choose the carried can of oil"
        );
        assert_eq!(tool.item, "oteryn:item.tibia.i17513");
    }
    let access = callbacks
        .iter()
        .find(|c| c.key.ends_with("authored-venore-sabotage-entry"))
        .ok_or("Corym hideout entry")?;
    assert_eq!(
        access.trigger,
        crate::quest::world_callbacks::Trigger::StepIn
    );
    assert!(access.is_read_only());
    assert_eq!(
        access.teleport,
        Some(crate::quest::world_callbacks::Position {
            x: 33029,
            y: 32141,
            floor: -8,
        })
    );
    Ok(())
}

#[test]
fn donor_quest_repenters_watering_and_mushrooms_have_independent_reports() -> Result<(), String> {
    let loaded = load_embedded_quest_state("repenters-source").map_err(|err| format!("{err:?}"))?;
    let catalogue = loaded.catalogue();
    let world = crate::quest::world_callbacks::load_for(catalogue)?;
    let rewards = crate::quest::reward_callbacks::load_for(catalogue)?;
    let find = |suffix: &str| {
        rewards
            .iter()
            .find(|r| r.key.ends_with(suffix))
            .ok_or_else(|| suffix.to_owned())
    };
    let water = find("/repenters/watering/accept")?;
    let mushroom = find("/repenters/mushrooms/accept")?;
    assert_eq!(water.npc, "oteryn:npc.ashari");
    assert_eq!(
        water.give.as_ref().ok_or("vial supply")?.item,
        "oteryn:item.tibia.i18991"
    );
    assert_eq!(
        mushroom.give.as_ref().ok_or("bug supply")?.item,
        "oteryn:item.tibia.i19124"
    );
    let apply = |key: &str, progress: &mut BTreeMap<String, i64>| -> Result<(), String> {
        let change = catalogue.transition(key).ok_or_else(|| key.to_owned())?;
        for change in catalogue
            .evaluate(change, progress, 100000)
            .map_err(|err| format!("{err:?}"))?
        {
            progress.insert(change.track, change.after);
        }
        Ok(())
    };
    let mut progress = BTreeMap::new();
    apply(&water.key, &mut progress)?;
    let fill = world
        .iter()
        .find(|c| c.key.ends_with("repenters-fill-vial"))
        .ok_or("spring")?;
    let tool = fill.item_on_target.as_ref().ok_or("selected vial")?;
    assert!(tool.selected_instance);
    assert_eq!(
        tool.replacement.as_deref(),
        Some("oteryn:item.tibia.i18992")
    );
    apply(&fill.key, &mut progress)?;
    let report_water = find("/repenters/watering/report")?;
    assert_eq!(report_water.experience, Some(2000));
    assert_eq!(report_water.take[0].item, "oteryn:item.tibia.i18992");
    let water_report = catalogue
        .transition(&report_water.key)
        .ok_or("water report")?;
    assert!(catalogue.evaluate(water_report, &progress, 100899).is_ok());
    assert!(catalogue.evaluate(water_report, &progress, 100900).is_err());
    let refill = world
        .iter()
        .find(|callback| callback.key.ends_with("repenters-refill-vial"))
        .ok_or("refill callback")?;
    assert_eq!(refill.selector, fill.selector);
    let refill_tool = refill.item_on_target.as_ref().ok_or("filled vial")?;
    assert_eq!(refill_tool.item, "oteryn:item.tibia.i18992");
    assert_eq!(
        refill_tool.replacement.as_deref(),
        Some(refill_tool.item.as_str())
    );
    let refill_transition = catalogue.transition(&refill.key).ok_or("refill")?;
    assert!(
        catalogue
            .evaluate(refill_transition, &progress, 100899)
            .is_err()
    );
    let refreshed = catalogue
        .evaluate(refill_transition, &progress, 100900)
        .map_err(|err| format!("{err:?}"))?;
    let stamp = refreshed
        .iter()
        .find(|change| change.track.ends_with("/filled-at"))
        .ok_or("refill clock")?;
    assert_eq!((stamp.before, stamp.after), (100000, 100900));
    let mut refilled = progress.clone();
    refilled.insert(stamp.track.clone(), stamp.after);
    assert!(catalogue.evaluate(water_report, &refilled, 101799).is_ok());
    assert!(catalogue.evaluate(water_report, &refilled, 101800).is_err());
    apply(&report_water.key, &mut progress)?;

    apply(&mushroom.key, &mut progress)?;
    let report_mushrooms = find("/repenters/mushrooms/report")?;
    assert_eq!(report_mushrooms.experience, Some(2100));
    assert!(apply(&report_mushrooms.key, &mut progress).is_err());
    let mut targets = std::collections::BTreeSet::new();
    for callback in world
        .iter()
        .filter(|c| c.key.contains("/authored-repenters-mushroom-"))
        .take(5)
    {
        let pos = callback.selector.position.ok_or("native mushroom")?;
        assert!(targets.insert((pos.x, pos.y, pos.floor)));
        let tool = callback
            .item_on_target
            .as_ref()
            .ok_or("selected trained fire bug")?;
        assert!(tool.selected_instance && tool.reusable);
        assert_eq!(tool.item, "oteryn:item.tibia.i19124");
        apply(&callback.key, &mut progress)?;
        assert!(apply(&callback.key, &mut progress).is_err());
    }
    assert_eq!(targets.len(), 5);
    apply(&report_mushrooms.key, &mut progress)?;
    assert!(apply(&report_mushrooms.key, &mut progress).is_err());
    Ok(())
}
