use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;

pub(super) const OUTFOX_CLAIM: &str =
    "oteryn:reward-claim.quest.u8_1.to_outfox_afox_quest.mining_helmet";

pub(super) async fn accept_outfox_mission(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    let [budrik] =
        super::classic_quests_fixtures::install(owner, admitted, ["oteryn:npc.budrik"]).await?;
    for (command, text) in [(9000, "hi"), (9001, "mission"), (9002, "yes")] {
        assert_eq!(
            super::qengine_rewards_fixtures::talk(owner, admitted, budrik, command, text).await?,
            NpcIntentDisposition::Ok,
            "{text}"
        );
    }
    Ok(())
}

/// A legacy once claim served by its exact World callback (legacy_world_claims.rs)
/// commits through the quest writer, which needs the admitted quest session.
pub(super) fn legacy_world_owned(claim: &str) -> TestResult<bool> {
    use oteryn_gameplay_quests::quest::world_callbacks::{legacy_world_claim, load};
    for callback in load()?.iter() {
        if legacy_world_claim(callback)?.is_some_and(|binding| binding.key == claim) {
            return Ok(true);
        }
    }
    Ok(false)
}
