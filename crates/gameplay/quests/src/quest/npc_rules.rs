//! Test-only source qualification for declarative NPC branches in quest batch three.
//!
//! Mounted only with `cfg(test)`: it validates captured programs and builds pure
//! plans for source tests. No production NPC API, adapter or writer executes them.
//! Coordinator ANSWER forbids changing those owners in this batch:
//! https://github.com/Oteryn/Oteryn-Game/pull/2843#issuecomment-6589900671
//! Crystal `201189f871d23597b82caf88fd611454a2ddefb8:data/npclib/npc.lua:13-20`
//! defines keyword matching; individual programs retain their source references.

mod evaluate;
mod keyword;
mod validate;

// Directory-scanned source data only. No NPC owner applies these plans yet;
// see PR2843#issuecomment-6589900671. Each quest can add its own file without
// editing a shared include list or depending on other quest commits.
include!(concat!(env!("OUT_DIR"), "/npc_rule_programs.rs"));

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// The NPC planner and durable receipt admit the same bounded, atomic write set.
/// Owner amendment, 2026-11-06: Father's Burden's eleven writes must not be split.
pub const MAX_NPC_TRACK_CHANGES: usize = super::QUESTSTATE0_RL_02;
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

    pub fn plan(&self, input: &NpcRuleInput<'_>) -> Result<Option<NpcRulePlan>, NpcRuleError> {
        self.plan_with_native(input, &BTreeMap::new())
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
