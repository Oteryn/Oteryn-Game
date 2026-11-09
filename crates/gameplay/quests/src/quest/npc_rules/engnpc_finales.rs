//! Real source finale must reach canonical completion only after the report prompt.
use super::*;

#[test]
fn m04_efreet_final_report_is_bound_to_its_source_topic_and_native_catalogue() -> Result<(), String>
{
    let lowered = crate::quest::loader::load_embedded_quest_state("content-1")
        .map_err(|e| format!("{e:?}"))?;
    let source = NpcRuleCatalog::runtime()
        .map_err(|e| e.to_string())?
        .admitted_for_npc("oteryn:npc.malor", lowered.catalogue())
        .map_err(|e| e.to_string())?
        .ok_or("Efreet final NPC programme is not admitted")?;
    let base = "oteryn:quest-progress/quest/u7_4/djinn_war/efreet_faction/";
    let mut tracks = BTreeMap::from([
        (format!("{base}mission02"), 3),
        (format!("{base}mission03"), 2),
        (format!("{base}door_to_marid_territory"), 0),
    ]);
    let inventory = NpcInventorySnapshot::default();
    let outfits = BTreeMap::new();
    let evaluate = |tracks: &BTreeMap<String, i64>, message: &str, topic| {
        source
            .plan_storage(&NpcRuleInput {
                npc: "oteryn:npc.malor",
                trigger: NpcRuleTrigger::Say,
                message,
                player_name: None,
                topic,
                tracks,
                inventory: &inventory,
                now_epoch_seconds: 1_799_999_999,
                calendar_month: 11,
                premium_current: false,
                level: 1,
                vocation: "oteryn:vocation.none",
                sex: 0,
                outfits: &outfits,
            })
            .map_err(|e| e.to_string())
    };
    let offer = evaluate(&tracks, "mission", 0)?.ok_or("report prompt")?;
    assert!(!offer.completes);
    assert!(offer.track_changes.is_empty());
    assert_eq!(offer.next_topic, 2);
    let final_plan = evaluate(&tracks, "yes", offer.next_topic)?.ok_or("final report")?;
    assert!(final_plan.completes);
    assert_eq!(
        final_plan.quest,
        "oteryn:quest.the_djinn_war_efreet_faction"
    );
    assert_eq!(final_plan.track_changes.len(), 2);
    assert!(
        final_plan
            .track_changes
            .iter()
            .any(|c| c.key == format!("{base}mission03") && c.before == 2 && c.after == 3)
    );
    assert!(
        final_plan
            .track_changes
            .iter()
            .any(|c| c.key == format!("{base}door_to_marid_territory") && c.after == 1)
    );
    assert!(final_plan.effects.contains(&NpcRuleEffect::Achievement {
        name: "Efreet Ally".into()
    }));
    assert!(
        final_plan
            .effects
            .contains(&NpcRuleEffect::EndConversation { delay_ms: 1000 })
    );
    assert_eq!(final_plan.next_topic, 0);
    assert!(evaluate(&tracks, "yes", 1)?.is_none());
    assert!(
        !evaluate(&tracks, "yes mission", 2)?
            .ok_or("source precedence")?
            .completes
    );
    tracks.insert(format!("{base}mission03"), 1);
    assert!(evaluate(&tracks, "yes", 2)?.is_none());
    tracks.insert(format!("{base}mission03"), 2);
    tracks.insert(format!("{base}mission02"), 2);
    assert!(evaluate(&tracks, "yes", 2)?.is_none());
    Ok(())
}

#[test]
fn m04_white_raven_final_hand_in_keeps_topic_and_exact_items() -> Result<(), String> {
    let lowered = crate::quest::loader::load_embedded_quest_state("content-1")
        .map_err(|e| format!("{e:?}"))?;
    let source = NpcRuleCatalog::runtime()
        .map_err(|e| e.to_string())?
        .admitted_for_npc("oteryn:npc.costello", lowered.catalogue())
        .map_err(|e| e.to_string())?
        .ok_or("Costello not admitted")?;
    let diary = "oteryn:quest-progress/quest/u7_24/the_white_raven_monastery/diary";
    let tracks = BTreeMap::from([(diary.into(), 1)]);
    let mut inventory = NpcInventorySnapshot::default();
    inventory.items.insert(
        NpcItemKey {
            item: "oteryn:item.tibia.i3212".into(),
            subtype: None,
        },
        1,
    );
    let outfits = BTreeMap::new();
    let evaluate = |message: &str, topic| {
        source
            .plan_item_exchange(&NpcRuleInput {
                npc: "oteryn:npc.costello",
                trigger: NpcRuleTrigger::Say,
                message,
                player_name: None,
                topic,
                tracks: &tracks,
                inventory: &inventory,
                now_epoch_seconds: 1_799_999_999,
                calendar_month: 11,
                premium_current: false,
                level: 1,
                vocation: "oteryn:vocation.none",
                sex: 0,
                outfits: &outfits,
            })
            .map_err(|e| e.to_string())
    };
    let offer = evaluate("diary", 0)?.ok_or("diary prompt")?;
    assert!(!offer.completes);
    assert_eq!(offer.next_topic, 2);
    let finish = evaluate("yes", 2)?.ok_or("final hand-in")?;
    assert!(finish.completes);
    assert_eq!(
        finish.next_topic, 2,
        "source deliberately retains its topic"
    );
    assert_eq!(finish.track_changes.len(), 1);
    assert_eq!(finish.track_changes[0].after, 2);
    assert!(finish.effects.contains(&NpcRuleEffect::TakeItem {
        item: "oteryn:item.tibia.i3212".into(),
        count: 1,
        subtype: None,
        optional: false
    }));
    assert!(finish.effects.contains(&NpcRuleEffect::GiveItem {
        item: "oteryn:item.tibia.i3214".into(),
        count: 1,
        subtype: None
    }));
    assert!(evaluate("yes", 1)?.is_none());
    assert!(
        !evaluate("yes diary", 2)?
            .ok_or("diary precedence")?
            .completes
    );
    Ok(())
}

#[test]
fn m04_all_five_terminal_bindings_are_unique_and_native_admitted() -> Result<(), String> {
    let loaded = crate::quest::loader::load_embedded_quest_state("content-1")
        .map_err(|e| format!("{e:?}"))?;
    let source = NpcRuleCatalog::runtime().map_err(|e| e.to_string())?;
    for (quest, npc) in [
        ("a_father_s_burden_quest", "tereban"),
        ("the_djinn_war_efreet_faction", "malor"),
        ("the_djinn_war_marid_faction", "gabel"),
        ("marlin_trophy_quest", "pemaret"),
        ("the_white_raven_monastery_quest", "costello"),
    ] {
        let quest = format!("oteryn:quest.{quest}");
        let npc = format!("oteryn:npc.{npc}");
        let admitted = source
            .admitted_for_npc(&npc, loaded.catalogue())
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("missing native NPC: {npc}"))?;
        let program = admitted
            .program(&quest)
            .ok_or_else(|| format!("missing programme: {quest}"))?;
        let terminal = program
            .document
            .rules
            .iter()
            .filter(|rule| rule.completes)
            .collect::<Vec<_>>();
        assert_eq!(terminal.len(), 1, "{quest}");
        assert_eq!(terminal[0].npc, npc);
        assert!(
            !terminal[0].guards.is_empty(),
            "terminal branch must recheck progress: {quest}"
        );
        assert!(
            !terminal[0].keywords.is_empty(),
            "terminal branch must require a source utterance: {quest}"
        );
    }
    Ok(())
}

#[test]
fn m04_father_finale_requires_all_eight_deliveries_and_keeps_cape_and_xp() -> Result<(), String> {
    let loaded = crate::quest::loader::load_embedded_quest_state("content-1")
        .map_err(|e| format!("{e:?}"))?;
    let source = NpcRuleCatalog::runtime()
        .map_err(|e| e.to_string())?
        .admitted_for_npc("oteryn:npc.tereban", loaded.catalogue())
        .map_err(|e| e.to_string())?
        .ok_or("Tereban not admitted")?;
    let base = "oteryn:quest-progress/quest/u8_6/afathers_burden/";
    let inventory = NpcInventorySnapshot::default();
    let outfits = BTreeMap::new();
    for progress in 0..=8 {
        let tracks = BTreeMap::from([
            (format!("{base}status"), 1),
            (format!("{base}progress"), progress),
        ]);
        let plan = source
            .plan_father_items(&NpcRuleInput {
                npc: "oteryn:npc.tereban",
                trigger: NpcRuleTrigger::Say,
                message: "mission",
                player_name: None,
                topic: 0,
                tracks: &tracks,
                inventory: &inventory,
                now_epoch_seconds: 1_799_999_999,
                calendar_month: 11,
                premium_current: false,
                level: 1,
                vocation: "oteryn:vocation.none",
                sex: 0,
                outfits: &outfits,
            })
            .map_err(|e| e.to_string())?;
        if progress < 8 {
            assert!(
                !plan.is_some_and(|p| p.completes),
                "premature completion at {progress}"
            );
            continue;
        }
        let plan = plan.ok_or("missing Father finale")?;
        assert!(plan.completes);
        assert_eq!(plan.track_changes.len(), 1);
        assert_eq!(plan.track_changes[0].key, format!("{base}status"));
        assert_eq!(
            (plan.track_changes[0].before, plan.track_changes[0].after),
            (1, 2)
        );
        assert!(plan.effects.contains(&NpcRuleEffect::GiveItem {
            item: "oteryn:item.tibia.i11701".into(),
            count: 1,
            subtype: None,
        }));
        assert!(
            plan.effects
                .contains(&NpcRuleEffect::Experience { amount: 8000 })
        );
        assert!(
            !plan
                .effects
                .iter()
                .any(|e| matches!(e, NpcRuleEffect::TakeItem { .. }))
        );
    }
    Ok(())
}
