//! Published Nomads Land rules exercised through the quest loader.
use oteryn_gameplay_quests::quest::{loader::load_embedded_quest_state, reward_callbacks};
use std::collections::BTreeMap;

const TASK: &str = "oteryn:quest-progress/quest/u8_1/tibia_tales/aritos_task";

#[test]
fn no_reward_dialogue_starts_aritos_task_while_the_cave_is_sealed() -> Result<(), String> {
    let loaded =
        load_embedded_quest_state("nomads-referral").map_err(|error| format!("{error:?}"))?;
    let catalogue = loaded.catalogue();
    assert!(
        catalogue
            .transition("oteryn:quest-reward/nomads-land-arito-referral")
            .is_none()
    );
    let callbacks = reward_callbacks::offered_for(catalogue)?;
    for callback in callbacks {
        assert!(
            callback.storage.iter().all(|write| write.track != TASK
                || write.preserve
                || !(write.minimum..=write.maximum).contains(&0)),
            "{} can advance Arito's Task from fresh progress",
            callback.key
        );
    }
    Ok(())
}

#[test]
fn aritos_journal_starts_after_the_blocked_negotiation() -> Result<(), String> {
    use oteryn_gameplay_quests::quest::log::{QuestJournal, loader::load_embedded_quest_log};

    let state = load_embedded_quest_state("nomads-sealed-entrance")
        .map_err(|error| format!("{error:?}"))?;
    let log = load_embedded_quest_log(&state)?;
    let mission = log
        .content
        .active()
        .quests()
        .iter()
        .find(|quest| quest.key == "oteryn:quest.tibia_tales")
        .and_then(|quest| quest.missions.iter().find(|mission| mission.track == TASK))
        .ok_or("Arito's Task journal")?;
    assert_eq!((mission.start_value, mission.end_value), (2, 3));
    let QuestJournal::PerStage(stages) = &mission.journal else {
        return Err("Arito's Task must keep the journals for later stages".into());
    };
    assert_eq!(stages.keys().copied().collect::<Vec<_>>(), [2, 3]);
    Ok(())
}

#[test]
fn muhad_assignment_requires_aritos_prior_progress_and_grants_hideout_access() -> Result<(), String>
{
    const DOOR: &str = "oteryn:quest-progress/quest/u8_1/tibia_tales/aritos_task_door";
    let loaded =
        load_embedded_quest_state("nomads-assignment").map_err(|error| format!("{error:?}"))?;
    let catalogue = loaded.catalogue();
    let callback = reward_callbacks::offered_for(catalogue)?
        .into_iter()
        .find(|row| row.key == "oteryn:quest-reward/nomads-land-assignment")
        .ok_or("Muhad assignment is missing")?;
    assert_eq!(callback.npc, "oteryn:npc.muhad");
    assert_eq!(callback.topic, 2);
    assert_eq!(callback.next_topic, 0);
    let transition = catalogue
        .transition(&callback.key)
        .ok_or("assignment transition")?;
    assert!(!transition.completes);
    for task in [0, 2, 3] {
        let tracks = BTreeMap::from([(TASK.to_owned(), task), (DOOR.to_owned(), 0)]);
        assert!(catalogue.evaluate(transition, &tracks, 0).is_err());
    }
    let tracks = BTreeMap::from([(TASK.to_owned(), 1), (DOOR.to_owned(), 0)]);
    let writes = catalogue
        .evaluate(transition, &tracks, 0)
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(
        writes
            .into_iter()
            .map(|write| (write.track, write.after))
            .collect::<BTreeMap<_, _>>(),
        BTreeMap::from([(TASK.to_owned(), 2), (DOOR.to_owned(), 1)])
    );
    Ok(())
}

#[test]
fn nomads_source_entrance_and_exit_have_exact_runtime_selectors() -> Result<(), String> {
    use oteryn_gameplay_quests::quest::world_callbacks::{self, Position, Trigger};

    let loaded = load_embedded_quest_state("nomads-route").map_err(|error| format!("{error:?}"))?;
    let catalogue = loaded.catalogue();
    let callbacks = world_callbacks::load_for(catalogue)?;
    let pos = |x, y, floor| Position { x, y, floor };
    for (key, trigger, at, appearance, teleport, track, minimum) in [
        (
            "oteryn:quest-callback/nomads-land-pillar",
            Trigger::Use,
            pos(33185, 32844, -7),
            2188,
            Some(pos(33185, 32844, -9)),
            "oteryn:quest-progress/quest/u8_1/tibia_tales/aritos_task",
            2,
        ),
        (
            "oteryn:quest-callback/nomads-land-bamboo-exit",
            Trigger::StepIn,
            pos(33227, 32862, -9),
            1949,
            Some(pos(33226, 32865, -7)),
            "oteryn:quest-progress/quest/u8_1/tibia_tales/nomads_land_cabinet",
            1,
        ),
    ] {
        let callback = callbacks.iter().find(|row| row.key == key).ok_or(key)?;
        assert_eq!(callback.trigger, trigger);
        assert_eq!(callback.selector.position, Some(at));
        assert_eq!(callback.selector.appearance, Some(appearance));
        assert_eq!(callback.teleport, teleport);
        assert!(callback.requires_world_admission);
        let transition = catalogue.transition(key).ok_or("route transition")?;
        let tracks = BTreeMap::from([(track.to_owned(), minimum - 1)]);
        assert_eq!(
            catalogue.evaluate(transition, &tracks, 0).is_ok(),
            trigger == Trigger::StepIn,
            "the exit is available even before the cabinet claim"
        );
        let tracks = BTreeMap::from([(track.to_owned(), minimum)]);
        assert!(catalogue.evaluate(transition, &tracks, 0).is_ok());
    }
    Ok(())
}

#[test]
fn muhad_return_selects_one_platinum_reward_and_completes_after_exit() -> Result<(), String> {
    const EXIT: &str = "oteryn:quest-progress/quest/u8_1/tibia_tales/nomads_land_exit_reached";
    const CABINET: &str = "oteryn:quest-progress/quest/u8_1/tibia_tales/nomads_land_cabinet";
    const RETURNED: &str = "oteryn:quest-progress/authored/nomads_land_quest/returned";
    const CASKET: &str = "oteryn:item.tibia.i7527";
    let loaded =
        load_embedded_quest_state("nomads-return").map_err(|error| format!("{error:?}"))?;
    let catalogue = loaded.catalogue();
    let callbacks = reward_callbacks::offered_for(catalogue)?;
    for (variant, carried) in [("with-casket", 1), ("without-casket", 0)] {
        let key = format!("oteryn:quest-reward/nomads-land-return-{variant}");
        let callback = callbacks.iter().find(|row| row.key == key).ok_or(key)?;
        assert_eq!(callback.npc, "oteryn:npc.muhad");
        let [window] = callback.required_receipts.as_slice() else {
            return Err("the return must require the window USE receipt".into());
        };
        assert_eq!(
            window.transition,
            "oteryn:quest-callback/engenc2/nomads-land-window"
        );
        assert_eq!(
            window.track,
            "oteryn:quest-progress/authored/nomads_land_quest/s4_window_obscured"
        );
        assert_eq!(window.value, 1);
        assert_eq!(callback.inventory.len(), 1);
        assert_eq!(callback.inventory[0].item, CASKET);
        for quantity in [0, 1] {
            assert_eq!(
                callback.inventory[0].holds(&BTreeMap::from([(CASKET.to_owned(), quantity)])),
                quantity == carried
            );
        }
        assert_eq!(callback.take.len(), carried as usize);
        if let Some(item) = callback.take.first() {
            assert_eq!((&*item.item, item.count), (CASKET, 1));
        }
        let grant = callback.give.as_ref().ok_or("platinum")?;
        assert_eq!((&*grant.item, grant.count), ("oteryn:item.tibia.i3035", 50));
        let transition = catalogue
            .transition(&callback.key)
            .ok_or("return transition")?;
        assert!(transition.completes);
        assert_eq!(transition.quest, "oteryn:quest.authored.nomads_land_quest");
        for (exit, cabinet) in [(0, 0), (0, 1), (1, 0), (1, 1)] {
            let tracks = BTreeMap::from([
                (EXIT.to_owned(), exit),
                (CABINET.to_owned(), cabinet),
                (RETURNED.to_owned(), 0),
            ]);
            assert_eq!(
                callback
                    .requires
                    .iter()
                    .all(|gate| gate.holds(catalogue, &tracks)),
                exit == 1 && cabinet == 1
            );
        }
        let tracks = BTreeMap::from([(EXIT.to_owned(), 1), (RETURNED.to_owned(), 1)]);
        assert!(catalogue.evaluate(transition, &tracks, 0).is_err());
    }
    Ok(())
}

#[test]
fn nomads_keeps_existing_tibia_tales_and_window_progress_writable() -> Result<(), String> {
    use oteryn_gameplay_quests::quest::npc_rules::legacy_defaults;

    let loaded =
        load_embedded_quest_state("nomads-upgrade").map_err(|error| format!("{error:?}"))?;
    let catalogue = loaded.catalogue();
    for (quest, before) in [
        (
            "oteryn:quest.tibia_tales",
            [
                0x8d, 0x75, 0x51, 0x7e, 0x41, 0x35, 0x9a, 0x25, 0xab, 0xea, 0x65, 0x3c, 0x8f, 0xd4,
                0x4f, 0xa5, 0xb4, 0xd2, 0x76, 0x8c, 0x14, 0x70, 0xd9, 0xac, 0xaa, 0x94, 0x42, 0xb7,
                0x3f, 0x54, 0x30, 0x65,
            ],
        ),
        (
            "oteryn:quest.tibia_tales",
            [
                0x26, 0xda, 0x6d, 0x39, 0x75, 0x4a, 0xc8, 0x53, 0xaf, 0x2a, 0x65, 0x43, 0x68, 0x56,
                0xb5, 0xb6, 0xf8, 0xb8, 0x60, 0xdb, 0xdd, 0x5a, 0x5d, 0xb4, 0x55, 0x3b, 0x16, 0xfe,
                0x20, 0x09, 0xce, 0x4e,
            ],
        ),
        (
            "oteryn:quest.authored.nomads_land_quest",
            [
                0x84, 0x77, 0x0d, 0x71, 0x75, 0x73, 0xd6, 0xe6, 0xd4, 0x8d, 0xc8, 0x36, 0xdd, 0x6b,
                0xea, 0x5d, 0x30, 0x33, 0x0c, 0xc9, 0x0b, 0x0e, 0x2e, 0x82, 0xe6, 0x3f, 0xf2, 0x98,
                0x25, 0x73, 0x53, 0x60,
            ],
        ),
    ] {
        let current = catalogue.definition_hash(quest).ok_or(quest)?;
        assert_ne!(current, before);
        assert_eq!(
            legacy_defaults(catalogue, quest, &before),
            Some(&BTreeMap::new())
        );
    }
    Ok(())
}

#[test]
fn nomads_cabinet_accepts_completed_arito_without_reissuing_rewards() -> Result<(), String> {
    use oteryn_gameplay_quests::quest::QuestRefusal;

    const CLAIMED: &str = "oteryn:quest-progress/quest/u8_1/tibia_tales/nomads_land_cabinet";
    const CALLBACK: &str = "oteryn:quest-callback/nomads-land-sacred-casket-cabinet";
    let loaded =
        load_embedded_quest_state("nomads-cabinet").map_err(|error| format!("{error:?}"))?;
    let catalogue = loaded.catalogue();
    let transition = catalogue.transition(CALLBACK).ok_or("cabinet transition")?;
    for (task, claimed) in [(0, 0), (1, 0), (2, 1), (3, 1)] {
        let tracks = BTreeMap::from([(TASK.to_owned(), task), (CLAIMED.to_owned(), claimed)]);
        assert_eq!(
            catalogue.evaluate(transition, &tracks, 0),
            Err(QuestRefusal::StageMismatch)
        );
    }
    for task in [2, 3] {
        let tracks = BTreeMap::from([(TASK.to_owned(), task), (CLAIMED.to_owned(), 0)]);
        let writes = catalogue
            .evaluate(transition, &tracks, 0)
            .map_err(|error| format!("cabinet refused Arito's stage {task}: {error:?}"))?;
        assert_eq!(
            writes
                .into_iter()
                .map(|write| (write.track, write.after))
                .collect::<BTreeMap<_, _>>(),
            BTreeMap::from([(TASK.to_owned(), task), (CLAIMED.to_owned(), 1)])
        );
    }
    Ok(())
}
