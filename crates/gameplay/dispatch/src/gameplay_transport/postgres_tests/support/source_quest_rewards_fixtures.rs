use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::spell_premium_coordinator::{
    SpellPremiumCoordinator, purchase_fixture,
};
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

pub(super) async fn reward_premium(
    h: &super::fixture::Harness,
    claim: &str,
) -> TestResult<Option<SpellPremiumCoordinator>> {
    if !claim.starts_with("oteryn:reward-claim.quest.u7_8.the_shattered_isles.")
        && !claim.starts_with("oteryn:reward-claim.quest.u8_54.the_new_frontier.")
    {
        return Ok(None);
    }
    let (consumer, clock, activation, _) = purchase_fixture::current(super::fixture::id(40));
    h.root
        .latch_premium_activation(
            &activation.activation_id,
            activation.switch_over_us,
            activation.switch_over_us + 60_000_000,
        )
        .await
        .map_err(super::fixture::debug)?;
    Ok(Some(SpellPremiumCoordinator::new(
        consumer,
        Some(clock),
        Some(activation),
        Some(h.root.clone()),
    )))
}

pub(super) async fn admit_reward_character(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    claim: &str,
) -> TestResult {
    if owner.premium_coordinator.is_some() {
        owner.admit_premium(super::fixture::id(40), admitted.game_session_id);
    }
    if claim.starts_with("oteryn:reward-claim.quest.u7_8.the_shattered_isles.") {
        super::questpc8_magic_sword_fixtures::raise_level(owner, admitted, 20, 203).await?;
    }
    Ok(())
}
