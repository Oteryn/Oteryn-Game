//! Crystal 201189f action_beaver.lua:1-44. The target events use the ordinary
//! selected-item receipt, then publish their ambient creatures at the tree.
use super::world_callbacks::{Callback, Position};

const BAIT: &str = "oteryn:item.tibia.i9843";
const QUEST: &str = "oteryn:quest.the_new_frontier_quest";
const QUESTLINE: &str = "oteryn:quest-progress/quest/u8_54/the_new_frontier/questline";
const SOURCE: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_new_frontier/action_beaver.lua:1-44";

pub struct Tree {
    pub key: &'static str,
    pub position: Position,
    pub creatures: &'static [(&'static str, u8)],
    bait_stolen: bool,
    track: &'static str,
}

pub const TREES: [Tree; 3] = [
    Tree {
        key: "oteryn:quest-callback/new-frontier/beaver-tree-1",
        position: Position {
            x: 32515,
            y: 31927,
            floor: -7,
        },
        creatures: &[("oteryn:creature.enraged_squirrel", 3)],
        bait_stolen: false,
        track: "oteryn:quest-progress/quest/u8_54/the_new_frontier/mission02/beaver1",
    },
    Tree {
        key: "oteryn:quest-callback/new-frontier/beaver-tree-2",
        position: Position {
            x: 32474,
            y: 31947,
            floor: -7,
        },
        creatures: &[("oteryn:creature.wolf", 5), ("oteryn:creature.war_wolf", 1)],
        bait_stolen: false,
        track: "oteryn:quest-progress/quest/u8_54/the_new_frontier/mission02/beaver2",
    },
    Tree {
        key: "oteryn:quest-callback/new-frontier/beaver-tree-3",
        position: Position {
            x: 32458,
            y: 31997,
            floor: -7,
        },
        creatures: &[("oteryn:creature.thieving_squirrel", 1)],
        bait_stolen: true,
        track: "oteryn:quest-progress/quest/u8_54/the_new_frontier/mission02/beaver3",
    },
];

pub fn for_callback(key: &str) -> Option<&'static Tree> {
    TREES.iter().find(|tree| tree.key == key)
}

pub(super) fn callbacks() -> Result<Vec<Callback>, String> {
    let callbacks = TREES
        .iter()
        .map(|tree| {
            let position = tree.position;
            let mut storage = vec![serde_json::json!({
                "track":tree.track,"minimum":0,"maximum":0,"set":1
            })];
            // The donor's two OR arms admit their positions independently of stage five.
            if tree.bait_stolen {
                storage.push(serde_json::json!({
                    "track":QUESTLINE,"minimum":5,"maximum":5,"set":5,"preserve":true
                }));
            }
            serde_json::json!({
                "key":tree.key,"quest":QUEST,"trigger":"use",
                "selector":{"position":position,"unique":null,"action":null,"appearance":3625,
                    "native":{"position":position,"ordinal":1,"appearance":3625}},
                "lever":null,"reversible":false,"requires_world_admission":true,
                "passages":[],"teleport":null,"storage":storage,"offerings":[],
                "item_on_target":{"selected_instance":true,"item":BAIT,"count":1,
                    "reusable":!tree.bait_stolen,"replacement":null,"target_replacement":null},
                "return_on_failure":false,"failure_message":null,"success_message":null,
                "sources":[SOURCE]
            })
        })
        .collect::<Vec<_>>();
    super::world_callbacks::parse(
        &serde_json::json!({"schema":1,"callbacks":callbacks}).to_string(),
    )
}
