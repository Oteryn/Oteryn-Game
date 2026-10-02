
#[test]
fn intrinsic_weapon_metadata_roundtrips_without_native_mutation_or_absent_defaults() {
    let original = item_candidate();
    let old = serde_json::to_value(&original.state.item_authoring[0]).expect("old source overlay");
    assert!(old.get("weapon_attack_modifier_points").is_none());
    assert!(old.get("weapon_absolute_hit_chance_percent").is_none());
    let reread: ProjectV2ItemAuthoring = serde_json::from_value(old.clone()).expect("old source");
    assert_eq!(serde_json::to_value(reread).expect("old rewrite"), old);
    let mut draft = original.clone();
    draft.state.item_authoring[0].weapon_attack_modifier_points = Some(ReferenceSignedPoints(-7));
    draft.state.item_authoring[0].weapon_absolute_hit_chance_percent = Some(ProjectV2ExactRatio {
        numerator: 90, denominator: 1,
    });
    let documents = CanonicalProjectDocuments::from_v2_draft(draft, limits()).expect("source metadata");
    let parsed = ProjectSnapshot::new(documents.documents().clone(), limits()).expect("admit")
        .parse(limits()).expect("parse");
    assert_eq!(parsed.reference.records, original.core.records);
    let source = &parsed.v2().expect("source state").item_authoring[0];
    assert_eq!(source.weapon_attack_modifier_points, Some(ReferenceSignedPoints(-7)));
    assert_eq!(source.weapon_absolute_hit_chance_percent, Some(ProjectV2ExactRatio { numerator:90, denominator:1 }));
    assert_eq!(parsed.canonical_documents(limits()).expect("rewrite").documents(), documents.documents());
    for (numerator, denominator) in [(-1,1),(101,1),(2,2),(0,2),(1,0)] {
        let mut bad = original.clone();
        bad.state.item_authoring[0].weapon_absolute_hit_chance_percent = Some(ProjectV2ExactRatio { numerator, denominator });
        assert!(CanonicalProjectDocuments::from_v2_draft(bad, limits()).is_err());
    }
    for points in [i32::MIN, i32::MAX] {
        let mut incoming = old.clone();
        incoming["weapon_attack_modifier_points"] = json!(points);
        assert!(serde_json::from_value::<ProjectV2ItemAuthoring>(incoming).is_ok());
    }
    for value in [json!(2_147_483_648_i64), json!(true)] {
        let mut incoming = old.clone(); incoming["weapon_attack_modifier_points"] = value;
        assert!(serde_json::from_value::<ProjectV2ItemAuthoring>(incoming).is_err());
    }
}
