//! Crystal 201189f court_warlock_fight.lua:26-40,363-414.
//! Numeric timings are explicitly unconfirmed in the donor; retain them verbatim.
use super::{Callback, Position};

pub const KEY: &str = "oteryn:quest-callback/cwarlock/lever";
pub const QUEST: &str = "oteryn:quest.the_order_of_the_stag_quest";
pub const TRACK: &str =
    "oteryn:quest-progress/crystalserver/quest/u15_20/the_order_of_the_stag/questline";
pub const BOSS: &str = "oteryn:boss.court_warlock";
pub const EXIT: Position = Position {
    x: 33842,
    y: 32141,
    floor: -3,
};

pub fn callback() -> Result<Callback, String> {
    let center = Position {
        x: 33838,
        y: 32141,
        floor: -2,
    };
    let players: Vec<_> = (32142..=32146)
        .map(|y| {
            serde_json::json!({
                "from": Position { x: 33842, y, floor: -3 }, "to": center
            })
        })
        .collect();
    serde_json::from_value(serde_json::json!({
        "key": KEY, "quest": QUEST, "trigger": "use",
        "selector": {"position": {"x":33843,"y":32147,"floor":-3},
                     "unique":null,"action":null,"appearance":null},
        "lever":null,"reversible":false,"passages":[],"teleport":null,
        "storage":[],"offerings":[],"return_on_failure":false,"failure_message":null,
        "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_order_of_the_stag/court_warlock_fight.lua:363-414"],
        "effects":{"encounter":{
            "minimum_level":0,"players":players,"monsters":[],
            "room_min":{"x":33830,"y":32133,"floor":-2},
            "room_max":{"x":33847,"y":32150,"floor":-2},
            "phase":"court_warlock"
        }}
    })).map_err(|e| e.to_string())
}
