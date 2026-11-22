//! Native lethal settlement and real dispatch into backpack and nested custody.
use super::boss_rewards_fixtures::{Case, exchange, run_case};
use super::fixture::{Harness, debug};
use super::{ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::connection::AdmittedSession;
use oteryn_protocol_oteryn::boss_rewards::*;

#[test]
fn boss_personal_reward_collect_mints_child_inside_parent_once() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    run_case(Case::NestedCustody)
}

pub(super) async fn assert_nested_custody(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    death: [u8; 16],
    entitlement_count: i64,
) -> TestResult {
    assert_eq!(entitlement_count, 2, "the bag and gold both settle");
    let browse = encode_browse_request(&BrowseRequest {
        death,
        after_ordinal: 0,
    })
    .map_err(debug)?;
    let (_, bytes) = exchange(owner, admitted, 1, COMMAND_TYPE_BOSS_REWARD_BROWSE, &browse).await?;
    let page = decode_browse_reply(&bytes).map_err(debug)?;
    assert_eq!(page.status, BossRewardStatus::Ok);
    assert_eq!(
        page.rows.len(),
        1,
        "only the bag is independently claimable"
    );
    let bag = page.rows.first().ok_or("bag reward")?;
    let child = encode_take_request(&TakeRequest {
        death,
        ordinal: bag.ordinal + 1,
    })
    .map_err(debug)?;
    let (_, bytes) = exchange(owner, admitted, 2, COMMAND_TYPE_BOSS_REWARD_TAKE, &child).await?;
    assert_eq!(decode_collect_reply(&bytes).map_err(debug)?.moved, 0);

    let take = encode_take_request(&TakeRequest {
        death,
        ordinal: bag.ordinal,
    })
    .map_err(debug)?;
    let (_, bytes) = exchange(owner, admitted, 3, COMMAND_TYPE_BOSS_REWARD_TAKE, &take).await?;
    let result = decode_collect_reply(&bytes).map_err(debug)?;
    assert_eq!(result.status, BossRewardStatus::Ok);
    assert_eq!((result.moved, result.remaining), (2, 0));
    let (_, replay) = exchange(owner, admitted, 3, COMMAND_TYPE_BOSS_REWARD_TAKE, &take).await?;
    assert_eq!(replay, bytes);

    let (_, bytes) = exchange(owner, admitted, 4, COMMAND_TYPE_BOSS_REWARD_TAKE, &take).await?;
    assert_eq!(decode_collect_reply(&bytes).map_err(debug)?.moved, 0);
    let (collections, nested, loose): (i64, i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM game_boss_reward_item_collections),
          (SELECT count(*) FROM game_boss_reward_item_collections child
           JOIN game_item_nested_entries n ON n.item_instance_id=child.item_instance_id
             AND n.placed_transaction_id=child.transaction_id
           JOIN game_item_instances gold ON gold.item_instance_id=child.item_instance_id
           JOIN game_boss_reward_item_collections parent ON parent.item_instance_id=n.parent_item_instance_id
             AND parent.death_id=child.death_id AND parent.command_id=child.command_id
           JOIN game_item_instances bag ON bag.item_instance_id=parent.item_instance_id
           JOIN game_item_container_entries entry ON entry.item_instance_id=bag.item_instance_id
           JOIN game_item_container_slots backpack ON backpack.item_instance_id=entry.parent_item_instance_id
             AND backpack.character_id=parent.character_id
           WHERE bag.definition_production_key='oteryn:item.tibia.i2863' AND bag.lifecycle=1
             AND gold.definition_production_key='oteryn:item.tibia.i3031' AND gold.lifecycle=1
             AND gold.quantity=1 AND n.placement_ordinal=1),
          (SELECT count(*) FROM game_boss_reward_item_collections c
           JOIN game_item_container_entries entry USING(item_instance_id)
           JOIN game_item_instances i USING(item_instance_id)
           WHERE i.definition_production_key='oteryn:item.tibia.i3031')",
    )
    .fetch_one(&h.pool)
    .await?;
    assert_eq!((collections, nested, loose), (2, 1, 0));
    Ok(())
}
