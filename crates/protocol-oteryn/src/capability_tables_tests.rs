use super::*;
use crate::world_spatial_entities::CAPABILITY_WORLD_SPATIAL_ENTITIES;
use serde_json::Value;
use std::error::Error;

const REGISTRY: &str = crate::PROTOCOL_OTERYN_V1_REGISTRY_JSON;

fn registry_capabilities() -> Result<Vec<Value>, Box<dyn Error>> {
    let registry: Value = serde_json::from_str(REGISTRY)?;
    Ok(registry["capabilities"]
        .as_array()
        .ok_or("capabilities")?
        .clone())
}

fn ids(value: &Value) -> Result<Vec<u32>, Box<dyn Error>> {
    match value {
        Value::Null => Ok(Vec::new()),
        Value::Array(items) => items
            .iter()
            .map(|item| {
                item.as_u64()
                    .and_then(|id| u32::try_from(id).ok())
                    .ok_or_else(|| "id".into())
            })
            .collect(),
        _ => Err("ids".into()),
    }
}

#[test]
fn the_gated_commands_and_domains_are_the_registry_ones() -> Result<(), Box<dyn Error>> {
    let mut expected = Vec::new();
    for capability in registry_capabilities()? {
        let id = u32::try_from(capability["id"].as_u64().ok_or("id")?)?;
        // Capability 6 extends core domain 1 with a payload type; it gates no whole domain.
        if id == CAPABILITY_WORLD_SPATIAL_ENTITIES {
            assert!(ids(&capability["command_types"])?.is_empty());
            assert_eq!(ids(&capability["state_domains"])?, [1]);
            continue;
        }
        expected.push((
            id,
            ids(&capability["command_types"])?,
            ids(&capability["state_domains"])?,
        ));
    }
    let table: Vec<(u32, Vec<u32>, Vec<u32>)> = GATED
        .iter()
        .map(|(id, commands, domains)| (*id, commands.to_vec(), domains.to_vec()))
        .collect();
    assert_eq!(table, expected);
    Ok(())
}
