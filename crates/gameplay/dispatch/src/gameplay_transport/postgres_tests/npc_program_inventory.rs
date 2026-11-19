//! Actual NPC routing must select replies and mutations from carried custody.
use super::npc_program_inventory_fixtures::{DEMERON, STAG};

use super::fixture::debug;
use super::TestResult;

use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn npc_inventory_selects_real_missing_partial_and_complete_source_branches() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("catalogue-npc-inventory", || {
        super::quest_world_callbacks_fixtures::scenario(
            "catalogue-npc-inventory",
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    })
}

#[test]
fn npc_inventory_raw_demeron_partial_and_complete_selection_is_not_live_custody() -> TestResult {
    use oteryn_gameplay_quests::quest::npc_rules::{
        NpcInventorySnapshot, NpcItemKey, NpcRuleCatalog, NpcRuleInput, NpcRuleTrigger,
    };
    use std::collections::BTreeMap;
    let source = NpcRuleCatalog::from_json_documents(&[include_str!(
        "../../../../../../content/dialogues/quest-rules/quest-batch3-c-the_order_of_the_stag.json"
    )])
    .map_err(debug)?;
    let tracks = BTreeMap::from([(STAG.into(), 4), (DEMERON.into(), 1)]);
    let outfits = BTreeMap::new();
    for count in [0, 29, 30] {
        // Typed pure planner inputs only: these counts are never live item evidence.
        let inventory = NpcInventorySnapshot {
            items: BTreeMap::from([
                (
                    NpcItemKey {
                        item: "oteryn:item.tibia.i52635".into(),
                        subtype: None,
                    },
                    count,
                ),
                (
                    NpcItemKey {
                        item: "oteryn:item.tibia.i52758".into(),
                        subtype: None,
                    },
                    20,
                ),
                (
                    NpcItemKey {
                        item: "oteryn:item.tibia.i52743".into(),
                        subtype: None,
                    },
                    10,
                ),
            ]),
            ..Default::default()
        };
        let plan = source
            .plan_item_exchange(&NpcRuleInput {
                npc: "oteryn:npc.second_in_command_demeron",
                trigger: NpcRuleTrigger::Say,
                message: "spares",
                topic: 0,
                tracks: &tracks,
                inventory: &inventory,
                player_name: None,
                now_epoch_seconds: 0,
                calendar_month: 1,
                premium_current: false,
                level: 1,
                vocation: "oteryn:vocation.none",
                sex: 0,
                outfits: &outfits,
            })
            .map_err(debug)?
            .ok_or("Demeron source branch")?;
        if count < 30 {
            assert!(
                !plan
                    .reply
                    .iter()
                    .any(|line| line.contains("Are those salvaged parts"))
            );
            assert_ne!(plan.next_topic, 20);
        } else {
            assert!(
                plan.reply
                    .iter()
                    .any(|line| line.contains("Are those salvaged parts"))
            );
            assert_eq!(plan.next_topic, 20);
        }
        assert!(
            plan.track_changes
                .iter()
                .all(|change| change.before == change.after)
        );
    }
    Ok(())
}
