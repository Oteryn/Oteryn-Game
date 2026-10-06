#![allow(clippy::unwrap_used)]
use super::*;
#[test]
fn all110_actual_source_creature_appearance_operations_construct_from_exact_native_members() {
    let registry = crate::content::native_spell_appearances::compile(
        include_bytes!("../../../content/presentations/bindings/spell-appearances.json"),
        [9; 32],
    )
    .unwrap();
    let cases: Vec<serde_json::Value> =
        serde_json::from_slice(include_bytes!("creature_appearance_test_data.json")).unwrap();
    let mut count = 0;
    for row in cases {
        let effect: ProjectV2InlineEffect = serde_json::from_value(row["effect"].clone()).unwrap();
        if let ProjectV2InlineEffectOperation::AppearanceTransform {
            creature: Some(creature),
            item: None,
            duration_ms,
        } = &effect.operation
        {
            let definition = lower_creature_member(
                &registry,
                creature,
                &effect.key,
                u32::try_from(*duration_ms).unwrap(),
            )
            .unwrap();
            let look_type = registry
                .for_creature(&creature.key, &creature.revision)
                .unwrap()
                .look_type();
            assert_eq!(
                definition.values(),
                ConditionValues::Outfit {
                    duration_ms: u32::try_from(*duration_ms).unwrap(),
                    look_type
                }
            );
            let mut wrong = creature.clone();
            wrong.revision = "definition-forged".into();
            assert_eq!(
                lower_creature_member(
                    &registry,
                    &wrong,
                    &effect.key,
                    u32::try_from(*duration_ms).unwrap()
                ),
                Err(AppearanceContentError::MissingQualifiedMember)
            );
            assert_eq!(
                lower_creature_member(&registry, creature, &effect.key, 0),
                Err(AppearanceContentError::InvalidDefinition)
            );
            count += 1;
        }
    }
    assert_eq!(count, 110); // Constructor qualification, not scheduled cast/presentation execution.
}
#[test]
fn ten_actual_item_appearance_operations_have_no_exact_active_policy_and_are_not_rekeyed() {
    let document: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../content/items/definitions/spell-native-profiles.json"
    ))
    .unwrap();
    let cases: Vec<serde_json::Value> =
        serde_json::from_slice(include_bytes!("creature_appearance_test_data.json")).unwrap();
    let mut count = 0;
    for row in cases {
        let effect: ProjectV2InlineEffect = serde_json::from_value(row["effect"].clone()).unwrap();
        if let ProjectV2InlineEffectOperation::AppearanceTransform {
            creature: None,
            item: Some(item),
            ..
        } = &effect.operation
        {
            let found = document["records"]
                .as_array()
                .unwrap()
                .iter()
                .any(|record| {
                    (record["authoring"]["item"]["key"] == item.key
                        && record["authoring"]["item"]["revision"] == item.revision)
                        || (record["production_definition"]["production_key"] == item.key
                            && record["production_definition"]["revision_ref"] == item.revision)
                });
            assert!(!found);
            count += 1;
        }
    }
    assert_eq!(count, 10); // Exact current data disposition; no raw item-ID/member alias.
}

#[test]
fn source_object_illusion_flag_does_not_grant_an_unqualified_outfit_cast() {
    let actual: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../content/creatures/definitions/spell-native-profiles.json"
    ))
    .unwrap();
    let row = actual["records"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["profile"]["target"]["key"] == "oteryn:creature.enraged_bookworm")
        .unwrap();
    assert_eq!(
        row["profile"]["data"]["profile"]["details"]["flags"]["illusionable"],
        true
    );
    let creature: crate::content::ProjectV2DefinitionRef =
        serde_json::from_value(row["profile"]["target"].clone()).unwrap();
    let registry = crate::content::native_spell_appearances::compile(
        include_bytes!("../../../content/presentations/bindings/spell-appearances.json"),
        [9; 32],
    )
    .unwrap();
    assert!(
        registry
            .for_creature(&creature.key, &creature.revision)
            .is_none()
    );
    assert_eq!(
        lower_creature_member(
            &registry,
            &creature,
            "oteryn:effect.source.object-illusion-refusal",
            1000
        ),
        Err(AppearanceContentError::MissingQualifiedMember)
    );
}
