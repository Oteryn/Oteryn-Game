//! Drive a real retained hammer USE and the resulting Disrepair hand-in.
use super::classic_quests::{install, talk};
use super::creature_push::scenario;
use super::fixture::{Harness, id};
use super::rottin_bushwhacking::use_quest_tool;
use super::rottin_rewards::experience;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::map::{overlay::TilePos, view::MapFacts};
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, authored_bindings::Bindings};
use oteryn_protocol_oteryn::world_object::UseDisposition;
use serde_json::json;
use sha2::{Digest, Sha256};

const QUEST: &str = "oteryn:quest.rottin_wood_and_the_married_men_quest";
const ROTTIN: &str = "oteryn:npc.rottin_wood";
const WALLS: &str = oteryn_gameplay_quests::quest::classic_talk::ROTTIN_WALLCOUNT;
const HAMMER: &str = "oteryn:item.tibia.i3460";
const REPAIR: &str = "oteryn:quest-callback/rottin-wood/repair-wall";
const DISREPAIR: &str = "oteryn:quest-reward/rottin-wood/disrepair";
const HANDYMAN: &str = "oteryn:quest-reward/rottin-wood/handyman";
const COOLDOWN: &str = "oteryn:quest-progress/crystalserver/kv/handyman-task";

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

#[test]
fn rottin_hammer_use_satisfies_handyman_turn_in_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| scenario("rottin_handyman", false, None))?
        .join()
        .map_err(|_| "Handyman PostgreSQL thread panicked")?
        .map_err(Into::into)
}

pub(super) fn catalogue(handyman: bool) -> TestResult<QuestStateCatalogue> {
    let repair_source = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/actions/tools/hammer.lua:64-101";
    let questline =
        "oteryn:quest-progress/crystalserver/quest/u8_7/rottin_wood_and_the_married_men/questline";
    let (turn_in, reward_source, track, quest_stage) = if handyman {
        (
            HANDYMAN,
            "201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/rottin_wood.lua:277-291",
            "oteryn:quest-progress/crystalserver/quest/u8_7/rottin_wood_and_the_married_men/task02",
            4,
        )
    } else {
        (
            DISREPAIR,
            "201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/rottin_wood.lua:180-194",
            "oteryn:quest-progress/crystalserver/quest/u8_7/rottin_wood_and_the_married_men/mission02",
            3,
        )
    };
    let questline_write = if handyman {
        json!({"track": questline, "minimum": 4, "maximum": 6, "set": 4, "preserve": true})
    } else {
        json!({"track": questline, "minimum": 3, "maximum": 3, "set": 4})
    };
    let cooldown = if handyman {
        json!([{"track": COOLDOWN, "from": {"kind": "elapsed_at_least", "seconds": 72001},
                "effect": {"kind": "set_now"}}])
    } else {
        json!([])
    };
    let digest = Sha256::digest(repair_source.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let bindings = Bindings::parse(&serde_json::to_vec(&json!({
        "schema": 1,
        "quests": [{
            "quest": QUEST, "recipe_sha256": digest, "sources": [repair_source, reward_source],
            "tracks": [
                {"key": WALLS, "initial": 4, "minimum": 0, "maximum": 6},
                {"key": track, "initial": 1, "minimum": 0, "maximum": 2},
                {"key": questline, "initial": quest_stage, "minimum": 0, "maximum": 6},
                {"key": COOLDOWN, "initial": 1, "minimum": 0, "maximum": i64::MAX}
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
                    "activation": {"track": WALLS, "minimum": 0, "maximum": 5},
                    "effects": {"transform": 6474, "reset_after_ms": 120000},
                    "sources": [repair_source]
                }}
            }, {
                "key": "disrepair-turn-in", "completes": false,
                "trigger": {"kind": "npc_dialogue", "callback": {
                    "key": turn_in, "quest": QUEST, "npc": ROTTIN,
                    "keywords": ["yes"], "topic": 0, "next_topic": 0,
                    "storage": [
                        {"track": track, "minimum": 1, "maximum": 1, "set": 2},
                        questline_write
                    ],
                    "state_ops": cooldown, "per_occurrence_mint": handyman,
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
    handyman: bool,
) -> TestResult {
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
    assert_eq!(
        map.facts
            .base_entry(wall, 1, 7)
            .ok_or("repaired wall")?
            .appearance_id,
        6474
    );
    assert_eq!(map.facts.expire_movement_items(due - 1), 0);
    assert_eq!(
        use_quest_tool(owner, admitted, wall, 1, 1603, 71).await?,
        UseDisposition::NothingToUse,
        "a repaired wall cannot count again before it breaks"
    );
    let map = owner.bundle_map().ok_or("repair map")?;
    assert_eq!(map.facts.expire_movement_items(due), 1);
    assert_eq!(
        map.facts
            .base_entry(wall, 1, 7)
            .ok_or("restored wall")?
            .appearance_id,
        12183
    );
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(121))
        .ok_or("wall reset clock")?;
    assert_eq!(
        use_quest_tool(owner, admitted, wall, 1, 1604, 71).await?,
        UseDisposition::Committed,
        "a wall can count again after the source reset"
    );
    let (fixed, retained): (i64, i64) = sqlx::query_as(
        "SELECT (SELECT value FROM game_character_quest_tracks WHERE track_key=$1),
                (SELECT count(*) FROM game_quest_world_tool_reads WHERE transition_key=$2)",
    )
    .bind(WALLS)
    .bind(REPAIR)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!((fixed, retained), (6, 2));
    // The live USE proved exact hammer custody. The retained content has no
    // carried-weight admission for 3460 yet, so park the seeded tool before TALK.
    // This keeps NPC MINT's carried-capacity checks enabled.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await?;
    let stored = sqlx::query(
        "WITH retained AS (
            DELETE FROM game_item_container_entries c USING game_item_instances i
            WHERE c.item_instance_id=i.item_instance_id AND i.item_instance_id=encode($1,'hex')::uuid
              AND i.definition_production_key=$2 AND i.lifecycle=1 AND i.quantity=1
            RETURNING c.item_instance_id,c.world_id,c.character_id
         )
         INSERT INTO game_item_character_inbox_locations(item_instance_id,world_id,character_id,ordinal)
         SELECT item_instance_id,world_id,character_id,1 FROM retained",
    )
    .bind(id(71).as_slice())
    .bind(HAMMER)
    .execute(&mut *tx)
    .await?;
    assert_eq!(stored.rows_affected(), 1);
    tx.commit().await?;
    let [npc] = install(owner, &admitted, [ROTTIN]).await?;
    talk(owner, &admitted, npc, 1605, "hi").await?;
    assert_eq!(
        talk(owner, &admitted, npc, 1606, "yes").await?,
        ["You repaired the walls."]
    );
    assert_eq!(experience(h).await?, before_xp + 500);
    let paid: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_quest_reward_sources WHERE transition_key=$1 AND mint_transaction_id IS NOT NULL",
    )
    .bind(if handyman { HANDYMAN } else { DISREPAIR })
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(paid, 1);
    if handyman {
        let stamp: i64 =
            sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=$1")
                .bind(COOLDOWN)
                .fetch_one(&h.pool)
                .await?;
        assert!(
            stamp > 1,
            "reward advances Handyman's shared cooldown clock"
        );
    }
    Ok(())
}
