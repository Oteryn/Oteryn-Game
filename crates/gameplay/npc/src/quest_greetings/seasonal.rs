//! SRV7 source literals only; stateful Cake/present choices remain in fenced dispatch.
use super::{Greeting, LazyLock, parse_with_count};

pub(super) fn get(npc: &str) -> Option<&'static Greeting> {
    static DATA: LazyLock<Option<Vec<Greeting>>> =
        LazyLock::new(|| parse_with_count(include_str!("seasonal.json"), 8));
    DATA.as_ref()?.iter().find(|entry| entry.npc == npc)
}

#[cfg(test)]
mod tests {
    #[test]
    fn all_eight_seasonal_npcs_have_source_qualified_greetings() {
        for npc in [
            "balduin_the_baker",
            "benny_the_baker",
            "biff_the_baker",
            "billy_the_baker",
            "brian_the_baker",
            "brutus_the_baker",
            "santa_claus",
            "messenger_of_santa",
        ] {
            let key = format!("oteryn:npc.{npc}");
            assert!(super::get(&key).is_some(), "{key}");
        }
    }
}
