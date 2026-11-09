//! Fire's fiftieth native death, achievement and completion are one durable commit.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use oteryn_gameplay_quests::quest::world_callbacks::Callback;
use oteryn_protocol_oteryn::world_object::UseDisposition;
use oteryn_protocol_oteryn::world_spatial::StepDirection;

const QUEST: &str = "oteryn:quest.authored.fire_from_the_earth_mini_world_change";
const KILLS: &str = "oteryn:quest-progress/world-change-journal/fire_from_the_earth/kills";
const ACHIEVEMENT: &str = "oteryn:achievement/fire_from_the_earth";
const CREATURES: &[&str] = &[
    "diabolic_imp",
    "dragon",
    "dragon_hatchling",
    "dragon_lord",
    "dragon_lord_hatchling",
    "dragonling",
    "hellfire_fighter",
];

async fn live(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
) -> TestResult<Vec<crate::foundation::ExactActorRef>> {
    let mut actors = Vec::new();
    for creature in CREATURES {
        actors.extend(
            super::engworldchg_journals::live_creatures(
                owner,
                &format!("oteryn:creature.{creature}"),
            )
            .await?,
        );
    }
    Ok(actors)
}

async fn achievements(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT count(*) FROM game_account_achievements WHERE achievement_key=$1",
    )
    .bind(ACHIEVEMENT)
    .fetch_one(&h.pool)
    .await?)
}

async fn completions(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_receipts WHERE quest_key=$1 AND completes",
    )
    .bind(QUEST)
    .fetch_one(&h.pool)
    .await?)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    entry: &Callback,
) -> TestResult<u64> {
    super::qengine_kill_stories::death(owner, admitted, "oteryn:creature.dragon", 26000).await?;
    assert_eq!(super::engworldchg_journals::value(h, KILLS).await?, 0);
    assert_eq!(achievements(h).await?, 0);
    let premium = owner.premium_coordinator;
    owner.premium_coordinator = None;
    assert_eq!(
        super::engworldchg_journals::use_marker(owner, admitted, entry, 1).await?,
        UseDisposition::Rejected
    );
    assert!(
        live(owner).await?.is_empty(),
        "unfenced activation cannot publish the fiery cohort"
    );
    owner.premium_coordinator = premium;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let mut command = 2;
    assert_eq!(
        super::engworldchg_journals::use_marker(owner, admitted, entry, command).await?,
        UseDisposition::Committed
    );
    command += 1;
    let mut credited = 0_i64;
    for wave in 0..8_u64 {
        let cohort = live(owner).await?;
        assert_eq!(
            cohort.len(),
            7,
            "native USE must publish all seven source creature kinds"
        );
        for (ordinal, actor) in cohort.into_iter().enumerate() {
            super::engworldchg_journals::kill_actor(
                owner,
                admitted,
                actor,
                27000 + wave * 7 + u64::try_from(ordinal)?,
            )
            .await?;
            credited += 1;
            assert_eq!(
                super::engworldchg_journals::value(h, KILLS).await?,
                credited
            );
            assert_eq!(
                achievements(h).await?,
                i64::from(credited == 50),
                "no early or repeat achievement"
            );
            assert_eq!(completions(h).await?, i64::from(credited == 50));
            if credited == 50 {
                assert!(
                    live(owner).await?.is_empty(),
                    "the last receipt must retire the six surviving actors"
                );
                break;
            }
        }
        if credited == 50 {
            break;
        }
        super::engworldchg_journals::step(owner, admitted, command, StepDirection::North).await?;
        command += 1;
        assert_eq!(
            super::engworldchg_journals::use_marker(owner, admitted, entry, command).await?,
            UseDisposition::Committed
        );
        command += 1;
    }
    assert_eq!(credited, 50);
    let atomic: bool = sqlx::query_scalar("SELECT q.xmin=r.xmin AND q.xmin=a.xmin AND q.xmin=t.xmin FROM game_character_quest_receipts q JOIN game_account_achievement_grant_requests r ON r.character_id=q.character_id AND r.achievement_key=$1 AND r.source_kind='oteryn:world-change-journal-goal' JOIN game_account_achievements a ON a.account_id=r.account_id AND a.achievement_key=r.achievement_key JOIN game_character_quest_tracks t ON t.character_id=q.character_id AND t.track_key=$2 WHERE q.quest_key=$3 AND q.completes")
        .bind(ACHIEVEMENT).bind(KILLS).bind(QUEST).fetch_one(&h.pool).await?;
    assert!(
        atomic,
        "counter, completion, grant request and account fact must share one PostgreSQL transaction"
    );
    super::engworldchg_journals::step(owner, admitted, command, StepDirection::North).await?;
    command += 1;
    assert_eq!(
        super::engworldchg_journals::use_marker(owner, admitted, entry, command).await?,
        UseDisposition::Rejected
    );
    command += 1;
    assert!(live(owner).await?.is_empty());
    super::qengine_kill_stories::death(owner, admitted, "oteryn:creature.dragon", 28000).await?;
    assert_eq!(super::engworldchg_journals::value(h, KILLS).await?, 50);
    assert_eq!(achievements(h).await?, 1);
    assert_eq!(completions(h).await?, 1);
    let requests: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_account_achievement_grant_requests WHERE achievement_key=$1",
    )
    .bind(ACHIEVEMENT)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(requests, 1);
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let state = h
        .root
        .read_character_quest_state(authority, fence.character_id)
        .await
        .map_err(debug)?;
    assert_eq!(state.tracks().get(KILLS), Some(&50));
    Ok(command)
}
