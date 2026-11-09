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
fn authored_counter_reuses_only_an_identical_owned_source_declaration() -> Result<(), String> {
    let bindings = Bindings::parse(include_bytes!("fixtures/collection.json"))?;
    let quest = &bindings.quests[0];
    let t = &quest.tracks[0];
    let track = QuestTrack {
        key: t.key.clone(),
        quest: quest.quest.clone(),
        initial: t.initial,
        min: t.minimum,
        max: t.maximum,
    };
    let base = |track| {
        QuestStateCatalogue::new("shared-counter", vec![track], vec![])
            .map_err(|e| format!("{e:?}"))
    };
    let original = base(track.clone())?;
    let attached = bindings.attach(&original)?;
    assert_eq!(
        attached.tracks.len(),
        1,
        "one durable track declaration, not two copies"
    );
    assert_eq!(attached.track(&t.key), Some(&track));
    assert_ne!(
        attached.definition_hash(&quest.quest),
        original.definition_hash(&quest.quest),
        "the new native consumer still changes the quest witness"
    );
    for conflicting in [
        QuestTrack {
            quest: "oteryn:quest.foreign".into(),
            ..track.clone()
        },
        QuestTrack {
            initial: 1,
            ..track.clone()
        },
        QuestTrack {
            min: -1,
            ..track.clone()
        },
        QuestTrack {
            max: 2,
            ..track.clone()
        },
    ] {
        assert!(
            bindings.attach(&base(conflicting)?).is_err(),
            "ownership, default and bounds must all match"
        );
    }
    assert_eq!(
        original.track(&t.key),
        Some(&track),
        "composition cannot mutate its source"
    );
    Ok(())
}
