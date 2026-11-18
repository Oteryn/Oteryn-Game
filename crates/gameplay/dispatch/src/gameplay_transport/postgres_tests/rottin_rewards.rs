//! Exercise the authored reward owner through live Rottin TALK and PostgreSQL.
use super::classic_quests::{install, talk};
use super::creature_push::scenario;
use super::fixture::Harness;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use oteryn_gameplay_quests::quest::QuestStateCatalogue;
use oteryn_gameplay_quests::quest::authored_bindings::Bindings;
use serde_json::json;
use sha2::{Digest, Sha256};

const QUEST: &str = "oteryn:quest.rottin_wood_and_the_married_men_quest";
const TRACK: &str =
    "oteryn:quest-progress/crystalserver/quest/u8_7/rottin_wood_and_the_married_men/task01";
const COOLDOWN: &str = "oteryn:quest-progress/crystalserver/kv/lucky-loop-task";
const ACHIEVEMENT: &str = "oteryn:achievement/bunny_slipped";
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
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| scenario("rottin_weighted_reward", false, None))?
        .join()
        .map_err(|_| "Rottin reward PostgreSQL thread panicked")?
        .map_err(Into::into)
}

pub(super) fn catalogue() -> TestResult<QuestStateCatalogue> {
    // This transaction fixture exercises the two gold bundles with admitted
    // output definitions. The quest unit case also checks the horn interval.
    let digest = Sha256::digest(DONOR.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let bindings = Bindings::parse(&serde_json::to_vec(&json!({
        "schema": 1,
        "quests": [{
            "quest": QUEST,
            "recipe_sha256": digest,
            "sources": [DONOR],
            "tracks": [
                {"key": TRACK, "initial": 1, "minimum": 1, "maximum": 2},
                {"key": COOLDOWN, "initial": 0, "minimum": 0, "maximum": 9223372036854775807_i64}
            ],
            "stages": [{
                "key": "lucky-loop",
                "completes": false,
                "trigger": {
                    "kind": "npc_dialogue",
                    "callback": {
                        "key": PAYOUT, "quest": QUEST, "npc": ROTTIN,
                        "keywords": ["yes"], "topic": 0, "next_topic": 0,
                        "storage": [{"track": TRACK, "minimum": 1, "maximum": 1, "set": 2}],
                        "state_ops": [{
                            "track": COOLDOWN,
                            "from": {"kind": "any"},
                            "effect": {"kind": "set_now"}
                        }],
                        "take": [{"item": CHARM, "count": 7}],
                        "give": {"item": PLATINUM, "count": 5},
                        "weighted_rewards": [{"chance": 9, "reward": {
                            "item": PLATINUM, "count": 3,
                            "additional": [{"item": RING, "count": 1}]
                        }}],
                        "outfit": null, "achievement": ACHIEVEMENT, "experience": 100,
                        "reply": ["Good hunt."], "sources": [DONOR]
                    }
                }
            }]
        }]
    }))?)?;
    let base = QuestStateCatalogue::empty("content-1").ok_or("quest catalogue")?;
    Ok(bindings.attach(&base)?)
}

pub(super) async fn seed_inventory(h: &Harness) -> TestResult {
    super::item_look::seed_main_backpack(h, &[(CHARM, 1); 7]).await
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let [npc] = install(owner, admitted, [ROTTIN]).await?;
    talk(owner, admitted, npc, 1701, "hi").await?;
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
    let (stamped, awarded): (bool, bool) = sqlx::query_as(
        "SELECT
         EXISTS(SELECT 1 FROM game_character_quest_tracks WHERE track_key=$1 AND value > 0),
         EXISTS(SELECT 1 FROM game_account_achievements WHERE achievement_key=$2)",
    )
    .bind(COOLDOWN)
    .bind(ACHIEVEMENT)
    .fetch_one(&h.pool)
    .await?;
    assert!(stamped && awarded);
    Ok(())
}
