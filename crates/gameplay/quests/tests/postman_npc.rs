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
