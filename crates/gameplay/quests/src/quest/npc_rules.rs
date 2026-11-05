//! Crystal `201189f871d23597b82caf88fd611454a2ddefb8:data/npclib/npc.lua:13-20`
//! defines keyword matching; individual programs retain their source references.

mod evaluate;
mod keyword;
mod validate;

// Directory-scanned, source-qualified programs. Durable effects require their
// composed owner; dialogue admission never silently omits an effect.
include!(concat!(env!("OUT_DIR"), "/npc_rule_programs.rs"));

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Preserve the existing QuestState bound; an oversized donor transaction is
/// refused whole, never split. Coordinator ANSWER, point 5:
/// https://github.com/Oteryn/Oteryn-Game/pull/2843#issuecomment-6589517205
pub const MAX_NPC_TRACK_CHANGES: usize = 8;
pub const MAX_NPC_EFFECTS: usize = 128;
const SOURCE_PIN: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NpcRuleError {
    Json(String),
    Invalid(String),
    Arithmetic(String),
    MissingTrack(String),
    MissingItems(String),
    MissingMoney,
    MissingNativeResolution(String),
    LimitExceeded,
    MissingRuntimeOwner(String),
}

impl fmt::Display for NpcRuleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}
impl std::error::Error for NpcRuleError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NpcRuleTrigger {
    Say,
    Greet,
    Farewell,
    Walkaway,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NpcKeywordMode {
    #[default]
    MsgContains,
    Exact,
    Substring,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NpcComparison {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    #[default]
    Ge,
    Between,
}

impl NpcComparison {
    fn holds(self, actual: i64, value: i64, maximum: Option<i64>) -> bool {
        match self {
            Self::Eq => actual == value,
            Self::Ne => actual != value,
            Self::Lt => actual < value,
            Self::Le => actual <= value,
            Self::Gt => actual > value,
            Self::Ge => actual >= value,
            Self::Between => maximum.is_some_and(|high| (value..=high).contains(&actual)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NpcTrackDefinition {
    pub key: String,
    pub quest: String,
    pub initial: i64,
    pub min: i64,
    pub max: i64,
    pub sources: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NpcRulePosition {
    pub x: i32,
    pub y: i32,
    pub floor: i16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum NpcRuleGuard {
    Storage {
        track: String,
        op: NpcComparison,
        value: i64,
        #[serde(default)]
        maximum: Option<i64>,
    },
    StorageTime {
        track: String,
        op: NpcComparison,
        #[serde(default)]
        offset_seconds: i64,
    },
    Item {
        item: String,
        count: u32,
        #[serde(default)]
        subtype: Option<u16>,
        #[serde(default)]
        op: NpcComparison,
    },
    Money {
        amount: u64,
        #[serde(default)]
        include_bank: bool,
    },
    Equipped {
        slot: String,
        item: String,
    },
    Level {
        op: NpcComparison,
        value: i64,
    },
    CalendarMonth {
        value: u8,
    },
    Vocation {
        value: String,
    },
    Sex {
        value: u8,
    },
    Outfit {
        look_type: u16,
        addon: u8,
    },
}

/// Effects retain source order. Storage/topic effects have already been evaluated
/// in the plan; the owner executes the other typed effects after durable admission.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum NpcRuleEffect {
    BigfootMatchmakerTarget,
    BigfootSoulMelody,
    Set {
        track: String,
        value: i64,
    },
    Add {
        track: String,
        value: i64,
    },
    SetNowPlus {
        track: String,
        value: i64,
    },
    SetTopic {
        value: i32,
    },
    TakeItem {
        item: String,
        count: u32,
        #[serde(default)]
        subtype: Option<u16>,
        /// Crystal sometimes ignores removeItem's boolean return. Such a source
        /// call consumes nothing when the complete requested count is absent.
        #[serde(default)]
        optional: bool,
    },
    GiveItem {
        item: String,
        count: u32,
        #[serde(default)]
        subtype: Option<u16>,
    },
    TakeMoney {
        amount: u64,
        #[serde(default)]
        include_bank: bool,
    },
    GiveMoney {
        amount: u64,
        #[serde(default)]
        include_bank: bool,
    },
    Experience {
        amount: u64,
    },
    Achievement {
        name: String,
    },
    Outfit {
        look_type: u16,
        addon: u8,
    },
    OutfitAddon {
        look_type: u16,
        addon: u8,
    },
    Teleport {
        position: NpcRulePosition,
    },
    MagicEffect {
        effect: String,
    },
    MapMark {
        position: NpcRulePosition,
        mark: u8,
        text: String,
    },
    EndConversation {
        delay_ms: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NpcRule {
    pub key: String,
    pub quest: String,
    pub npc: String,
    pub trigger: NpcRuleTrigger,
    #[serde(default)]
    pub keywords: Vec<String>,
    #[serde(default)]
    pub exclude_keywords: Vec<String>,
    #[serde(default)]
    pub keyword_mode: NpcKeywordMode,
    #[serde(default)]
    pub any_message: bool,
    #[serde(default)]
    pub topics: Option<Vec<i32>>,
    #[serde(default)]
    pub guards: Vec<NpcRuleGuard>,
    #[serde(default)]
    pub reply: Vec<String>,
    #[serde(default)]
    pub effects: Vec<NpcRuleEffect>,
    #[serde(default)]
    pub continue_matching: bool,
    #[serde(default)]
    pub inventory_failure_reply: Vec<String>,
    pub sources: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NpcSourceFile {
    pub path: String,
    pub sha256: String,
    pub revision: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NpcRuleDocument {
    pub schema: u32,
    pub quest: String,
    pub tracks: Vec<NpcTrackDefinition>,
    pub rules: Vec<NpcRule>,
    #[serde(default)]
    pub source_files: Vec<NpcSourceFile>,
}

#[derive(Debug, Clone)]
pub struct NpcRuleProgram {
    pub document: NpcRuleDocument,
    pub digest: [u8; 32],
}

#[derive(Debug, Clone, Default)]
pub struct NpcRuleCatalog {
    programs: Vec<NpcRuleProgram>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct NpcItemKey {
    pub item: String,
    pub subtype: Option<u16>,
}

/// The inventory owner supplies observed counts, never client-reported counts.
/// `subtype=None` is an untyped instance. A rule without a subtype sums all
/// variants; a fluid rule matches only its exact subtype.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NpcInventorySnapshot {
    pub items: BTreeMap<NpcItemKey, u32>,
    pub equipped: BTreeMap<String, String>,
    pub money: u64,
    pub bank: u64,
}

pub struct NpcRuleInput<'a> {
    pub npc: &'a str,
    pub trigger: NpcRuleTrigger,
    pub message: &'a str,
    pub topic: i32,
    pub tracks: &'a BTreeMap<String, i64>,
    pub inventory: &'a NpcInventorySnapshot,
    pub now_epoch_seconds: i64,
    pub calendar_month: u8,
    pub level: u32,
    pub vocation: &'a str,
    pub sex: u8,
    pub outfits: &'a BTreeMap<u16, u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NpcTrackChange {
    pub key: String,
    pub before: i64,
    pub after: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NpcRulePlan {
    pub quest: String,
    pub program_digest: [u8; 32],
    pub selected_rule_keys: Vec<String>,
    pub reply: Vec<String>,
    pub next_topic: i32,
    pub observed_tracks: BTreeMap<String, i64>,
    pub track_changes: Vec<NpcTrackChange>,
    pub effects: Vec<NpcRuleEffect>,
    pub inventory_before: NpcInventorySnapshot,
    pub inventory_after: NpcInventorySnapshot,
    pub inventory_failure_reply: Vec<String>,
}

#[must_use]
pub fn program_transition_key(digest: [u8; 32]) -> String {
    use std::fmt::Write;
    let mut key = String::from("oteryn:quest-transition/npc-program/");
    for byte in digest {
        // Writing to a String is infallible.
        let _ = write!(&mut key, "{byte:02x}");
    }
    key
}

impl NpcRuleCatalog {
    pub fn from_json_documents(documents: &[&str]) -> Result<Self, NpcRuleError> {
        let mut programs = Vec::with_capacity(documents.len());
        for source in documents {
            let document: NpcRuleDocument = serde_json::from_str(source)
                .map_err(|error| NpcRuleError::Json(error.to_string()))?;
            let canonical = serde_json::to_vec(&document)
                .map_err(|error| NpcRuleError::Json(error.to_string()))?;
            programs.push(NpcRuleProgram {
                document,
                digest: Sha256::digest(canonical).into(),
            });
        }
        let catalogue = Self { programs };
        validate::catalogue(&catalogue)?;
        Ok(catalogue)
    }

    pub fn load_embedded() -> Result<Self, NpcRuleError> {
        Self::from_json_documents(NPC_RULE_PROGRAMS)
    }

    #[must_use]
    pub fn programs(&self) -> &[NpcRuleProgram] {
        &self.programs
    }

    #[must_use]
    pub fn program(&self, quest: &str) -> Option<&NpcRuleProgram> {
        self.programs
            .iter()
            .find(|program| program.document.quest == quest)
    }

    /// Merge source tracks and one non-executable transition per full program.
    /// Existing compatible wider bounds are retained; owner/initial conflicts fail.
    /// The ordinary catalogue hash consequently binds both existing callbacks
    /// and every source branch/reply, without a second definition-hash authority.
    pub fn extend_quest_state(
        &self,
        catalogue: &super::QuestStateCatalogue,
    ) -> Result<super::QuestStateCatalogue, NpcRuleError> {
        let mut tracks = catalogue.tracks.clone();
        let mut transitions = catalogue.transitions.clone();
        for program in &self.programs {
            if !super::in_runtime_scope(&program.document.quest) {
                continue;
            }
            for source in &program.document.tracks {
                let track = super::QuestTrack {
                    key: source.key.clone(),
                    quest: source.quest.clone(),
                    initial: source.initial,
                    min: source.min,
                    max: source.max,
                };
                if let Some(existing) = tracks.get(&track.key) {
                    if existing.quest != track.quest
                        || existing.initial != track.initial
                        || existing.min > track.min
                        || existing.max < track.max
                    {
                        return Err(NpcRuleError::Invalid(format!(
                            "conflicting NPC track: {}",
                            track.key
                        )));
                    }
                    // Native and source callback owners may already admit a
                    // wider range; source extension never narrows that contract.
                } else {
                    tracks.insert(track.key.clone(), track);
                }
            }
            let track = program
                .document
                .tracks
                .first()
                .ok_or_else(|| NpcRuleError::Invalid("NPC program has no tracks".into()))?;
            let transition = super::QuestTransition {
                key: program_transition_key(program.digest),
                quest: program.document.quest.clone(),
                effects: vec![super::QuestEffect {
                    track: track.key.clone(),
                    from: super::QuestComparison::Any,
                    effect: super::QuestEffectKind::Computed,
                }],
                completes: false,
                experience: None,
            };
            if transitions
                .get(&transition.key)
                .is_some_and(|existing| existing != &transition)
            {
                return Err(NpcRuleError::Invalid(
                    "conflicting NPC program transition".into(),
                ));
            }
            transitions.insert(transition.key.clone(), transition);
        }
        super::QuestStateCatalogue::new(
            catalogue.content_revision(),
            tracks.into_values().collect(),
            transitions.into_values().collect(),
        )
        .map_err(|error| NpcRuleError::Invalid(format!("{error:?}")))
    }

    pub fn plan(&self, input: &NpcRuleInput<'_>) -> Result<Option<NpcRulePlan>, NpcRuleError> {
        self.plan_with_native(input, &BTreeMap::new())
    }

    /// Admit reply/topic branches with only the facts supplied by the dialogue
    /// owner. An unavailable guard or effect refuses the complete ordered match,
    /// including any earlier continuing branches; no successful source text leaks.
    pub fn plan_dialogue(
        &self,
        input: &NpcRuleInput<'_>,
    ) -> Result<Option<NpcRulePlan>, NpcRuleError> {
        evaluate::plan_dialogue(self, input)
    }

    /// The QuestState owner supplies locked tracks and its database clock.
    /// Inventory, currency and appearance effects still require their atomic owners.
    pub fn plan_storage(
        &self,
        input: &NpcRuleInput<'_>,
    ) -> Result<Option<NpcRulePlan>, NpcRuleError> {
        evaluate::plan_storage(self, input)
    }

    /// Native effects require a bounded result from the encounter/RNG owner. The
    /// key is the full selected rule key. Missing or malformed results refuse the
    /// whole plan; no unresolved obligation reaches the durable writer.
    pub fn plan_with_native(
        &self,
        input: &NpcRuleInput<'_>,
        resolutions: &BTreeMap<String, Vec<NpcRuleEffect>>,
    ) -> Result<Option<NpcRulePlan>, NpcRuleError> {
        evaluate::plan(self, input, resolutions)
    }
}

#[cfg(test)]
mod tests;
