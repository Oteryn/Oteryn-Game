use std::collections::BTreeMap;

use oteryn_gameplay_quests::quest::{
    QuestRefusal, QuestStateCatalogue, loader,
    npc_rules::{
        NpcInventorySnapshot, NpcItemKey, NpcRuleCatalog, NpcRuleInput, NpcRulePlan, NpcRuleTrigger,
    },
    world_callbacks::{self, Position, Trigger},
};

const QUEST: &str = "oteryn:quest.unnatural_selection_quest";
const LAZARAN: &str = "oteryn:npc.lazaran";
const ULALA: &str = "oteryn:npc.ulala";
const FRONTIER: &str = "oteryn:quest-progress/quest/u8_54/the_new_frontier/mission03";
const SKULL: u16 = 10159;

type Case = Result<(), Box<dyn std::error::Error>>;
type Tracks = BTreeMap<String, i64>;

fn track(name: &str) -> String {
    format!("oteryn:quest-progress/quest/u8_54/unnatural_selection/{name}")
}

fn item(number: u16, subtype: Option<u16>) -> NpcItemKey {
    NpcItemKey {
        item: format!("oteryn:item.tibia.i{number}"),
        subtype,
    }
}

struct Conversation {
    npc: &'static str,
    rules: NpcRuleCatalog,
    tracks: Tracks,
    inventory: NpcInventorySnapshot,
    topic: i32,
}

impl Conversation {
    fn new(
        catalogue: &QuestStateCatalogue,
        npc: &'static str,
        stage: i64,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let rules = NpcRuleCatalog::runtime()?
            .admitted_for_npc(npc, catalogue)?
            .ok_or_else(|| format!("missing admitted program for {npc}"))?;
        Ok(Self {
            npc,
            rules,
            tracks: [(track("questline"), stage)].into(),
            inventory: Default::default(),
            topic: 0,
        })
    }

    fn say(&mut self, message: &str) -> Result<Option<NpcRulePlan>, Box<dyn std::error::Error>> {
        let plan = self.rules.plan_item_exchange(&NpcRuleInput {
            npc: self.npc,
            trigger: NpcRuleTrigger::Say,
            message,
            player_name: None,
            topic: self.topic,
            tracks: &self.tracks,
            inventory: &self.inventory,
            now_epoch_seconds: 1_790_000_000,
            calendar_month: 9,
            premium_current: true,
            level: 40,
            vocation: "",
            sex: 0,
            outfits: &BTreeMap::new(),
        })?;
        if let Some(plan) = &plan {
            self.topic = plan.next_topic;
            for change in &plan.track_changes {
                self.tracks.insert(change.key.clone(), change.after);
            }
            self.inventory = plan.inventory_after.clone();
        }
        Ok(plan)
    }

    fn value(&self, name: &str) -> i64 {
        self.tracks.get(&track(name)).copied().unwrap_or_default()
    }

    fn carry(&mut self, number: u16, count: u32, subtype: Option<u16>) {
        self.inventory.items.insert(item(number, subtype), count);
    }

    fn count(&self, number: u16, subtype: Option<u16>) -> u32 {
        self.inventory
            .items
            .get(&item(number, subtype))
            .copied()
            .unwrap_or_default()
    }
}

#[test]
fn unnatural_skulled_offer_pickup_and_report_use_source_state() -> Case {
    let loaded =
        loader::load_embedded_quest_state("content-1").map_err(|error| format!("{error:?}"))?;
    let catalogue = loaded.catalogue();
    let mut lazaran =
        Conversation::new(catalogue, LAZARAN, 0).map_err(|error| format!("{error:?}"))?;
    lazaran.say("help")?.ok_or("help")?;
    lazaran.say("yes")?.ok_or("accept help")?;
    lazaran.tracks.insert(FRONTIER.into(), 2);
    assert!(lazaran.say("mission")?.is_none());
    lazaran.tracks.insert(FRONTIER.into(), 3);
    lazaran.say("mission")?.ok_or("Skulled offer")?;
    lazaran.say("yes")?.ok_or("Skulled acceptance")?;
    assert_eq!(
        (lazaran.value("questline"), lazaran.value("mission01")),
        (1, 1)
    );
    lazaran.say("mission")?.ok_or("Skulled return offer")?;
    lazaran.say("yes")?.ok_or("missing skull response")?;
    assert_eq!(lazaran.value("questline"), 1);

    let callbacks = world_callbacks::load_for(catalogue)?;
    let pickup = callbacks
        .iter()
        .find(|row| row.key.ends_with("skulled-pickup"))
        .ok_or("pickup callback")?;
    assert_eq!(pickup.quest, QUEST);
    assert_eq!(pickup.trigger, Trigger::Use);
    assert_eq!(pickup.selector.unique, Some(9259));
    let native = pickup.selector.native.as_ref().ok_or("native bone pile")?;
    assert_eq!(
        native.position,
        Position {
            x: 33054,
            y: 31312,
            floor: -8
        }
    );
    assert_eq!((native.ordinal, native.appearance), (2, 10205));
    let reward = pickup.item_reward.as_ref().ok_or("skull reward")?;
    assert_eq!((&reward.item, reward.count), (&item(SKULL, None).item, 1));
    let transition = catalogue
        .transition(&pickup.key)
        .ok_or("pickup transition")?;
    let changes = catalogue
        .evaluate(transition, &lazaran.tracks, 0)
        .map_err(|error| format!("{error:?}"))?;
    for change in changes {
        lazaran.tracks.insert(change.track, change.after);
    }
    assert_eq!(lazaran.value("mission01"), 2);
    assert_eq!(
        catalogue.evaluate(transition, &lazaran.tracks, 0),
        Err(QuestRefusal::StageMismatch)
    );
    lazaran.carry(SKULL, 1, None);
    lazaran.say("mission")?.ok_or("Skulled return offer")?;
    lazaran.say("yes")?.ok_or("Skulled return")?;
    assert_eq!(
        (
            lazaran.value("questline"),
            lazaran.value("mission01"),
            lazaran.count(SKULL, None)
        ),
        (2, 3, 0)
    );
    Ok(())
}

#[test]
fn unnatural_tour_requires_skull_and_order_at_all_source_stops() -> Case {
    let loaded =
        loader::load_embedded_quest_state("content-1").map_err(|error| format!("{error:?}"))?;
    let catalogue = loaded.catalogue();
    let mut lazaran = Conversation::new(catalogue, LAZARAN, 2)?;
    lazaran.say("mission")?.ok_or("tour offer")?;
    lazaran.say("yes")?.ok_or("tour acceptance")?;
    assert_eq!(
        (
            lazaran.value("questline"),
            lazaran.value("mission02"),
            lazaran.count(SKULL, None)
        ),
        (3, 1, 1)
    );
    lazaran.say("mission")?.ok_or("tour return offer")?;
    lazaran.say("yes")?.ok_or("unfinished tour response")?;
    assert_eq!(lazaran.count(SKULL, None), 1);

    let callbacks = world_callbacks::load_for(catalogue)?;
    let expected = [
        (33263, 31834, -1, 1),
        (32711, 31668, -1, 1),
        (32537, 31772, -1, 1),
        (33216, 32450, -1, 1),
        (33153, 32845, -2, 16),
        (32689, 32742, -4, 5),
        (32346, 32808, -2, 1),
        (32789, 31238, -3, 1),
        (32236, 31096, -2, 1),
        (32344, 32265, 0, 1),
        (32316, 31752, 0, 1),
    ];
    for (i, (x, y, floor, count)) in expected.into_iter().enumerate() {
        let prefix = format!(
            "oteryn:quest-callback/unnatural-selection-tour-{:02}",
            i + 1
        );
        let stops: Vec<_> = callbacks
            .iter()
            .filter(|c| c.key.starts_with(&prefix))
            .collect();
        assert_eq!(
            stops
                .iter()
                .map(|c| 1 + c.selector.position_aliases.len())
                .sum::<usize>(),
            count
        );
        let stop = stops
            .iter()
            .find(|c| c.selector.matches_position(Position { x, y, floor }))
            .ok_or("missing lookout")?;
        for row in &stops {
            assert_eq!(row.trigger, Trigger::StepIn);
            assert_eq!(row.present.len(), 1);
            assert_eq!(
                (row.present[0].item.as_str(), row.present[0].count),
                (item(SKULL, None).item.as_str(), 1)
            );
            assert!(row.offerings.is_empty());
        }
        let transition = catalogue.transition(&stop.key).ok_or("tour transition")?;
        let wrong = [(track("mission02"), lazaran.value("mission02") + 1)].into();
        assert_eq!(
            catalogue.evaluate(transition, &wrong, 0),
            Err(QuestRefusal::StageMismatch)
        );
        let changes = catalogue
            .evaluate(transition, &lazaran.tracks, 0)
            .map_err(|error| format!("{error:?}"))?;
        for change in changes {
            lazaran.tracks.insert(change.track, change.after);
        }
    }
    lazaran.say("mission")?.ok_or("tour return offer")?;
    lazaran.say("yes")?.ok_or("tour return")?;
    assert_eq!(
        (
            lazaran.value("questline"),
            lazaran.value("mission02"),
            lazaran.count(SKULL, None)
        ),
        (4, 13, 0)
    );
    Ok(())
}

#[test]
fn unnatural_dance_handoff_and_materials_feed_fasuon_report() -> Case {
    let loaded =
        loader::load_embedded_quest_state("content-1").map_err(|error| format!("{error:?}"))?;
    let catalogue = loaded.catalogue();
    let mut lazaran = Conversation::new(catalogue, LAZARAN, 4)?;
    lazaran.say("mission")?.ok_or("dance offer")?;
    lazaran.say("yes")?.ok_or("dance handoff")?;
    assert_eq!(
        (lazaran.value("questline"), lazaran.value("mission03")),
        (5, 1)
    );
    let mut ulala = Conversation::new(catalogue, ULALA, 0).map_err(|error| format!("{error:?}"))?;
    ulala.tracks = lazaran.tracks;
    assert!(ulala.say("krunus")?.is_none());
    ulala.say("mission")?.ok_or("Krunus dialogue")?;
    ulala.say("krunus")?.ok_or("dance opening")?;
    assert_eq!(
        (
            ulala.value("questline"),
            ulala.value("mission03"),
            ulala.value("dance_status")
        ),
        (6, 2, 1)
    );
    ulala.say("mission")?.ok_or("unfinished dance response")?;
    assert_eq!(ulala.value("mission04"), 0);
    let dance = oteryn_gameplay_quests::quest::unnatural_selection::SEQUENCE;
    for tile in dance {
        let key = format!("oteryn:quest-callback/unnatural-selection-dance-{tile}");
        let transition = catalogue.transition(&key).ok_or("dance tile transition")?;
        for change in catalogue
            .evaluate(transition, &ulala.tracks, 0)
            .map_err(|error| format!("{error:?}"))?
        {
            ulala.tracks.insert(change.track, change.after);
        }
    }
    assert_eq!((ulala.value("questline"), ulala.value("mission03")), (7, 3));
    ulala
        .say("mission")?
        .ok_or("dance report and material opening")?;
    assert_eq!((ulala.value("questline"), ulala.value("mission04")), (8, 1));
    ulala.carry(10196, 6, None);
    ulala.carry(5878, 5, None);
    ulala.carry(5876, 4, None);
    ulala.say("mission")?.ok_or("materials offer")?;
    ulala.say("yes")?.ok_or("missing materials response")?;
    assert_eq!(
        [
            ulala.value("questline"),
            ulala.count(10196, None).into(),
            ulala.count(5878, None).into(),
            ulala.count(5876, None).into()
        ],
        [8, 6, 5, 4]
    );

    ulala.carry(5876, 5, None);
    ulala.say("mission")?.ok_or("materials offer")?;
    ulala.say("yes")?.ok_or("materials report")?;
    assert_eq!(
        [
            ulala.value("questline"),
            ulala.count(10196, None).into(),
            ulala.count(5878, None).into(),
            ulala.count(5876, None).into()
        ],
        [9, 1, 0, 0]
    );
    assert_eq!(ulala.value("mission04"), 2);
    ulala.say("mission")?.ok_or("Fasuon opening")?;
    assert_eq!(
        (ulala.value("questline"), ulala.value("mission05")),
        (10, 1)
    );
    ulala.say("mission")?.ok_or("unfinished Fasuon response")?;
    assert_eq!(ulala.value("questline"), 10);

    let callbacks = world_callbacks::load_for(catalogue)?;
    let crystal = callbacks
        .iter()
        .find(|callback| callback.key.ends_with("unnatural-selection-ray-of-light"))
        .ok_or("native crystal callback")?;
    let magic = crystal.effects.iter().next().ok_or("Fasuon holy light")?;
    assert_eq!(magic.effect, 50);
    assert_eq!(
        magic.position,
        crystal
            .selector
            .native
            .as_ref()
            .ok_or("crystal position")?
            .position
    );
    let prayer = catalogue
        .transition(&crystal.key)
        .ok_or("native crystal transition")?;
    for change in catalogue
        .evaluate(prayer, &ulala.tracks, 0)
        .map_err(|error| format!("{error:?}"))?
    {
        ulala.tracks.insert(change.track, change.after);
    }
    assert_eq!(
        (ulala.value("questline"), ulala.value("mission05")),
        (11, 2)
    );
    ulala.say("mission")?.ok_or("Fasuon report")?;
    assert_eq!(
        (ulala.value("questline"), ulala.value("mission05")),
        (12, 3)
    );
    ulala.say("mission")?.ok_or("gods' approval")?;
    assert_eq!(
        (ulala.value("questline"), ulala.value("mission06")),
        (13, 1)
    );
    Ok(())
}

#[test]
fn unnatural_firewater_consumes_only_beer_and_completes_with_crest() -> Case {
    let loaded =
        loader::load_embedded_quest_state("content-1").map_err(|error| format!("{error:?}"))?;
    let mut lazaran = Conversation::new(loaded.catalogue(), LAZARAN, 12)?;
    assert!(lazaran.say("mission")?.is_none());
    lazaran.tracks.insert(track("questline"), 13);
    lazaran.say("mission")?.ok_or("Firewater opening")?;
    assert_eq!(
        (lazaran.value("questline"), lazaran.value("mission06")),
        (14, 2)
    );
    lazaran.carry(3465, 1, None);
    lazaran.carry(3465, 1, Some(1));
    lazaran.say("mission")?.ok_or("beer offer")?;
    let denied = lazaran.say("yes")?.ok_or("wrong fluid response")?;
    assert!(!denied.completes);
    assert_eq!(
        (
            lazaran.value("questline"),
            lazaran.count(3465, None),
            lazaran.count(3465, Some(1))
        ),
        (14, 1, 1)
    );
    assert_eq!(lazaran.count(10198, None), 0);

    lazaran.carry(3465, 1, Some(3));
    lazaran.say("mission")?.ok_or("beer offer")?;
    let accepted = lazaran.say("yes")?.ok_or("beer exchange")?;
    assert!(accepted.completes);
    assert_eq!(
        (lazaran.value("questline"), lazaran.value("mission06")),
        (15, 3)
    );
    assert_eq!(
        (
            lazaran.count(3465, None),
            lazaran.count(3465, Some(1)),
            lazaran.count(3465, Some(3))
        ),
        (1, 1, 0)
    );
    assert_eq!(lazaran.count(10198, None), 1);
    assert!(lazaran.say("yes")?.is_none());
    Ok(())
}
