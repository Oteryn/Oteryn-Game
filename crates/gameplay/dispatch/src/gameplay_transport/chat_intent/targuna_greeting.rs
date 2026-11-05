//! Presentation only for the three stateful Targuna CALLBACK_GREET functions.
//! Reads the existing admitted quest copy; it grants no conversation or quest authority.
use crate::chat::ChatPosition;
use oteryn_gameplay_quests::quest::targuna::{
    dialogue,
    npc_rules::{self, Npc, NpcState, Reply},
};

pub(super) enum Greeting {
    Other,
    Line(String),
    Unavailable,
}

pub(super) fn opening(
    key: &str,
    state: Option<NpcState>,
    npc_position: ChatPosition,
    player_position: ChatPosition,
    player_name: &str,
) -> Greeting {
    let npc = match key {
        // zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/lizzie.lua:65–73.
        "oteryn:npc.lizzie" => Npc::Lizzie,
        // zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/sterling.lua:69–77.
        "oteryn:npc.sterling" => Npc::Sterling,
        // zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/emiliana.lua:72–103.
        "oteryn:npc.emiliana" => Npc::Emiliana,
        _ => return Greeting::Other,
    };
    let Some(state) = state else {
        return Greeting::Unavailable;
    };
    let place = npc_rules::emiliana_place(
        i16::from(npc_position.floor),
        player_position.x,
        player_position.y,
        i16::from(player_position.floor),
    );
    let reply = Reply {
        source_line: npc_rules::greet(npc, state, place),
        numbers: Vec::new(),
    };
    let Some(mut lines) = dialogue::render(npc, &reply) else {
        return Greeting::Unavailable;
    };
    if lines.len() != 1 {
        return Greeting::Unavailable;
    }
    let Some(line) = lines.pop() else {
        return Greeting::Unavailable;
    };
    Greeting::Line(line.replace("|PLAYERNAME|", player_name))
}
