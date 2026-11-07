//! Every source entry reaches the actual activated native Item facts and mature room planner.
use super::{
    TestResult,
    fixture::{debug, id},
};
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_transfer::{BackpackEntry, InventoryItem};
use crate::durability::reward_claim_mint::{
    RewardClaimRefusal, RewardPlanInput, plan_reward_claim_mint,
};
use crate::gameplay_transport::item_ref_admission::generation_item_facts;
use oteryn_gameplay_items::pumpkin_skinning as rules;
#[test]
fn srv7_pumpkin_all_thirteen_rewards_admit_current_native_facts_quantity_and_room() -> TestResult {
    let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, _) =
        crate::content::native_gameplay::test_support::activated_with_loot_tables(world, None);
    let active = controller.active().ok_or("activated generation")?;
    let native = active.native_gameplay().ok_or("native generation")?;
    let reference = |identifier| TypedDefinitionRef {
        family: "Item".into(),
        production_key: format!("oteryn:item.tibia.i{identifier}"),
        revision_ref: "definition-r1".into(),
    };
    let bag = generation_item_facts(native, &reference(2854)).map_err(debug)?;
    assert_eq!(bag.container_capacity, Some(20));
    let slot = InventoryItem {
        item_instance_id: id(213),
        definition: bag.definition.clone(),
        quantity: 1,
    };
    let knife = BackpackEntry {
        item: InventoryItem {
            item_instance_id: id(214),
            definition: reference(5908),
            quantity: 1,
        },
        placement_ordinal: 1,
    };
    let full: Vec<_> = (0..20)
        .map(|i| BackpackEntry {
            item: InventoryItem {
                item_instance_id: id(171 + i),
                definition: reference(5908),
                quantity: 1,
            },
            placement_ordinal: u64::from(i) + 1,
        })
        .collect();
    for index in 0..13_u64 {
        for count in [0, 49] {
            let reward = rules::select(5908, 12816, |domain, upper| match (domain, upper) {
                (213, 13) => Some(index),
                (214, 50) => Some(count),
                _ => None,
            })
            .ok_or("source reward domain")?;
            let facts = generation_item_facts(native, &reference(reward.item_id))
                .map_err(|e| format!("source index{index}, Item{}: {e:?}", reward.item_id))?;
            assert!(
                facts.weight.is_some(),
                "source Item{} known native incoming mass",
                reward.item_id
            );
            let plan = |entries| {
                plan_reward_claim_mint(&RewardPlanInput {
                    item: &facts,
                    quantity: reward.count,
                    backpack: &bag,
                    already_claimed: false,
                    claim_pending: false,
                    slot: Some(&slot),
                    entries,
                })
            };
            assert!(
                plan(std::slice::from_ref(&knife)).is_ok(),
                "source{index} Item{} quantity{} actual native stack/room admission",
                reward.item_id,
                reward.count
            );
            assert_eq!(
                plan(&full),
                Err(RewardClaimRefusal::MainBackpackFull),
                "NoRoom remains definitive for every source output"
            );
        }
    }
    Ok(())
}
