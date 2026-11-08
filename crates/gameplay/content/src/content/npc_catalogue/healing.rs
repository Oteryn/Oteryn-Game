//! Source-qualified free healing profiles, independent of spells and quest dialogue.
use serde::Deserialize;
use std::sync::OnceLock;

pub const DATA: &str = include_str!("../../../../../../content/npcs/s10-healing.json");

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HealingProfile {
    Standard,
    RookGuard,
    RookPriest,
    PoisonGuard,
    Full,
}

#[derive(Deserialize)]
struct Catalogue {
    schema: String,
    healers: Vec<Healer>,
}
#[derive(Deserialize)]
struct Healer {
    npc: String,
    profile: HealingProfile,
}

pub fn profile(npc: &str) -> Option<HealingProfile> {
    static CATALOGUE: OnceLock<Option<Catalogue>> = OnceLock::new();
    let catalogue = CATALOGUE
        .get_or_init(|| {
            let data: Catalogue = serde_json::from_str(DATA).ok()?;
            let mut keys = std::collections::BTreeSet::new();
            (data.schema == "OTERYN_NPC_HEALING/v1"
                && data
                    .healers
                    .iter()
                    .all(|row| row.npc.starts_with("oteryn:npc.") && keys.insert(&row.npc)))
            .then_some(data)
        })
        .as_ref()?;
    catalogue
        .healers
        .iter()
        .find(|row| row.npc == npc)
        .map(|row| row.profile)
}
