#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
use super::*;
use serde_json::json;

const EVIDENCE_LAST_LEVEL: u32 = 2000;

fn level_xp(level: u64) -> u64 {
    let l = i128::from(level);
    u64::try_from(50 * (l.pow(3) - 6 * l.pow(2) + 17 * l - 12) / 3).unwrap()
}

pub(super) fn with_revision(prefix: &str, mut doc: Value) -> Value {
    let revision = document_revision(prefix, &doc).unwrap();
    doc["revision"] = Value::String(revision);
    doc
}

pub(super) fn section_value() -> Value {
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
pub fn section_bytes() -> Vec<u8> {
    canonical(&section_value()).unwrap()
}
