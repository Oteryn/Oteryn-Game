//! QB-1: every NPC that serves a reward callback of the Assassin, Beggar, Hunter, Mage,
//! Summoner and Wizard outfit quests stands in the native World placement index, and the
//! whole source outfit story still completes against PostgreSQL.
//!
//! Reference: TibiaWiki `Assassin_Outfits_Quest/Spoiler`, `Beggar_Outfits_Quest/Spoiler`,
//! `Hunter_Outfits_Quest/Spoiler`, `Mage_Outfits_Quest/Spoiler`,
//! `Summoner_Outfits_Quest/Spoiler` and `Wizard_Outfits_Quest/Spoiler`.
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

/// NPCs of the shared and per-quest reward callbacks whose key names `marker`.
fn callback_npcs(marker: &str) -> TestResult<BTreeSet<String>> {
    let root = root();
    let mut files = vec![root.join("content/quests/shared-reward-callbacks.json")];
    for entry in std::fs::read_dir(root.join("content/quests/npc-callbacks"))? {
        let path = entry?.path();
        if path.extension().is_some_and(|ext| ext == "json") {
            files.push(path);
        }
    }
    let mut npcs = BTreeSet::new();
    for file in files {
        let data = read(&file)?;
        let Some(callbacks) = data["callbacks"].as_array() else {
            continue;
        };
        for callback in callbacks {
            let key = callback["key"].as_str().ok_or("callback key")?;
            if key
                .rsplit('/')
                .next()
                .is_some_and(|tail| tail.contains(marker))
            {
                npcs.insert(callback["npc"].as_str().ok_or("callback npc")?.to_owned());
            }
        }
    }
    Ok(npcs)
}

fn hand_ins_are_placed(marker: &str, expected: &[&str]) -> TestResult {
    let npcs = callback_npcs(marker)?;
    let wanted: BTreeSet<String> = expected.iter().map(|npc| (*npc).to_owned()).collect();
    assert!(
        wanted.is_subset(&npcs),
        "QB {marker}: expected hand-in NPCs {wanted:?} are not all reward-callback NPCs {npcs:?}"
    );
    let placed = placed()?;
    let missing: Vec<&String> = npcs.iter().filter(|npc| !placed.contains(*npc)).collect();
    assert!(
        missing.is_empty(),
        "QB_OUTFIT_PLACEMENT_RED {marker}: reward-callback NPCs absent from the native placement index: {missing:?}"
    );
    Ok(())
}

#[test]
fn qb_assassin_hand_ins_are_placed_and_story_completes() -> TestResult {
    hand_ins_are_placed(
        "assassin",
        &["oteryn:npc.atrad", "oteryn:npc.erayo", "oteryn:npc.vescu"],
    )?;
    super::qengine_rewards::scenario("qengine-rewards-l7-assassin-story")
}

#[test]
fn qb_beggar_hand_ins_are_placed_and_stories_complete() -> TestResult {
    hand_ins_are_placed(
        "beggar",
        &["oteryn:npc.hugo", "oteryn:npc.simon_the_beggar"],
    )?;
    super::qengine_rewards::scenario("qengine-rewards-l6-beggar-male-story")?;
    super::qengine_rewards::scenario("qengine-rewards-l6-beggar-female-story")
}

#[test]
fn qb_hunter_hand_ins_are_placed_and_stories_complete() -> TestResult {
    hand_ins_are_placed("hunter", &["oteryn:npc.elane", "oteryn:npc.legola"])?;
    super::qengine_rewards::scenario("qengine-rewards-l8-hunter-male-story")?;
    super::qengine_rewards::scenario("qengine-rewards-l8-hunter-female-story")
}

#[test]
fn qb_mage_and_summoner_hand_ins_are_placed_and_stories_complete() -> TestResult {
    hand_ins_are_placed(
        "mage",
        &[
            "oteryn:npc.angelina",
            "oteryn:npc.lynda",
            "oteryn:npc.myra",
            "oteryn:npc.sandra",
            "oteryn:npc.zoltan",
        ],
    )?;
    super::qengine_rewards::scenario("qengine-rewards-l8-mage-male-story")?;
    super::qengine_rewards::scenario("qengine-rewards-l8-mage-female-story")
}

#[test]
fn qb_wizard_hand_ins_are_placed_and_story_completes() -> TestResult {
    hand_ins_are_placed(
        "wizard",
        &["oteryn:npc.lugri", "oteryn:npc.the_queen_of_the_banshees"],
    )?;
    super::qengine_rewards::scenario("qengine-rewards-l2-wizard-skull-story")
}
