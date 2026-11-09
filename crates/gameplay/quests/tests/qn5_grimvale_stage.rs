use oteryn_gameplay_quests::quest::{loader::load_embedded_quest_state, world_callbacks};
use std::collections::BTreeMap;

#[test]
fn qn5_grimvale_entry_uses_registered_stage_without_claiming_completion() -> Result<(), String> {
    let lowered = load_embedded_quest_state("qn5-grimvale-stage").map_err(|e| format!("{e:?}"))?;
    let catalogue = lowered.catalogue();
    let callbacks = world_callbacks::load_for(catalogue)?;
    let callback = callbacks
        .iter()
        .find(|c| c.key == "oteryn:quest-callback/qn5/grimvale-encounter-entry")
        .ok_or("Grimvale authored stage is not registered at boot")?;
    assert_eq!(callback.quest, "oteryn:quest.grimvale_mini_world_change");
    assert_eq!(callback.trigger, world_callbacks::Trigger::StepIn);
    assert!(callback.requires_world_admission);
    assert!(callback.admission.as_ref().is_some_and(|a| a.premium));
    assert_eq!(callback.world_change.as_deref(), Some("grimvale"));
    assert_eq!(callback.world_requirements.len(), 1);
    assert_eq!(callback.world_requirements[0].appearance, 22761);
    assert_eq!(
        callback.world_requirements[0].position,
        world_callbacks::Position {
            x: 33430,
            y: 31537,
            floor: -11
        }
    );
    assert_eq!(
        callback.selector.position,
        Some(world_callbacks::Position {
            x: 33420,
            y: 31539,
            floor: -10
        })
    );
    let transition = catalogue
        .transition(&callback.key)
        .ok_or("registered transition")?;
    assert!(!transition.completes);
    let track = "oteryn:quest-progress/qn5/grimvale-entry".to_owned();
    let before = BTreeMap::from([(track.clone(), 0)]);
    let changes = catalogue
        .evaluate(transition, &before, 0)
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].after, 1);
    assert!(
        catalogue
            .evaluate(transition, &BTreeMap::from([(track, 1)]), 0)
            .is_err()
    );
    Ok(())
}

#[test]
fn qn5_grimvale_werewolf_stage_advances_only_at_ten_without_completing_quest() -> Result<(), String>
{
    let lowered = load_embedded_quest_state("qn5-grimvale-stage").map_err(|e| format!("{e:?}"))?;
    let catalogue = lowered.catalogue();
    let key = "oteryn:quest-transition/native-kills/qn5-grimvale-werewolves";
    let counter = oteryn_gameplay_quests::quest::native_kills::counter_for(catalogue, key)
        .ok_or("Grimvale s2 is not registered")?;
    assert_eq!(counter.creature, "oteryn:creature.werewolf");
    assert_eq!(counter.maximum, 10);
    let transition = catalogue.transition(key).ok_or("s2 transition")?;
    assert_eq!(transition.world_change.as_deref(), Some("grimvale"));
    assert!(!transition.completes);
    let mut tracks = BTreeMap::from([(counter.gate.to_owned(), 1), (counter.track.to_owned(), 0)]);
    for count in 1..=10 {
        let changes = catalogue
            .evaluate(transition, &tracks, 0)
            .map_err(|e| format!("{e:?}"))?;
        for change in changes {
            tracks.insert(change.track, change.after);
        }
        assert_eq!(tracks[counter.track], count);
        assert_eq!(tracks[counter.gate], if count == 10 { 2 } else { 1 });
    }
    assert!(catalogue.evaluate(transition, &tracks, 0).is_err());
    Ok(())
}

#[test]
fn qn5_grimvale_kill_area_is_bounded_and_part_of_the_definition_witness() -> Result<(), String> {
    use oteryn_gameplay_quests::quest::{
        QuestStateCatalogue,
        authored_bindings::{Bindings, Trigger},
    };
    let bytes = include_bytes!("../../../../content/quests/authored-bindings/qn5-grimvale.json");
    let mut bindings = Bindings::parse(bytes)?;
    let base = QuestStateCatalogue::empty("qn5-area-bounds").ok_or("base")?;
    let quest = bindings.quests[0].quest.clone();
    let hash = bindings.attach(&base)?.definition_hash(&quest);
    let Trigger::Kill { counter } = &mut bindings.quests[0].stages[1].trigger else {
        return Err("s2".into());
    };
    let area = counter.death_area.ok_or("s2 lacks its island bounds")?;
    assert!(area.contains([33385, 31532, 10]));
    assert!(area.contains([33492, 31619, 11]));
    assert!(!area.contains([i32::MIN, i32::MAX, 10]));
    counter.death_area.as_mut().ok_or("area")?.maximum[0] -= 1;
    assert_ne!(bindings.attach(&base)?.definition_hash(&quest), hash);
    let Trigger::Kill { counter } = &mut bindings.quests[0].stages[1].trigger else {
        return Err("s2".into());
    };
    counter.death_area.as_mut().ok_or("area")?.maximum[2] = 16;
    assert!(bindings.validate().is_err());
    Ok(())
}
