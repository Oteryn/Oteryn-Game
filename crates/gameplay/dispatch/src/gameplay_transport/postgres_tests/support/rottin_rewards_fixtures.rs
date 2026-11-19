//! Exercise the authored reward owner through live Rottin TALK and PostgreSQL.
use super::classic_quests_fixtures::{install, talk};
use super::fixture::{Harness, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use oteryn_gameplay_quests::quest::{classic_dialogue, classic_talk::ROTTIN_STATE};
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use serde_json::json;

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
fn payout_key(repeat: bool) -> &'static str {
    if repeat {
        "oteryn:quest-reward/rottin-wood/lucky-loop-repeat"
    } else {
        PAYOUT
    }
}

const RIBBON: &str = "oteryn:item.tibia.i12171";
const HORN: &str = "oteryn:item.tibia.i12260";

pub(super) async fn seed_tracks(h: &Harness, tracks: &[(&str, i64)]) -> TestResult {
    // Establish the predecessor snapshot without synthetic callback definitions.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    for (track, value) in tracks {
        let inserted = sqlx::query(
            "INSERT INTO game_character_quest_tracks
              (character_id,track_key,quest_key,value,committed_character_revision)
             SELECT character_id,$2,$3,$4,character_revision FROM game_character_roots
             WHERE character_id=encode($1,'hex')::uuid",
        )
        .bind(id(41).as_slice())
        .bind(track)
        .bind(QUEST)
        .bind(value)
        .execute(&mut *tx)
        .await?;
        assert_eq!(inserted.rows_affected(), 1);
    }
    tx.commit().await?;
    Ok(())
}

pub(super) async fn seed_loop(h: &Harness, repeat: bool) -> TestResult {
    seed_tracks(h, &[
        (&format!("{ROTTIN_STATE}questline"), 2),
        (TRACK, if repeat { 2 } else { 0 }),
        (COOLDOWN, i64::from(repeat)),
    ]).await
}

async fn ribbons(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT COALESCE(sum(quantity),0)::bigint FROM game_item_instances
         WHERE definition_production_key=$1 AND lifecycle=1",
    )
    .bind(RIBBON)
    .fetch_one(&h.pool)
    .await?)
}

async fn open_lucky_loop(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
) -> TestResult<oteryn_protocol_oteryn::world_spatial_entities::EntityRef> {
    let [npc] = install(owner, admitted, [ROTTIN]).await?;
    for (command, text) in [(1700, "hi"), (1701, "task"), (1702, "lucky charms")] {
        talk(owner, admitted, npc, command, text).await?;
    }
    assert_eq!(talk(owner, admitted, npc, 1703, "yes").await?, source_reply("line-225")?);
    assert_eq!(ribbons(h).await?, 7);
    let active: i64 = sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=$1")
        .bind(TRACK).fetch_one(&h.pool).await?;
    assert_eq!(active, 1);
    talk(owner, admitted, npc, 1704, "task").await?;
    assert_eq!(talk(owner, admitted, npc, 1705, "lucky charms").await?, source_reply("line-345")?);
    Ok(npc)
}

pub(super) async fn lucky_charms(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
) -> TestResult {
    let [npc] = install(owner, admitted, [ROTTIN]).await?;
    for (command, text) in [(1700, "hi"), (1701, "mission"), (1702, "yes")] {
        talk(owner, admitted, npc, command, text).await?;
    }
    assert_eq!(talk(owner, admitted, npc, 1703, "yes").await?, source_reply("line-152")?);
    assert_eq!(ribbons(h).await?, 7);
    let mission = format!("{ROTTIN_STATE}mission01");
    let stage = || sqlx::query_scalar::<_, i64>(
        "SELECT value FROM game_character_quest_tracks WHERE track_key=$1",
    ).bind(&mission).fetch_one(&h.pool);
    assert_eq!(stage().await?, 1);
    let before_xp = experience(h).await?;
    talk(owner, admitted, npc, 1704, "mission").await?;
    assert_eq!(talk(owner, admitted, npc, 1705, "yes").await?, source_reply("line-163")?);
    assert_eq!(stage().await?, 2);
    assert_eq!(experience(h).await?, before_xp + 100);
    Ok(())
}

pub(super) fn source_reply(branch: &str) -> TestResult<&'static [String]> {
    classic_dialogue::reply(ROTTIN, branch).ok_or_else(|| format!("missing source reply {branch}").into())
}

pub(super) async fn seed_inventory(h: &Harness, count: usize) -> TestResult {
    super::item_look_fixtures::seed_main_backpack(h, &vec![(CHARM, 1); count]).await
}

pub(super) async fn refuse_short_hand_in(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
) -> TestResult {
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let npc = open_lucky_loop(owner, admitted, h).await?;
    let before_xp = experience(h).await?;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("dialogue clock")?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "quest fence")?
        .ok_or("quest fence")?;
    let refusal = owner
        .npc_talk(
            admitted.runtime_actor.ok_or("quest actor")?,
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id: 1706,
                item_fence: Some(fence.item_fence()),
            },
            NpcTalkIntent {
                npc_actor: npc,
                text: "yes".into(),
            },
        )
        .await;
    assert_eq!(refusal.disposition, NpcIntentDisposition::Rejected);
    assert!(!refusal.unknown);
    assert_eq!(
        refusal.conversation.ok_or("refusal reply")?.lines,
        source_reply("line-258")?
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
    let npc = open_lucky_loop(owner, admitted, h).await?;
    let before_xp = experience(h).await?;
    let before_stamp = i64::from(repeat);
    assert_eq!(
        talk(owner, admitted, npc, 1706, "yes").await?,
        source_reply("line-239")?
    );
    assert_eq!(experience(h).await?, before_xp + 100);
    let actual: (i64, i64, i64, i64, i64) = sqlx::query_as(
        "SELECT
         (SELECT COALESCE(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1),
         (SELECT COALESCE(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key=$2 AND lifecycle=1),
         (SELECT COALESCE(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key=$3 AND lifecycle=1),
         (SELECT COALESCE(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key=$4 AND lifecycle=1),
         (SELECT count(*) FROM game_quest_reward_sources WHERE transition_key=$5 AND mint_transaction_id IS NOT NULL)"
    ).bind(CHARM).bind(PLATINUM).bind(RING).bind(HORN).bind(payout_key(repeat)).fetch_one(&h.pool).await?;
    assert!(matches!(actual, (0, 5, 0, 0, 1) | (0, 3, 1, 0, 1) | (0, 0, 0, 1, 1)), "{actual:?}");
    let (awarded_bundle, retained_odds, horn_odds, percentile): (serde_json::Value, i32, i32, Option<i32>) =
        sqlx::query_as(
            "SELECT plan->'give', (plan #>> '{weighted_rewards,0,chance}')::integer,
                (plan #>> '{weighted_rewards,1,chance}')::integer,
                (plan->>'reward_percentile')::integer
         FROM game_quest_reward_sources WHERE transition_key=$1",
        )
        .bind(payout_key(repeat))
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(retained_odds, 9);
    let percentile = percentile.ok_or("weighted payout receipt lacks the recorded draw")?;
    assert!((0..100).contains(&percentile));
    assert_eq!(horn_odds, 1);
    assert_eq!((90..99).contains(&percentile), actual.2 == 1);
    assert_eq!(percentile == 99, actual.3 == 1);
    if actual.3 == 1 {
        assert_eq!(awarded_bundle["item"], HORN);
        assert_eq!(awarded_bundle["count"], 1);
    } else {
        assert_eq!(awarded_bundle["item"], PLATINUM);
        assert_eq!(awarded_bundle["count"], actual.1);
    }
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
