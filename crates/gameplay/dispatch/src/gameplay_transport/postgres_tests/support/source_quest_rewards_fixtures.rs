use super::{AdmittedSession, ComposedFreshAdmission, TestResult, fixture::Harness};
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
use crate::durability::character_authority::ReconciledCharacterAuthority;

pub(super) fn paradox_reward(claim: &str) -> bool {
    claim.starts_with("oteryn:reward-claim.quest.u7_24.the_paradox_tower.reward.")
}

pub(super) async fn seed_paradox_riddle(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    harness: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    // This cohort isolates chest USE. The story case earns this value by NPC talk.
    let fence = owner.current_quest_fence(admitted.game_session_id).await
        .map_err(|()| "quest fence")?.ok_or("quest fence")?;
    let character_id = fence.character_id.as_bytes().as_slice();
    let level: i64 = sqlx::query_scalar("SELECT level FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid")
        .bind(character_id).fetch_one(&harness.pool).await?;
    super::l6_paradox_fixtures::raise_level(owner, admitted, authority, harness, u32::try_from(level)? + 1).await?;
    let mut tx = harness.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx).await?;
    let seeded = sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) SELECT character_id,'oteryn:quest-progress/quest/u7_24/the_paradox_tower/mathemagics','oteryn:quest.the_paradox_tower_quest',2,character_revision FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
        .bind(character_id)
        .execute(&mut *tx).await?;
    assert_eq!(seeded.rows_affected(), 1);
    tx.commit().await?;
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
    if claim != "oteryn:reward-claim.quest.u8_2.the_thieves_guild_quest.reward_oasis"
        && !claim.starts_with("oteryn:reward-claim.quest.u7_8.the_shattered_isles.")
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
