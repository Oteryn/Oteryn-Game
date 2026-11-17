//! QB Barbarian Outfits Quest: Bron and Ajax stand in the native World placement index, the
//! one-hour and two-hour waits are the global ones, and the whole addon story completes
//! against PostgreSQL.
//!
//! Reference: TibiaWiki `Barbarian_Outfits_Quest/Spoiler` and `Barbarian_Outfits`.
use super::TestResult;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../")).to_path_buf()
}

fn read(path: &Path) -> TestResult<serde_json::Value> {
    Ok(serde_json::from_slice(&std::fs::read(path)?)?)
}

/// NPC keys that have a written native World placement.
fn placed() -> TestResult<BTreeSet<String>> {
    let root = root();
    let index = read(&root.join("content/world/npc-placements/index.json"))?;
    let mut npcs = BTreeSet::new();
    for shard in index["shards"].as_array().ok_or("shards")? {
        let data = read(&root.join(shard.as_str().ok_or("shard")?))?;
        for record in data["records"].as_array().ok_or("records")? {
            npcs.insert(
                record["declaration"]["npc"]
                    .as_str()
                    .ok_or("placement npc")?
                    .to_owned(),
            );
        }
    }
    Ok(npcs)
}

/// The Barbarian callbacks of the shared reward file: (NPC, elapsed-wait seconds if any).
fn barbarian_callbacks() -> TestResult<Vec<(String, Option<i64>)>> {
    let data = read(&root().join("content/quests/shared-reward-callbacks.json"))?;
    let mut rows = Vec::new();
    for callback in data["callbacks"].as_array().ok_or("callbacks")? {
        let key = callback["key"].as_str().ok_or("callback key")?;
        if !key.contains("/qb-barbarian-") {
            continue;
        }
        let wait = callback["state_ops"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|operation| operation["from"]["kind"] == "elapsed_at_least")
            .and_then(|operation| operation["from"]["seconds"].as_i64());
        rows.push((
            callback["npc"].as_str().ok_or("callback npc")?.to_owned(),
            wait,
        ));
    }
    Ok(rows)
}

#[test]
fn qb_barbarian_hand_ins_are_placed_and_story_completes() -> TestResult {
    let rows = barbarian_callbacks()?;
    let npcs: BTreeSet<&str> = rows.iter().map(|(npc, _)| npc.as_str()).collect();
    assert_eq!(
        npcs,
        BTreeSet::from(["oteryn:npc.ajax", "oteryn:npc.bron"]),
        "QB Barbarian: both addon NPCs serve reward callbacks"
    );
    let placed = placed()?;
    let missing: Vec<&&str> = npcs.iter().filter(|npc| !placed.contains(**npc)).collect();
    assert!(
        missing.is_empty(),
        "QB_OUTFIT_PLACEMENT_RED barbarian: reward-callback NPCs absent from the native placement index: {missing:?}"
    );
    // Global waits: one hour after "say please" and two hours before the axe (the donor adds
    // one second so that its strict `<` comparison matches "at least").
    let mut waits: Vec<i64> = rows.iter().filter_map(|(_, wait)| *wait).collect();
    waits.sort_unstable();
    assert_eq!(waits, [3601, 7201], "QB barbarian: timed conversations");
    super::qengine_rewards::scenario("qengine-rewards-qb-barbarian-story")
}
