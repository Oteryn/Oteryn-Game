//! MINING-1: only the non-quest shiny-stone rule in Canary's pinned `onUsePick`.
//! There is no generic ore mining table. A pick stays intact; one shiny stone
//! becomes exactly one item. The owner supplies a retained, occurrence-bound draw.

pub const PICK: &str = "oteryn:item.tibia.i3456";
pub const SHINY_STONE: &str = "oteryn:item.tibia.i10310";

/// Pinned Canary 52550d5e, register_actions.lua, shiny stone refining branch.
/// Reject out-of-domain draws rather than biasing or silently clamping them.
pub fn refined_item(tool: &str, target: &str, percentile: u8) -> Option<&'static str> {
    if tool != PICK || target != SHINY_STONE {
        return None;
    }
    match percentile {
        1 => Some("oteryn:item.tibia.i3043"),
        2..=6 => Some("oteryn:item.tibia.i3031"),
        7..=51 => Some("oteryn:item.tibia.i3035"),
        52..=100 => Some("oteryn:item.tibia.i3028"),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
