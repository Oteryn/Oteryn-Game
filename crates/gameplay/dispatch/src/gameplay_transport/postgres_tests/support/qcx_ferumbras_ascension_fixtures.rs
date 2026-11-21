use super::TestResult;
use super::fixture::Harness;

pub(super) async fn assert_chest_bundle(h: &Harness) -> TestResult {
    for (item, count) in [(22516, 5), (22731, 4), (22737, 4), (22775, 1), (3043, 10)] {
        let quantity: i64 = sqlx::query_scalar(
            "SELECT coalesce(sum(child.quantity),0)::bigint FROM game_item_instances child JOIN game_item_nested_entries e ON e.item_instance_id=child.item_instance_id JOIN game_item_instances bag ON bag.item_instance_id=e.parent_item_instance_id WHERE child.lifecycle=1 AND bag.lifecycle=1 AND child.definition_production_key=$1 AND bag.definition_production_key='oteryn:item.tibia.i2853'"
        ).bind(format!("oteryn:item.tibia.i{item}")).fetch_one(&h.pool).await?;
        assert_eq!(quantity, count, "reward bag item {item}");
    }
    Ok(())
}
