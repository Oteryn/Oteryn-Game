//! The current qualified Native artifact must supply restrictive static source physics.
#![cfg(feature = "test-support")]
use oteryn_gameplay_content::content::native_gameplay::{self, NativeItemDestination};
type TestResult = Result<(), Box<dyn std::error::Error>>;
#[test]
fn srv7_spike_source19387_native_static_physics_never_admits_inventory_mint() -> TestResult {
    let world = oteryn_protocol_oteryn::WorldId::decode(&[
        1, 144, 0, 0, 0, 8, 112, 0, 128, 0, 0, 0, 0, 0, 0, 42,
    ])
    .map_err(|error| format!("valid fixtureWorld: {error:?}"))?;
    let native = native_gameplay::retained_callback_native_fixture(world);
    let policy = native
        .item_policy_for_source_id(19387)
        .ok_or("actual qualified Native has no source19387 static physical policy")?;
    let record = policy.record();
    assert_eq!(record.production_definition.family, "Item");
    assert_eq!(
        record.production_definition.production_key,
        "oteryn:item.tibia.i19387"
    );
    assert_eq!(record.production_definition.revision_ref, "definition-r1");
    assert_eq!(record.attributes.blocks_movement, Some(false));
    assert_eq!(record.attributes.blocks_projectile, Some(false));
    assert_eq!(record.attributes.immovable_block_solid, Some(false));
    assert_eq!(record.attributes.has_height, Some(false));
    assert!(
        !record.admission.materializable,
        "static map source qualification cannot mint this object"
    );
    assert_eq!(
        record.admission.legal_destinations,
        [NativeItemDestination::Ground]
    );
    Ok(())
}

#[test]
fn srv7_spike_actual_native_nine_hub_physics_preserve_source_blockers() -> TestResult {
    qualify_source_physics(
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../tools/content-schema/world-authoring/samples/spike-pocket-nine-physics.json"
        )),
        9,
    )
}

#[test]
fn srv7_spike_actual_native_33_corridor_physics_preserve_source_and_mint_boundaries() -> TestResult
{
    let raw = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../tools/content-schema/world-authoring/samples/spike-corridor-source-physics.json"
    ));
    let source: serde_json::Value = serde_json::from_str(raw)?;
    assert_eq!(
        source["schema"],
        "OTERYN_NATIVE_SPIKE_CORRIDOR_SOURCE_PHYSICS/v1"
    );
    assert_eq!(
        source["withheld"][0]["id"], 15012,
        "never invent an Item alias for the dropped appearance"
    );
    assert_eq!(
        source["crosscheck_world_physics"]["differences"],
        serde_json::json!([])
    );
    assert_eq!(
        source["floor_change_default_evidence"]
            .as_array()
            .ok_or("exact two source loader/default proofs")?
            .len(),
        4
    );
    for row in source["source_world_physics"]
        .as_array()
        .ok_or("explicit source floor facts")?
    {
        assert_eq!(
            row["floor_change"], false,
            "all captured source33 entries explicitly inherit TILESTATE_NONE"
        );
        assert!(
            row["floor_change_kind"].is_null(),
            "no unknown-floor default"
        );
    }
    qualify_source_physics(raw, 33)
}

fn qualify_source_physics(raw: &str, expected: usize) -> TestResult {
    let world = oteryn_protocol_oteryn::WorldId::decode(&[
        1, 144, 0, 0, 0, 8, 112, 0, 128, 0, 0, 0, 0, 0, 0, 42,
    ])
    .map_err(|error| format!("valid fixture World: {error:?}"))?;
    let native = native_gameplay::retained_callback_native_fixture(world);
    let source: serde_json::Value = serde_json::from_str(raw)?;
    let records = source["records"]
        .as_array()
        .ok_or("nine pinned physical records")?;
    assert_eq!(records.len(), expected);
    let mut missing = Vec::new();
    for captured in records {
        let id: u16 = captured["production_binding"]["external_id"]
            .as_str()
            .ok_or("source ID")?
            .parse()?;
        let Some(policy) = native.item_policy_for_source_id(u32::from(id)) else {
            missing.push(id);
            continue;
        };
        let record = policy.record();
        assert_eq!(
            record.production_definition.production_key,
            format!("oteryn:item.tibia.i{id}")
        );
        assert_eq!(record.production_definition.revision_ref, "definition-r1");
        let attributes = serde_json::to_value(&record.attributes)?;
        for field in [
            "blocks_movement",
            "blocks_projectile",
            "immovable_block_solid",
            "has_height",
        ] {
            assert!(
                captured["attributes"][field].is_boolean(),
                "source{field} must be qualified"
            );
            assert_eq!(
                attributes[field], captured["attributes"][field],
                "source{id} field{field}; preserve real18368blocker/15466height"
            );
        }
        assert!(
            !record.admission.materializable,
            "static source{id} grants no MINT entitlement"
        );
        assert_eq!(
            record.admission.legal_destinations,
            [NativeItemDestination::Ground]
        );
    }
    assert!(
        missing.is_empty(),
        "actual Native missing source-qualified hub physics: {missing:?}"
    );
    Ok(())
}
