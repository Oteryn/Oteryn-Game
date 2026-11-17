//! Shared resolver coverage only: this does not claim ownership of these quests.
use super::{Dependencies, TestResult};
use crate::content::world_reward_claims::{REWARD_CLAIM_SHARDS, bind_embedded_reward_claims};
use crate::durability::item_mint::TypedDefinitionRef;
use crate::gameplay_transport::item_ref_admission::generation_item_facts;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

#[test]
fn definition_ready_claims_bind_source_placements_with_native_items() -> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| coverage().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "definition-ready coverage panicked")?
        .map_err(Into::into)
}

fn coverage() -> TestResult {
    // The derived queue selects test coverage, never runtime authority. All binding
    // and item admission below use the ordinary embedded production catalogues.
    let matrix: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../tools/content-schema/quest-authoring/samples/completion-matrix/all373.json"
    )))?;
    let mut keys = BTreeSet::new();
    let mut quests = 0;
    for record in matrix["records"].as_array().ok_or("matrix records")? {
        if !record["canonical"]
            .as_array()
            .ok_or("canonical records")?
            .iter()
            .any(|quest| quest["readiness"] == "definition_ready")
        {
            continue;
        }
        quests += 1;
        for key in record["reward_claim_owner"]["claim_keys"]
            .as_array()
            .ok_or("definition-ready claim keys")?
        {
            keys.insert(key.as_str().ok_or("claim key")?.to_owned());
        }
    }
    assert_eq!(quests, 42);
    assert_eq!(keys.len(), 59);
    let mut definitions = BTreeMap::new();
    for shard in REWARD_CLAIM_SHARDS {
        let document: Value = serde_json::from_str(shard)?;
        for record in document["records"].as_array().ok_or("claim records")? {
            let definition = &record["definition"];
            let key = definition["identity"]["key"]
                .as_str()
                .ok_or("claim identity")?;
            assert!(
                definitions
                    .insert(key.to_owned(), definition.clone())
                    .is_none()
            );
        }
    }
    let deps = Dependencies::new()?;
    let (controller, _) = super::death_ml_loss_composed::production_generation_with_pin(
        deps.admitted.world_id,
        "content-1",
    )?;
    let generation = controller.active().ok_or("active generation")?;
    let native = generation.native_gameplay().ok_or("native gameplay")?;
    let mut placements = 0;
    let mut unserved = BTreeSet::new();
    for key in keys {
        let definition = definitions.get(&key).ok_or("missing source claim")?;
        for placement in definition["placements"].as_array().ok_or("placements")? {
            let binding = &placement["source_binding"];
            let position = &binding["project_position"];
            let x = u16::try_from(position["x"].as_u64().ok_or("x")?)?;
            let y = u16::try_from(position["y"].as_u64().ok_or("y")?)?;
            let floor = -i8::try_from(position["z"].as_u64().ok_or("z")?)?;
            let appearance = placement["appearance_tibia_id"]
                .as_u64()
                .ok_or("appearance")?;
            let mut choices = Vec::new();
            for sibling in definition["placements"].as_array().ok_or("placements")? {
                let source = &sibling["source_binding"];
                let at = &source["project_position"];
                assert_eq!(at["z"], position["z"]);
                assert_eq!(
                    sibling["appearance_tibia_id"],
                    placement["appearance_tibia_id"]
                );
                let unique = source["legacy_unique_ids"]
                    .as_array()
                    .ok_or("unique ids")?
                    .iter()
                    .find(|entry| entry["server"] == "crystalserver")
                    .and_then(|entry| entry["unique_id"].as_u64())
                    .ok_or("source unique id")?;
                choices.push((
                    u16::try_from(at["x"].as_u64().ok_or("x")?)?,
                    u16::try_from(at["y"].as_u64().ok_or("y")?)?,
                    u16::try_from(unique)?,
                ));
            }
            let world = super::quest_callbacks::source_world_positions(
                (x, y, floor),
                &choices,
                u16::try_from(appearance)?,
            )?;
            let report = bind_embedded_reward_claims(world.base().as_ref(), &|item| {
                generation_item_facts(
                    native,
                    &TypedDefinitionRef {
                        family: "Item".into(),
                        production_key: item.key().as_str().into(),
                        revision_ref: item.revision().as_str().into(),
                    },
                )
                .is_ok_and(|facts| {
                    facts.container_capacity.is_none_or(|capacity| capacity > 0)
                        && facts.stack != crate::durability::item_transfer::ItemStackClass::Unknown
                })
            })?;
            // Shared claims require every source placement to bind together.
            if !report.served.iter().any(|claim| {
                claim.key.as_str() == key
                    && claim.placements.iter().any(|bound| {
                        bound.cell.x == x && bound.cell.y == y && bound.cell.floor == floor
                    })
            }) {
                unserved.insert(key.clone());
            }
            placements += 1;
        }
    }
    assert_eq!(placements, 60);
    assert!(
        unserved.is_empty(),
        "source claims did not bind with native items: {unserved:?}"
    );
    Ok(())
}

#[test]
fn definition_ready_claim_use_and_replay_dispatch_postgres() -> TestResult {
    postgres_scenario(super::quest_callbacks::DefinitionReadyClaim::RuneEmblem)
}

#[test]
fn definition_ready_claim_bloodherb_use_and_replay_dispatch_postgres() -> TestResult {
    postgres_scenario(super::quest_callbacks::DefinitionReadyClaim::BloodHerb)
}

fn postgres_scenario(claim: super::quest_callbacks::DefinitionReadyClaim) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            super::quest_callbacks::scenario_with_definition_ready_claim(false, Some(claim))
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "definition-ready dispatch panicked".to_owned())
        .and_then(|result| result);
    eprintln!(
        "QENGINE2_DEFINITION_READY_PG elapsed={:?} passed={}",
        started.elapsed(),
        result.is_ok()
    );
    result.map_err(Into::into)
}
