use super::{CAPABILITY_REQUIREMENTS_V1, REGISTERED_CAPABILITY_IDS_V1, build_capabilities};
use crate::capability_tables::{
    BUNDLE_WORLD_OFFERS_V1, GATED, ITEM_VIEW_OFFERS_V1, PRODUCTION_OFFERS_V1,
};
use serde_json::{Value, json};
use std::error::Error;
use std::path::Path;

fn facts() -> Result<Vec<Value>, Box<dyn Error>> {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("capabilities");
    let mut entries = Vec::new();
    for item in std::fs::read_dir(directory)? {
        let item = item?;
        if item.path().extension().is_some_and(|ext| ext == "json") {
            entries.push(build_capabilities::parse(
                item.file_name().to_str().ok_or("filename")?,
                &std::fs::read_to_string(item.path())?,
            )?);
        }
    }
    entries.sort_by_key(|entry| entry["id"].as_u64());
    Ok(entries)
}

fn ids(value: &Value) -> Result<Vec<u32>, Box<dyn Error>> {
    value
        .as_array()
        .ok_or("ids")?
        .iter()
        .map(|id| Ok(u32::try_from(id.as_u64().ok_or("id")?)?))
        .collect()
}

#[test]
fn capability_tables_are_all_fragment_facts() -> Result<(), Box<dyn Error>> {
    let entries = facts()?;
    assert_eq!(
        REGISTERED_CAPABILITY_IDS_V1,
        ids(&Value::Array(
            entries.iter().map(|e| e["id"].clone()).collect()
        ))?
    );
    let requirements: Vec<_> = entries
        .iter()
        .filter(|e| e["acceptance_requires"] == true)
        .map(|e| {
            Ok((
                u32::try_from(e["id"].as_u64().ok_or("id")?)?,
                ids(&e["requires"])?,
            ))
        })
        .collect::<Result<_, Box<dyn Error>>>()?;
    let pairs = |table: &[(u32, &[u32])]| {
        table
            .iter()
            .map(|(id, requires)| (*id, requires.to_vec()))
            .collect::<Vec<_>>()
    };
    assert_eq!(pairs(CAPABILITY_REQUIREMENTS_V1), requirements);
    let gates: Vec<_> = entries
        .iter()
        .filter(|e| !e["domains"].is_null())
        .map(|e| {
            Ok((
                u32::try_from(e["id"].as_u64().ok_or("id")?)?,
                ids(&e["commands"])?,
                ids(&e["domains"])?,
            ))
        })
        .collect::<Result<_, Box<dyn Error>>>()?;
    assert_eq!(
        GATED
            .iter()
            .map(|(id, commands, domains)| (*id, commands.to_vec(), domains.to_vec()))
            .collect::<Vec<_>>(),
        gates
    );
    for (set, table) in [
        ("production", PRODUCTION_OFFERS_V1),
        ("item_view", ITEM_VIEW_OFFERS_V1),
        ("bundle_world", BUNDLE_WORLD_OFFERS_V1),
    ] {
        let expected: Vec<_> = entries
            .iter()
            .filter(|e| e["offers"][set] == true)
            .map(|e| {
                Ok((
                    u32::try_from(e["id"].as_u64().ok_or("id")?)?,
                    ids(&e["requires"])?,
                ))
            })
            .collect::<Result<_, Box<dyn Error>>>()?;
        assert_eq!(pairs(table), expected, "{set}");
    }
    Ok(())
}

#[test]
fn caps_split_1_golden_preserves_every_current_capability_fact() -> Result<(), Box<dyn Error>> {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/tests/capabilities_golden");
    let mut expected: Vec<Value> = Vec::new();
    for item in std::fs::read_dir(directory)? {
        let item = item?;
        if item.path().extension().is_some_and(|ext| ext == "json") {
            expected.push(serde_json::from_str(&std::fs::read_to_string(
                item.path(),
            )?)?);
        }
    }
    expected.sort_by_key(|entry| entry["id"].as_u64());
    assert_eq!(facts()?, expected);
    Ok(())
}

#[test]
fn capability_generator_is_deterministic_and_rejects_invalid_registration()
-> Result<(), Box<dyn Error>> {
    let entries = facts()?;
    let registry: Value = serde_json::from_str(crate::PROTOCOL_OTERYN_V1_REGISTRY_JSON)?;
    let expected = build_capabilities::render(&entries, &registry)?;
    let mut reversed = entries.clone();
    reversed.reverse();
    assert_eq!(build_capabilities::render(&reversed, &registry)?, expected);
    for (field, value) in [
        ("id", json!(0)),
        ("id", json!(u64::from(u32::MAX) + 1)),
        ("name", json!("WRONG_NAME")),
        ("constant", json!("bad; injection")),
        ("requires", json!([999])),
        ("commands", json!([5, 4])),
        ("domains", json!([999])),
        ("acceptance_requires", Value::Null),
        ("offers", json!({})),
        ("supersedes", json!([2])),
    ] {
        let mut bad = entries.clone();
        bad[0][field] = value;
        assert!(
            build_capabilities::render(&bad, &registry).is_err(),
            "{field}"
        );
    }
    let mut duplicate = entries.clone();
    duplicate.push(entries[0].clone());
    assert!(build_capabilities::render(&duplicate, &registry).is_err());
    assert!(build_capabilities::render(&entries[1..], &registry).is_err());
    let mut unsupported = entries.clone();
    unsupported[0]["offers"]["bundle_world"] = json!("true");
    assert!(build_capabilities::render(&unsupported, &registry).is_err());
    let mut unclosed = entries.clone();
    unclosed.iter_mut().find(|e| e["id"] == 6).ok_or("6")?["offers"]["bundle_world"] = json!(false);
    assert!(build_capabilities::render(&unclosed, &registry).is_err());
    let mut ownership = registry.clone();
    ownership["command_types"]
        .as_array_mut()
        .ok_or("commands")?
        .iter_mut()
        .find(|e| e["id"] == 4)
        .ok_or("4")?["capability"] = json!(3);
    assert!(build_capabilities::render(&entries, &ownership).is_err());
    assert!(build_capabilities::parse("0002-wrong.json", &entries[0].to_string()).is_err());
    assert!(build_capabilities::parse("0001-bestiary-charms-v1.json", "{broken").is_err());
    Ok(())
}

#[test]
fn adding_a_capability_command_and_domain_needs_only_new_fragments() -> Result<(), Box<dyn Error>> {
    let mut entries = facts()?;
    let mut registry: Value = serde_json::from_str(crate::PROTOCOL_OTERYN_V1_REGISTRY_JSON)?;
    let new = json!({
        "id": 1000, "name": "FRAGMENT_TEST", "constant": "fragment_test::CAPABILITY",
        "requires": [6], "acceptance_requires": true, "commands": [1000], "domains": [1000],
        "supersedes": [], "offers": {"production": true, "item_view": true, "bundle_world": true}
    });
    entries.push(build_capabilities::parse(
        "1000-fragment-test.json",
        &new.to_string(),
    )?);
    registry["capabilities"]
        .as_array_mut()
        .ok_or("capabilities")?
        .push(json!({
            "id": 1000, "name": "FRAGMENT_TEST", "requires": [6], "command_types": [1000],
            "state_domains": [1000], "offered": true
        }));
    for collection in ["command_types", "state_domains"] {
        registry[collection]
            .as_array_mut()
            .ok_or("collection")?
            .push(json!({"id": 1000, "capability": 1000}));
    }
    let (registered, tables) = build_capabilities::render(&entries, &registry)?;
    assert!(registered.contains("    1000,\n"));
    assert!(registered.contains("    (1000, &[6]),\n"));
    assert!(registered.contains("assert!(crate::fragment_test::CAPABILITY == 1000)"));
    assert!(tables.contains("    (1000, &[1000], &[1000]),\n"));
    assert_eq!(tables.matches("    (1000, &[6]),\n").count(), 3);
    // A bundle-only prerequisite cannot satisfy a production offer's requirements.
    let mut missing = entries.clone();
    missing
        .iter_mut()
        .find(|entry| entry["id"] == 1000)
        .ok_or("1000")?["requires"] = json!([3]);
    registry["capabilities"]
        .as_array_mut()
        .ok_or("capabilities")?
        .iter_mut()
        .find(|entry| entry["id"] == 1000)
        .ok_or("1000")?["requires"] = json!([3]);
    assert!(build_capabilities::render(&missing, &registry).is_err());
    Ok(())
}
