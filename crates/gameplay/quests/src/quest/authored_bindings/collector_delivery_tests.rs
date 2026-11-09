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
