//! Buddel and Iskan select their native offers inside the existing paid travel conversation.
use super::*;
use crate::durability::npc_travel::svargrond::{self, BUDDEL, DESTINATIONS, ISKAN, Payment};

pub(super) fn select(
    npc: &str,
    text: &str,
    declined: Option<&Prompt>,
) -> Option<(TravelRoute, Payment)> {
    let keyword = match npc {
        BUDDEL => declined.map_or(text, |prompt| &prompt.route.keyword),
        ISKAN if text == "passage" => text,
        _ => return None,
    };
    let payment = if npc == ISKAN {
        Payment::Fare
    } else if declined.is_some() {
        Payment::DeclinedRing
    } else {
        Payment::Ring
    };
    Some((svargrond::route(npc, keyword, payment)?, payment))
}

pub(super) fn offer(npc: &str, route: &TravelRoute, fare: u64) -> String {
    if npc == ISKAN {
        return "Would you like a passage to Nibelor?".into();
    }
    if svargrond::pays_ring(route) {
        return "Give me that ring and I'll take you straight there for free. Agreed?".into();
    }
    format!(
        "I'll take you to {} for {fare} gold. Agreed?",
        route.keyword
    )
}

pub(super) fn select_buddel_arrival(route: &mut TravelRoute) -> Option<()> {
    if svargrond::pays_ring(route) {
        return Some(());
    }
    // Two ordered conditional yes branches. Each checks a fresh 4-in-5 roll.
    // The second success samples all four islands, including the named one.
    fn succeeds() -> Option<bool> {
        loop {
            let [roll] = crate::gameplay_transport::SecureIdentifiers::random()?;
            if roll < 255 {
                return Some(roll % 5 != 0);
            }
        }
    }
    if succeeds()? {
        return Some(());
    }
    if !succeeds()? {
        return None;
    }
    let [index] = crate::gameplay_transport::SecureIdentifiers::random()?;
    let (_, x, y) = DESTINATIONS[usize::from(index & 3)];
    route.route.destination.x = x;
    route.route.destination.y = y;
    Some(())
}
