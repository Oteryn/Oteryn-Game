//! Only the exact, source-published SRX-3 static records get this map-only route.
//! Their canonical family stays WorldObject; no Item reference is created.
use super::*;
use sha2::{Digest, Sha256};
use std::sync::OnceLock;
#[path = "../../../../crates/gameplay/content/src/content/activity_static_surface_pin.rs"]
mod pin;

#[derive(Debug, Clone, Copy)]
pub(super) struct StaticSurface {
    pub solid: bool,
    pub ground: bool,
    pub blocks_projectile: bool,
}

fn expected() -> Option<&'static BTreeMap<String, (Value, StaticSurface)>> {
    static RECORDS: OnceLock<Option<BTreeMap<String, (Value, StaticSurface)>>> = OnceLock::new();
    RECORDS
        .get_or_init(|| {
            let bytes = include_bytes!("../../../../content/world/source-static-definitions.json");
            if Sha256::digest(bytes)
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
                != pin::REGISTRY_SHA256
            {
                return None;
            }
            let registry: Value = serde_json::from_slice(bytes).ok()?;
            let mut records = BTreeMap::new();
            for row in registry["records"].as_array()? {
                let record = row["catalogue_record"].clone();
                let key = row["identity"]["key"].as_str()?;
                if record["identity"] != row["identity"] {
                    return None;
                }
                records.insert(
                    key.to_owned(),
                    (
                        record,
                        StaticSurface {
                            solid: row["policy"]["block_solid"].as_bool()?,
                            ground: row["policy"]["ground"].as_bool()?,
                            blocks_projectile: row["policy"]["block_projectile"].as_bool()?,
                        },
                    ),
                );
            }
            if records.len() != 238 {
                return None;
            }
            Some(records)
        })
        .as_ref()
}

pub(super) fn qualify(
    family: &str,
    bytes: &[u8],
) -> Result<BTreeMap<String, StaticSurface>, Error> {
    let document: Value =
        serde_json::from_slice(bytes).map_err(|error| format(error.to_string()))?;
    let expected = expected().ok_or_else(|| format("source static registry"))?;
    let mut qualified = BTreeMap::new();
    for record in document["records"]
        .as_array()
        .ok_or_else(|| format("catalogue records"))?
    {
        let Some(key) = record["identity"]["key"].as_str() else {
            continue;
        };
        let Some((expected, facts)) = expected.get(key) else {
            continue;
        };
        if expected != record || record["identity"]["family"].as_str() != Some(family) {
            return Err(Error::Key(format!(
                "source static catalogue body changed: {key}"
            )));
        }
        qualified.insert(key.to_owned(), *facts);
    }
    Ok(qualified)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn m25_source_static_registry_includes_exact_donor_cohort() -> Result<(), Error> {
        let records = expected().ok_or_else(|| format("source-pinned static registry"))?;
        assert_eq!(records.len(), 238);
        for id in pin::M25_SOURCE_IDS {
            let terrain = format!("oteryn:terrain.tibia.i{id}");
            let object = format!("oteryn:world-object.tibia.i{id}");
            assert_ne!(
                records.contains_key(&terrain),
                records.contains_key(&object)
            );
        }
        Ok(())
    }
}
