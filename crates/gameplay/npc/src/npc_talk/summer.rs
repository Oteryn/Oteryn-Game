//! Closed Summer 2026 literal greetings. Quest callbacks remain with their own owners.
use std::sync::LazyLock;

pub(super) const SOURCE: &str = include_str!("summer.json");

pub(super) fn get(npc: &str) -> Option<&'static crate::quest_greetings::Greeting> {
    static DATA: LazyLock<Option<Vec<crate::quest_greetings::Greeting>>> =
        LazyLock::new(|| crate::quest_greetings::parse_with_count(SOURCE, 19));
    DATA.as_ref()?.iter().find(|entry| entry.npc == npc)
}

#[cfg(test)]
mod tests {
    #[test]
    fn nineteen_summer_npcs_have_source_literal_greetings() {
        for npc in [
            "captain_corsarah",
            "mayor_pocaro",
            "zofia_bolter",
            "doctor_marrow",
            "wayland_smythers",
            "g_ezkho",
            "omar",
            "thorim",
            "goldro",
            "pescadu",
            "nekaret",
            "saraki",
            "sundara",
            "tarisu",
            "dhira",
            "udu",
            "sharai",
            "javala",
            "niral",
        ] {
            let key = format!("oteryn:npc.{npc}");
            assert!(super::get(&key).is_some(), "{key}");
        }
    }
}
