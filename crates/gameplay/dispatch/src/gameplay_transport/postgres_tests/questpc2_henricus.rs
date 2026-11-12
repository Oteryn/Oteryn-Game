//! Henricus's source recruitment and first mission; no interrogation completion seed.
use super::classic_quests::{install, progress, talk};
use super::fixture::Harness;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;

const PREFIX: &str = "oteryn:quest-progress/quest/u8_2/the_inquisition_quest/";

#[test]
fn questpc2_henricus_recruitment_and_first_mission_are_topic_fenced_and_atomic() -> TestResult {
    super::qengine_rewards::scenario("questpc2-henricus-start")
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    let version: String = sqlx::query_scalar("SHOW server_version_num")
        .fetch_one(&h.pool).await?;
    eprintln!("QUESTPC2_HENRICUS_POSTGRES_SERVER_VERSION={version}");
    let [npc] = install(owner, admitted, ["oteryn:npc.henricus"]).await?;
    let before: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
        .fetch_one(&h.pool).await?;
    talk(owner, admitted, npc, 31000, "hi").await?;
    super::questpc2_oldrak::unoffered_yes(owner, admitted, npc, 31001).await?;
    assert_eq!(progress(owner, admitted, authority, &format!("{PREFIX}questline")).await?, 0);
    let offer = talk(owner, admitted, npc, 31002, "join").await?;
    assert!(offer.iter().any(|s| s.contains("Do you want to join the inquisition?")), "missing source recruitment offer: {offer:?}");
    let declined = talk(owner, admitted, npc, 31003, "no").await?;
    assert!(declined.iter().any(|s| s == "Then no."));
    super::questpc2_oldrak::unoffered_yes(owner, admitted, npc, 31004).await?;
    assert_eq!(progress(owner, admitted, authority, &format!("{PREFIX}questline")).await?, 0);
    talk(owner, admitted, npc, 31005, "mission").await?;
    talk(owner, admitted, npc, 31006, "bye").await?;
    talk(owner, admitted, npc, 31007, "hi").await?;
    super::questpc2_oldrak::unoffered_yes(owner, admitted, npc, 31008).await?;
    assert_eq!(progress(owner, admitted, authority, &format!("{PREFIX}questline")).await?, 0);
    talk(owner, admitted, npc, 31009, "join").await?;
    let joined = talk(owner, admitted, npc, 31010, "yes").await?;
    assert!(joined.iter().any(|s| s.contains("Now you are a member of the inquisition")));
    assert_eq!(progress(owner, admitted, authority, &format!("{PREFIX}questline")).await?, 1);
    super::questpc2_oldrak::unoffered_yes(owner, admitted, npc, 31011).await?;
    let first = talk(owner, admitted, npc, 31012, "mission").await?;
    assert_eq!(first.len(), 2);
    assert!(first[0].contains("inquisitor's field guide"));
    assert!(first[1].contains("Thaian guards"));
    assert_eq!(progress(owner, admitted, authority, &format!("{PREFIX}questline")).await?, 2);
    assert_eq!(progress(owner, admitted, authority, &format!("{PREFIX}mission01")).await?, 1);
    talk(owner, admitted, npc, 31013, "report").await?;
    let refused = talk(owner, admitted, npc, 31014, "yes").await?;
    assert!(refused.iter().any(|s| s == "You haven't done your mission yet."));
    for flag in ["walter_guard", "kulag_guard", "grof_guard", "miles_guard", "tim_guard"] {
        assert_eq!(progress(owner, admitted, authority, &format!("{PREFIX}{flag}")).await?, 0);
    }
    assert_eq!(progress(owner, admitted, authority, &format!("{PREFIX}questline")).await?, 2);
    assert_eq!(progress(owner, admitted, authority, &format!("{PREFIX}mission01")).await?, 1);
    let after: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
        .fetch_one(&h.pool).await?;
    assert_eq!(after - before, 2, "only joining and the two-track mission start write receipts");
    Ok(())
}
