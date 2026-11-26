use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::world_callbacks::Position;

use super::{
    AdmittedSession, ComposedFreshAdmission, TestResult,
    fixture::{Harness, debug},
};

pub(super) const NAME: &str = "qengine-rewards-newhaven-gustavo";
pub(super) const START: Position = Position {
    x: 32559,
    y: 32488,
    floor: -7,
};
const STAGE: &str = "oteryn:quest-progress/newhaven/protect-the-village";
const KILLS: &str = "oteryn:quest-progress/newhaven/protect-the-village/kills";
const FOOTMAN: &str = "oteryn:creature.muglex_clan_footman";
const ASSASSIN: &str = "oteryn:creature.muglex_clan_assassin";

async fn value(h: &Harness, track: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT COALESCE((SELECT value FROM game_character_quest_tracks WHERE track_key=$1),0)",
    )
    .bind(track)
    .fetch_one(&h.pool)
    .await?)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    use super::{
        classic_quests_fixtures::{install_positions, talk},
        qengine_kill_stories_fixtures::death,
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
    talk(owner, admitted, npc, 88003, "mission").await?;
    assert_eq!(value(h, STAGE).await?, 1);

    for credit in 1..=20 {
        let creature = if credit % 2 == 0 { ASSASSIN } else { FOOTMAN };
        death(owner, admitted, creature, 88100 + credit).await?;
        assert_eq!(value(h, KILLS).await?, i64::try_from(credit)?);
    }
    death(owner, admitted, ASSASSIN, 88121).await?;
    assert_eq!(value(h, KILLS).await?, 20);

    talk(owner, admitted, npc, 88122, "goblins").await?;
    assert_eq!(value(h, STAGE).await?, 2);
    talk(owner, admitted, npc, 88123, "goblins").await?;
    let minted: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i3052'",
    )
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(minted, 1);
    Ok(())
}
