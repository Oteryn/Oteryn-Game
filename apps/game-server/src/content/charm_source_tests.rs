use super::*;
use serde_json::Value;

type TestResult = Result<(), Box<dyn std::error::Error>>;
const INDEX: &[u8] = include_bytes!("../../../../content/charms/index.json");
const SHARD: &[u8] = include_bytes!("../../../../content/charms/charms-00000-00024.json");

#[test]
fn embedded_server_import_retains_the_complete_canonical_catalogue() -> TestResult {
    let imported = CanonicalCharmCatalogue::embedded()?;
    let expected = decode(SHARD)?;
    assert_eq!(imported.catalogue(), expected.catalogue());
    assert_eq!(imported.source_bytes(), (INDEX, SHARD));
    assert_eq!(imported.source_digest(), expected.source_digest());
    for row in expected.catalogue().definitions() {
        assert_eq!(
            imported.charm(row.key.as_str()),
            expected.charm(row.key.as_str())
        );
    }
    // Missing gameplay consumers must not filter static imported definitions.
    assert!(imported.charm("oteryn:charm.cleanse").is_some());
    assert!(imported.charm("oteryn:charm.scavenge").is_some());
    assert_eq!(imported.catalogue().definitions().count(), 25);
    Ok(())
}

// Test budgets are measured from these input bytes, not a new production profile.
fn budgets(shard: &[u8]) -> Result<[usize; 8], Box<dyn std::error::Error>> {
    fn maxima(value: &Value, key: &mut usize, text: &mut usize) {
        match value {
            Value::String(value) => *text = (*text).max(value.len()),
            Value::Object(values) => {
                for (name, value) in values {
                    *key = (*key).max(name.len());
                    maxima(value, key, text);
                }
            }
            Value::Array(values) => {
                for value in values {
                    maxima(value, key, text);
                }
            }
            _ => {}
        }
    }
    let index: Value = serde_json::from_slice(INDEX)?;
    let source: Value = serde_json::from_slice(shard)?;
    let mut key = 0;
    let mut text = 0;
    maxima(&index, &mut key, &mut text);
    maxima(&source, &mut key, &mut text);
    let shard: Shard = serde_json::from_slice(shard)?;
    for row in &shard.records {
        let definition: Definition = serde_json::from_str(row.definition.get())?;
        key = key.max(definition.identity.key.len());
    }
    Ok([
        INDEX.len() + SHARD.len(),
        2,
        INDEX.len().max(SHARD.len()),
        COUNT,
        shard
            .records
            .iter()
            .map(|row| row.definition.get().len())
            .max()
            .ok_or("records")?,
        key,
        text,
        COUNT,
    ])
}
fn limits(values: [usize; 8]) -> Result<EvidenceLimits, ContentError> {
    EvidenceLimits::new(
        "evidence:charm-source-test",
        values[0],
        values[1],
        values[2],
        values[3],
        values[4],
        values[5],
        values[6],
        values[7],
        1,
        1,
    )
}
fn decode(shard: &[u8]) -> Result<CanonicalCharmCatalogue, Box<dyn std::error::Error>> {
    let mut values = budgets(SHARD)?;
    // Mutations need byte/text headroom derived from their actual data.
    values[0] = INDEX.len() + shard.len();
    values[2] = INDEX.len().max(shard.len());
    values[4] = shard.len();
    values[5] = shard.len();
    values[6] = shard.len();
    Ok(decode_canonical_charms(
        INDEX,
        &[(SHARD_PATH, shard)],
        &limits(values)?,
    )?)
}
fn mutated(change: impl FnOnce(&mut Value)) -> Result<Vec<u8>, serde_json::Error> {
    let mut source: Value = serde_json::from_slice(SHARD)?;
    change(&mut source);
    serde_json::to_vec(&source)
}

fn reject_mutation(change: impl FnOnce(&mut Value)) -> TestResult {
    let bytes = mutated(change)?;
    assert!(decode(&bytes).is_err());
    Ok(())
}

#[test]
fn all_canonical_rows_share_domain_costs_and_exact_combat_stages() -> TestResult {
    let decoded = decode(SHARD)?;
    let source: Shard = serde_json::from_slice(SHARD)?;
    assert_eq!(decoded.catalogue().definitions().count(), COUNT);
    assert_eq!(
        decoded
            .catalogue()
            .definitions()
            .filter(|row| row.category == DomainCategory::Major)
            .count(),
        14
    );
    for record in source.records {
        let row: Definition = serde_json::from_str(record.definition.get())?;
        let key = CharmKey::new(&row.identity.key).map_err(|error| format!("{error:?}"))?;
        let domain = decoded
            .catalogue()
            .get(&key)
            .map_err(|error| format!("{error:?}"))?;
        let combat = decoded.charm(&row.identity.key).ok_or("combat")?;
        let major = row.category == "major";
        assert_eq!(domain.category == DomainCategory::Major, major);
        assert_eq!(combat.category() == CharmCategory::Major, major);
        assert_eq!(combat.effect(), lower_effect(&row.effect)?);
        for (index, stage) in row.stages.into_iter().enumerate() {
            assert_eq!(domain.stage_costs[index], stage.cost);
            let value = combat.stage(stage.stage).ok_or("stage")?;
            assert_eq!(value.hundredths(), stage.value.0);
        }
    }
    assert_eq!(decoded.source_bytes(), (INDEX, SHARD));
    assert_eq!(decoded.source_digest(), decode(SHARD)?.source_digest());
    Ok(())
}

#[test]
fn source_reference_ids_and_display_names_do_not_define_runtime_identity() -> TestResult {
    let original = decode(SHARD)?;
    for mode in 0..4 {
        let bytes = mutated(|source| {
            for (index, row) in source["records"]
                .as_array_mut()
                .into_iter()
                .flatten()
                .enumerate()
            {
                let definition = &mut row["definition"];
                match mode {
                    0 => {
                        if let Some(members) = definition["sources"].as_object_mut() {
                            members.remove("canary");
                        }
                    }
                    1 => definition["sources"]["canary"]["charm_id"] = Value::from(index + 1000),
                    2 => definition["sources"]["canary"]["charm_id"] = Value::from(0),
                    _ => definition["name"] = Value::String("Same new display name".into()),
                }
            }
        })?;
        let changed = decode(&bytes)?;
        assert_eq!(original.catalogue(), changed.catalogue());
        for domain in original.catalogue().definitions() {
            assert_eq!(
                original.charm(domain.key.as_str()),
                changed.charm(domain.key.as_str())
            );
        }
        assert_ne!(original.source_digest(), changed.source_digest());
        assert_eq!(changed.source_bytes().1, bytes);
    }
    Ok(())
}

#[test]
fn closed_shapes_registration_costs_and_percentages_reject_malformed_inputs() -> TestResult {
    for (field, value) in [
        ("cost_currency", Value::String("charm_points".into())),
        ("category", Value::String("unsupported".into())),
        ("stage_value", Value::String("effect_percent".into())),
        ("kind", Value::String("unsupported".into())),
        ("invented_active_flag", Value::Bool(true)),
    ] {
        reject_mutation(|source| source["records"][0]["definition"][field] = value)?;
    }
    for (field, value) in [("stage", 2), ("cost", 0), ("value", 0), ("cost", 100_001)] {
        let index = if value == 100_001 { 2 } else { 0 };
        reject_mutation(|source| {
            source["records"][0]["definition"]["stages"][index][field] = Value::from(value)
        })?;
    }
    for (index, field, value) in [(1, "cost", 100), (1, "value", 6), (2, "value", 101)] {
        reject_mutation(|source| {
            source["records"][0]["definition"]["stages"][index][field] = Value::from(value)
        })?;
    }
    reject_mutation(|source| source["records"][1] = source["records"][0].clone())?;
    for extra in [false, true] {
        let bytes = mutated(|source| {
            let record = source["records"][0].clone();
            if let Some(rows) = source["records"].as_array_mut() {
                if extra {
                    rows.push(record);
                } else {
                    rows.pop();
                }
            }
        })?;
        assert!(decode(&bytes).is_err());
    }
    let raw = std::str::from_utf8(SHARD)?;
    for token in [
        "0.80000000000000001",
        "0.801",
        "8e-1",
        "-0.8",
        "00.8",
        "0.",
        "4294967296",
        "\"0.8\"",
        "{\"hundredths\":80}",
    ] {
        let bytes = raw.replacen("\"value\":0.8", &format!("\"value\":{token}"), 1);
        assert_ne!(bytes, raw);
        assert!(decode(bytes.as_bytes()).is_err(), "{token}");
    }
    for (needle, replacement) in [
        ("\"value\":0.8", "\"value\":0.8,\"v\\u0061lue\":0.8"),
        ("\"cost\":100", "\"cost\":100,\"cost\":100"),
    ] {
        let bytes = raw.replacen(needle, replacement, 1);
        assert_ne!(bytes, raw);
        assert!(decode(bytes.as_bytes()).is_err(), "duplicate {needle}");
    }
    let escaped = raw.replacen("\"value\":0.8", "\"v\\u0061lue\":0.8", 1);
    assert_eq!(
        decode(escaped.as_bytes())?
            .charm("oteryn:charm.voids_call")
            .ok_or("Void")?
            .stage(1)
            .ok_or("stage")?
            .hundredths(),
        80
    );
    assert!(
        decode(&mutated(
            |source| source["records"][0]["definition"]["sources"]["note"] =
                Value::String("\\\"value\\\":0.80000000000000001; slash\\; Unicode λ".into())
        )?)
        .is_ok()
    );
    let caps = limits(budgets(SHARD)?)?;
    assert!(decode_canonical_charms(INDEX, &[], &caps).is_err());
    assert!(decode_canonical_charms(INDEX, &[("../substitute.json", SHARD)], &caps).is_err());
    let mut index: Value = serde_json::from_slice(INDEX)?;
    index["record_count"] = Value::from(24);
    assert!(
        decode_canonical_charms(&serde_json::to_vec(&index)?, &[(SHARD_PATH, SHARD)], &caps)
            .is_err()
    );
    Ok(())
}

#[test]
fn caller_limits_accept_actual_maxima_and_reject_each_max_plus_one() -> TestResult {
    let maxima = budgets(SHARD)?;
    assert!(decode_canonical_charms(INDEX, &[(SHARD_PATH, SHARD)], &limits(maxima)?).is_ok());
    for index in 0..maxima.len() {
        let mut smaller = maxima;
        smaller[index] -= 1;
        assert!(
            matches!(decode_canonical_charms(INDEX, &[(SHARD_PATH, SHARD)], &limits(smaller)?), Err(ContentError::LimitExceeded { actual, limit, .. }) if actual == limit + 1),
            "budget {index}"
        );
    }
    let raw = std::str::from_utf8(SHARD)?;
    let large = "x".repeat(maxima[6] + 1);
    let duplicate = raw.replacen(
        "\"sources\":{",
        &format!("\"sources\":{{\"note\":\"{large}\",\"n\\u006fte\":\"x\","),
        1,
    );
    assert_ne!(duplicate, raw);
    let mut headroom = maxima;
    headroom[0] = INDEX.len() + duplicate.len();
    headroom[2] = duplicate.len();
    headroom[4] = duplicate.len();
    assert!(
        decode_canonical_charms(
            INDEX,
            &[(SHARD_PATH, duplicate.as_bytes())],
            &limits(headroom)?
        )
        .is_err()
    );
    Ok(())
}
