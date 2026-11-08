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
        player_name: None,
        npc: "oteryn:npc.riddler",
        trigger: NpcRuleTrigger::Say,
        message: "1",
        topic: 13,
        tracks: &tracks,
        inventory: &inventory,
        now_epoch_seconds: 1_700_000_000,
        calendar_month: 11,
        level: 100,
        premium_current: false,
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

#[test]
fn m37_riddler_runtime_final_answer_preserves_two_track_progress() -> Result<(), NpcRuleError> {
    let native = crate::quest::loader::load_embedded_quest_state("content-1")
        .map_err(|e| NpcRuleError::Invalid(format!("native Riddler admission: {e:?}")))?;
    let catalogue = NpcRuleCatalog::runtime()?
        .admitted_for_npc("oteryn:npc.riddler", native.catalogue())?
        .ok_or_else(|| NpcRuleError::Invalid("Riddler native binding missing".into()))?;
    let prefix = "oteryn:quest-progress/quest/u7_24/the_paradox_tower";
    let tracks = BTreeMap::from([
        (format!("{prefix}/mathemagics"), 1),
        (format!("{prefix}/favorite_colour"), 1),
    ]);
    let inventory = NpcInventorySnapshot::default();
    let outfits = BTreeMap::new();
    let input = NpcRuleInput {
        player_name: None,
        npc: "oteryn:npc.riddler",
        trigger: NpcRuleTrigger::Say,
        message: "1",
        topic: 13,
        tracks: &tracks,
        inventory: &inventory,
        now_epoch_seconds: 1_700_000_000,
        calendar_month: 11,
        level: 100,
        premium_current: false,
        vocation: "oteryn:vocation.knight",
        sex: 0,
        outfits: &outfits,
    };
    let planned = catalogue
        .plan_storage_teleport(&input)?
        .ok_or_else(|| NpcRuleError::Invalid("Riddler programme missing from runtime".into()))?;
    assert_eq!(
        planned.destination,
        NpcRulePosition {
            x: 32478,
            y: 31905,
            floor: -1
        }
    );
    assert_eq!(planned.source.track_changes.len(), 2);
    for change in &planned.source.track_changes {
        assert_eq!(change.after, 2);
        assert!(change.key.ends_with("/mathemagics") || change.key.ends_with("/favorite_colour"));
    }
    Ok(())
}

#[test]
fn m37_riddler_runtime_walks_seals_and_retains_keyword_precedence() -> Result<(), NpcRuleError> {
    let catalogue = NpcRuleCatalog::runtime()?;
    let prefix = "oteryn:quest-progress/quest/u7_24/the_paradox_tower";
    let mut tracks = BTreeMap::from([
        (format!("{prefix}/the_feared_hugo"), 4),
        (format!("{prefix}/mathemagics"), 1),
    ]);
    let inventory = NpcInventorySnapshot::default();
    let outfits = BTreeMap::new();
    let mut topic = 0;
    for (message, expected) in [
        ("test", 1),
        ("yes", 2),
        ("goshnar", 3),
        ("demonbunny", 4),
        ("tha'kull", 6),
        ("yes", 7),
        ("breath", 8),
        ("silence", 9),
        ("old", 10),
        ("yes", 11),
        ("green", 12),
        ("none", 13),
    ] {
        let input = NpcRuleInput {
            player_name: None,
            npc: "oteryn:npc.riddler",
            trigger: NpcRuleTrigger::Say,
            message,
            topic,
            tracks: &tracks,
            inventory: &inventory,
            now_epoch_seconds: 1_700_000_000,
            calendar_month: 11,
            level: 100,
            premium_current: false,
            vocation: "oteryn:vocation.knight",
            sex: 0,
            outfits: &outfits,
        };
        let plan = catalogue
            .plan_storage(&input)?
            .ok_or_else(|| NpcRuleError::Invalid(format!("missing Riddler answer: {message}")))?;
        assert_eq!(plan.next_topic, expected, "answer {message}");
        for change in plan.track_changes {
            tracks.insert(change.key, change.after);
        }
        topic = plan.next_topic;
    }
    assert_eq!(tracks.get(&format!("{prefix}/favorite_colour")), Some(&1));
    let mut input = NpcRuleInput {
        player_name: None,
        npc: "oteryn:npc.riddler",
        trigger: NpcRuleTrigger::Say,
        message: "treasure",
        topic: 5,
        tracks: &tracks,
        inventory: &inventory,
        now_epoch_seconds: 1_700_000_000,
        calendar_month: 11,
        level: 100,
        premium_current: false,
        vocation: "oteryn:vocation.knight",
        sex: 0,
        outfits: &outfits,
    };
    assert!(
        catalogue.plan_storage_teleport(&input)?.is_none(),
        "keyword handler runs before wrong-answer trap"
    );
    input.message = "wrong";
    let trap = catalogue
        .plan_storage_teleport(&input)?
        .ok_or_else(|| NpcRuleError::Invalid("wrong-answer destination missing".into()))?;
    assert_eq!(
        trap.destination,
        NpcRulePosition {
            x: 32725,
            y: 31589,
            floor: -12
        }
    );
    assert!(trap.source.track_changes.is_empty());
    Ok(())
}
