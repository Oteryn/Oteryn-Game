//! Crystal's carried Secret Agent tool action rolls before applying any tool effect.
use crate::reward_bag::reward_bag_bounded_draw;

/// The three tools transform to their jammed successor and decay back to this form.
pub fn jammed_successor(key: &str) -> Option<u32> {
    match key {
        "oteryn:item.tibia.i9594" => Some(9595),
        "oteryn:item.tibia.i9596" => Some(9597),
        "oteryn:item.tibia.i9598" => Some(9599),
        _ => None,
    }
}

/// Select the same outcome for every retry of this exact session command.
pub fn jams(session: [u8; 16], command: u64, item: [u8; 16]) -> bool {
    reward_bag_bounded_draw(session, command, item, 4, 1_000).is_some_and(|roll| roll < 10)
}
