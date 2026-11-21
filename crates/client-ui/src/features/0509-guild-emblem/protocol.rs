//! Capability 509: Tibia Global's guild emblem beside players, from the server's relation.
use oteryn_client_runtime::guild_emblem::EntityRef;
use oteryn_client_runtime::guild_emblem::{Emblem, GuildEmblem};

/// The server's list replaces the previous one whole; an empty list clears every emblem.
pub fn apply(current: &mut Option<Vec<GuildEmblem>>, update: Vec<GuildEmblem>) {
    *current = Some(update);
}

/// The emblem the actor HUD draws beside `entity`, if the server sent one.
#[must_use]
pub fn emblem_of(emblems: &[GuildEmblem], entity: EntityRef) -> Option<Emblem> {
    emblems
        .iter()
        .find(|emblem| emblem.entity == entity)
        .map(|emblem| emblem.emblem)
}
#[cfg(test)]
#[path = "tests.rs"]
mod tests;
