//! Drive a real retained hammer USE and the resulting Disrepair hand-in.
use super::classic_quests_fixtures::{install, talk};
use super::fixture::Harness;
use super::rottin_bushwhacking_fixtures::use_quest_tool;
use super::rottin_rewards_fixtures::experience;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::map::{overlay::TilePos, view::MapFacts};
use oteryn_protocol_oteryn::world_object::UseDisposition;

const ROTTIN: &str = "oteryn:npc.rottin_wood";
const WALLS: &str = oteryn_gameplay_quests::quest::classic_talk::ROTTIN_WALLCOUNT;
const REPAIR: &str = "oteryn:quest-callback/rottin-repair-42501";
const DISREPAIR: &str = "oteryn:quest-reward/rottin-wood/disrepair";
const HANDYMAN: &str = "oteryn:quest-reward/rottin-wood/handyman";
const COOLDOWN: &str = "oteryn:quest-progress/crystalserver/kv/handyman-task";

pub(super) const WALL: TilePos = TilePos { x: 32647, y: 32216, floor: -7 };

pub(super) async fn seed_prerequisites(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    handyman: bool,
) -> TestResult {
    let prefix = "oteryn:quest-progress/crystalserver/quest/u8_7/rottin_wood_and_the_married_men/";
    let mut tracks = vec![(WALLS.into(), 4)];
    if handyman {
        tracks.extend([(format!("{prefix}questline"), 4), (format!("{prefix}task02"), 1), (COOLDOWN.into(), 1)]);
    } else {
        tracks.extend([(format!("{prefix}questline"), 3), (format!("{prefix}mission02"), 1)]);
    }
    super::qengine_outfit_stories_fixtures::seed_quest_prerequisites(h, owner, admitted, &tracks, 169)
        .await
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    h: &Harness,
    handyman: bool,
) -> TestResult {
    let before_xp = experience(h).await?;
    let wall = WALL;
    assert_eq!(
        use_quest_tool(owner, admitted, wall, 2, 1602, 71).await?,
        UseDisposition::Committed
    );
    let map = owner.bundle_map().ok_or("repair map")?;
    let due = map.facts.next_movement_decay_time().ok_or("wall reset")?;
    assert_eq!(
        map.facts
            .base_entry(wall, 2, 7)
            .ok_or("repaired wall")?
            .appearance_id,
        6474
    );
    assert_eq!(map.facts.expire_movement_items(due - 1), 0);
    assert_eq!(
        use_quest_tool(owner, admitted, wall, 2, 1603, 71).await?,
        UseDisposition::NothingToUse,
        "a repaired wall cannot count again before it breaks"
    );
    let map = owner.bundle_map().ok_or("repair map")?;
    assert_eq!(map.facts.expire_movement_items(due), 1);
    assert_eq!(
        map.facts
            .base_entry(wall, 2, 7)
            .ok_or("restored wall")?
            .appearance_id,
        12183
    );
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(121))
        .ok_or("wall reset clock")?;
    assert_eq!(
        use_quest_tool(owner, admitted, wall, 2, 1604, 71).await?,
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
    let [npc] = install(owner, &admitted, [ROTTIN]).await?;
    talk(owner, &admitted, npc, 1605, "hi").await?;
    if handyman {
        talk(owner, &admitted, npc, 1606, "task").await?;
        talk(owner, &admitted, npc, 1607, "repairs").await?;
    } else {
        talk(owner, &admitted, npc, 1606, "mission").await?;
    }
    let branch = if handyman { "line-279" } else { "line-182" };
    assert_eq!(
        talk(owner, &admitted, npc, 1608, "yes").await?,
        oteryn_gameplay_quests::quest::classic_dialogue::reply(ROTTIN, branch)
            .ok_or("shipped repair payout reply")?,
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
