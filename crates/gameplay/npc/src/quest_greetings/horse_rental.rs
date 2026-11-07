//! Two source literal greetings; rental authority stays in the current native NPC owner.
use super::{Greeting, LazyLock, parse_with_count};
pub(super) fn get(npc: &str) -> Option<&'static Greeting> {
    static DATA: LazyLock<Option<Vec<Greeting>>> =
        LazyLock::new(|| parse_with_count(include_str!("horse_rental.json"), 2));
    DATA.as_ref()?.iter().find(|row| row.npc == npc)
}
