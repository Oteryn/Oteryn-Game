//! Exercise the authored reward owner through live Rottin TALK and PostgreSQL.
use super::classic_quests_fixtures::{install, talk};
use super::fixture::{Harness, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use serde_json::json;

const TRACK: &str =
    "oteryn:quest-progress/crystalserver/quest/u8_7/rottin_wood_and_the_married_men/task01";
const COOLDOWN: &str = "oteryn:quest-progress/crystalserver/kv/lucky-loop-task";
const ACHIEVEMENT: &str = "oteryn:achievement/bunny_slipped";
const PAYOUT: &str = "oteryn:quest-reward/rottin-wood/lucky-loop";
const ROTTIN: &str = "oteryn:npc.rottin_wood";
const PLATINUM: &str = "oteryn:item.tibia.i3035";
const RING: &str = "oteryn:item.tibia.i3053";
const CHARM: &str = "oteryn:item.tibia.i12173";
const HORN: &str = "oteryn:item.tibia.i12260";
const RIBBON: &str = "oteryn:item.tibia.i12171";
pub(super) const QUESTLINE: &str =
    "oteryn:quest-progress/crystalserver/quest/u8_7/rottin_wood_and_the_married_men/questline";
const MISSION: &str =
    "oteryn:quest-progress/crystalserver/quest/u8_7/rottin_wood_and_the_married_men/mission01";

fn payout_key(repeat: bool) -> &'static str {
    if repeat {
        "oteryn:quest-reward/rottin-wood/lucky-loop-repeat"
    } else {
        PAYOUT
    }
}

pub(super) async fn seed_loop_prerequisites(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    repeat: bool,
) -> TestResult {
    let mut tracks = vec![(QUESTLINE.into(), 2)];
    if repeat {
        tracks.extend([
            (TRACK.into(), 2),
            (COOLDOWN.into(), 1),
            ("oteryn:quest-progress/crystalserver/kv/lucky-loop-rewarded".into(), 1),
        ]);
    }
    super::qengine_outfit_stories_fixtures::seed_quest_prerequisites(h, owner, admitted, &tracks, 168)
        .await
}

async fn open_lucky_loop(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
) -> TestResult<oteryn_protocol_oteryn::world_spatial_entities::EntityRef> {
    let [npc] = install(owner, admitted, [ROTTIN]).await?;
    for (command, text) in [
        (1701, "hi"),
        (1702, "task"),
        (1703, "lucky charms"),
        (1704, "yes"),
        (1705, "task"),
        (1706, "lucky charms"),
    ] {
        talk(owner, admitted, npc, command, text).await?;
    }
    assert_ribbons(h, TRACK, 1).await?;
    Ok(npc)
}

async fn assert_ribbons(h: &Harness, track: &str, expected_stage: i64) -> TestResult {
    let (ribbons, stage): (i64, i64) = sqlx::query_as(
        "SELECT
         (SELECT COALESCE(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1),
         (SELECT value FROM game_character_quest_tracks WHERE track_key=$2)",
    )
    .bind(RIBBON)
    .bind(track)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!((ribbons, stage), (7, expected_stage));
    Ok(())
}

pub(super) async fn accept_lucky_charms(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
) -> TestResult {
    let [npc] = install(owner, admitted, [ROTTIN]).await?;
    for (command, text) in [(1701, "hi"), (1702, "mission"), (1703, "yes"), (1704, "yes")] {
        talk(owner, admitted, npc, command, text).await?;
    }
    assert_ribbons(h, MISSION, 1).await?;
    let stage: i64 = sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=$1")
        .bind(QUESTLINE)
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(stage, 1);
    Ok(())
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
                command_id: 1707,
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
        ["You do not have sufficient rabbit's foot."]
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
        talk(owner, admitted, npc, 1707, "yes").await?,
        ["Good hunt. That will be enough to help us uhm... get through the winter yes. Here is your reward for today."]
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
    let (awarded_bundle, ordinary_bonus, rare_bonus, percentile): (serde_json::Value, i32, i32, Option<i32>) =
        sqlx::query_as(
            "SELECT plan->'give', (plan #>> '{weighted_rewards,0,chance}')::integer,
                (plan #>> '{weighted_rewards,1,chance}')::integer,
                (plan->>'reward_percentile')::integer
         FROM game_quest_reward_sources WHERE transition_key=$1",
        )
        .bind(payout_key(repeat))
        .fetch_one(&h.pool)
        .await?;
    assert_eq!((ordinary_bonus, rare_bonus), (9, 1));
    let percentile = percentile.ok_or("weighted payout receipt lacks the recorded draw")?;
    let (expected, output, count) = match percentile {
        0..=89 => ((0, 5, 0, 0, 1), PLATINUM, 5),
        90..=98 => ((0, 3, 1, 0, 1), PLATINUM, 3),
        99 => ((0, 0, 0, 1, 1), HORN, 1),
        _ => return Err("invalid payout percentile".into()),
    };
    assert_eq!(actual, expected);
    assert_eq!(awarded_bundle["item"], output);
    assert_eq!(awarded_bundle["count"], count);
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
