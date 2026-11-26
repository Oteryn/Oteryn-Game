use oteryn_gameplay_quests::quest::{loader, reward_callbacks, world_callbacks};
use std::collections::BTreeMap;

#[test]
fn quest_contacts_are_loaded_by_the_native_quest_registries() -> Result<(), String> {
    let state =
        loader::load_embedded_quest_state("definition-r1").map_err(|error| format!("{error:?}"))?;
    assert!(
        state
            .catalogue()
            .transition("oteryn:quest-reward/thieves-guild-report-08")
            .is_some()
    );
    let rewards = reward_callbacks::load()?;
    for (key, topic, item, quantity) in [
        ("thieves-guild-report-01", 2, 3044, 10),
        ("thieves-guild-report-02", 3, 227, 1),
        ("thieves-guild-report-03", 4, 7933, 1),
        ("thieves-guild-report-05", 6, 7870, 1),
        ("thieves-guild-report-06", 7, 7936, 1),
        ("thieves-guild-report-07", 8, 7935, 1),
    ] {
        let row = rewards.iter().find(|r| r.key.ends_with(key)).ok_or(key)?;
        assert_eq!(row.topic, topic);
        assert_eq!(row.take.len(), 1);
        assert_eq!(row.take[0].item, format!("oteryn:item.tibia.i{item}"));
        assert_eq!(row.take[0].count, quantity);
    }
    let gobbler = rewards
        .iter()
        .find(|row| row.key.ends_with("their-masters-voice-slime-gobbler"))
        .ok_or("Servant Sentry reward")?;
    assert_eq!(
        gobbler.give.as_ref().ok_or("Gobbler item")?.item,
        "oteryn:item.tibia.i12077"
    );
    assert_eq!(gobbler.storage[0].maximum, 0);
    assert_eq!(gobbler.storage[0].set, 1);
    Ok(())
}

#[test]
fn thieves_guild_retains_selected_tools_claim_custody_and_final_door() -> Result<(), String> {
    let callbacks = world_callbacks::load()?;
    for (name, expected, look, tool, before, after) in [
        ("sarina-vase", (32337, 31812, -6, 1), 7892, 7889, 1, 2),
        ("fish-room-key", (32359, 32787, -6, 1), 7941, 7934, 2, 2),
        ("cathedral-note", (32598, 32381, -10, 1), 1451, 7873, 1, 2),
    ] {
        let row = callbacks
            .iter()
            .find(|row| row.key == format!("oteryn:quest-callback/thieves-guild-{name}"))
            .ok_or(name)?;
        let native = row.selector.native.as_ref().ok_or("native target")?;
        assert_eq!(
            (
                native.position.x,
                native.position.y,
                native.position.floor,
                native.ordinal
            ),
            expected
        );
        assert_eq!(native.appearance, look);
        let use_item = row.item_on_target.as_ref().ok_or("selected item")?;
        assert_eq!(use_item.item, format!("oteryn:item.tibia.i{tool}"));
        assert!(use_item.selected_instance && !use_item.reusable && row.requires_world_admission);
        assert_eq!(
            (
                row.storage[0].minimum,
                row.storage[0].maximum,
                row.storage[0].set
            ),
            (before, before, after)
        );
    }
    let fish = callbacks
        .iter()
        .find(|row| row.key.ends_with("thieves-guild-goldfish"))
        .ok_or("fixed goldfish")?;
    assert_eq!(fish.selector.unique, Some(9266));
    assert_eq!(fish.selector.ordinal, Some(3));
    assert_eq!(
        fish.item_reward().as_ref().ok_or("goldfish reward")?.item,
        "oteryn:item.tibia.i7936"
    );
    let rewards = reward_callbacks::load()?;
    let blackmail = rewards
        .iter()
        .find(|row| row.key.ends_with("thieves-guild-report-07"))
        .ok_or("Blackmail report")?;
    assert_eq!(
        blackmail.required_claims,
        ["oteryn:reward-claim.quest.u8_2.the_thieves_guild_quest.compromising_letter"]
    );
    let report = rewards
        .iter()
        .find(|row| row.key.ends_with("thieves-guild-report-08"))
        .ok_or("Message report")?;
    assert!(report.completes);
    assert!(
        report
            .storage
            .iter()
            .any(|row| row.track.ends_with("/mission08") && row.minimum == 2 && row.set == 3)
    );
    assert!(
        report
            .storage
            .iter()
            .any(|row| row.track.ends_with("/door") && row.set == 1)
    );
    assert!(
        report
            .storage
            .iter()
            .any(|row| row.track.ends_with("/questline") && row.minimum == 8 && row.set == 9)
    );
    let state =
        loader::load_embedded_quest_state("definition-r1").map_err(|error| format!("{error:?}"))?;
    let catalogue = state.catalogue();
    let note = catalogue
        .transition("oteryn:quest-callback/thieves-guild-cathedral-note")
        .ok_or("Cathedral Note transition")?;
    let completion = catalogue
        .transition(&report.key)
        .ok_or("completion transition")?;
    let root = "oteryn:quest-progress/quest/u8_2/the_thieves_guild_quest";
    let mission = format!("{root}/mission08");
    let line = format!("{root}/questline");
    let door = format!("{root}/door");
    let mut stored = BTreeMap::from([(mission.clone(), 1), (line.clone(), 8)]);
    assert!(catalogue.evaluate(completion, &stored, 100).is_err());
    for transition in [note, completion] {
        for change in catalogue
            .evaluate(transition, &stored, 100)
            .map_err(|error| format!("{error:?}"))?
        {
            stored.insert(change.track, change.after);
        }
    }
    assert_eq!(stored[&mission], 3);
    assert_eq!(stored[&line], 9);
    assert_eq!(stored[&door], 1);
    assert!(completion.completes);
    assert!(catalogue.evaluate(completion, &stored, 101).is_err());
    Ok(())
}

#[test]
fn native_monks_diary_uses_the_ghostlands_corpse_and_earned_access() -> Result<(), String> {
    let callbacks = world_callbacks::load()?;
    let diary = callbacks
        .iter()
        .find(|row| row.key.ends_with("white-raven-monks-diary"))
        .ok_or("Ghostlands diary binding")?;
    let native = diary
        .selector
        .native
        .as_ref()
        .ok_or("native corpse selector")?;
    assert_eq!(
        (
            native.position.x,
            native.position.y,
            native.position.floor,
            native.ordinal,
            native.appearance
        ),
        (32262, 31861, -11, 2, 4240)
    );
    assert_eq!(diary.trigger, world_callbacks::Trigger::Use);
    assert!(diary.requires_world_admission);
    assert_eq!(
        diary.item_reward().as_ref().ok_or("diary reward")?.item,
        "oteryn:item.tibia.i3212"
    );
    assert!(
        diary
            .storage
            .iter()
            .any(|state| state.track.ends_with("/door") && state.minimum == 1 && state.preserve)
    );
    let rewards = reward_callbacks::load()?;
    let report = rewards
        .iter()
        .find(|r| r.key.ends_with("l2-white-raven-diary"))
        .ok_or("Costello report")?;
    assert!(report.completes);
    assert!(
        report
            .required_receipts
            .iter()
            .any(|receipt| receipt.transition == diary.key)
    );
    Ok(())
}
