//! Only the exact, source-published SRX-3 static records get this map-only route.
//! Their canonical family stays WorldObject; no Item reference is created.
use super::*;
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy)]
pub(super) struct StaticSurface {
    pub solid: bool,
    pub ground: bool,
}

fn expected() -> &'static BTreeMap<String, (Value, StaticSurface)> {
    static RECORDS: OnceLock<BTreeMap<String, (Value, StaticSurface)>> = OnceLock::new();
    RECORDS.get_or_init(|| {
        let registry: Value = serde_json::from_slice(include_bytes!(
            "../../../../content/encounters/source-static-definitions.json"
        ))
        .expect("generated static source registry");
        let mut records = BTreeMap::new();
        for row in registry["records"].as_array().expect("records") {
            let record = row["catalogue_record"].clone();
            let key = row["identity"]["key"].as_str().expect("key");
            assert_eq!(record["identity"], row["identity"]);
            records.insert(
                key.to_owned(),
                (
                    record,
                    StaticSurface {
                        solid: row["policy"]["block_solid"]
                            .as_bool()
                            .expect("source solidity"),
                        ground: row["policy"]["ground"].as_bool().expect("source ground"),
                    },
                ),
            );
        }
        assert_eq!(records.len(), 162);
        records
    })
}

pub(super) fn qualify(
    family: &str,
    bytes: &[u8],
) -> Result<BTreeMap<String, StaticSurface>, Error> {
    let document: Value =
        serde_json::from_slice(bytes).map_err(|error| format(error.to_string()))?;
    let mut qualified = BTreeMap::new();
    for record in document["records"]
        .as_array()
        .ok_or_else(|| format("catalogue records"))?
    {
        let Some(key) = record["identity"]["key"].as_str() else {
            continue;
        };
        let Some((expected, facts)) = expected().get(key) else {
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
