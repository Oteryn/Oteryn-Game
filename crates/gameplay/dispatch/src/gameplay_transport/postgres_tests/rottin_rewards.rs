//! Exercise the authored reward owner through live Rottin TALK and PostgreSQL.
use super::classic_quests::{install, talk};
use super::creature_push::scenario;
use super::fixture::{Harness, id};
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
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| scenario("rottin_weighted_reward", false, None))?
        .join()
        .map_err(|_| "Rottin reward PostgreSQL thread panicked")?
        .map_err(Into::into)
}

#[test]
fn rottin_lucky_loop_talk_refuses_six_charms_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| scenario("rottin_weighted_short", false, None))?
        .join()
        .map_err(|_| "Rottin reward PostgreSQL thread panicked")?
        .map_err(Into::into)
}

#[test]
fn rottin_lucky_loop_talk_mints_after_cooldown_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| scenario("rottin_weighted_repeat", false, None))?
        .join()
        .map_err(|_| "Rottin repeat reward PostgreSQL thread panicked")?
        .map_err(Into::into)
}

fn payout_key(repeat: bool) -> &'static str {
    if repeat {
        "oteryn:quest-reward/rottin-wood/lucky-loop-repeat"
    } else {
        PAYOUT
    }
}

pub(super) fn catalogue(repeat: bool) -> TestResult<QuestStateCatalogue> {
    // This transaction fixture exercises the two gold bundles with admitted
    // output definitions. The quest unit case also checks the horn interval.
    let digest = Sha256::digest(DONOR.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    // First completion grants the account achievement. Later completions can
    // reuse the item-only occurrence owner after the quest's daily cooldown.
    let (prior_stamp, cooldown, achievement) = if repeat {
        (
            1,
            json!({"kind": "elapsed_at_least", "seconds": 72001}),
            None,
        )
    } else {
        (
            0,
            json!({"kind": "between", "minimum": 0, "maximum": 0}),
            Some(ACHIEVEMENT),
        )
    };
    let bindings = Bindings::parse(&serde_json::to_vec(&json!({
        "schema": 1,
        "quests": [{
            "quest": QUEST,
            "recipe_sha256": digest,
            "sources": [DONOR],
            "tracks": [
                {"key": TRACK, "initial": 1, "minimum": 1, "maximum": 2},
                {"key": COOLDOWN, "initial": prior_stamp, "minimum": 0, "maximum": i64::MAX}
            ],
            "stages": [{
                "key": "lucky-loop",
                "completes": false,
                "trigger": {
                    "kind": "npc_dialogue",
                    "callback": {
                        "key": payout_key(repeat), "quest": QUEST, "npc": ROTTIN,
                        "keywords": ["yes"], "topic": 0, "next_topic": 0,
                        "storage": [{"track": TRACK, "minimum": 1, "maximum": 1, "set": 2}],
                        "state_ops": [{
                            "track": COOLDOWN,
                            "from": cooldown,
                            "effect": {"kind": "set_now"}
                        }],
                        "take": [{"item": CHARM, "count": 7}],
                        "give": {"item": PLATINUM, "count": 5},
                        "weighted_rewards": [{"chance": 9, "reward": {
                            "item": PLATINUM, "count": 3,
                            "additional": [{"item": RING, "count": 1}]
                        }}],
                        "outfit": null, "achievement": achievement, "experience": 100,
                        "per_occurrence_mint": repeat,
                        "reply": ["Good hunt."], "sources": [DONOR]
                    }
                }
            }]
        }]
    }))?)?;
    let base = QuestStateCatalogue::empty("content-1").ok_or("quest catalogue")?;
    Ok(bindings.attach(&base)?)
}

pub(super) async fn seed_inventory(h: &Harness, count: usize) -> TestResult {
    super::item_look::seed_main_backpack(h, &vec![(CHARM, 1); count]).await
}

pub(super) async fn refuse_short_hand_in(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
) -> TestResult {
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let [npc] = install(owner, admitted, [ROTTIN]).await?;
    talk(owner, admitted, npc, 1701, "hi").await?;
    let before_xp = experience(h).await?;
    assert_ne!(
        talk(owner, admitted, npc, 1702, "yes").await?,
        ["Good hunt."]
    );
    assert_eq!(experience(h).await?, before_xp);
    let actual: (i64, i64, i64) = sqlx::query_as(
        "SELECT
         (SELECT COALESCE(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1),
         (SELECT count(*) FROM game_quest_reward_sources WHERE transition_key=$2),
         (SELECT count(*) FROM game_character_quest_tracks
          WHERE (track_key=$3 AND value<>1) OR (track_key=$4 AND value<>0))",
    )
    .bind(CHARM)
    .bind(PAYOUT)
    .bind(TRACK)
    .bind(COOLDOWN)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(actual, (6, 0, 0));
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    repeat: bool,
) -> TestResult {
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let [npc] = install(owner, admitted, [ROTTIN]).await?;
    talk(owner, admitted, npc, 1701, "hi").await?;
    let before_xp = experience(h).await?;
    let before_stamp = owner
        .quest_catalogue
        .as_ref()
        .and_then(|catalogue| catalogue.track(COOLDOWN))
        .ok_or("fixture cooldown track")?
        .initial;
    assert_eq!(
        talk(owner, admitted, npc, 1702, "yes").await?,
        ["Good hunt."]
    );
    assert_eq!(experience(h).await?, before_xp + 100);
    let actual: (i64, i64, i64, i64) = sqlx::query_as(
        "SELECT
         (SELECT COALESCE(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1),
         (SELECT COALESCE(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key=$2 AND lifecycle=1),
         (SELECT COALESCE(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key=$3 AND lifecycle=1),
         (SELECT count(*) FROM game_quest_reward_sources WHERE transition_key=$4 AND mint_transaction_id IS NOT NULL)"
    ).bind(CHARM).bind(PLATINUM).bind(RING).bind(payout_key(repeat)).fetch_one(&h.pool).await?;
    assert!(matches!(actual, (0, 5, 0, 1) | (0, 3, 1, 1)), "{actual:?}");
    let (awarded_bundle, retained_odds, percentile): (serde_json::Value, i32, Option<i32>) =
        sqlx::query_as(
            "SELECT plan->'give', (plan #>> '{weighted_rewards,0,chance}')::integer,
                (plan->>'reward_percentile')::integer
         FROM game_quest_reward_sources WHERE transition_key=$1",
        )
        .bind(payout_key(repeat))
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(retained_odds, 9);
    let percentile = percentile.ok_or("weighted payout receipt lacks the recorded draw")?;
    assert!((0..100).contains(&percentile));
    assert_eq!(percentile >= 100 - retained_odds, actual.2 == 1);
    assert_eq!(awarded_bundle["item"], PLATINUM);
    assert_eq!(awarded_bundle["count"], actual.1);
    if actual.2 == 1 {
        assert_eq!(
            awarded_bundle["additional"],
            json!([{"item": RING, "count": 1}])
        );
    }
    let stage: i64 =
        sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=$1")
            .bind(TRACK)
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(stage, 2);
    let (stamped, awarded): (bool, bool) = sqlx::query_as(
        "SELECT
         EXISTS(SELECT 1 FROM game_character_quest_tracks WHERE track_key=$1 AND value > $3),
         EXISTS(SELECT 1 FROM game_account_achievements WHERE achievement_key=$2)",
    )
    .bind(COOLDOWN)
    .bind(ACHIEVEMENT)
    .bind(before_stamp)
    .fetch_one(&h.pool)
    .await?;
    assert!(stamped);
    assert_eq!(awarded, !repeat);
    Ok(())
}

pub(super) async fn experience(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT total_experience FROM game_character_progression_state
         WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(id(41).as_slice())
    .fetch_one(&h.pool)
    .await?)
}
