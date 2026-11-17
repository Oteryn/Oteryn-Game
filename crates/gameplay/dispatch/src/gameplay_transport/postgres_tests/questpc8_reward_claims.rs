//! Whole standalone source contracts allocated to questpc8.
//! The shared harness rejects quests with extra gates, progress or effects.
use super::TestResult;
use crate::content::world_reward_claims::{REWARD_CLAIM_SHARDS, UnboundReason, bind_reward_claims};
use serde_json::Value;

#[test]
fn questpc8_lone_medusa_claim_use_replay_and_refusal() -> TestResult {
    super::quest_catalogue_chests::standalone_claim_cohort(
        &["lone_medusa_quest"],
        "questpc8_lone_medusa_claim_use_replay_and_refusal",
    )
}

#[test]
fn questpc8_authored_selector_cannot_impersonate_a_crystal_reward_source() -> TestResult {
    let mut source = None;
    for shard in REWARD_CLAIM_SHARDS {
        let shard: Value = serde_json::from_str(shard)?;
        for row in shard["records"].as_array().ok_or("claim records")? {
            if row["definition"]["identity"]["key"] == "oteryn:reward-claim.kv.steelhelmet1" {
                assert!(source.replace(row.clone()).is_none(), "duplicate claim");
            }
        }
    }
    let mut row = source.ok_or("source leaf claim")?;
    let placement = &row["definition"]["placements"][0];
    let binding = &placement["source_binding"];
    let position = &binding["project_position"];
    let unique = binding["legacy_unique_ids"]
        .as_array()
        .ok_or("legacy ids")?
        .iter()
        .find(|id| id["server"] == "crystalserver")
        .and_then(|id| id["unique_id"].as_u64())
        .ok_or("Crystal selector")?;
    let world = super::quest_callbacks::source_world(
        (
            u16::try_from(position["x"].as_u64().ok_or("source x")?)?,
            u16::try_from(position["y"].as_u64().ok_or("source y")?)?,
            -i8::try_from(position["z"].as_u64().ok_or("source z")?)?,
        ),
        u16::try_from(unique)?,
        u16::try_from(
            placement["appearance_tibia_id"]
                .as_u64()
                .ok_or("appearance")?,
        )?,
        0,
    )?;
    let baseline = serde_json::json!({"records": [&row]}).to_string();
    let report = bind_reward_claims(world.base().as_ref(), &[&baseline], &|_| true)?;
    assert_eq!(report.served.len(), 1);
    assert!(report.unbound.is_empty());
    assert!(report.left_out.is_empty());

    // A native authored placement cannot borrow a donor's authority merely by
    // occupying the same coordinates and numeric unique ID. Such placements need
    // an explicit native binding contract before their rewards can be served.
    row["definition"]["placements"][0]["source_binding"]["legacy_unique_ids"] =
        serde_json::json!([{"server": "oteryn", "unique_id": unique}]);
    let native = serde_json::json!({"records": [row]}).to_string();
    let report = bind_reward_claims(world.base().as_ref(), &[&native], &|_| true)?;
    assert!(report.served.is_empty());
    assert!(report.left_out.is_empty());
    assert_eq!(report.unbound.len(), 1);
    assert_eq!(report.unbound[0].reason, UnboundReason::NoCrystalUniqueId);
    Ok(())
}
