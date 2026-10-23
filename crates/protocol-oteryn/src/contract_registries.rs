//! The contract registries as one JSON text each, assembled by `build.rs` from their fragment
//! files (REGISTRY-SPLIT-1). Read these instead of the fragment files.

/// `PROTOCOL_OTERYN_V1_REGISTRY.json`: `docs/contracts/protocol-oteryn/v1/registry/registry.json`
/// with its `capabilities`, `command_types` and `state_domains` fragments.
pub const PROTOCOL_OTERYN_V1_REGISTRY_JSON: &str = include_str!(concat!(
    env!("OUT_DIR"),
    "/PROTOCOL_OTERYN_V1_REGISTRY.json"
));

/// `RESOURCE_LIMITS_REGISTRY.json`: `docs/contracts/resource-limits/registry.json` with its
/// `entries` fragments.
pub const RESOURCE_LIMITS_REGISTRY_JSON: &str =
    include_str!(concat!(env!("OUT_DIR"), "/RESOURCE_LIMITS_REGISTRY.json"));

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::{PROTOCOL_OTERYN_V1_REGISTRY_JSON, RESOURCE_LIMITS_REGISTRY_JSON};
    use serde_json::Value;
    use std::collections::BTreeSet;
    use std::path::Path;

    fn fragment_count(directory: &Path) -> usize {
        std::fs::read_dir(directory)
            .expect("fragment directory")
            .filter(|item| {
                item.as_ref()
                    .expect("fragment entry")
                    .file_name()
                    .to_string_lossy()
                    .ends_with(".json")
            })
            .count()
    }

    /// The aggregate is the base file plus every fragment file, in order, and nothing else.
    fn check(aggregate: &str, base: &str, collections: &[&str], ascending_ids: bool) {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let base_path = root.join(base);
        let base: Value =
            serde_json::from_str(&std::fs::read_to_string(&base_path).expect("base file"))
                .expect("base JSON");
        let aggregate: Value = serde_json::from_str(aggregate).expect("aggregate JSON");
        let (aggregate, base) = (
            aggregate.as_object().expect("aggregate object"),
            base.as_object().expect("base object"),
        );
        assert_eq!(
            aggregate.keys().collect::<Vec<_>>(),
            base.keys().collect::<Vec<_>>()
        );
        for (key, value) in base {
            if !collections.contains(&key.as_str()) {
                assert_eq!(&aggregate[key], value, "`{key}` is the base's");
                continue;
            }
            let entries = aggregate[key].as_array().expect("collection");
            let directory = base_path.parent().expect("base directory").join(key);
            assert_eq!(
                entries.len(),
                fragment_count(&directory),
                "`{key}` fragments"
            );
            let ids: Vec<String> = entries
                .iter()
                .map(|entry| entry["id"].to_string().to_lowercase())
                .collect();
            assert_eq!(
                ids.iter().collect::<BTreeSet<_>>().len(),
                ids.len(),
                "`{key}` ids are unique"
            );
            if ascending_ids {
                let numbers: Vec<u64> = entries
                    .iter()
                    .map(|entry| entry["id"].as_u64().expect("numeric id"))
                    .collect();
                assert!(
                    numbers.windows(2).all(|pair| pair[0] < pair[1]),
                    "`{key}` is sorted by id"
                );
            }
        }
    }

    #[test]
    fn protocol_registry_is_its_base_and_fragments() {
        check(
            PROTOCOL_OTERYN_V1_REGISTRY_JSON,
            "docs/contracts/protocol-oteryn/v1/registry/registry.json",
            &["capabilities", "command_types", "state_domains"],
            true,
        );
    }

    #[test]
    fn resource_limits_registry_is_its_base_and_fragments() {
        check(
            RESOURCE_LIMITS_REGISTRY_JSON,
            "docs/contracts/resource-limits/registry.json",
            &["entries"],
            false,
        );
    }
}
