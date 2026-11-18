//! Drive a real retained hammer USE and the resulting Disrepair hand-in.
use super::classic_quests::{install, talk};
use super::creature_push::scenario;
use super::fixture::Harness;
use super::rottin_bushwhacking::use_quest_tool;
use super::rottin_rewards::experience;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::map::overlay::TilePos;
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, authored_bindings::Bindings};
use oteryn_protocol_oteryn::world_object::UseDisposition;
use serde_json::json;
use sha2::{Digest, Sha256};

const QUEST: &str = "oteryn:quest.rottin_wood_and_the_married_men_quest";
const ROTTIN: &str = "oteryn:npc.rottin_wood";
const WALLS: &str = oteryn_gameplay_quests::quest::classic_talk::ROTTIN_WALLCOUNT;
const HAMMER: &str = "oteryn:item.tibia.i3460";
const REPAIR: &str = "oteryn:quest-callback/rottin-wood/repair-wall";
const TURN_IN: &str = "oteryn:quest-reward/rottin-wood/disrepair";

#[test]
fn rottin_hammer_use_satisfies_disrepair_turn_in_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| scenario("rottin_repairs", false, None))?
        .join()
        .map_err(|_| "Disrepair PostgreSQL thread panicked")?
        .map_err(Into::into)
}

pub(super) fn catalogue() -> TestResult<QuestStateCatalogue> {
    let repair_source = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/actions/tools/hammer.lua:64-101";
    let reward_source =
        "201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/rottin_wood.lua:180-194";
    let questline =
        "oteryn:quest-progress/crystalserver/quest/u8_7/rottin_wood_and_the_married_men/questline";
    let mission =
        "oteryn:quest-progress/crystalserver/quest/u8_7/rottin_wood_and_the_married_men/mission02";
    let digest = Sha256::digest(repair_source.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let bindings = Bindings::parse(&serde_json::to_vec(&json!({
        "schema": 1,
        "quests": [{
            "quest": QUEST, "recipe_sha256": digest, "sources": [repair_source, reward_source],
            "tracks": [
                {"key": WALLS, "initial": 5, "minimum": 0, "maximum": 6},
                {"key": mission, "initial": 1, "minimum": 0, "maximum": 2},
                {"key": questline, "initial": 3, "minimum": 0, "maximum": 6}
            ],
            "stages": [{
                "key": "repair-wall", "completes": false,
                "trigger": {"kind": "map_use", "callback": {
                    "key": REPAIR, "quest": QUEST, "trigger": "use",
                    "selector": {"position": {"x": 3, "y": 2, "floor": -7}, "appearance": 12183},
                    "reversible": false, "requires_world_admission": true,
                    "passages": [], "offerings": [], "return_on_failure": false,
                    "item_on_target": {"item": HAMMER, "count": 1, "selected_instance": true, "reusable": true},
                    "storage": [{"track": WALLS, "minimum": 0, "maximum": 5, "set": 6, "add": 1}],
                    "effects": {"transform": 6474, "reset_after_ms": 120000},
                    "sources": [repair_source]
                }}
            }, {
                "key": "disrepair-turn-in", "completes": false,
                "trigger": {"kind": "npc_dialogue", "callback": {
                    "key": TURN_IN, "quest": QUEST, "npc": ROTTIN,
                    "keywords": ["yes"], "topic": 0, "next_topic": 0,
                    "storage": [
                        {"track": mission, "minimum": 1, "maximum": 1, "set": 2},
                        {"track": questline, "minimum": 3, "maximum": 3, "set": 4}
                    ],
                    "requires": [{"track": WALLS, "minimum": 6, "maximum": 6}],
                    "take": [], "give": {"item": "oteryn:item.tibia.i3035", "count": 5},
                    "outfit": null, "achievement": null, "experience": 500,
                    "reply": ["You repaired the walls."], "sources": [reward_source]
                }}
            }]
        }]
    }))?)?;
    Ok(bindings.attach(&QuestStateCatalogue::empty("content-1").ok_or("empty catalogue")?)?)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    h: &Harness,
) -> TestResult {
    let [npc] = install(owner, &admitted, [ROTTIN]).await?;
    talk(owner, &admitted, npc, 1601, "hi").await?;
    let before_xp = experience(h).await?;
    let wall = TilePos {
        x: 3,
        y: 2,
        floor: -7,
    };
    assert_eq!(
        use_quest_tool(owner, admitted, wall, 1, 1602, 71).await?,
        UseDisposition::Committed
    );
    let map = owner.bundle_map().ok_or("repair map")?;
    let due = map.facts.next_movement_decay_time().ok_or("wall reset")?;
    let (fixed, retained): (i64, i64) = sqlx::query_as(
        "SELECT (SELECT value FROM game_character_quest_tracks WHERE track_key=$1),
                (SELECT count(*) FROM game_quest_world_tool_reads WHERE transition_key=$2)",
    )
    .bind(WALLS)
    .bind(REPAIR)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!((fixed, retained), (6, 1));
    assert_eq!(
        talk(owner, &admitted, npc, 1603, "yes").await?,
        ["You repaired the walls."]
    );
    assert_eq!(experience(h).await?, before_xp + 500);
    let paid: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_quest_reward_sources WHERE transition_key=$1 AND mint_transaction_id IS NOT NULL",
    )
    .bind(TURN_IN)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(paid, 1);
    assert_eq!(
        owner
            .bundle_map()
            .ok_or("repair map")?
            .facts
            .next_movement_decay_time(),
        Some(due)
    );
    Ok(())
}
