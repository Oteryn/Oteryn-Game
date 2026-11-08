use oteryn_gameplay_quests::quest::{
    loader,
    npc_rules::{NpcInventorySnapshot, NpcRuleCatalog, NpcRuleInput, NpcRuleTrigger},
};
use std::collections::BTreeMap;

#[test]
fn qn3_m05_gnomus_measurements_report_keeps_item_points_and_deadline() -> Result<(), String> {
    let source = NpcRuleCatalog::runtime().map_err(|e| e.to_string())?;
    let loaded = loader::load_embedded_quest_state("qn3-m05").map_err(|e| format!("{e:?}"))?;
    let source = source
        .admitted_for_npc("oteryn:npc.gnomus", loaded.catalogue())
        .map_err(|e| e.to_string())?
        .ok_or("Gnomus not admitted")?;
    let prefix = "oteryn:quest-progress/quest/u11_50/dangerous_depths/gnomes/";
    let tracks = BTreeMap::from([
        (format!("{prefix}measurements"), 1),
        (format!("{prefix}location_count"), 5),
        (format!("{prefix}points"), 7),
    ]);
    let inventory = NpcInventorySnapshot::default();
    let outfits = BTreeMap::new();
    let input = NpcRuleInput {
        npc: "oteryn:npc.gnomus",
        trigger: NpcRuleTrigger::Say,
        message: "measurements",
        player_name: None,
        topic: 0,
        tracks: &tracks,
        inventory: &inventory,
        now_epoch_seconds: 1_800_000_000,
        calendar_month: 1,
        premium_current: false,
        level: 0,
        vocation: "",
        sex: 0,
        outfits: &outfits,
    };
    let plan = source
        .plan_item_exchange(&input)
        .map_err(|e| e.to_string())?
        .ok_or("no report")?;
    assert_eq!(
        plan.track_changes
            .iter()
            .find(|c| c.key == format!("{prefix}points"))
            .ok_or("no points")?
            .after,
        8
    );
    assert_eq!(
        plan.track_changes
            .iter()
            .find(|c| c.key == format!("{prefix}time_task_measurements"))
            .ok_or("no deadline")?
            .after,
        1_800_072_000
    );
    assert!(plan.effects.iter().any(|effect| matches!(effect, oteryn_gameplay_quests::quest::npc_rules::NpcRuleEffect::GiveItem { item, count: 1, .. } if item == "oteryn:item.tibia.i27654")));
    Ok(())
}
