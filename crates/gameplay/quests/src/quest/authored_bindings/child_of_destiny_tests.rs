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

/// CHILDDEST: the production objectives credit only the Party Skeleton from the
/// authored source; another target or source never matches.
#[test]
fn childdest_training_objectives_refuse_wrong_target_or_source() -> Result<(), String> {
    let production = crate::quest::loader::load_embedded_quest_state("childdest")
        .map_err(|error| format!("{error:?}"))?;
    let catalogue = production.catalogue();
    let skeleton = "oteryn:creature.party_skeleton";
    let counter = crate::quest::native_kills::counter_for(
        catalogue,
        "oteryn:quest-transition/native-kills/engenc-child-s2",
    )
    .ok_or("Party Skeleton counter")?;
    assert_eq!(counter.creature, skeleton);
    assert!(crate::quest::native_kills::tracks_creature_for(
        catalogue, skeleton
    ));
    let objective = |stage: &str| {
        hits::objective_for(
            catalogue,
            &format!("oteryn:quest-transition/native-hits/engenc-child-{stage}"),
        )
        .map(|(_, objective)| objective)
        .ok_or(format!("{stage} objective"))
    };
    let (spear, rune) = (objective("s3")?, objective("s4")?);
    let (spear_item, rune_item) = ("oteryn:item.tibia.i17110", "oteryn:item.tibia.i17512");
    let missile = Some("candidate:spell/rune/lightest_magic_missile_rune");
    assert!(spear.matches(skeleton, spear_item, None));
    assert!(rune.matches(skeleton, rune_item, missile));
    for creature in ["oteryn:creature.rat", "oteryn:creature.skeleton"] {
        assert!(!spear.matches(creature, spear_item, None), "{creature}");
        assert!(!rune.matches(creature, rune_item, missile), "{creature}");
    }
    assert!(!spear.matches(skeleton, "oteryn:item.tibia.i3277", None));
    assert!(!spear.matches(skeleton, rune_item, missile));
    assert!(!spear.matches(skeleton, spear_item, missile));
    assert!(!rune.matches(skeleton, spear_item, None));
    assert!(!rune.matches(skeleton, rune_item, None));
    assert!(!rune.matches(
        skeleton,
        rune_item,
        Some("candidate:spell/rune/light_magic_missile_rune")
    ));
    Ok(())
}
