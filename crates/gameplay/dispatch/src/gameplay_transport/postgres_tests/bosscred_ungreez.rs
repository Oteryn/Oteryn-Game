//! Ungreez death credit is consumed by Henricus's native mission report.
use super::classic_quests::{install, progress, talk};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;

pub(super) const CREATURE: &str = "oteryn:creature.ungreez";
const MISSION: &str = "oteryn:quest-progress/quest/u8_2/the_inquisition_quest/mission06";
const QUESTLINE: &str = "oteryn:quest-progress/quest/u8_2/the_inquisition_quest/questline";

#[test]
fn bosscred_ungreez_commits_only_top_damage_mission_credit() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::srvenc_source_deaths::scenario("bosscred-ungreez", "ungreez")
}

pub(super) async fn report(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    lesser: &mut AdmittedSession,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    let [npc] = install(owner, admitted, ["oteryn:npc.henricus"]).await?;
    talk(owner, admitted, npc, 31_000, "hi").await?;
    talk(owner, admitted, npc, 31_001, "report").await?;
    assert_eq!(progress(owner, admitted, authority, QUESTLINE).await?, 20);
    assert_eq!(progress(owner, admitted, authority, MISSION).await?, 3);
    owner
        .states
        .chat
        .admit(lesser.game_session_id, "Second witness", owner.owner_now());
    owner.states.chat.set_npc_service(lesser.game_session_id);
    talk(owner, lesser, npc, 31_000, "hi").await?;
    talk(owner, lesser, npc, 31_001, "report").await?;
    assert_eq!(progress(owner, lesser, authority, QUESTLINE).await?, 18);
    assert_eq!(progress(owner, lesser, authority, MISSION).await?, 0);
    Ok(())
}
