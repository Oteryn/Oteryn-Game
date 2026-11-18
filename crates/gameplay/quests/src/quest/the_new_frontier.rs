//! Source selectors and quest-state bindings for the New Frontier arena mechanics.
use super::world_callbacks::{Callback, Position};

pub const CUROS: &str = "oteryn:quest-callback/nfront2-curos-survival";
pub const CUROS_ARENA: Position = Position {
    x: 33154,
    y: 31415,
    floor: -7,
};
pub const CUROS_RADIUS: u16 = 6;
pub const CUROS_SURVIVAL_US: u64 = 120_000_000;

pub(super) fn curos_callback() -> Result<Callback, String> {
    let value = serde_json::json!({"schema":1,"callbacks":[{
        "key":CUROS,
        "quest":"oteryn:quest.the_new_frontier_quest",
        "trigger":"step_in",
        "selector":{"position":{"x":33146,"y":31413,"floor":-6},"ordinal":0},
        "lever":null,"reversible":false,"requires_world_admission":true,"passages":[],
        "teleport":{"x":33145,"y":31419,"floor":-7},
        "storage":[
            {"track":"oteryn:quest-progress/quest/u8_54/the_new_frontier/questline","minimum":17,"maximum":17,"set":18},
            {"track":"oteryn:quest-progress/quest/u8_54/the_new_frontier/mission06","minimum":0,"maximum":3,"set":3}
        ],
        "offerings":[],"return_on_failure":true,
        "failure_message":"You don't have access to this area.",
        "effects":{"teleport_effect":11,"player_speech":"You have braved the tiral of the Mooh'tah master."},
        "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_new_frontier/movement_minotaur_boss.lua:1-41"]
    }]});
    super::world_callbacks::parse(&value.to_string())?
        .pop()
        .ok_or_else(|| "missing Curos survival callback".into())
}
