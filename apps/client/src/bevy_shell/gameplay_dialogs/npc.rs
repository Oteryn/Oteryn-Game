//! Talk and Trade entries of the world context menu for an NPC tile. Both greet the NPC
//! through the same dialogue owner the NPC conversation window and its key use.
use super::HostOwner;
use oteryn_renderer::TileCoord;
use oteryn_session::EntityRef;

/// The NPC on the tile while the session admits NPC dialogue.
pub(super) fn target(host: &HostOwner, tile: TileCoord) -> Option<EntityRef> {
    let play = host.play.as_ref()?;
    if !play.link.npc_dialogue_panel().selected {
        return None;
    }
    play.view.npc_at(tile)
}

/// Says `hi` (Talk) or `trade` (Trade) to the NPC that still stands on the chosen tile.
pub(super) fn talk(host: &HostOwner, tile: TileCoord, npc: EntityRef, trade: bool) -> bool {
    if host.modal_active() {
        return false;
    }
    host.play.as_ref().is_some_and(|play| {
        !play.view.gameplay_input_blocked(&play.link)
            && target(host, tile) == Some(npc)
            && play
                .link
                .npc_dialogue_submit(npc, if trade { "trade" } else { "hi" })
    })
}
