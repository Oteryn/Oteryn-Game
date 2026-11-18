//! Cult reports and finale through Gerimor's persisted dialogue programme.
use super::classic_quests::{install, progress, talk};
use super::fixture::{Harness, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;

pub(super) const NAME: &str = "qengine-rewards-cooldmech-gerimor";
const QUEST: &str = "oteryn:quest.cults_of_tibia_quest";
const PREFIX: &str = "oteryn:quest-progress/quest/u11_40/cults_of_tibia/";

#[test]
fn cooldmech_gerimor_reports_and_finale_settle_once_postgres() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::qengine_rewards::scenario(NAME)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    harness: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    let [gerimor] = install(owner, admitted, ["oteryn:npc.gerimor"]).await?;
    for (id, text) in [(1, "hi"), (2, "mission"), (3, "humans"), (4, "yes")] {
        talk(owner, admitted, gerimor, id, text).await?;
    }
    assert_eq!(
        progress(
            owner,
            admitted,
            authority,
            &format!("{PREFIX}humans/mission")
        )
        .await?,
        1,
    );
    // Import the existing character's completed investigations; only Gerimor can report them.
    for (track, value) in [
        ("barkless/mission", 7),
        ("humans/mission", 2),
        ("life/mission", 10),
        ("minotaurs/mission", 6),
        ("misguided/mission", 5),
        ("mot_a/mission", 15),
        ("orcs/mission", 3),
    ] {
        set_track(harness, track, value).await?;
    }
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let initial = experience(harness).await?;
    talk(owner, admitted, gerimor, 5, "mission").await?;
    assert_eq!(
        progress(
            owner,
            admitted,
            authority,
            &format!("{PREFIX}final_boss/mission")
        )
        .await?,
        0
    );
    talk(owner, admitted, gerimor, 6, "humans").await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    assert_eq!(
        progress(
            owner,
            admitted,
            authority,
            &format!("{PREFIX}humans/mission")
        )
        .await?,
        3
    );
    assert_eq!(experience(harness).await?, initial + 25_000);

    talk(owner, admitted, gerimor, 7, "mission").await?;
    assert_eq!(
        progress(
            owner,
            admitted,
            authority,
            &format!("{PREFIX}final_boss/access_door")
        )
        .await?,
        1
    );
    assert_eq!(
        progress(
            owner,
            admitted,
            authority,
            &format!("{PREFIX}final_boss/mission")
        )
        .await?,
        1
    );
    set_track(harness, "final_boss/mission", 2).await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let boxes = super::qengine_voucher_stories::quantity(harness, 26186).await?;
    let crowns = super::qengine_voucher_stories::quantity(harness, 50313).await?;
    talk(owner, admitted, gerimor, 8, "mission").await?;
    talk(owner, admitted, gerimor, 9, "yes").await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    assert_eq!(
        progress(
            owner,
            admitted,
            authority,
            &format!("{PREFIX}final_boss/mission")
        )
        .await?,
        3
    );
    assert_eq!(experience(harness).await?, initial + 75_000);
    assert_eq!(
        super::qengine_voucher_stories::quantity(harness, 26186).await?,
        boxes + 1
    );
    assert_eq!(
        super::qengine_voucher_stories::quantity(harness, 50313).await?,
        crowns + 1
    );
    talk(owner, admitted, gerimor, 10, "mission").await?;
    talk(owner, admitted, gerimor, 11, "yes").await?;
    assert_eq!(experience(harness).await?, initial + 75_000);
    assert_eq!(
        super::qengine_voucher_stories::quantity(harness, 26186).await?,
        boxes + 1
    );
    assert_eq!(
        super::qengine_voucher_stories::quantity(harness, 50313).await?,
        crowns + 1
    );
    Ok(())
}

async fn set_track(harness: &Harness, suffix: &str, value: i64) -> TestResult {
    let mut tx = harness.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    let result = sqlx::query(
        "INSERT INTO game_character_quest_tracks(
             character_id, track_key, quest_key, value, committed_character_revision
         ) SELECT character_id, $2, $3, $4, character_revision
             FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid
         ON CONFLICT (character_id, track_key)
         DO UPDATE SET value=EXCLUDED.value, committed_character_revision=EXCLUDED.committed_character_revision",
    ).bind(id(41).as_slice()).bind(format!("{PREFIX}{suffix}")).bind(QUEST).bind(value)
    .execute(&mut *tx).await?;
    assert_eq!(result.rows_affected(), 1);
    tx.commit().await?;
    Ok(())
}

async fn experience(harness: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT total_experience FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid",
    ).bind(id(41).as_slice()).fetch_one(&harness.pool).await?)
}
