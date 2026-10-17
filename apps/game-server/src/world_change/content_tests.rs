use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;
const FURY: &str =
    include_str!("../../../../content/encounters/world-changes/fury_gates/selection-source.json");
const NIGHTMARE: &str = include_str!(
    "../../../../content/encounters/world-changes/nightmare_isles/selection-source.json"
);

fn candidates() -> Result<Vec<WorldChangeCandidate>, serde_json::Error> {
    Ok(vec![
        serde_json::from_str(FURY)?,
        serde_json::from_str(NIGHTMARE)?,
    ])
}

#[test]
fn accepted_selection_facts_retain_reference_names_and_explicit_missing_bindings() -> TestResult {
    let values = candidates()?;
    assert_eq!(validate_candidates(&values), Ok(()));
    for (candidate, chance, count) in [(&values[0], 343, 10), (&values[1], 1000, 3)] {
        assert_eq!(
            candidate.trigger,
            Trigger::AtReset {
                chance_permille: chance
            }
        );
        assert_eq!(candidate.variants.len(), count);
        assert!(candidate.variants.iter().all(|variant| variant.weight == 1));
        assert_eq!(candidate.unresolved.len(), 6);
    }
    assert_eq!(values[0].key, "fury_gates");
    assert_eq!(
        values[0]
            .variants
            .iter()
            .map(|v| v.variant_key.as_str())
            .collect::<Vec<_>>(),
        [
            "abdendriel",
            "ankrahmun",
            "carlin",
            "darashia",
            "edron",
            "kazordoon",
            "libertybay",
            "porthope",
            "thais",
            "venore"
        ]
    );
    assert_eq!(values[1].key, "nightmare_isles");
    assert_eq!(
        values[1]
            .variants
            .iter()
            .map(|v| v.variant_key.as_str())
            .collect::<Vec<_>>(),
        ["ankrahmun-north", "darashia-north", "darashia-west"]
    );
    Ok(())
}

#[test]
fn cardinality_identity_and_weight_failures_are_distinct() -> TestResult {
    let source = candidates()?.remove(0);
    assert_eq!(
        validate_candidates(&vec![source.clone(); MAX_DEFINITIONS + 1]),
        Err(CandidateError::DefinitionCount)
    );
    assert_eq!(
        validate_candidates(&[source.clone(), source.clone()]),
        Err(CandidateError::DuplicateDefinition)
    );
    for (count, error) in [
        (0, CandidateError::VariantCount),
        (MAX_VARIANTS + 1, CandidateError::VariantCount),
    ] {
        let mut value = source.clone();
        value.variants = vec![source.variants[0].clone(); count];
        assert_eq!(validate_candidates(&[value]), Err(error));
    }
    let mut value = source.clone();
    value.variants[1].variant_key = value.variants[0].variant_key.clone();
    assert_eq!(
        validate_candidates(&[value]),
        Err(CandidateError::DuplicateVariant)
    );
    let mut value = source.clone();
    value.variants[0].weight = 0;
    assert_eq!(validate_candidates(&[value]), Err(CandidateError::Weight));
    let mut value = source;
    value.variants[0].weight = u64::MAX;
    assert_eq!(validate_candidates(&[value]), Err(CandidateError::Weight));
    Ok(())
}

#[test]
fn unsupported_triggers_unknown_fields_and_unsourced_facts_refuse() -> TestResult {
    let source = candidates()?.remove(0);
    let mut value = source.clone();
    value.trigger = Trigger::AtReset {
        chance_permille: 1001,
    };
    assert_eq!(validate_candidates(&[value]), Err(CandidateError::Trigger));
    for trigger in [
        Trigger::Calendar {
            month: 2,
            day: 30,
            epochs: 1,
        },
        Trigger::Calendar {
            month: 1,
            day: 1,
            epochs: 0,
        },
    ] {
        let mut value = source.clone();
        value.trigger = trigger;
        assert_eq!(validate_candidates(&[value]), Err(CandidateError::Trigger));
    }
    let mut json: serde_json::Value = serde_json::from_str(FURY)?;
    json["minimum_gap"] = 1.into();
    assert!(serde_json::from_value::<WorldChangeCandidate>(json).is_err());
    let mut value = source.clone();
    value.provenance.remove("trigger");
    assert_eq!(
        validate_candidates(&[value]),
        Err(CandidateError::Provenance)
    );
    let mut value = source.clone();
    value
        .provenance
        .insert("trigger".into(), "missing:W10".into());
    assert_eq!(
        validate_candidates(&[value]),
        Err(CandidateError::Provenance)
    );
    let mut value = source;
    value
        .sources
        .get_mut("canary")
        .ok_or("source fixture")?
        .sha256 = "bad".into();
    assert_eq!(
        validate_candidates(&[value]),
        Err(CandidateError::Provenance)
    );
    Ok(())
}

#[test]
fn selection_source_cannot_disguise_missing_native_bindings_as_completed() -> TestResult {
    let mut value = candidates()?.remove(0);
    value.unresolved.clear();
    assert_eq!(
        validate_candidates(&[value]),
        Err(CandidateError::MissingBindingGaps)
    );
    // Native executable fields are not part of this candidate source schema.
    let mut json: serde_json::Value = serde_json::from_str(NIGHTMARE)?;
    json["spawns"] = serde_json::json!([]);
    assert!(serde_json::from_value::<WorldChangeCandidate>(json).is_err());
    Ok(())
}

#[test]
fn partial_binding_gap_set_refuses() -> TestResult {
    let mut value = candidates()?.remove(0);
    value.unresolved.truncate(1);
    assert_eq!(
        validate_candidates(&[value]),
        Err(CandidateError::MissingBindingGaps)
    );
    Ok(())
}

#[test]
fn typed_selection_compiles_but_w1_without_native_bindings_refuses() -> TestResult {
    let values = candidates()?;
    let selections = compile_selection_source(&values).map_err(|error| format!("{error:?}"))?;
    assert_eq!(selections.len(), 2);
    for selection in selections {
        assert_eq!(
            selection.require_w1_bindings(),
            Err(WorldChangeBindingsUnavailable)
        );
    }
    // Malformed input never becomes a typed selection in the first place.
    let mut malformed = values;
    malformed[0].variants[0].weight = 0;
    assert!(matches!(
        compile_selection_source(&malformed),
        Err(CandidateError::Weight)
    ));
    // A source-only compilation has no native bindings to request.
    assert!(
        compile_selection_source(&[])
            .map_err(|error| format!("{error:?}"))?
            .is_empty()
    );
    Ok(())
}
