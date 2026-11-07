//! Venore job supplies retain exact source custody facts, without fabricated charges.
use super::fixture::Harness;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};

#[test]
fn questpc8_venore_supplies_mint_and_carry_without_invented_charges() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("questpc8-venore-supplies", || {
        super::quest_world_callbacks::scenario(
            "authored-venore-supplies",
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED questpc8_venore_supplies_mint_and_carry_without_invented_charges duration={:?}",
        started.elapsed()
    );
    Ok(())
}

pub(super) async fn exercise_supplies(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    eprintln!(
        "POSTGRES_CASE_STARTED questpc8_venore_supplies after PostgreSQL fixture initialization"
    );
    for (offset, item) in [17513, 17514, 17617].into_iter().enumerate() {
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "fence")?
            .ok_or("fence")?;
        super::quest_world_callbacks::input_item(
            owner,
            fence.item_fence(),
            &Offering {
                position: Position {
                    x: 33293,
                    y: 31677,
                    floor: -15,
                },
                item: format!("oteryn:item.tibia.i{item}"),
                count: 1,
            },
            700 + u64::try_from(offset)?,
            true,
        )
        .await?;
        let key = format!("oteryn:item.tibia.i{item}");
        let carried: i64 = sqlx::query_scalar("SELECT COALESCE(sum(i.quantity),0)::bigint FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) WHERE i.definition_production_key=$1 AND i.lifecycle=1 AND e.character_id=encode($2,'hex')::uuid")
            .bind(&key).bind(fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(
            carried, 1,
            "native supplies create one carried nonstacking Item"
        );
        let timers:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_timed_states t JOIN game_item_instances i USING(item_instance_id) WHERE i.definition_production_key=$1")
            .bind(&key).fetch_one(&h.pool).await?;
        assert_eq!(
            timers, 0,
            "plain authored constructor creates no timer or charge state"
        );
        let policy = owner
            .active_generation
            .and_then(|generation| generation.native_gameplay())
            .ok_or("native")?
            .item_policy(&key, "definition-r1")
            .ok_or("constructor policy")?;
        assert_eq!(
            policy.record().production_binding.source_key,
            "oteryn:source.crystalserver"
        );
        owner
            .refresh_quest_session(admitted.game_session_id, false)
            .await;
    }
    Ok(())
}
