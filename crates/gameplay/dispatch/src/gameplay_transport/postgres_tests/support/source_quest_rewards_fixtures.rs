use super::{AdmittedSession, ComposedFreshAdmission, TestResult, fixture::Harness};
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
