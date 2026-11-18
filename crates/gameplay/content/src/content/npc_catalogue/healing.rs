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
    QuestEmergency,
}

/// In this healing profile, recovery takes precedence over the ordered cures.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuestEmergency {
    pub track: String,
    pub minimum: i64,
    pub health_floor: u32,
    pub refusal: String,
    pub health_effect: String,
    pub fire_effect: String,
    pub poison_effect: String,
}

#[derive(Debug, Deserialize)]
pub struct Catalogue {
    schema: String,
    healers: Vec<Healer>,
}

#[derive(Debug, Deserialize)]
struct Healer {
    npc: String,
    profile: HealingProfile,
    quest_emergency: Option<QuestEmergency>,
}

impl Catalogue {
    pub fn from_json(source: &str) -> Option<Self> {
        let data: Self = serde_json::from_str(source).ok()?;
        let mut keys = std::collections::BTreeSet::new();
        (data.schema == "OTERYN_NPC_HEALING/v1"
            && data.healers.iter().all(|row| {
                row.npc.starts_with("oteryn:npc.")
                    && keys.insert(&row.npc)
                    && match (&row.quest_emergency, row.profile) {
                        (Some(quest), HealingProfile::QuestEmergency) => {
                            quest.track.starts_with("oteryn:quest-progress/")
                                && quest.health_floor > 0
                                && !quest.refusal.trim().is_empty()
                                && [
                                    &quest.health_effect,
                                    &quest.fire_effect,
                                    &quest.poison_effect,
                                ]
                                .iter()
                                .all(|effect| effect.starts_with("CONST_ME_"))
                        }
                        (None, profile) => profile != HealingProfile::QuestEmergency,
                        (Some(_), _) => false,
                    }
            }))
        .then_some(data)
    }

    fn healer(&self, npc: &str) -> Option<&Healer> {
        self.healers.iter().find(|row| row.npc == npc)
    }

    pub fn quest_emergency(&self, npc: &str) -> Option<&QuestEmergency> {
        self.healer(npc)?.quest_emergency.as_ref()
    }
}

pub fn catalogue() -> Option<&'static Catalogue> {
    static CATALOGUE: OnceLock<Option<Catalogue>> = OnceLock::new();
    CATALOGUE
        .get_or_init(|| Catalogue::from_json(DATA))
        .as_ref()
}

pub fn profile(npc: &str) -> Option<HealingProfile> {
    Some(catalogue()?.healer(npc)?.profile)
}
