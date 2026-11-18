use super::TestResult;
use super::fixture::Harness;
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn reward_chest_grants_the_complete_bundle_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("qcx36-ferumbras-reward", || {
        super::quest_world_callbacks::scenario(
            "questpc7-ferumbras",
            Position {
                x: 33275,
                y: 32390,
                floor: -8,
            },
            false,
        )
    });
    eprintln!(
        "QCX36 reward_chest duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

pub(super) async fn assert_chest_bundle(h: &Harness) -> TestResult {
    for (item, count) in [(22516, 5), (22731, 4), (22737, 4), (22775, 1), (3043, 10)] {
        let quantity: i64 = sqlx::query_scalar(
            "SELECT coalesce(sum(child.quantity),0)::bigint FROM game_item_instances child JOIN game_item_nested_entries e ON e.item_instance_id=child.item_instance_id JOIN game_item_instances bag ON bag.item_instance_id=e.parent_item_instance_id WHERE child.lifecycle=1 AND bag.lifecycle=1 AND child.definition_production_key=$1 AND bag.definition_production_key='oteryn:item.tibia.i2853'"
        ).bind(format!("oteryn:item.tibia.i{item}")).fetch_one(&h.pool).await?;
        assert_eq!(quantity, count, "reward bag item {item}");
    }
    Ok(())
}
