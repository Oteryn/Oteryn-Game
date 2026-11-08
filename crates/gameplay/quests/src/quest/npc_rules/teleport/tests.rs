use super::*;
use crate::quest::npc_rules::{NpcInventorySnapshot, NpcRuleTrigger};
use std::collections::BTreeMap;

fn document() -> serde_json::Value {
    serde_json::json!({"schema":1,"quest":"oteryn:quest.the_paradox_tower_quest","tracks":[],"rules":[{
        "key":"oteryn:quest-npc/the_paradox_tower_quest/riddler/wrong-mathemagics",
        "quest":"oteryn:quest.the_paradox_tower_quest","npc":"oteryn:npc.riddler","trigger":"say",
        "keywords":["1"],"topics":[13],"guards":[],"reply":["WRONG!"],
        "effects":[{"kind":"teleport","position":{"x":32725,"y":31589,"floor":-12}},
                   {"kind":"magic_effect","effect":"CONST_ME_TELEPORT"}],
        "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/riddler.lua:143-146"]
    }]})
}

fn plan(
    document: serde_json::Value,
    legacy: bool,
) -> Result<Option<NpcTeleportPlan>, NpcRuleError> {
    let encoded = document.to_string();
    let catalogue = NpcRuleCatalog::from_json_documents(&[&encoded])?;
    let tracks = BTreeMap::new();
    let inventory = NpcInventorySnapshot::default();
    let outfits = BTreeMap::new();
    let input = NpcRuleInput {
        npc: "oteryn:npc.riddler",
        trigger: NpcRuleTrigger::Say,
        message: "1",
        topic: 13,
        tracks: &tracks,
        inventory: &inventory,
        now_epoch_seconds: 1_700_000_000,
        calendar_month: 11,
        premium_current: false,
        level: 100,
        vocation: "oteryn:vocation.knight",
        sex: 0,
        outfits: &outfits,
    };
    if legacy {
        catalogue.plan_storage(&input)?;
        Ok(None)
    } else {
        catalogue.plan_storage_teleport(&input)
    }
}

#[test]
fn m37_source_teleport_plans_one_destination_and_keeps_legacy_storage_closed()
-> Result<(), NpcRuleError> {
    assert!(plan(document(), true).is_err());
    let result =
        plan(document(), false)?.ok_or_else(|| NpcRuleError::Invalid("missing teleport".into()))?;
    assert_eq!(
        result.destination,
        NpcRulePosition {
            x: 32725,
            y: 31589,
            floor: -12
        }
    );
    assert!(
        result.source.track_changes.is_empty(),
        "pure teleport invents no track writes"
    );
    assert_eq!(
        result.source.next_topic, 13,
        "teleport itself does not invent a topic write"
    );
    assert_eq!(result.magic, ["CONST_ME_TELEPORT"]);
    Ok(())
}

#[test]
fn m37_source_teleport_refuses_multiple_destinations_and_unowned_guards() -> Result<(), String> {
    let mut multiple = document();
    let destination = multiple["rules"][0]["effects"][0].clone();
    multiple["rules"][0]["effects"]
        .as_array_mut()
        .ok_or("effects")?
        .push(destination);
    assert!(plan(multiple, false).is_err());
    let mut money = document();
    money["rules"][0]["guards"] =
        serde_json::json!([{"kind":"money","amount":1,"include_bank":false}]);
    assert!(
        plan(money, false).is_err(),
        "a preview balance is not a value-owner proof"
    );
    Ok(())
}
