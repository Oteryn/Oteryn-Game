use crate::quest::{QuestRefusal, loader};
use std::collections::BTreeMap;

#[test]
fn pilgrimage_collector_requires_all_ordered_visits_and_delivers_only_at_completion()
-> Result<(), String> {
    let loaded = loader::load_embedded_quest_state("content-1").map_err(|e| format!("{e:?}"))?;
    let catalogue = loaded.catalogue();
    let quest = catalogue
        .authored_bindings()
        .and_then(|b| {
            b.quests
                .iter()
                .find(|q| q.quest == "oteryn:quest.pilgrimage_of_ashes")
        })
        .ok_or("Pilgrimage collector binding missing")?;
    assert_eq!(quest.stages.len(), 8);
    let mut state = BTreeMap::new();
    let last = catalogue
        .transition("oteryn:quest-reward/m01c-pilgrimage-s8")
        .ok_or("collector")?;
    assert_eq!(
        catalogue.evaluate(last, &state, 0),
        Err(QuestRefusal::StageMismatch)
    );
    for index in 1..=8 {
        let key = format!("oteryn:quest-reward/m01c-pilgrimage-s{index}");
        let transition = catalogue.transition(&key).ok_or("visit")?;
        let changes = catalogue
            .evaluate(transition, &state, 0)
            .map_err(|e| format!("{e:?}"))?;
        assert_eq!(catalogue.completes_after(transition, &changes), index == 8);
        for change in changes {
            state.insert(change.track, change.after);
        }
        let callback = quest.stages[index - 1]
            .trigger
            .reward_callbacks()
            .next()
            .ok_or("NPC callback")?;
        if index == 8 {
            let give = callback.give.as_ref().ok_or("platinum delivery")?;
            assert_eq!(
                (give.item.as_str(), give.count),
                ("oteryn:item.tibia.i3035", 20)
            );
        } else {
            assert!(callback.give.is_none());
            assert_eq!(catalogue.evaluate(last, &state, 0).is_ok(), index == 7);
        }
    }
    assert_eq!(
        catalogue.evaluate(last, &state, 0),
        Err(QuestRefusal::StageMismatch)
    );
    Ok(())
}

#[test]
fn stampede_collector_is_bound_to_personal_trial_and_five_elephant_kills() -> Result<(), String> {
    let loaded = loader::load_embedded_quest_state("content-1").map_err(|e| format!("{e:?}"))?;
    let catalogue = loaded.catalogue();
    let quest = catalogue
        .authored_bindings()
        .and_then(|b| {
            b.quests
                .iter()
                .find(|q| q.quest == "oteryn:quest.stampede_mini_world_change")
        })
        .ok_or("Stampede collector binding missing")?;
    let crate::quest::authored_bindings::Trigger::StepIn { callback } = &quest.stages[0].trigger
    else {
        return Err("native trial entry missing".into());
    };
    assert!(callback.requires_world_admission);
    assert!(crate::quest::world_callbacks::personal_trial::admitted(
        callback
    ));
    for case in 0..4 {
        let mut unsafe_entry = callback.as_ref().clone();
        match case {
            0 => unsafe_entry.requires_world_admission = false,
            1 => unsafe_entry.actor_kind = crate::quest::world_callbacks::ActorKind::Monster,
            2 => unsafe_entry.storage.clear(),
            _ => unsafe_entry.lever = Some([1945, 1946]),
        }
        assert!(!crate::quest::world_callbacks::personal_trial::admitted(
            &unsafe_entry
        ));
        assert!(crate::quest::world_callbacks::validate_callbacks(&[unsafe_entry]).is_err());
    }
    assert!(
        callback.world_change.is_none(),
        "personal trial must not depend on daily raid state"
    );
    let encounter = callback
        .effects
        .encounter
        .as_ref()
        .ok_or("personal encounter")?;
    assert_eq!(encounter.monsters.len(), 5);
    assert!(
        encounter
            .monsters
            .iter()
            .all(|m| m.creature == "oteryn:creature.terrified_elephant")
    );
    let crate::quest::authored_bindings::Trigger::Kill { counter } = &quest.stages[1].trigger
    else {
        return Err("native kills missing".into());
    };
    assert_eq!((counter.maximum, counter.advance_gate_to), (5, Some(2)));
    let finish = catalogue
        .transition("oteryn:quest-reward/m01c-stampede-collector")
        .ok_or("collector")?;
    let mut state = BTreeMap::from([(counter.gate.clone(), 1), (counter.track.clone(), 4)]);
    assert_eq!(
        catalogue.evaluate(finish, &state, 0),
        Err(QuestRefusal::StageMismatch)
    );
    state.insert(counter.gate.clone(), 2);
    assert_eq!(
        catalogue.evaluate(finish, &state, 0),
        Err(QuestRefusal::StageMismatch)
    );
    state.insert(counter.track.clone(), 5);
    let changes = catalogue
        .evaluate(finish, &state, 0)
        .map_err(|e| format!("{e:?}"))?;
    assert!(catalogue.completes_after(finish, &changes));
    let reward = quest.stages[2]
        .trigger
        .reward_callbacks()
        .next()
        .ok_or("collector reward")?;
    assert_eq!(
        reward.achievement.as_deref(),
        Some("oteryn:achievement/trail_of_the_ape_god")
    );
    assert!(reward.give.is_none(), "no guaranteed elephant tusks");
    Ok(())
}
