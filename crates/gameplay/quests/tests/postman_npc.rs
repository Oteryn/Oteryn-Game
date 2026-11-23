use oteryn_gameplay_quests::quest::{
    loader,
    npc_rules::{
        NpcInventorySnapshot, NpcItemKey, NpcRuleCatalog, NpcRuleEffect, NpcRuleInput,
        NpcRuleTrigger,
    },
};
use std::collections::BTreeMap;

#[test]
fn postman_kevin_starts_missions_and_promotes_only_after_the_required_stage() -> Result<(), String>
{
    let native = loader::load_embedded_quest_state("postman-npc").map_err(|e| format!("{e:?}"))?;
    let npc = "oteryn:npc.kevin";
    let source = NpcRuleCatalog::runtime()
        .and_then(|source| source.admitted_for_npc(npc, native.catalogue()))
        .map_err(|e| e.to_string())?
        .ok_or("Kevin has no admitted programme")?;
    let mission01 = "oteryn:quest-progress/quest/u7_24/the_postman_missions/mission01";
    let mission06 = "oteryn:quest-progress/quest/u7_24/the_postman_missions/mission06";
    let rank = "oteryn:quest-progress/quest/u7_24/the_postman_missions/rank";
    let inventory = NpcInventorySnapshot::default();
    let outfits = BTreeMap::new();
    let plan = |text, topic, tracks: &BTreeMap<String, i64>| {
        source
            .plan_storage(&NpcRuleInput {
                npc,
                trigger: NpcRuleTrigger::Say,
                message: text,
                player_name: None,
                topic,
                tracks,
                inventory: &inventory,
                now_epoch_seconds: 0,
                calendar_month: 1,
                premium_current: false,
                level: 0,
                vocation: "",
                sex: 0,
                outfits: &outfits,
            })
            .map_err(|e| e.to_string())
    };
    let empty = BTreeMap::new();
    assert!(plan("yes", 0, &empty)?.is_none());
    let mut topic = plan("mission", 0, &empty)?
        .ok_or("joining offer")?
        .next_topic;
    for expected in 2..=7 {
        let next = plan("yes", topic, &empty)?.ok_or("joining step")?;
        assert_eq!(next.next_topic, expected);
        assert!(next.track_changes.is_empty());
        topic = expected;
    }
    let accepted = plan("yes", topic, &empty)?.ok_or("accepted assignment")?;
    assert_eq!(accepted.next_topic, 0);
    assert_eq!(accepted.track_changes.len(), 1);
    assert_eq!(accepted.track_changes[0].key, mission01);
    assert_eq!(accepted.track_changes[0].after, 1);

    let mut tracks = BTreeMap::new();
    tracks.insert(mission01.into(), 6);
    tracks.insert(rank.into(), 2);
    tracks.insert(mission06.into(), 12);
    assert!(plan("advancement", 0, &tracks)?.is_none());
    let completion = plan("mission", 0, &tracks)?.ok_or("uniform report")?;
    assert_eq!(completion.track_changes[0].key, mission06);
    assert_eq!(completion.track_changes[0].after, 13);
    tracks.insert(mission06.into(), 13);
    let advance = plan("advancement", 0, &tracks)?.ok_or("rank offer")?;
    let promoted = plan("yes", advance.next_topic, &tracks)?.ok_or("rank award")?;
    assert_eq!(promoted.track_changes[0].key, rank);
    assert_eq!(promoted.track_changes[0].after, 3);

    let mission10 = "oteryn:quest-progress/quest/u7_24/the_postman_missions/mission10";
    let door = "oteryn:quest-progress/quest/u7_24/the_postman_missions/door";
    tracks.insert(rank.into(), 4);
    tracks.insert(mission10.into(), 2);
    assert!(plan("advancement", 0, &tracks)?.is_none());
    let report = plan("mission", 0, &tracks)?.ok_or("final delivery report")?;
    assert_eq!(report.track_changes[0].key, mission10);
    assert_eq!(report.track_changes[0].after, 3);
    tracks.insert(mission10.into(), 3);
    let advance = plan("advancement", 0, &tracks)?.ok_or("Archpostman offer")?;
    let promoted = plan("yes", advance.next_topic, &tracks)?.ok_or("Archpostman award")?;
    assert!(
        promoted
            .track_changes
            .iter()
            .any(|change| change.key == rank && change.after == 5)
    );
    assert!(
        promoted
            .track_changes
            .iter()
            .any(|change| change.key == door && change.after == 1)
    );
    assert!(promoted.effects.contains(&NpcRuleEffect::Achievement {
        name: "Archpostman".into()
    }));
    Ok(())
}

#[test]
fn postman_peer_conversations_write_the_source_mission_stages() -> Result<(), String> {
    let native = loader::load_embedded_quest_state("postman-npc").map_err(|e| format!("{e:?}"))?;
    let source = NpcRuleCatalog::runtime().map_err(|e| e.to_string())?;
    let track = |name| format!("oteryn:quest-progress/quest/u7_24/the_postman_missions/{name}");
    let cases = [
        (
            "a_strange_fellow",
            "yes",
            7,
            "mission03",
            1,
            2,
            Some((3216, 1)),
        ),
        ("dermot", "yes", 1, "mission05", 2, 3, Some((3218, 1))),
        ("hugo", "dress pattern", 1, "mission06", 1, 2, None),
        ("hugo", "dress pattern", 0, "mission06", 11, 12, None),
        ("talphion", "dress pattern", 4, "mission06", 3, 4, None),
        ("queen_eloise", "uniforms", 0, "mission06", 5, 6, None),
        ("benjamin", "measurements", 0, "mission07", 1, 2, None),
        ("ninos", "measurements", 0, "mission07", 1, 2, None),
        ("chrystal", "measurements", 0, "mission07", 1, 2, None),
        ("dove", "yes", 1, "mission07", 1, 2, Some((3592, 1))),
        ("liane", "yes", 1, "mission07", 1, 2, Some((3447, 12))),
        ("lokur", "measurements", 0, "mission07", 6, 7, None),
        ("kroox", "lokurs measurements", 0, "mission07", 7, 8, None),
        ("markwin", "yes", 1, "mission10", 1, 2, Some((3220, 1))),
    ];
    for (stem, message, topic, name, before, after, payment) in cases {
        let npc = format!("oteryn:npc.{stem}");
        let catalogue = source
            .admitted_for_npc(&npc, native.catalogue())
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("{npc} has no admitted programme"))?;
        let key = track(name);
        let tracks = BTreeMap::from([(key.clone(), before)]);
        let mut inventory = NpcInventorySnapshot::default();
        if let Some((item, count)) = payment {
            inventory.items.insert(
                NpcItemKey {
                    item: format!("oteryn:item.tibia.i{item}"),
                    subtype: None,
                },
                count,
            );
        }
        let input = NpcRuleInput {
            npc: &npc,
            trigger: NpcRuleTrigger::Say,
            message,
            player_name: None,
            topic,
            tracks: &tracks,
            inventory: &inventory,
            now_epoch_seconds: 0,
            calendar_month: 1,
            premium_current: false,
            level: 0,
            vocation: "",
            sex: 0,
            outfits: &BTreeMap::new(),
        };
        let plan = if payment.is_some() {
            catalogue.plan_item_exchange(&input)
        } else {
            catalogue.plan_storage(&input)
        }
        .map_err(|e| format!("{npc}: {e}"))?
        .ok_or_else(|| format!("{npc} has no matching transition"))?;
        assert_eq!(plan.next_topic, 0, "{npc}");
        assert!(
            plan.track_changes.iter().any(|change| change.key == key
                && change.before == before
                && change.after == after),
            "{npc}: {:?}",
            plan.track_changes
        );
        let expected_items = inventory.items.into_keys().map(|item| (item, 0)).collect();
        assert_eq!(plan.inventory_after.items, expected_items, "{npc}");
    }
    Ok(())
}

#[test]
fn postman_world_tasks_have_admitted_stage_writers() -> Result<(), String> {
    use oteryn_gameplay_quests::quest::{QuestRefusal, world_callbacks};
    let native =
        loader::load_embedded_quest_state("postman-world").map_err(|e| format!("{e:?}"))?;
    let catalogue = native.catalogue();
    let callbacks = world_callbacks::load_for(catalogue)?;
    // The production loader produced this hash before the new writers were admitted.
    let previous = [
        0x9f, 0x64, 0x52, 0x8b, 0x3c, 0x6c, 0x06, 0xaf, 0xa5, 0xc5, 0x6d, 0x2c, 0x83, 0xf7, 0x6c,
        0x34, 0xe1, 0x98, 0x45, 0x1f, 0x4f, 0x03, 0x78, 0x69, 0x8c, 0x78, 0x87, 0x02, 0xdf, 0x5f,
        0x83, 0x68,
    ];
    assert!(
        oteryn_gameplay_quests::quest::npc_rules::legacy_defaults(
            catalogue,
            "oteryn:quest.the_postman_missions_quest",
            &previous,
        )
        .is_some()
    );
    let cases = [
        ("folda-mailbox", "mission02", 1, 2, None),
        (
            "dermot-present",
            "mission05",
            1,
            2,
            Some("oteryn:item.tibia.i3218"),
        ),
        (
            "waldo-posthorn",
            "mission08",
            1,
            2,
            Some("oteryn:item.tibia.i3219"),
        ),
        (
            "santa-letterbag",
            "mission09",
            1,
            2,
            Some("oteryn:item.tibia.i3217"),
        ),
        ("letterbag-delivery", "mission09", 2, 3, None),
    ];
    let missing: Vec<_> = cases
        .iter()
        .filter_map(|(name, ..)| {
            let key = format!("oteryn:quest-callback/postman-{name}");
            (!callbacks.iter().any(|callback| callback.key == key)).then_some(name)
        })
        .collect();
    assert!(missing.is_empty(), "missing world transitions: {missing:?}");
    for (name, mission, before, after, reward) in cases {
        let key = format!("oteryn:quest-callback/postman-{name}");
        let callback = callbacks
            .iter()
            .find(|callback| callback.key == key)
            .ok_or(&key)?;
        assert!(callback.requires_world_admission, "{key}");
        assert!(callback.selector.native.is_some(), "{key}");
        assert_eq!(
            callback
                .item_reward()
                .as_ref()
                .map(|reward| reward.item.as_str()),
            reward,
            "{key}"
        );
        let transition = catalogue.transition(&key).ok_or(&key)?;
        let track = format!("oteryn:quest-progress/quest/u7_24/the_postman_missions/{mission}");
        let result =
            |stage| catalogue.evaluate(transition, &BTreeMap::from([(track.clone(), stage)]), 0);
        assert_eq!(
            result(before - 1),
            Err(QuestRefusal::StageMismatch),
            "{key}"
        );
        let changes = result(before).map_err(|e| format!("{key}: {e:?}"))?;
        assert_eq!(changes[0].after, after, "{key}");
        assert_eq!(result(after), Err(QuestRefusal::StageMismatch), "{key}");
    }
    Ok(())
}

#[test]
fn postman_noodles_trials_require_the_source_items() -> Result<(), String> {
    let native =
        loader::load_embedded_quest_state("postman-noodles").map_err(|e| format!("{e:?}"))?;
    let npc = "oteryn:npc.noodles";
    let catalogue = NpcRuleCatalog::runtime()
        .and_then(|source| source.admitted_for_npc(npc, native.catalogue()))
        .map_err(|e| e.to_string())?
        .ok_or("Noodles has no admitted programme")?;
    let track = "oteryn:quest-progress/quest/u7_24/the_postman_missions/mission06";
    for (stage, item, message) in [
        (7, 3104, "banana skin"),
        (8, 3105, "dirty fur"),
        (9, 3120, "mouldy cheese"),
    ] {
        let tracks = BTreeMap::from([(track.into(), stage)]);
        let mut inventory = NpcInventorySnapshot::default();
        let plan = |inventory: &NpcInventorySnapshot, topic, message| {
            let input = NpcRuleInput {
                npc,
                trigger: NpcRuleTrigger::Say,
                message,
                player_name: None,
                topic,
                tracks: &tracks,
                inventory,
                now_epoch_seconds: 0,
                calendar_month: 1,
                premium_current: false,
                level: 0,
                vocation: "",
                sex: 0,
                outfits: &BTreeMap::new(),
            };
            catalogue
                .plan_item_exchange(&input)
                .map_err(|e| e.to_string())
        };
        assert!(plan(&NpcInventorySnapshot::default(), 0, message)?.is_none());
        inventory.items.insert(
            NpcItemKey {
                item: format!("oteryn:item.tibia.i{item}"),
                subtype: None,
            },
            1,
        );
        let sniff = plan(&inventory, 0, message)?.ok_or("missing sniff reply")?;
        assert!(sniff.track_changes.is_empty());
        let liked = plan(&inventory, sniff.next_topic, "like")?.ok_or("missing trial outcome")?;
        assert_eq!(liked.track_changes[0].after, stage + 1);
        assert_eq!(liked.inventory_after, inventory);
    }
    Ok(())
}

#[test]
fn postman_kevin_bones_hand_in_combines_both_source_items() -> Result<(), String> {
    let native =
        loader::load_embedded_quest_state("postman-bones").map_err(|e| format!("{e:?}"))?;
    let npc = "oteryn:npc.kevin";
    let catalogue = NpcRuleCatalog::runtime()
        .and_then(|source| source.admitted_for_npc(npc, native.catalogue()))
        .map_err(|e| e.to_string())?
        .ok_or("Kevin has no admitted programme")?;
    let track = "oteryn:quest-progress/quest/u7_24/the_postman_missions/mission04";
    let tracks = BTreeMap::from([
        (track.into(), 1),
        (
            "oteryn:quest-progress/quest/u7_24/the_postman_missions/mission01".into(),
            6,
        ),
        (
            "oteryn:quest-progress/quest/u7_24/the_postman_missions/mission03".into(),
            3,
        ),
        (
            "oteryn:quest-progress/quest/u7_24/the_postman_missions/rank".into(),
            1,
        ),
    ]);
    let plan = |message, topic, first, second| {
        let item = |id| NpcItemKey {
            item: format!("oteryn:item.tibia.i{id}"),
            subtype: None,
        };
        let inventory = NpcInventorySnapshot {
            items: BTreeMap::from([(item(3115), first), (item(3116), second)]),
            ..Default::default()
        };
        catalogue
            .plan_item_exchange(&NpcRuleInput {
                npc,
                trigger: NpcRuleTrigger::Say,
                message,
                player_name: None,
                topic,
                tracks: &tracks,
                inventory: &inventory,
                now_epoch_seconds: 0,
                calendar_month: 1,
                premium_current: false,
                level: 0,
                vocation: "",
                sex: 0,
                outfits: &BTreeMap::new(),
            })
            .map_err(|e| e.to_string())
    };
    let prompt = plan("mission", 0, 9, 10)?.ok_or("missing bone hand-in prompt")?;
    let refused = plan("yes", prompt.next_topic, 9, 10)?.ok_or("missing bone refusal")?;
    assert!(refused.track_changes.is_empty());
    assert_eq!(refused.next_topic, 0);
    for (first, second) in [(0, 20), (20, 0), (12, 9)] {
        let handed_in =
            plan("yes", prompt.next_topic, first, second)?.ok_or("missing bone hand-in")?;
        assert_eq!(handed_in.track_changes[0].after, 2);
        assert!(
            handed_in
                .inventory_after
                .items
                .values()
                .all(|count| *count == 0)
        );
    }
    Ok(())
}

#[test]
fn postman_partial_lokur_refuses_before_the_paid_olrik_measurement() -> Result<(), String> {
    let native =
        loader::load_embedded_quest_state("postman-lokur").map_err(|e| format!("{e:?}"))?;
    let npc = "oteryn:npc.lokur";
    let source = NpcRuleCatalog::runtime().map_err(|e| e.to_string())?;
    let catalogue = source
        .admitted_for_npc(npc, native.catalogue())
        .map_err(|e| e.to_string())?
        .ok_or("Lokur has no admitted programme")?;
    let track = "oteryn:quest-progress/quest/u7_24/the_postman_missions/mission07";
    let plan = |stage| {
        catalogue
            .plan_storage(&NpcRuleInput {
                npc,
                trigger: NpcRuleTrigger::Say,
                message: "measurements",
                player_name: None,
                topic: 0,
                tracks: &BTreeMap::from([(track.into(), stage)]),
                inventory: &NpcInventorySnapshot::default(),
                now_epoch_seconds: 0,
                calendar_month: 1,
                premium_current: false,
                level: 0,
                vocation: "",
                sex: 0,
                outfits: &BTreeMap::new(),
            })
            .map_err(|e| e.to_string())
    };
    assert!(plan(5)?.is_none());
    assert_eq!(
        plan(6)?.ok_or("Lokur at stage 6")?.track_changes[0].after,
        7
    );
    Ok(())
}

#[test]
fn postman_olrik_is_admitted_to_supply_the_fifth_measurement() -> Result<(), String> {
    let native =
        loader::load_embedded_quest_state("postman-olrik").map_err(|e| format!("{e:?}"))?;
    let npc = "oteryn:npc.olrik";
    let source = NpcRuleCatalog::runtime()
        .and_then(|source| source.admitted_for_npc(npc, native.catalogue()))
        .map_err(|e| e.to_string())?
        .ok_or("Olrik has no admitted programme")?;
    let stage = "oteryn:quest-progress/quest/u7_24/the_postman_missions/mission07";
    let tracks = BTreeMap::from([(stage.into(), 5)]);
    let outfits = BTreeMap::new();
    let mut input = NpcRuleInput {
        npc,
        trigger: NpcRuleTrigger::Say,
        message: "measurements",
        player_name: None,
        topic: 0,
        tracks: &tracks,
        inventory: &NpcInventorySnapshot::default(),
        now_epoch_seconds: 0,
        calendar_month: 1,
        premium_current: false,
        level: 1,
        vocation: "",
        sex: 0,
        outfits: &outfits,
    };
    let offer = source
        .plan_storage(&input)
        .map_err(|e| e.to_string())?
        .ok_or("dice offer")?;
    input.topic = offer.next_topic;
    input.message = "yes";
    assert!(
        source
            .plan_item_exchange(&input)
            .map_err(|e| e.to_string())?
            .is_none()
    );
    for (label, money, bank, item) in [
        (
            "platinum coin",
            100,
            0,
            Some(("oteryn:item.tibia.i3035", 1)),
        ),
        ("bank only", 0, 5, None),
        ("mixed", 2, 3, Some(("oteryn:item.tibia.i3031", 2))),
        ("gold coins", 5, 0, Some(("oteryn:item.tibia.i3031", 5))),
    ] {
        let inventory = NpcInventorySnapshot {
            money,
            bank,
            items: item
                .into_iter()
                .map(|(item, count)| {
                    (
                        NpcItemKey {
                            item: item.into(),
                            subtype: None,
                        },
                        count,
                    )
                })
                .collect(),
            ..Default::default()
        };
        let roll = source
            .plan_item_exchange(&NpcRuleInput {
                inventory: &inventory,
                ..input
            })
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("wager refused with {label}"))?;
        assert_eq!(
            roll.inventory_after.money,
            money.saturating_sub(5),
            "{label}"
        );
        assert_eq!(
            roll.inventory_after.bank,
            bank - 5_u64.saturating_sub(money),
            "{label}"
        );
        assert!(
            roll.track_changes
                .iter()
                .any(|change| change.key == stage && change.before == 5 && change.after == 6)
        );
    }
    Ok(())
}
