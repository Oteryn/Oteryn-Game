use oteryn_gameplay_quests::quest::{
    QuestEffectKind, QuestRefusal, loader,
    npc_rules::{
        NpcInventorySnapshot, NpcItemKey, NpcRuleCatalog, NpcRuleEffect, NpcRuleInput,
        NpcRuleTrigger, legacy_defaults,
    },
    source_deaths,
    world_callbacks::{self, Position, Trigger},
};
use std::collections::BTreeMap;

const STAGE: &str =
    "oteryn:quest-progress/crystalserver/quest/u15_20/the_order_of_the_stag/questline";
const LION: &str =
    "oteryn:quest-progress/crystalserver/quest/u12_40/the_order_of_the_lion/kesar_mission";
const BEAKS: &str =
    "oteryn:quest-progress/crystalserver/quest/u15_20/the_order_of_the_stag/harpy_beaks";
const CORPSES: &str =
    "oteryn:quest-progress/crystalserver/quest/u15_20/the_order_of_the_stag/corpses";
const KESAR: &str = "oteryn:npc.kesar_the_younger_day";
const DEMERON: &str = "oteryn:npc.second_in_command_demeron";
const HARTFORD: &str = "oteryn:npc.guard_captain_hartford";
const ELDORAN: &str = "oteryn:npc.eldoran_the_ambitious";
const BANNER: &str = "oteryn:npc.captain_marie_denise_banner";

#[test]
fn order_of_the_stag_certificate_unlocks_ada_and_the_dialogue_finale()
-> Result<(), Box<dyn std::error::Error>> {
    let native = loader::load_embedded_quest_state("stag-dialogue-test")
        .map_err(|error| format!("{error:?}"))?;
    let native = native.catalogue();
    let catalogue = NpcRuleCatalog::runtime()?;
    let mut tracks = BTreeMap::from([(LION.to_owned(), 5)]);
    let mut inventory = NpcInventorySnapshot::default();
    let mut topic = 0;
    let outfits = BTreeMap::new();
    let talk = |npc,
                message,
                topic: &mut i32,
                tracks: &mut BTreeMap<String, i64>,
                inventory: &NpcInventorySnapshot| {
        let catalogue = catalogue
            .admitted_for_npc(npc, native)?
            .ok_or_else(|| format!("unbound Stag NPC: {npc}"))?;
        let input = NpcRuleInput {
            npc,
            trigger: NpcRuleTrigger::Say,
            message,
            player_name: None,
            topic: *topic,
            tracks,
            inventory,
            now_epoch_seconds: 0,
            calendar_month: 1,
            premium_current: true,
            level: 1,
            vocation: "oteryn:vocation.none",
            sex: 0,
            outfits: &outfits,
        };
        let plan = if catalogue.item_exchange_keys(npc).is_some() {
            catalogue.plan_item_exchange(&input)?
        } else {
            catalogue.plan_storage(&input)?
        }
        .ok_or_else(|| format!("{npc}: {message}"))?;
        for change in &plan.track_changes {
            tracks.insert(change.key.clone(), change.after);
        }
        *topic = plan.next_topic;
        Ok::<_, Box<dyn std::error::Error>>(plan)
    };

    talk(KESAR, "help", &mut topic, &mut tracks, &inventory)?;
    assert_eq!(tracks[STAGE], 1);
    talk(KESAR, "help", &mut topic, &mut tracks, &inventory)?;
    assert_eq!(topic, 0, "Ada is locked while the writ is missing");

    let callbacks = world_callbacks::load()?;
    let chest = callbacks
        .iter()
        .find(|callback| {
            callback.trigger == Trigger::Use
                && callback.selector.position
                    == Some(Position {
                        x: 32401,
                        y: 32469,
                        floor: -5,
                    })
        })
        .ok_or("stage 1 has no cathedral chest USE callback")?;
    let base_hash = [
        0x50, 0xaa, 0xaf, 0x2a, 0xdf, 0x95, 0x2a, 0xca, 0x1f, 0xba, 0xd2, 0x7a, 0xc5, 0x37, 0x90,
        0xdc, 0x70, 0x13, 0x38, 0x97, 0x50, 0x6e, 0xba, 0x1e, 0x64, 0xd0, 0x25, 0x9e, 0xd8, 0xf5,
        0x3c, 0xb7,
    ];
    assert!(
        legacy_defaults(native, &chest.quest, &base_hash)
            .ok_or("stage-1 saves must retain their published definition")?
            .is_empty()
    );
    let transition = native
        .transition(&chest.key)
        .ok_or("chest transaction missing")?;
    let effect = transition
        .effects
        .first()
        .ok_or("chest transition has no effect")?;
    assert!(matches!(
        effect.effect,
        QuestEffectKind::SetWithBinding { value: 2, .. }
    ));
    let reward = chest
        .item_reward()
        .as_ref()
        .ok_or("certificate reward missing")?;
    assert_eq!(reward.item, "oteryn:item.tibia.i52586");
    assert_eq!(reward.count, 1);
    // Feed the chest transaction's planned outputs into the NPC rules.
    let changes = native
        .evaluate(transition, &tracks, 0)
        .map_err(|error| format!("{error:?}"))?;
    for change in changes {
        tracks.insert(change.track, change.after);
    }
    assert_eq!(tracks[STAGE], 2);
    assert_eq!(
        native.evaluate(transition, &tracks, 0),
        Err(QuestRefusal::StageMismatch)
    );
    inventory.items.insert(
        NpcItemKey {
            item: reward.item.clone(),
            subtype: None,
        },
        u32::from(reward.count),
    );

    talk(KESAR, "help", &mut topic, &mut tracks, &inventory)?;
    assert_eq!(topic, 10);
    talk(KESAR, "yes", &mut topic, &mut tracks, &inventory)?;
    assert_eq!(tracks[STAGE], 3);

    for message in ["tell", "yes", "yes", "yes"] {
        talk(DEMERON, message, &mut topic, &mut tracks, &inventory)?;
    }
    assert_eq!(tracks[STAGE], 4);

    // Exploration, skinning, corpse burning, and the kill are world inputs.
    tracks.insert(STAGE.to_owned(), 5);
    for message in ["explain", "yes"] {
        talk(HARTFORD, message, &mut topic, &mut tracks, &inventory)?;
    }
    assert_eq!(tracks[STAGE], 6);
    tracks.insert(BEAKS.to_owned(), 20);
    for message in ["news", "yes"] {
        talk(HARTFORD, message, &mut topic, &mut tracks, &inventory)?;
    }
    assert_eq!(tracks[STAGE], 7);

    for message in ["word", "yes", "yes"] {
        talk(ELDORAN, message, &mut topic, &mut tracks, &inventory)?;
    }
    assert_eq!(tracks[STAGE], 8);
    tracks.insert(CORPSES.to_owned(), 7);
    for message in ["news", "yes", "mission"] {
        talk(ELDORAN, message, &mut topic, &mut tracks, &inventory)?;
    }
    assert_eq!(tracks[STAGE], 10);
    talk(BANNER, "wall", &mut topic, &mut tracks, &inventory)?;
    assert_eq!(tracks[STAGE], 11);

    let death =
        source_deaths::rule("oteryn:quest-transition/order-of-the-stag/court-warlock-room-death")
            .ok_or("Court Warlock death rule missing")?;
    assert!(death.active(&tracks));
    let transition = native
        .transition(&death.transition)
        .ok_or("Court Warlock transition missing")?;
    for change in native
        .evaluate(transition, &tracks, 0)
        .map_err(|error| format!("{error:?}"))?
    {
        tracks.insert(change.track, change.after);
    }
    assert_eq!(tracks[STAGE], 12);
    let finale = talk(ELDORAN, "news", &mut topic, &mut tracks, &inventory)?;
    assert_eq!(tracks[STAGE], 13);
    assert!(finale.effects.contains(&NpcRuleEffect::GiveItem {
        item: "oteryn:item.tibia.i52824".into(),
        count: 1,
        subtype: None,
    }));
    assert!(finale.effects.contains(&NpcRuleEffect::Achievement {
        name: "Bastard".into()
    }));
    let replay = talk(ELDORAN, "news", &mut topic, &mut tracks, &inventory)?;
    assert!(replay.effects.is_empty());
    Ok(())
}
