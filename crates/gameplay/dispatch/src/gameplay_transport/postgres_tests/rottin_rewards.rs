//! Exercise the authored reward owner through live Rottin TALK and PostgreSQL.
use super::classic_quests::{install, talk};
use super::fixture::{Harness, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use oteryn_gameplay_quests::quest::QuestStateCatalogue;
use oteryn_gameplay_quests::quest::authored_bindings::Bindings;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::sync::Arc;

const QUEST: &str = "oteryn:quest.rottin_wood_and_the_married_men_quest";
const TRACK: &str =
    "oteryn:quest-progress/crystalserver/quest/u8_7/rottin_wood_and_the_married_men/task01";
const PAYOUT: &str = "oteryn:quest-reward/rottin-wood/lucky-loop";
const ROTTIN: &str = "oteryn:npc.rottin_wood";
const PLATINUM: &str = "oteryn:item.tibia.i3035";
const RING: &str = "oteryn:item.tibia.i3053";
const CHARM: &str = "oteryn:item.tibia.i12173";
const DONOR: &str =
    "201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/rottin_wood.lua:238-255";

#[test]
fn rottin_lucky_loop_talk_mints_one_weighted_bundle_postgres() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run("rottin-weighted-reward", || {
        super::quest_world_callbacks::scenario(
            "rottin-weighted-reward",
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    })
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    // This transaction fixture exercises the two gold bundles with admitted
    // output definitions. The quest unit case also checks the horn interval.
    let digest = format!("{:x}", Sha256::digest(DONOR.as_bytes()));
    let bindings = Bindings::parse(&serde_json::to_vec(&json!({
        "schema": 1,
        "quests": [{
            "quest": QUEST,
            "recipe_sha256": digest,
            "sources": [DONOR],
            "tracks": [{"key": TRACK, "initial": 1, "minimum": 1, "maximum": 2}],
            "stages": [{
                "key": "lucky-loop",
                "completes": false,
                "trigger": {
                    "kind": "npc_dialogue",
                    "callback": {
                        "key": PAYOUT, "quest": QUEST, "npc": ROTTIN,
                        "keywords": ["yes"], "topic": 0, "next_topic": 0,
                        "storage": [{"track": TRACK, "minimum": 1, "maximum": 1, "set": 2}],
                        "take": [{"item": CHARM, "count": 7}],
                        "give": {"item": PLATINUM, "count": 5},
                        "weighted_rewards": [{"chance": 9, "reward": {
                            "item": PLATINUM, "count": 3,
                            "additional": [{"item": RING, "count": 1}]
                        }}],
                        "outfit": null, "achievement": null, "experience": 100,
                        "reply": ["Good hunt."], "sources": [DONOR]
                    }
                }
            }]
        }]
    }))?)?;
    let base = QuestStateCatalogue::empty("content-1").ok_or("quest catalogue")?;
    owner.quest_catalogue = Some(Arc::new(bindings.attach(&base)?));
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let [npc] = install(owner, admitted, [ROTTIN]).await?;
    talk(owner, admitted, npc, 1701, "hi").await?;

    // Provision the exact donor objective as seven non-stackable carried items.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    let backpack: Vec<u8> = sqlx::query_scalar(
        "SELECT uuid_send(item_instance_id) FROM game_item_container_slots WHERE character_id=encode($1,'hex')::uuid"
    ).bind(id(41).as_slice()).fetch_one(&mut *tx).await?;
    let ordinal: i64 = sqlx::query_scalar(
        "SELECT COALESCE(max(placement_ordinal),0)::bigint FROM game_item_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid"
    ).bind(&backpack).fetch_one(&mut *tx).await?;
    for offset in 1..=7_i64 {
        let instance: String = sqlx::query_scalar(
            "INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id)
             SELECT gen_random_uuid(),world_id,'Item',$2,'definition-r1',1,1,minted_transaction_id FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid RETURNING item_instance_id::text"
        ).bind(&backpack).bind(CHARM).fetch_one(&mut *tx).await?;
        sqlx::query(
            "INSERT INTO game_item_container_entries(character_id,world_id,parent_item_instance_id,item_instance_id,placement_ordinal,placed_transaction_id)
             SELECT slots.character_id,slots.world_id,slots.item_instance_id,$2::uuid,$3::bigint,items.minted_transaction_id FROM game_item_container_slots slots JOIN game_item_instances items USING (item_instance_id,world_id) WHERE slots.item_instance_id=encode($1,'hex')::uuid"
        ).bind(&backpack).bind(instance).bind(ordinal + offset).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    assert_eq!(
        talk(owner, admitted, npc, 1702, "yes").await?,
        ["Good hunt."]
    );
    let actual: (i64, i64, i64, i64) = sqlx::query_as(
        "SELECT
         (SELECT COALESCE(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1),
         (SELECT COALESCE(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key=$2 AND lifecycle=1),
         (SELECT COALESCE(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key=$3 AND lifecycle=1),
         (SELECT count(*) FROM game_quest_reward_sources WHERE transition_key=$4 AND mint_transaction_id IS NOT NULL)"
    ).bind(CHARM).bind(PLATINUM).bind(RING).bind(PAYOUT).fetch_one(&h.pool).await?;
    assert!(matches!(actual, (0, 5, 0, 1) | (0, 3, 1, 1)), "{actual:?}");
    let stage: i64 =
        sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=$1")
            .bind(TRACK)
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(stage, 2);
    Ok(())
}
