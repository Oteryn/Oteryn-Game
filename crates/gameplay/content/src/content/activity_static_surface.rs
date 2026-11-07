//! Closed source-owned static activity geometry. These keys grant no Item identity.
use serde::Deserialize;
use std::collections::BTreeMap;
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StaticSurfaceFacts {
    pub source_id: u16,
    pub block_solid: bool,
    pub blocks_projectile: bool,
    pub ground: Option<(bool, u16)>,
}
#[derive(Deserialize)]
struct Registry {
    source_revision: String,
    technical_terrain_keys: Vec<String>,
    records: Vec<Record>,
}
#[derive(Deserialize)]
struct Record {
    source_item_id: u16,
    identity: Identity,
    policy: Policy,
}
#[derive(Deserialize)]
struct Identity {
    family: String,
    key: String,
    revision: String,
}
#[derive(Deserialize)]
struct Policy {
    ground: bool,
    ground_speed: u16,
    block_solid: bool,
    block_projectile: bool,
    movable: bool,
    pickupable: bool,
    dynamic_kind: Option<String>,
}
fn records() -> &'static BTreeMap<String, StaticSurfaceFacts> {
    static RECORDS: OnceLock<BTreeMap<String, StaticSurfaceFacts>> = OnceLock::new();
    RECORDS.get_or_init(|| {
        let registry: Registry = serde_json::from_slice(include_bytes!(
            "../../../../../content/encounters/source-static-definitions.json"
        ))
        .expect("repository-generated bounded static surface registry");
        assert_eq!(
            registry.source_revision,
            "201189f871d23597b82caf88fd611454a2ddefb8"
        );
        assert_eq!(registry.records.len(), 162);
        registry
            .records
            .into_iter()
            .map(|record| {
                let family = if record.policy.ground {
                    "Terrain"
                } else {
                    "WorldObject"
                };
                let prefix = if record.policy.ground {
                    "terrain"
                } else {
                    "world-object"
                };
                assert_eq!(record.identity.family, family);
                assert_eq!(record.identity.revision, "definition-r1");
                assert_eq!(
                    record.identity.key,
                    format!("oteryn:{prefix}.tibia.i{}", record.source_item_id)
                );
                assert!(
                    !record.policy.movable
                        && !record.policy.pickupable
                        && record.policy.dynamic_kind.is_none()
                );
                (
                    record.identity.key,
                    StaticSurfaceFacts {
                        source_id: record.source_item_id,
                        block_solid: record.policy.block_solid,
                        blocks_projectile: record.policy.block_projectile,
                        ground: record
                            .policy
                            .ground
                            .then_some((!record.policy.block_solid, record.policy.ground_speed)),
                    },
                )
            })
            .collect()
    })
}
pub fn qualified_static_surface(key: &str) -> Option<StaticSurfaceFacts> {
    records().get(key).copied()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn static_primal_decoration_has_no_item_identity() {
        let facts = qualified_static_surface("oteryn:world-object.tibia.i39095").unwrap();
        assert_eq!(facts.source_id, 39095);
        assert_eq!(facts.ground, None);
        assert_eq!(qualified_static_surface("oteryn:item.tibia.i39095"), None);
        assert_eq!(
            qualified_static_surface("oteryn:world-object.tibia.i39096"),
            None
        );
    }
}

/// A source-only static binding never substitutes for a native Item reference.
pub fn qualified_static_binding(family: &str, key: &str, revision: &str, source_id: u32) -> bool {
    qualified_static_surface(key).is_some_and(|facts| {
        revision == "definition-r1"
            && u32::from(facts.source_id) == source_id
            && family
                == if facts.ground.is_some() {
                    "Terrain"
                } else {
                    "WorldObject"
                }
    })
}

/// Compact map-only definition id shared with the bundle compiler. Never an ItemRef.
pub fn static_surface_definition_id(key: &str) -> Option<u32> {
    qualified_static_surface(key)?;
    static IDS: OnceLock<BTreeMap<String, u32>> = OnceLock::new();
    IDS.get_or_init(|| {
        let registry: Registry = serde_json::from_slice(include_bytes!(
            "../../../../../content/encounters/source-static-definitions.json"
        ))
        .expect("repository-generated static map definition index");
        assert!(
            registry
                .technical_terrain_keys
                .windows(2)
                .all(|keys| keys[0] < keys[1])
        );
        registry
            .technical_terrain_keys
            .into_iter()
            .enumerate()
            .map(|(index, key)| {
                (
                    key,
                    u32::try_from(index).expect("bounded map definition index"),
                )
            })
            .collect()
    })
    .get(key)
    .copied()
}
