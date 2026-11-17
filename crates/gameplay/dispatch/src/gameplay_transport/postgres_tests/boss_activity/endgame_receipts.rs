//! Component proof after real native Instance handoff. The death is supplied
//! through the existing captured-death seam; this does not claim a Primal fight
//! or qualify a Primal template. No Primal catalogue row is seeded.
use super::super::soul_war::{CapturedEndgameDeath, settle_captured_endgame_death};
use super::*;

pub(super) async fn assert_receipts(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
) -> TestResult {
    let fence = crate::gameplay_transport::character_gameplay_fence::current(
        owner,
        admitted.game_session_id,
        false,
    )
    .await
    .ok_or("current Instance gameplay fence")?;
    let RuntimeScopeRefV1::Instance {
        world_id,
        instance_id,
    } = fence.runtime_scope
    else {
        return Err("native handoff must produce an Instance".into());
    };
    let mut death = vec![2];
    death.extend_from_slice(world_id.as_bytes());
    death.extend_from_slice(&instance_id);
    death.extend_from_slice(&fence.scope_ownership_generation.get().to_be_bytes());
    death.extend_from_slice(&901_u32.to_be_bytes());
    death.extend_from_slice(&1_u64.to_be_bytes());
    let now = owner.owner_now().get();
    let mut source = CapturedEndgameDeath {
        death_key: death,
        occurrence: fixture::id(91),
        creature: "oteryn:creature.the_primal_menace".into(),
        raw_xp: 0,
        death_at_ms: now / 1000,
        death_at_us: now,
        damaged: true,
    };
    settle_captured_endgame_death(owner, &h.pool, admitted.game_session_id, &source).await?;
    settle_captured_endgame_death(owner, &h.pool, admitted.game_session_id, &source).await?;
    let state: (i16, i16) = sqlx::query_as("SELECT unlocked,selected FROM game_character_hazard WHERE character_id=encode($1,'hex')::uuid")
        .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        state,
        (2, 1),
        "one native Instance death unlocks exactly once"
    );
    let retained: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_primal_hazard_receipts WHERE death_key=$1",
    )
    .bind(&source.death_key)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(retained, 1);
    source.death_key.remove(0);
    source.occurrence = fixture::id(92);
    assert!(
        settle_captured_endgame_death(owner, &h.pool, admitted.game_session_id, &source)
            .await
            .is_err(),
        "stripping the Instance tag must not reinterpret its UUID as a Channel"
    );
    let rejected_receipts: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_party_kill_xp_receipts WHERE death_key=$1")
            .bind(&source.death_key)
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(
        rejected_receipts, 0,
        "scope rejection must not leave an XP receipt"
    );
    let state_after: (i16, i16) = sqlx::query_as("SELECT unlocked,selected FROM game_character_hazard WHERE character_id=encode($1,'hex')::uuid")
        .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(state_after, state);
    eprintln!("ENDGAME_INSTANCE_RECEIPT POSTGRES_CASE_COMPLETED replay_scope_fences");
    Ok(())
}
