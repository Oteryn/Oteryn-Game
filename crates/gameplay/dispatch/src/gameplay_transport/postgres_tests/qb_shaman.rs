//! QB Shaman Outfits Quest: Hairycles and Chondur stand in the native World placement index and
//! the whole outfit and addon story completes against PostgreSQL.
//!
//! Reference: TibiaWiki `Shaman_Outfits_Quest/Spoiler` and `Shaman_Outfits`.
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

/// The NPCs of the Shaman callbacks of the shared reward file.
fn shaman_callback_npcs() -> TestResult<BTreeSet<String>> {
    let data = read(&root().join("content/quests/shared-reward-callbacks.json"))?;
    let mut npcs = BTreeSet::new();
    for callback in data["callbacks"].as_array().ok_or("callbacks")? {
        if callback["key"]
            .as_str()
            .ok_or("callback key")?
            .contains("/qb-shaman-")
        {
            npcs.insert(callback["npc"].as_str().ok_or("callback npc")?.to_owned());
        }
    }
    Ok(npcs)
}

#[test]
fn qb_shaman_hand_ins_are_placed_and_story_completes() -> TestResult {
    let npcs = shaman_callback_npcs()?;
    assert_eq!(
        npcs,
        BTreeSet::from([
            "oteryn:npc.chondur".to_owned(),
            "oteryn:npc.hairycles".to_owned()
        ]),
        "QB Shaman: both outfit NPCs serve reward callbacks"
    );
    let placed = placed()?;
    let missing: Vec<&String> = npcs.iter().filter(|npc| !placed.contains(*npc)).collect();
    assert!(
        missing.is_empty(),
        "QB_OUTFIT_PLACEMENT_RED shaman: reward-callback NPCs absent from the native placement index: {missing:?}"
    );
    super::qengine_rewards_fixtures::scenario("qengine-rewards-qb-shaman-story")
}
