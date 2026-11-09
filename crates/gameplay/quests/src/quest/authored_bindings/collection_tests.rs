use super::*;

fn fixture() -> Result<Bindings, String> {
    Bindings::parse(include_bytes!("fixtures/collection.json"))
}

#[test]
fn collection_exact_key_number_and_quantity_are_bound_into_the_operation() -> Result<(), String> {
    let bindings = fixture()?;
    let base = QuestStateCatalogue::empty("collection-fixture").ok_or("revision")?;
    let catalogue = bindings.attach(&base)?;
    let original = catalogue
        .definition_hash(&bindings.quests[0].quest)
        .ok_or("hash")?;
    let mut changed = bindings.clone();
    let Trigger::Collect { collection } = &mut changed.quests[0].stages[0].trigger else {
        return Err("collection".into());
    };
    collection.items[0].key_number = Some(5011);
    assert_ne!(
        changed
            .attach(&base)?
            .definition_hash(&bindings.quests[0].quest),
        Some(original)
    );
    let Trigger::Collect { collection } = &bindings.quests[0].stages[0].trigger else {
        return Err("collection".into());
    };
    let transition = catalogue.transition(&collection.key).ok_or("transition")?;
    crate::quest::admission_binding::validate_for_catalogue(&catalogue, transition, false)?;
    let mut renamed = transition.clone();
    renamed.key = "oteryn:quest-collection/forged".into();
    assert!(
        crate::quest::admission_binding::validate_for_catalogue(&catalogue, &renamed, false)
            .is_err()
    );
    let mut stripped = transition.clone();
    stripped.effects[0].effect = crate::quest::QuestEffectKind::Set(1);
    assert!(
        crate::quest::admission_binding::validate_for_catalogue(&catalogue, &stripped, false)
            .is_err()
    );
    Ok(())
}

#[test]
fn collection_refuses_ambiguous_overlapping_or_zero_inputs() -> Result<(), String> {
    for case in 0..4 {
        let mut bindings = fixture()?;
        let Trigger::Collect { collection } = &mut bindings.quests[0].stages[0].trigger else {
            return Err("collection".into());
        };
        match case {
            0 => collection.items[0].count = 0,
            1 => collection.items[0].key_number = Some(0),
            2 => collection.items.push(collection.items[0].clone()),
            _ => {
                let mut item = collection.items[0].clone();
                item.key_number = None;
                collection.items.push(item);
            }
        }
        assert!(
            bindings.validate().is_err(),
            "invalid collection {case} accepted"
        );
    }
    Ok(())
}

#[test]
fn collection_preserves_existing_quest_hashes_and_metadata() -> Result<(), String> {
    let base = crate::quest::loader::load_embedded_quest_state_with_bindings(
        "collection-fixture",
        &Bindings {
            schema: 1,
            quests: Vec::new(),
        },
    )
    .map_err(|e| format!("{e:?}"))?
    .catalogue()
    .clone();
    let bindings = fixture()?;
    let before = base
        .definition_hash(crate::quest::rotten_blood::QUEST)
        .ok_or("retained hash")?;
    // A synthetic collection is not a production qualification for this quest.
    assert!(bindings.attach(&base).is_err());
    let attached = Bindings {
        schema: 1,
        quests: Vec::new(),
    }
    .attach(&base)?;
    assert_eq!(attached.authored_bindings(), base.authored_bindings());
    assert_eq!(attached.hashes, base.hashes);
    assert!(bindings.attach(&attached).is_err());
    assert_eq!(attached.native_recipe_evidence, base.native_recipe_evidence);
    assert_eq!(attached.npc_legacy, base.npc_legacy);
    assert_eq!(attached.predecessors, base.predecessors);
    assert_eq!(attached.authored_bindings, base.authored_bindings);
    assert_eq!(
        attached.definition_hash(crate::quest::rotten_blood::QUEST),
        Some(before)
    );
    Ok(())
}

#[test]
fn collection_raise_preserves_progress_and_inventory_checks_cannot_count_replays()
-> Result<(), String> {
    let mut bindings = fixture()?;
    bindings.quests[0].tracks[0].maximum = 3;
    let Trigger::Collect { collection } = &mut bindings.quests[0].stages[0].trigger else {
        return Err("collection".into());
    };
    collection.storage[0].maximum = 3;
    collection.storage[0].at_least = true;
    let key = collection.key.clone();
    let track = collection.storage[0].track.clone();
    let catalogue = bindings.attach(&QuestStateCatalogue::empty("content-1").ok_or("revision")?)?;
    let changes = catalogue
        .evaluate(
            catalogue.transition(&key).ok_or("transition")?,
            &std::collections::BTreeMap::from([(track, 3)]),
            0,
        )
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(changes[0].after, 3);
    let Trigger::Collect { collection } = &mut bindings.quests[0].stages[0].trigger else {
        return Err("collection".into());
    };
    collection.storage[0].at_least = false;
    collection.storage[0].increment = true;
    assert!(
        bindings.validate().is_err(),
        "a repeated inventory predicate is not a new acquisition"
    );
    Ok(())
}

#[test]
fn production_fanfare_collect_is_numbered_and_does_not_complete() -> Result<(), String> {
    let loaded = crate::quest::loader::load_embedded_quest_state("fanfare-fixture")
        .map_err(|e| format!("{e:?}"))?;
    let catalogue = loaded.catalogue();
    let transition = catalogue
        .transition("oteryn:quest-collection/m01c-fanfare-key")
        .ok_or("missing production Fanfare collection")?;
    assert_eq!(transition.quest, "oteryn:quest.fanfare_quest");
    assert!(!transition.completes);
    let bindings = catalogue.authored_bindings().ok_or("bindings")?;
    let collection = bindings
        .quests
        .iter()
        .find(|quest| quest.quest == transition.quest)
        .and_then(|quest| quest.stages.first())
        .and_then(|stage| match &stage.trigger {
            Trigger::Collect { collection } => Some(collection),
            _ => None,
        })
        .ok_or("collection")?;
    assert_eq!(collection.items[0].item, "oteryn:item.tibia.i2973");
    assert_eq!(collection.items[0].key_number, Some(3520));
    Ok(())
}
