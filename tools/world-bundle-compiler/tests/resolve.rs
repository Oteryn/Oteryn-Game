//! MAP-BUNDLE-1b-2: palette keys resolved against the Item registry and the Terrain and
//! WorldObject catalogues (rulings #162 5910173902 Q1b/Q2a and 5915258560).

use std::error::Error as StdError;

use oteryn_world_bundle_compiler::Error;
use oteryn_world_bundle_compiler::bundle::Family;
use oteryn_world_bundle_compiler::compile::{KeyResolver, Resolution};
use oteryn_world_bundle_compiler::resolve::Registry;

type TestResult = Result<(), Box<dyn StdError>>;

fn reference(family: &str, key: &str) -> String {
    format!(r#"{{"family":"{family}","key":"{key}","revision":"definition-r1"}}"#)
}

fn items(records: &[(&str, Option<(&str, &str)>)]) -> Vec<u8> {
    let records: Vec<String> = records
        .iter()
        .map(|(key, routed_to)| {
            let routed_to = routed_to
                .map(|(family, key)| format!(r#","routed_to":{}"#, reference(family, key)))
                .unwrap_or_default();
            format!(
                r#"{{"definition":{{"identity":{}{routed_to}}}}}"#,
                reference("Item", key)
            )
        })
        .collect();
    format!(r#"{{"family":"Item","records":[{}]}}"#, records.join(",")).into_bytes()
}

fn catalogue(family: &str, records: &[(&str, Option<&str>)]) -> Vec<u8> {
    let records: Vec<String> = records
        .iter()
        .map(|(key, item)| {
            let pointer = item
                .map(|item| format!(r#""item_pointer":{}"#, reference("Item", item)))
                .unwrap_or_default();
            format!(
                r#"{{"identity":{},"provenance":{{{pointer}}}}}"#,
                reference(family, key)
            )
        })
        .collect();
    format!(
        r#"{{"family":"{family}","records":[{}]}}"#,
        records.join(",")
    )
    .into_bytes()
}

/// Items i1..i3; Terrain t1 routes i1 and t9 has no Item; WorldObject o2 routes i2.
fn registry(routed_to: Option<(&str, &str)>) -> Result<Registry, Error> {
    let mut registry = Registry::default();
    registry.add_items(&items(&[
        ("item:i2", None),
        ("item:i1", routed_to),
        ("item:i3", None),
    ]))?;
    registry.add_catalogue(
        "Terrain",
        &catalogue(
            "Terrain",
            &[("terrain:t1", Some("item:i1")), ("terrain:t9", None)],
        ),
    )?;
    registry.add_catalogue(
        "WorldObject",
        &catalogue("WorldObject", &[("object:o2", Some("item:i2"))]),
    )?;
    registry.add_provisional("donor:99".into());
    registry.add_provisional("item:i3".into());
    Ok(registry)
}

#[test]
fn keys_resolve_through_the_item_registry_with_compact_ids() -> TestResult {
    let mut registry = registry(None)?;
    // Nothing resolves before the registry is sealed.
    assert_eq!(registry.resolve("item:i1"), Resolution::Unknown);
    registry.seal()?;
    // Compact ids follow ascending key order within the family.
    assert_eq!(
        registry.resolve("item:i1"),
        Resolution::Resolved(Family::Item, 0)
    );
    assert_eq!(
        registry.resolve("item:i2"),
        Resolution::Resolved(Family::Item, 1)
    );
    // A flagged key that is a registry key resolves; the flag never hides a real key.
    assert_eq!(
        registry.resolve("item:i3"),
        Resolution::Resolved(Family::Item, 2)
    );
    // A Terrain key resolves directly only without an Item record; its id counts every
    // Terrain key of the revision.
    assert_eq!(
        registry.resolve("terrain:t9"),
        Resolution::Resolved(Family::Terrain, 1)
    );
    // A catalogue key whose record points at an Item is not a palette key (Q1b).
    assert_eq!(registry.resolve("terrain:t1"), Resolution::Unknown);
    assert_eq!(registry.resolve("object:o2"), Resolution::Unknown);
    assert_eq!(registry.resolve("donor:99"), Resolution::Provisional);
    assert_eq!(registry.resolve("item:i4"), Resolution::Unknown);
    // The route comes from the catalogue pointer.
    assert_eq!(
        registry.route("item:i2").map(|r| r.key.as_str()),
        Some("object:o2")
    );
    assert!(registry.route("item:i3").is_none());
    Ok(())
}

#[test]
fn routes_must_be_one_to_one_and_agree_with_routed_to() -> TestResult {
    let key = |result: Result<(), Error>| matches!(result, Err(Error::Key(_)));
    // routed_to agreeing with the pointer is accepted.
    registry(Some(("Terrain", "terrain:t1")))?.seal()?;
    // routed_to naming another record, or a route no pointer confirms, fails.
    assert!(key(registry(Some(("WorldObject", "object:o2")))?.seal()));
    let mut unconfirmed = registry(None)?;
    unconfirmed.add_items(&items(&[("item:i5", Some(("Terrain", "terrain:t9")))]))?;
    assert!(key(unconfirmed.seal()));
    // Two records pointing at one Item fail as they are added.
    let mut twice = registry(None)?;
    let second = catalogue("WorldObject", &[("object:o9", Some("item:i1"))]);
    assert!(key(twice.add_catalogue("WorldObject", &second)));
    // A pointer at a missing Item fails when sealed.
    let mut missing = registry(None)?;
    missing.add_catalogue(
        "WorldObject",
        &catalogue("WorldObject", &[("object:o7", Some("item:i7"))]),
    )?;
    assert!(key(missing.seal()));
    // A key given twice, or a shard of another family, is malformed.
    let mut again = registry(None)?;
    assert!(again.add_items(&items(&[("item:i1", None)])).is_err());
    assert!(
        again
            .add_catalogue("Terrain", &catalogue("WorldObject", &[]))
            .is_err()
    );
    assert!(
        again
            .add_catalogue("House", &catalogue("House", &[]))
            .is_err()
    );
    Ok(())
}
