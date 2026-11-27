use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::reward_callbacks::fixture_newhaven_gustavo_predecessor;
use oteryn_gameplay_quests::quest::world_callbacks::Position;

use super::{
    AdmittedSession, ComposedFreshAdmission, TestResult,
    fixture::{Harness, debug, id},
};

pub(super) const NAME: &str = "qengine-rewards-newhaven-gustavo";
pub(super) const PUBLISHED_SAVE: &str = "qengine-rewards-l11-newhaven-published-save";
pub(super) const START: Position = Position {
    x: 32559,
    y: 32488,
    floor: -7,
};
const STAGE: &str = "oteryn:quest-progress/newhaven/protect-the-village";
const KILLS: &str = "oteryn:quest-progress/newhaven/protect-the-village/kills";
const FOOTMAN: &str = "oteryn:creature.muglex_clan_footman";
const ASSASSIN: &str = "oteryn:creature.muglex_clan_assassin";
const QUEST: &str = "oteryn:quest.newhaven_quest";
const REPORT: &str = "oteryn:quest-reward/newhaven-gustavo-turn-in";
const RING: u32 = 52732;

async fn value(h: &Harness, track: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT COALESCE((SELECT value FROM game_character_quest_tracks WHERE track_key=$1),0)",
    )
    .bind(track)
    .fetch_one(&h.pool)
    .await?)
}

async fn experience(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT total_experience FROM game_character_progression_state
         WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(id(41).as_slice())
    .fetch_one(&h.pool)
    .await?)
}

async fn restore_published_pin(h: &Harness) -> TestResult {
    let mut save = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *save)
        .await?;
    let pin = fixture_newhaven_gustavo_predecessor();
    for sql in [
        "UPDATE game_character_quest_states SET definition_hash=$1
         WHERE quest_key=$2 AND character_id=encode($3,'hex')::uuid",
        "UPDATE game_character_quest_receipts SET definition_hash=$1
         WHERE quest_key=$2 AND character_id=encode($3,'hex')::uuid",
    ] {
        let updated = sqlx::query(sql)
            .bind(pin.as_slice())
            .bind(QUEST)
            .bind(id(41).as_slice())
            .execute(&mut *save)
            .await?;
        assert_eq!(updated.rows_affected(), 1);
    }
    save.commit().await?;
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    published_save: bool,
) -> TestResult {
    use super::{
        classic_quests_fixtures::{install_positions, talk},
        qengine_kill_stories_fixtures::death,
        qengine_rewards_fixtures::talk_outcome,
        questpc7_outfit_items_fixtures::quantity,
    };

    let [npc] = install_positions(
        owner,
        admitted,
        ["oteryn:npc.gustavo_the_guard"],
        [Position { x: 32560, ..START }],
    )
    .await?;
    owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .ok_or("native policies")?
        .install_companion_policies(&mut *owner.runtime.lock().await)
        .map_err(debug)?;

    death(owner, admitted, FOOTMAN, 88000).await?;
    assert_eq!(value(h, KILLS).await?, 0);

    talk(owner, admitted, npc, 88001, "hi").await?;
    talk(owner, admitted, npc, 88002, "goblins").await?;
    assert_eq!(value(h, STAGE).await?, 1);
    if published_save {
        restore_published_pin(h).await?;
    }
    talk(owner, admitted, npc, 88003, "mission").await?;
    assert_eq!(value(h, STAGE).await?, 1);

    let initial_rings = quantity(h, RING).await?;
    for credit in 1..=19 {
        let creature = if credit % 2 == 0 { ASSASSIN } else { FOOTMAN };
        death(owner, admitted, creature, 88100 + credit).await?;
        assert_eq!(value(h, KILLS).await?, i64::try_from(credit)?);
    }
    talk(owner, admitted, npc, 88130, "hi").await?;
    talk(owner, admitted, npc, 88131, "mission").await?;
    assert_eq!(value(h, STAGE).await?, 1);
    assert_eq!(quantity(h, RING).await?, initial_rings);

    death(owner, admitted, ASSASSIN, 88120).await?;
    death(owner, admitted, ASSASSIN, 88121).await?;
    assert_eq!(value(h, KILLS).await?, 20);
    let deaths: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_receipts
         WHERE quest_key=$1 AND cause_kind='creature_death'",
    )
    .bind(QUEST)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(deaths, 20, "one receipt per eligible kill including replay");

    let before = experience(h).await?;
    talk(owner, admitted, npc, 88132, "hi").await?;
    talk(owner, admitted, npc, 88133, "goblins").await?;
    assert_eq!(value(h, STAGE).await?, 2);
    assert_eq!(quantity(h, RING).await?, initial_rings + 1);
    assert_eq!(experience(h).await?, before + 800);
    talk_outcome(owner, admitted, npc, 88133, "goblins").await?;
    talk(owner, admitted, npc, 88134, "goblins").await?;
    assert_eq!(quantity(h, RING).await?, initial_rings + 1);
    assert_eq!(experience(h).await?, before + 800);
    let saved_hashes: Vec<Vec<u8>> = sqlx::query_scalar(
        "SELECT definition_hash FROM game_character_quest_receipts WHERE transition_key=$1",
    )
    .bind(REPORT)
    .fetch_all(&h.pool)
    .await?;
    assert_eq!(saved_hashes.len(), 1, "one report receipt after retries");
    if published_save {
        assert_eq!(saved_hashes[0], fixture_newhaven_gustavo_predecessor());
    }
    let sources: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_quest_reward_sources WHERE transition_key=$1",
    )
    .bind(REPORT)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(sources, 1);
    Ok(())
}
