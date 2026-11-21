//! Named M30 binding regression; no inferred equipment or character-template grants.
use super::*;

#[test]
fn engenc_child_of_destiny_binds_all_chosen_stages() -> Result<(), String> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../content/quests/authored-bindings/engenc-child-of-destiny.json");
    let bytes = std::fs::read(path).map_err(|error| error.to_string())?;
    let bindings = Bindings::parse(&bytes)?;
    bindings.verify_embedded_recipe_evidence()?;
    let embedded = embedded::load()?.ok_or("embedded bindings")?;
    assert!(
        embedded
            .quests
            .iter()
            .any(|quest| quest.quest == "oteryn:quest.authored.child_of_destiny_quest")
    );
    let quest = bindings.quests.first().ok_or("quest")?;
    assert_eq!(quest.quest, "oteryn:quest.authored.child_of_destiny_quest");
    assert_eq!(
        quest
            .stages
            .iter()
            .map(|stage| stage.key.as_str())
            .collect::<Vec<_>>(),
        ["s1", "s2", "s3", "s4", "s5", "s6"]
    );
    let production = crate::quest::loader::load_embedded_quest_state("engenc-child")
        .map_err(|error| format!("{error:?}"))?;
    assert!(
        production
            .catalogue()
            .definition_hash(&quest.quest)
            .is_some()
    );
    let base = QuestStateCatalogue::empty("engenc-child").ok_or("base")?;
    let catalogue = bindings.attach(&base)?;
    let mut state = std::collections::BTreeMap::new();
    for (index, stage) in quest.stages.iter().enumerate() {
        let transition = stage.transition(&quest.quest)?;
        for change in catalogue
            .evaluate(&transition, &state, 0)
            .map_err(|error| format!("{error:?}"))?
        {
            state.insert(change.track, change.after);
        }
        assert_eq!(
            state.get("oteryn:quest-progress/engenc/child/phase"),
            Some(&i64::try_from(index + 1).map_err(|error| error.to_string())?)
        );
        assert_eq!(transition.completes, index == 5);
    }
    let Trigger::Hit { objective } = &quest.stages[2].trigger else {
        return Err("spear hit".into());
    };
    assert!(objective.matches(
        "oteryn:creature.party_skeleton",
        "oteryn:item.tibia.i17110",
        None
    ));
    let Trigger::Hit { objective } = &quest.stages[3].trigger else {
        return Err("rune hit".into());
    };
    assert!(objective.matches(
        "oteryn:creature.party_skeleton",
        "oteryn:item.tibia.i17512",
        Some("candidate:spell/rune/lightest_magic_missile_rune")
    ));
    let Trigger::NpcDialogue { callback, .. } = &quest.stages[5].trigger else {
        return Err("exit completion".into());
    };
    assert_eq!(callback.npc, "oteryn:npc.the_destiny_seer");
    let reward = callback.give.as_ref().ok_or("reward")?;
    assert_eq!(
        (reward.item.as_str(), reward.count),
        ("oteryn:item.tibia.i3577", 3)
    );
    assert_eq!(reward.additional.len(), 1);
    assert_eq!(
        (
            reward.additional[0].item.as_str(),
            reward.additional[0].count
        ),
        ("oteryn:item.tibia.i268", 1)
    );
    Ok(())
}
