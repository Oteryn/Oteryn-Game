//! Tereban pays the wiki-packet completion reward through NPC dispatch.
use super::TestResult;
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn qfather_tereban_grants_eight_thousand_gold_once() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("qfather-completion", || {
        super::quest_world_callbacks::scenario(
            "qfather-completion",
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED qfather_tereban_grants_eight_thousand_gold_once duration={:?}",
        started.elapsed()
    );
    Ok(())
}

pub(super) async fn gold(pool: &sqlx::PgPool) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT coalesce(sum(i.quantity * 100), 0)::bigint
         FROM game_item_container_entries e JOIN game_item_instances i USING(item_instance_id)
         WHERE e.character_id=encode($1,'hex')::uuid
           AND i.lifecycle=1 AND i.definition_production_key='oteryn:item.tibia.i3035'",
    )
    .bind(super::fixture::id(41).as_slice())
    .fetch_one(pool)
    .await?)
}
