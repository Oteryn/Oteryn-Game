use super::*;
use serde_json::json;

const EVIDENCE_LAST_LEVEL: u32 = 2000;

fn level_xp(level: u64) -> u64 {
    let l = i128::from(level);
    u64::try_from(50 * (l.pow(3) - 6 * l.pow(2) + 17 * l - 12) / 3).unwrap()
}

fn with_revision(prefix: &str, mut doc: Value) -> Value {
    let revision = document_revision(prefix, &doc).unwrap();
    doc["revision"] = Value::String(revision);
    doc
}

fn section_value() -> Value {
    let evidence_rev = format!("{PREFIX_EVIDENCE}{}", "0".repeat(32));
    let levels: Vec<Value> = (1..=CHARACTER_EXPERIENCE_TABLE_LEVELS as u64)
        .map(|l| json!({"level": l, "minimum_experience": level_xp(l).to_string()}))
        .collect();
    let table = with_revision(
        PREFIX_TABLE,
        json!({
            "schema": TABLE_SCHEMA,
            "evidence_revision": evidence_rev,
            "evidence_coverage": {"first_level": 1, "last_level": EVIDENCE_LAST_LEVEL},
            "levels": levels,
            "terminal_exclusive_experience": level_xp(2001).to_string(),
        }),
    );
    let death = with_revision(
        PREFIX_DEATH,
        json!({"schema": DEATH_SCHEMA, "loss_numerator": 1, "loss_denominator": 1, "rounding": "floor"}),
    );
    let reward = with_revision(PREFIX_REWARD, json!({"schema": REWARD_SCHEMA}));
    let differences = with_revision(
        PREFIX_DIFFERENCES,
        json!({"schema": DIFFERENCES_SCHEMA, "records": [{"id": "a", "reference": "r", "oteryn": "o"}]}),
    );
    let revisions = json!({
        "policy_revision": policy_revision(
            table["revision"].as_str().unwrap(), death["revision"].as_str().unwrap()).unwrap(),
        "experience_table_revision": table["revision"],
        "death_policy_revision": death["revision"],
        "reward_revision": reward["revision"],
        "declaration": differences["revision"],
        "evidence": table["evidence_revision"],
        "simulation": PROGRESSION_SIMULATION_REVISION,
    });
    json!({
        "schema": PROGRESSION_SECTION_SCHEMA,
        "experience_table": table,
        "death_policy": death,
        "reward_policy": reward,
        "declared_differences": differences,
        "revisions": revisions,
    })
}

/// Canonical section bytes for the native gameplay envelope tests.
pub(crate) fn section_bytes() -> Vec<u8> {
    canonical(&section_value()).unwrap()
}

fn bytes(value: &Value) -> Vec<u8> {
    canonical(value).unwrap()
}

#[test]
fn decodes_canonical_section() {
    let content = CharacterProgressionContent::decode(&bytes(&section_value())).unwrap();
    assert_eq!(content.thresholds.len(), CHARACTER_EXPERIENCE_TABLE_LEVELS);
    assert_eq!(content.thresholds[0].minimum_experience.get(), 0);
    assert_eq!(content.thresholds[1].minimum_experience.get(), 100);
    assert_eq!(content.simulation(), PROGRESSION_SIMULATION_REVISION);
    assert!(content.policy_for("p", "r", "c").is_ok());
}

#[test]
fn refuses_non_canonical_and_tampered() {
    let good = bytes(&section_value());
    let mut spaced = good.clone();
    spaced.push(b'\n');
    assert!(CharacterProgressionContent::decode(&spaced).is_err());
    assert!(CharacterProgressionContent::decode(&[]).is_err());
    let text = String::from_utf8(good).unwrap();
    let tampered = text.replacen("\"100\"", "\"101\"", 1);
    assert!(CharacterProgressionContent::decode(tampered.as_bytes()).is_err());
}

fn refused(edit: impl FnOnce(&mut Value)) -> bool {
    let mut v = section_value();
    edit(&mut v);
    CharacterProgressionContent::decode(&bytes(&v)).is_err()
}

#[test]
fn refuses_shape_and_revision_defects() {
    assert!(refused(|v| v["schema"] = json!("x")));
    assert!(refused(|v| v["revisions"]["simulation"] = json!("x")));
    assert!(refused(
        |v| v["revisions"]["policy_revision"] = json!("character-progression-policy-v1-00")
    ));
    assert!(refused(
        |v| v["revisions"]["evidence"] = json!("experience-table-evidence-20261005-1")
    ));
    assert!(refused(|v| {
        v["experience_table"]["levels"]
            .as_array_mut()
            .unwrap()
            .pop();
    }));
    assert!(refused(|v| {
        v["experience_table"]["levels"]
            .as_array_mut()
            .unwrap()
            .push(json!({"level":2001,"minimum_experience":"1"}));
    }));
    assert!(refused(|v| v["death_policy"]["loss_numerator"] = json!(2)));
    assert!(refused(|v| v["death_policy"]["rounding"] = json!("ceil")));
    assert!(refused(|v| v["unknown"] = json!(1)));
}

#[test]
fn refuses_non_monotonic_table_even_with_matching_revisions() {
    let mut v = section_value();
    v["experience_table"]["levels"][5]["minimum_experience"] = json!("0");
    let table = with_revision(PREFIX_TABLE, {
        let mut t = v["experience_table"].clone();
        t.as_object_mut().unwrap().remove("revision");
        t
    });
    v["revisions"]["experience_table_revision"] = table["revision"].clone();
    v["revisions"]["policy_revision"] = json!(
        policy_revision(
            table["revision"].as_str().unwrap(),
            v["death_policy"]["revision"].as_str().unwrap()
        )
        .unwrap()
    );
    v["experience_table"] = table;
    assert!(CharacterProgressionContent::decode(&bytes(&v)).is_err());
}

#[test]
fn strict_parse_refuses_profile_violations() {
    for bad in [
        r#"{"a":1,"a":2}"#,
        r#"{"a":{"b":1,"b":1}}"#,
        r#"{"a":1.5}"#,
        r#"{"a":1e2}"#,
        r#"{"a":-1}"#,
        r#"{"a":null}"#,
        r#"{"é":1}"#,
        r#"{"a":9007199254740992}"#,
        r#"{"a":"\ud800"}"#,
    ] {
        assert!(strict_parse(bad.as_bytes()).is_err(), "{bad}");
    }
    assert!(strict_parse(br#"{"a":9007199254740991}"#).is_ok());
}

#[test]
fn canonical_vectors_match_producer() {
    let vectors: Value = serde_json::from_str(include_str!(
        "../../../../tools/content-schema/character-progression/canonical-vectors.json"
    ))
    .unwrap();
    for case in vectors["cases"].as_array().unwrap() {
        let input = case["input"].as_str().unwrap();
        let expected = case["canonical"].as_str().unwrap();
        let out = canonical(&strict_parse(input.as_bytes()).unwrap()).unwrap();
        assert_eq!(String::from_utf8(out).unwrap(), expected);
    }
}
