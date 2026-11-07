use oteryn_gameplay_content::content::ProjectV2Declaration;

#[test]
fn public_visibility_defaults_and_rejects_unknown_values() -> Result<(), Box<dyn std::error::Error>>
{
    for kind in ["NPC", "Quest", "Area"] {
        let mut record = serde_json::json!({
            "kind": kind,
            "identity": {"key": "oteryn:test.visibility", "revision": "definition-r1"},
            "fields": []
        });
        if kind == "NPC" {
            record["services"] = serde_json::json!([]);
        }
        let ordinary: ProjectV2Declaration = serde_json::from_value(record.clone())?;
        assert_eq!(ordinary.public_visibility(), Default::default());
        assert!(serde_json::to_value(&ordinary)?.get("visibility").is_none());
        record["visibility"] = serde_json::json!("hidden");
        let hidden: ProjectV2Declaration = serde_json::from_value(record.clone())?;
        assert_ne!(hidden.public_visibility(), ordinary.public_visibility());
        assert_eq!(serde_json::to_value(hidden)?["visibility"], "hidden");
        record["visibility"] = serde_json::json!("secret");
        assert!(serde_json::from_value::<ProjectV2Declaration>(record).is_err());
    }
    Ok(())
}

#[test]
fn new_public_fields_and_categories_fail_closed() {
    assert!(
        serde_json::from_value::<ProjectV2Declaration>(serde_json::json!({
            "kind":"Quest", "identity":{"key":"oteryn:quest.test","revision":"definition-r1"},
            "fields":[], "new_public_field":true
        }))
        .is_err()
    );
    assert!(serde_json::from_value::<ProjectV2Declaration>(serde_json::json!({
        "kind":"UnknownCategory", "identity":{"key":"oteryn:quest.test","revision":"definition-r1"},
        "fields":[]
    })).is_err());
}
