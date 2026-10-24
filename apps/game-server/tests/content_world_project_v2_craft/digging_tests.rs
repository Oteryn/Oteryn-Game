use super::*;

#[test]
fn item_conversion_digging_returns_match_xml_through_real_content_dispatch() {
    let catalogue: Value =
        serde_json::from_str(include_str!("../../../../content/craft/recipes.json"))
            .expect("catalogue");
    let rows: Vec<_> = catalogue["records"]
        .as_array()
        .expect("rows")
        .iter()
        .filter(|row| row["recipe"].get("opened_target").is_some())
        .cloned()
        .collect();
    assert_eq!(rows.len(), 12);
    for row in &rows {
        let (draft, wide) = item_conversion_candidate(vec![row.clone()]);
        let documents = CanonicalProjectDocuments::from_v2_draft(draft, wide)
            .expect("duration-backed digging row");
        let parsed = ProjectSnapshot::new(documents.documents().clone(), wide)
            .expect("snapshot")
            .parse(wide)
            .expect("real dispatch");
        assert_eq!(parsed.v2().expect("v2").declarations.len(), 1);
        for (pointer, value) in [
            ("/recipe/revert_ms", Value::Null),
            ("/recipe/revert_ms", json!(999)),
            ("/recipe/revert_ms", json!(3600001)),
            ("/recipe/revert_ms", json!(1000)),
            ("/recipe/opened_target", Value::Null),
            (
                "/recipe/opened_target/key",
                json!("oteryn:item.tibia.i7236"),
            ),
            ("/recipe/operands/0/effect", json!("CONSUME")),
            ("/recipe/target/kind", json!("tile_kind")),
            ("/recipe/tool", json!("none")),
        ] {
            let mut invalid = row.clone();
            *invalid.pointer_mut(pointer).expect("existing field") = value;
            let (draft, wide) = item_conversion_candidate(vec![invalid]);
            assert!(
                CanonicalProjectDocuments::from_v2_draft(draft, wide).is_err(),
                "{pointer}"
            );
        }
    }
}

#[test]
fn item_conversion_digging_facts_exclude_untimed_forms_and_preserve_floor_ownership() {
    let facts: Value =
        serde_json::from_str(include_str!("../../../../content/craft/map-facts.json"))
            .expect("facts");
    assert_eq!(
        facts["excluded_transitions"],
        json!([
            {"closed":7200,"opened":7236,"reason":"pinned XML has no return duration"},
            {"closed":21341,"opened":21342,"reason":"pinned XML has no return duration"}
        ])
    );
    assert_eq!(
        facts["floor_ownership"],
        "C5 preserves FLOOR-1 LocalObject transitions; these facts grant no runtime ownership"
    );
}
