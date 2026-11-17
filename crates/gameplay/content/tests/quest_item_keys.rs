use serde_json::Value;
use std::{collections::BTreeMap, error::Error, fs, path::Path};

fn documents(directory: &Path) -> Result<Vec<Value>, Box<dyn Error>> {
    let mut values = Vec::new();
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path.is_dir() {
            values.extend(documents(&path)?);
        } else if path
            .extension()
            .is_some_and(|extension| extension == "json")
        {
            values.push(serde_json::from_slice(&fs::read(path)?)?);
        }
    }
    Ok(values)
}

fn check_references(value: &Value, items: &BTreeMap<String, Value>) -> usize {
    match value {
        Value::Object(object) => {
            let mut count = 0;
            if let Some(item) = object.get("item").filter(|item| {
                item["family"] == "Item"
                    && item["key"]
                        .as_str()
                        .is_some_and(|key| key.starts_with("oteryn:"))
            }) {
                let key = item["key"].as_str().unwrap_or_default();
                assert_eq!(items.get(key), Some(item), "missing or stale Item: {key}");
                count += 1;
            }
            count
                + object
                    .values()
                    .map(|v| check_references(v, items))
                    .sum::<usize>()
        }
        Value::Array(values) => values.iter().map(|v| check_references(v, items)).sum(),
        _ => 0,
    }
}

#[test]
fn regenerated_quest_item_keys_resolve() -> Result<(), Box<dyn Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let mut items = BTreeMap::new();
    for document in documents(&root.join("content/items/definitions"))? {
        if let Some(records) = document["records"].as_array() {
            for record in records {
                let identity = &record["definition"]["identity"];
                if let Some(key) = identity["key"].as_str() {
                    items.insert(key.to_owned(), identity.clone());
                }
            }
        }
    }
    let mut references = 0;
    for document in documents(&root.join("content/quests"))? {
        references += check_references(&document, &items);
    }
    assert!(
        references > 0,
        "quest producer must emit resolved Item references"
    );
    Ok(())
}
