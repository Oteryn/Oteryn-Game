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
    let base = crate::quest::loader::load_embedded_quest_state("collection-fixture")
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
