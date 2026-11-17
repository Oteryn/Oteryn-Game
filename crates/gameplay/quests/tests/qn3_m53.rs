use oteryn_gameplay_quests::quest::{
    loader,
    npc_rules::{NpcInventorySnapshot, NpcItemKey, NpcRuleCatalog, NpcRuleInput, NpcRuleTrigger},
};
use std::collections::BTreeMap;

#[test]
fn qn3_m53_julius_requires_worn_necklace_not_carried_evidence() -> Result<(), String> {
    let loaded = loader::load_embedded_quest_state("qn3-m53").map_err(|e| format!("{e:?}"))?;
    let source = NpcRuleCatalog::runtime()
        .map_err(|e| e.to_string())?
        .admitted_for_npc("oteryn:npc.julius", loaded.catalogue())
        .map_err(|e| e.to_string())?
        .ok_or("Julius missing")?;
    let prefix = "oteryn:quest-progress/quest/u8_4/blood_brothers/";
    let mut tracks = (1..=10)
        .map(|i| (format!("{prefix}mission{i:02}"), 0))
        .collect::<BTreeMap<_, _>>();
    tracks.insert(format!("{prefix}mission01"), 1);
    let outfits = BTreeMap::new();
    for (slot, item, accepted) in [
        (None, "oteryn:item.tibia.i3083", false),
        (Some("armor"), "oteryn:item.tibia.i3083", false),
        (Some("necklace"), "oteryn:item.tibia.i140", false),
        (Some("necklace"), "oteryn:item.tibia.i3083", true),
    ] {
        let mut inventory = NpcInventorySnapshot::default();
        inventory.items.insert(
            NpcItemKey {
                item: "oteryn:item.tibia.i3083".into(),
                subtype: None,
            },
            1,
        );
        if let Some(slot) = slot {
            inventory.equipped.insert(slot.into(), item.into());
        }
        let input = NpcRuleInput {
            npc: "oteryn:npc.julius",
            trigger: NpcRuleTrigger::Say,
            message: "mission",
            player_name: None,
            topic: 0,
            tracks: &tracks,
            inventory: &inventory,
            now_epoch_seconds: 0,
            calendar_month: 1,
            premium_current: false,
            level: 0,
            vocation: "",
            sex: 0,
            outfits: &outfits,
        };
        let plan = source
            .plan_equipped_storage(&input)
            .map_err(|e| e.to_string())?
            .ok_or("no source branch")?;
        assert_eq!(!plan.track_changes.is_empty(), accepted);
        if accepted {
            assert_eq!(plan.track_changes[0].after, 2);
            assert_eq!(plan.next_topic, 2);
        } else {
            assert_eq!(plan.reply, ["I fear that will not do. Sorry."]);
        }
    }
    Ok(())
}
