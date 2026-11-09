use super::*;
use crate::quest::NpcDefinitionPredecessor;
use std::collections::BTreeMap;

fn fixture() -> Result<(Bindings, QuestStateCatalogue), String> {
    let bindings = Bindings::parse(include_bytes!("fixtures/collection.json"))?;
    let quest = bindings.quests[0].quest.clone();
    let base = QuestStateCatalogue::new(
        "composition-fixture",
        vec![QuestTrack {
            key: "oteryn:quest-progress/m01c/source-admission".into(),
            quest,
            initial: 0,
            min: 0,
            max: 1,
        }],
        Vec::new(),
    )
    .map_err(|error| format!("{error:?}"))?;
    Ok((bindings, base))
}

#[test]
fn collection_composition_binds_the_existing_source_admission_hash() -> Result<(), String> {
    let (bindings, mut source) = fixture()?;
    let quest = &bindings.quests[0].quest;
    let original = bindings.attach(&source)?.definition_hash(quest);
    // Additional source admission terms can change without changing track bounds.
    source.hashes.insert(quest.clone(), [0x61; 32]);
    let changed = bindings.attach(&source)?.definition_hash(quest);
    assert_ne!(
        original, changed,
        "collection cannot erase source admission terms"
    );
    Ok(())
}

#[test]
fn collection_composition_does_not_migrate_an_old_npc_route_into_a_new_binding()
-> Result<(), String> {
    let (bindings, mut source) = fixture()?;
    let quest = &bindings.quests[0].quest;
    source
        .npc_legacy
        .insert(quest.clone(), ([0x51; 32], BTreeMap::new()));
    source.predecessors.insert(
        quest.clone(),
        vec![NpcDefinitionPredecessor {
            hash: [0x52; 32],
            defaults: BTreeMap::new(),
        }],
    );
    let attached = bindings.attach(&source)?;
    assert!(
        !attached.npc_legacy.contains_key(quest),
        "old NPC aliases cannot bypass the authored binding"
    );
    assert!(
        !attached.predecessors.contains_key(quest),
        "old NPC predecessor migrations require their original owner"
    );
    assert!(
        source.npc_legacy.contains_key(quest),
        "composition must not mutate the source catalogue"
    );
    Ok(())
}

#[test]
fn authored_composition_reuses_only_identical_owned_track_declarations() -> Result<(), String> {
    let (bindings, _) = fixture()?;
    let declared = &bindings.quests[0].tracks[0];
    let track = QuestTrack {
        key: declared.key.clone(),
        quest: bindings.quests[0].quest.clone(),
        initial: declared.initial,
        min: declared.minimum,
        max: declared.maximum,
    };
    let base = QuestStateCatalogue::new("shared-track", vec![track.clone()], Vec::new())
        .map_err(|error| format!("{error:?}"))?;
    let attached = bindings.attach(&base)?;
    assert_eq!(
        attached
            .tracks
            .values()
            .filter(|t| t.key == track.key)
            .count(),
        1
    );
    for variant in 0..3 {
        let mut conflict = track.clone();
        match variant {
            0 => conflict.quest = "oteryn:quest.other_owner".into(),
            1 => conflict.max += 1,
            _ => {
                conflict.initial = if conflict.initial == conflict.min {
                    conflict.max
                } else {
                    conflict.min
                }
            }
        }
        let conflicting = QuestStateCatalogue::new("conflict", vec![conflict], Vec::new())
            .map_err(|error| format!("{error:?}"))?;
        assert!(
            bindings.attach(&conflicting).is_err(),
            "conflicting track variant {variant}"
        );
    }
    Ok(())
}
