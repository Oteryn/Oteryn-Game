//! Vanguard's actual reward-boss death earns the shared private loot receipt.
use super::fixture::Harness;
use super::galthen_vanguard_exit::Case;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::foundation::RuntimeScopeRefV1;
use crate::gameplay_transport::kill_reward::DrainOutcome;

const CREATURE: &str = "oteryn:creature.mitmah_vanguard";

#[test]
fn galthen_vanguard_native_death_binds_private_loot_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::m25_ahau_exit::vanguard_scenario(Case::Reward)
}

pub(super) async fn verify(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
) -> TestResult {
    let fence = admitted.item_fence.ok_or("current Vanguard participant")?;
    let RuntimeScopeRefV1::Instance { instance_id, .. } = fence.runtime_scope else {
        return Err("the actual arena admission is required".into());
    };
    let native = owner
        .active_generation
        .and_then(|active| active.native_gameplay())
        .ok_or("qualified boss content")?;
    let policy = native
        .policies()
        .policies()
        .iter()
        .find(|policy| policy.definition_key == CREATURE)
        .ok_or("native Vanguard combat policy")?;
    assert_eq!(policy.reward_boss, Some(true));
    super::boss_activity::commit_lethal_boss_hit(owner, admitted, policy).await?;
    for _ in 0..2 {
        assert_eq!(
            owner.drain_kill_rewards(admitted.game_session_id).await,
            DrainOutcome::Drained
        );
    }
    let row: (i16, i16, bool, i32, i64) = sqlx::query_as(
        "SELECT a.state,d.settlement_outcome,c.eligible,c.item_count,
            (SELECT count(*) FROM game_boss_reward_items i WHERE (i.world_id,i.death_id,i.character_id)
                =(c.world_id,c.death_id,c.character_id))
         FROM game_boss_room_activities a
         JOIN game_boss_reward_deaths d ON (d.world_id,d.death_id)=(a.world_id,a.defeat_id)
         JOIN game_boss_reward_contributors c ON (c.world_id,c.death_id)=(d.world_id,d.death_id)
         WHERE d.boss_key=$1 AND d.scope_kind=2 AND d.instance_id=encode($2,'hex')::uuid
           AND c.character_id=encode($3,'hex')::uuid AND c.source_game_session_id=encode($4,'hex')::uuid",
    ).bind(CREATURE).bind(instance_id.as_slice()).bind(fence.character_id.as_bytes().as_slice())
        .bind(admitted.game_session_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!((row.0, row.1, row.2), (2, 1, true));
    assert!(
        row.3 > 0,
        "the sole Vanguard contributor earns private loot"
    );
    assert_eq!(
        i64::from(row.3),
        row.4,
        "one entitlement set belongs to the matching defeat"
    );
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_boss_reward_deaths")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(
        count, 1,
        "repeated drains must retain the same private death settlement"
    );
    Ok(())
}
