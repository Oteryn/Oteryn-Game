//! Initial Gnomus ordnance admission only; no escort or warzone completion seeds.
use super::classic_quests::{install, progress, talk};
use super::fixture::Harness;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;

const PREFIX: &str = "oteryn:quest-progress/quest/u11_50/dangerous_depths/";

#[test]
fn questpc2_gnomus_ordnance_requires_live_confirmation_and_commits_initial_tracks() -> TestResult {
    super::qengine_rewards::scenario("questpc2-gnomus-ordnance")
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    let version: String = sqlx::query_scalar("SHOW server_version_num")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("QUESTPC2_GNOMUS_POSTGRES_SERVER_VERSION={version}");
    let [npc] = install(owner, admitted, ["oteryn:npc.gnomus"]).await?;
    let before: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
        .fetch_one(&h.pool)
        .await?;
    talk(owner, admitted, npc, 34000, "hi").await?;
    super::questpc2_oldrak::unoffered_yes(owner, admitted, npc, 34001).await?;
    // Unlike Albinius's independent ifs, this source elseif only offers here.
    let combined = talk(owner, admitted, npc, 34002, "ordnance yes").await?;
    assert_eq!(
        combined.len(),
        5,
        "source ordnance briefing missing: {combined:?}"
    );
    assert_eq!(
        progress(owner, admitted, authority, &format!("{PREFIX}questline")).await?,
        0
    );
    assert_eq!(
        progress(
            owner,
            admitted,
            authority,
            &format!("{PREFIX}gnomes/ordnance")
        )
        .await?,
        0
    );
    // An unimplemented task must never leave the previous ordnance offer usable.
    talk(owner, admitted, npc, 34003, "measurements").await?;
    talk(owner, admitted, npc, 34004, "yes").await?;
    assert_eq!(
        progress(
            owner,
            admitted,
            authority,
            &format!("{PREFIX}gnomes/ordnance")
        )
        .await?,
        0,
        "unsupported measurements must invalidate the old ordnance confirmation"
    );
    talk(owner, admitted, npc, 34005, "bye").await?;
    talk(owner, admitted, npc, 34006, "hi").await?;
    super::questpc2_oldrak::unoffered_yes(owner, admitted, npc, 34007).await?;
    assert_eq!(
        progress(owner, admitted, authority, &format!("{PREFIX}questline")).await?,
        0
    );
    let offer = talk(owner, admitted, npc, 34008, "ordnance").await?;
    assert_eq!(offer.len(), 5);
    assert!(offer[2].contains("reinforcements arrive at the crystal teleporter"));
    let accepted = talk(owner, admitted, npc, 34009, "yes").await?;
    assert_eq!(accepted.len(), 2);
    assert!(accepted[0].starts_with("Excellent, just follow the path"));
    assert_eq!(
        progress(owner, admitted, authority, &format!("{PREFIX}questline")).await?,
        1
    );
    assert_eq!(
        progress(
            owner,
            admitted,
            authority,
            &format!("{PREFIX}gnomes/ordnance")
        )
        .await?,
        1
    );
    for field in [
        "gnomes/gnomes_count",
        "gnomes/crawlers_count",
        "gnomes/time_task_ordnance",
        "gnomes/points",
    ] {
        assert_eq!(
            progress(owner, admitted, authority, &format!("{PREFIX}{field}")).await?,
            0,
            "an admission is not an earned escort or reward"
        );
    }
    super::questpc2_oldrak::unoffered_yes(owner, admitted, npc, 34010).await?;
    let pending = talk(owner, admitted, npc, 34011, "ordnance").await?;
    assert!(
        pending
            .iter()
            .any(|s| s == "Come back when you have finished your job.")
    );
    super::questpc2_oldrak::unoffered_yes(owner, admitted, npc, 34012).await?;
    let after: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(
        after - before,
        1,
        "only the accepted admission creates a receipt"
    );
    Ok(())
}
