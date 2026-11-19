//! Drive a real retained hammer USE and the resulting Disrepair hand-in.
use super::classic_quests_fixtures::{install, talk};
use super::fixture::{Harness, id};
use super::rottin_bushwhacking_fixtures::use_quest_tool;
use super::rottin_rewards_fixtures::{experience, seed_tracks, source_reply};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::map::{overlay::TilePos, view::MapFacts};
use oteryn_gameplay_quests::quest::classic_talk::ROTTIN_STATE;
use oteryn_protocol_oteryn::world_object::UseDisposition;

const ROTTIN: &str = "oteryn:npc.rottin_wood";
const HAMMER: &str = "oteryn:item.tibia.i3460";
const WALLS: &str = oteryn_gameplay_quests::quest::classic_talk::ROTTIN_WALLCOUNT;
const REPAIR: &str = "oteryn:quest-callback/rottin-repair-42501";
const DISREPAIR: &str = "oteryn:quest-reward/rottin-wood/disrepair";
const HANDYMAN: &str = "oteryn:quest-reward/rottin-wood/handyman";
const COOLDOWN: &str = "oteryn:quest-progress/crystalserver/kv/handyman-task";
const QUESTLINE: &str =
    "oteryn:quest-progress/crystalserver/quest/u8_7/rottin_wood_and_the_married_men/questline";

pub(super) async fn seed(h: &Harness, handyman: bool) -> TestResult {
    let (track, stage) = if handyman { ("task02", 4) } else { ("mission02", 3) };
    seed_tracks(
        h,
        &[
            (&format!("{ROTTIN_STATE}{track}"), 1),
            (QUESTLINE, stage),
            (WALLS, 4),
            (COOLDOWN, 1),
        ],
    )
    .await
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    h: &Harness,
    handyman: bool,
) -> TestResult {
    let before_xp = experience(h).await?;
    let wall = TilePos {
        x: 32647,
        y: 32216,
        floor: -7,
    };
    let source_id = *owner
        .bundle_map()
        .ok_or("repair map")?
        .overlay
        .base()
        .tile(wall.x, wall.y, wall.floor)
        .ok_or("wall tile")?
        .ids()
        .get(2)
        .ok_or("wall source")?;
    assert_eq!(
        use_quest_tool(owner, admitted, wall, 2, 1602, 71).await?,
        UseDisposition::Committed
    );
    let map = owner.bundle_map().ok_or("repair map")?;
    let due = map.facts.next_movement_decay_time().ok_or("wall reset")?;
    assert_eq!(
        map.facts
            .base_entry(wall, 2, source_id)
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
            .base_entry(wall, 2, source_id)
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
    let (fixed, retained, shipped_hammer): (i64, i64, i64) = sqlx::query_as(
        "SELECT (SELECT value FROM game_character_quest_tracks WHERE track_key=$1),
                (SELECT count(*) FROM game_quest_world_tool_reads WHERE transition_key=$2),
                (SELECT count(*) FROM game_quest_world_tool_reads AS tool
                 JOIN game_item_instances AS item USING (item_instance_id)
                 WHERE tool.transition_key=$2 AND item.item_instance_id=encode($3,'hex')::uuid
                   AND item.definition_production_key=$4 AND item.lifecycle=1)",
    )
    .bind(WALLS)
    .bind(REPAIR)
    .bind(id(71).as_slice())
    .bind(HAMMER)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!((fixed, retained, shipped_hammer), (6, 2, 2));
    let [npc] = install(owner, &admitted, [ROTTIN]).await?;
    talk(owner, &admitted, npc, 1605, "hi").await?;
    if handyman {
        talk(owner, &admitted, npc, 1606, "task").await?;
        talk(owner, &admitted, npc, 1607, "repairs").await?;
    } else {
        talk(owner, &admitted, npc, 1606, "mission").await?;
    }
    assert_eq!(
        talk(owner, &admitted, npc, 1608, "yes").await?,
        source_reply(if handyman { "line-279" } else { "line-182" })?
    );
    assert_eq!(experience(h).await?, before_xp + 500);
    let completed_track = format!("{ROTTIN_STATE}{}", if handyman { "task02" } else { "mission02" });
    let (paid, platinum, completion, questline): (i64, i64, i64, i64) = sqlx::query_as(
        "SELECT
         (SELECT count(*) FROM game_quest_reward_sources
          WHERE transition_key=$1 AND mint_transaction_id IS NOT NULL),
         (SELECT COALESCE(sum(item.quantity),0)::bigint FROM game_quest_reward_sources AS reward
          JOIN game_item_instances AS item ON item.minted_transaction_id=reward.mint_transaction_id
          WHERE reward.transition_key=$1 AND item.definition_production_key='oteryn:item.tibia.i3035'
            AND item.lifecycle=1),
         (SELECT value FROM game_character_quest_tracks WHERE track_key=$2),
         (SELECT value FROM game_character_quest_tracks WHERE track_key=$3)",
    )
    .bind(if handyman { HANDYMAN } else { DISREPAIR })
    .bind(completed_track)
    .bind(QUESTLINE)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!((paid, platinum, completion, questline), (1, 5, 2, 4));
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
    } else {
        super::rottin_bushwhacking_fixtures::accept(owner, &admitted, h, npc).await?;
    }
    Ok(())
}
