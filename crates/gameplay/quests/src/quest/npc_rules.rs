//! Source-qualified, bounded NPC programs consumed by the fenced NPC quest owner.
//! Crystal source references and source-order branches are retained in each program.

#[cfg(test)]
mod achievement_tests;
mod admission;
mod bounds;
mod calendar;
mod equipped_storage;
pub use calendar::NpcUtcCalendar;
mod evaluate;
mod hidden_threats;
mod item_exchange;
mod keyword;
mod marlin_trophy;
mod player_name;
pub mod teleport;
mod validate;

// Directory-scanned source programs retain their exact source custody.
include!(concat!(env!("OUT_DIR"), "/npc_rule_programs.rs"));

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Share the receipt writer's atomic bound; oversized plans are refused whole.
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
    /// Inclusive, annually repeating UTC dates; the durable owner supplies epoch time.
    CalendarWindow {
        start_month: u8,
        start_day: u8,
        end_month: u8,
        end_day: u8,
        within: bool,
    },
    Vocation {
        value: String,
    },
    Premium {
        value: bool,
    },
    Sex {
        value: u8,
    },
    Outfit {
        look_type: u16,
        addon: u8,
    },
}

/// Effects retain source order. Storage/topic effects are evaluated in the plan;
/// the production storage owner refuses effects requiring another native owner.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum NpcRuleEffect {
    /// Once-only proof of one source saga; never closes the canonical quest.
    /// The declared 0..1 track includes the canonical quest and source identity.
    CompleteSource {
        track: String,
    },
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
    /// Match the current Character owner's name instead of a source literal.
    /// Omit false to preserve existing programme digests byte for byte.
    #[serde(default, skip_serializing_if = "player_name::is_false")]
    pub match_player_name: bool,
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
    /// Canonical personal completion, committed with this selected source branch.
    #[serde(default, skip_serializing_if = "is_false")]
    pub completes: bool,
    pub sources: Vec<String>,
}

fn is_false(value: &bool) -> bool {
    !*value
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
    /// Independent source programmes may share one canonical aggregate quest.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub program_key: Option<String>,
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
    pub player_name: Option<&'a str>,
    pub topic: i32,
    pub tracks: &'a BTreeMap<String, i64>,
    pub inventory: &'a NpcInventorySnapshot,
    pub now_epoch_seconds: i64,
    pub calendar_month: u8,
    pub premium_current: bool,
    pub level: u32,
    pub vocation: &'a str,
    pub sex: u8,
    pub outfits: &'a BTreeMap<u16, u8>,
}

impl NpcRuleInput<'_> {
    /// M17: preview and commit project the epoch supplied by their existing owners.
    pub fn utc_calendar(&self) -> Result<NpcUtcCalendar, NpcRuleError> {
        NpcUtcCalendar::from_epoch(self.now_epoch_seconds)
    }
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
    pub completes: bool,
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
    /// Resolve only programmes explicitly bound into this catalogue for this NPC.
    pub fn admitted_for_npc(
        &self,
        npc: &str,
        native: &super::QuestStateCatalogue,
    ) -> Result<Option<Self>, NpcRuleError> {
        admission::for_npc(self, npc, native)
    }
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

    /// Preserve the source's absent value while accepting the complete native
    /// track range shared with other quest owners. Source bytes keep their digest.
    pub fn with_native_bounds(
        mut self,
        native: &super::QuestStateCatalogue,
    ) -> Result<Self, NpcRuleError> {
        for program in &mut self.programs {
            let writes = bounds::literal_write_bounds(program);
            for track in &mut program.document.tracks {
                let definition = native
                    .track(&track.key)
                    .ok_or_else(|| NpcRuleError::MissingTrack(track.key.clone()))?;
                if definition.quest != track.quest
                    || (definition.initial != track.initial
                        && !(definition.initial == 0
                            && track.initial == -1
                            && native
                                .npc_legacy
                                .get(&track.quest)
                                .is_some_and(|(_, defaults)| defaults.get(&track.key) == Some(&0))))
                    || writes
                        .get(&track.key)
                        .is_some_and(|&(min, max)| definition.min > min || definition.max < max)
                {
                    return Err(NpcRuleError::Invalid(format!(
                        "native NPC track refinement: {}",
                        track.key
                    )));
                }
                track.min = definition.min;
                track.max = definition.max;
            }
        }
        Ok(self)
    }

    pub fn load_embedded() -> Result<Self, NpcRuleError> {
        Self::embedded().cloned()
    }

    /// Parse and validate immutable source content once before owner transactions.
    pub fn embedded() -> Result<&'static Self, NpcRuleError> {
        static SOURCE: std::sync::OnceLock<Result<NpcRuleCatalog, NpcRuleError>> =
            std::sync::OnceLock::new();
        SOURCE
            .get_or_init(|| Self::from_json_documents(NPC_RULE_PROGRAMS))
            .as_ref()
            .map_err(Clone::clone)
    }

    /// A..M programs with read-compatible legacy definitions, plus the already active Stag.
    pub fn runtime() -> Result<&'static Self, NpcRuleError> {
        static RUNTIME: std::sync::OnceLock<Result<NpcRuleCatalog, NpcRuleError>> =
            std::sync::OnceLock::new();
        RUNTIME
            .get_or_init(|| {
                let mut catalogue = Self::load_embedded()?;
                let supplemental = Self::from_json_documents(&[include_str!(
                    "../../../../../content/dialogues/quest-rules-supplemental/draconia.json"
                )])?;
                catalogue.programs.extend(supplemental.programs);
                validate::catalogue(&catalogue)?;
                catalogue.programs.retain(|p| {
                    p.document.program_key.as_deref() == Some(hidden_threats::PROGRAM)
                        || p.document.program_key.as_deref()
                            == Some("oteryn:quest-npc-program.first-dragon-calendar")
                        || p.document.program_key.as_deref()
                            == Some("oteryn:quest-npc-program.draconia")
                        || matches!(
                            p.document.quest.as_str(),
                            "oteryn:quest.a_father_s_burden_quest"
                                | "oteryn:quest.a_pirates_tail"
                                | "oteryn:quest.blood_brothers_quest"
                                | "oteryn:quest.dangerous_depths_quest"
                                | "oteryn:quest.elemental_spheres_quest"
                                | "oteryn:quest.bigfoot_s_burden_quest"
                                | "oteryn:quest.hero_of_rathleton_quest"
                                | "oteryn:quest.hot_cuisine_quest"
                                | "oteryn:quest.in_service_of_yalahar_quest"
                                | "oteryn:quest.marlin_trophy_quest"
                                | "oteryn:quest.the_dream_courts_quest"
                                | "oteryn:quest.the_hidden_city_of_beregar_quest"
                                | "oteryn:quest.the_order_of_the_stag_quest"
                                | "oteryn:quest.tower_defence_quest"
                        )
                });
                Ok(catalogue)
            })
            .as_ref()
            .map_err(Clone::clone)
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

    #[cfg(test)]
    pub(super) fn plan(
        &self,
        input: &NpcRuleInput<'_>,
    ) -> Result<Option<NpcRulePlan>, NpcRuleError> {
        self.plan_with_native(input, &BTreeMap::new())
    }

    /// Storage-only owner: every reached guard must be storage-owned, including
    /// guards of skipped branches. Unowned effects refuse the entire operation.
    pub fn plan_storage(
        &self,
        input: &NpcRuleInput<'_>,
    ) -> Result<Option<NpcRulePlan>, NpcRuleError> {
        evaluate::plan(self, input, &BTreeMap::new(), true)
    }

    /// Inventory recipes own exact carried items and UTC calendar facts. Other
    /// effect/guard families remain refused until their durable owner participates.
    pub fn plan_inventory(
        &self,
        input: &NpcRuleInput<'_>,
    ) -> Result<Option<NpcRulePlan>, NpcRuleError> {
        if self
            .programs
            .iter()
            .flat_map(|program| &program.document.rules)
            .filter(|rule| rule.npc == input.npc && rule.trigger == input.trigger)
            .flat_map(|rule| &rule.guards)
            .any(|guard| {
                !matches!(
                    guard,
                    NpcRuleGuard::Storage { .. }
                        | NpcRuleGuard::StorageTime { .. }
                        | NpcRuleGuard::Item { .. }
                        | NpcRuleGuard::CalendarMonth { .. }
                )
            })
        {
            return Err(NpcRuleError::MissingNativeResolution(
                "inventory recipe guard".into(),
            ));
        }
        let plan = evaluate::plan(self, input, &BTreeMap::new(), false)?;
        if plan.as_ref().is_some_and(|plan| {
            (plan.track_changes.is_empty()
                && plan.effects.iter().any(|effect| {
                    matches!(
                        effect,
                        NpcRuleEffect::TakeItem { .. } | NpcRuleEffect::GiveItem { .. }
                    )
                }))
                || plan.effects.iter().any(|effect| {
                    !matches!(
                        effect,
                        NpcRuleEffect::Set { .. }
                            | NpcRuleEffect::Add { .. }
                            | NpcRuleEffect::SetNowPlus { .. }
                            | NpcRuleEffect::SetTopic { .. }
                            | NpcRuleEffect::TakeItem { .. }
                            | NpcRuleEffect::GiveItem { .. }
                    ) && !matches!(effect, NpcRuleEffect::Achievement { name }
                        if plan.quest == crate::quest::hot_cuisine::QUEST && name == "Culinary Master")
                })
                || plan
                    .effects
                    .iter()
                    .filter(|effect| matches!(effect, NpcRuleEffect::GiveItem { .. }))
                    .count()
                    > 1
        }) {
            return Err(NpcRuleError::MissingNativeResolution(
                "inventory recipe effect".into(),
            ));
        }
        Ok(plan)
    }

    /// Tereban's fixed material hand-ins have an inventory and quest-XP owner.
    /// Other NPCs and effect families retain their storage-only boundary.
    pub fn plan_father_items(
        &self,
        input: &NpcRuleInput<'_>,
    ) -> Result<Option<NpcRulePlan>, NpcRuleError> {
        if input.npc != "oteryn:npc.tereban" {
            return Err(NpcRuleError::Invalid("not Tereban".into()));
        }
        if self
            .program("oteryn:quest.a_father_s_burden_quest")
            .is_none_or(|program| {
                program
                    .document
                    .rules
                    .iter()
                    .flat_map(|rule| &rule.guards)
                    .any(|guard| {
                        !matches!(
                            guard,
                            NpcRuleGuard::Storage { .. } | NpcRuleGuard::Item { .. }
                        )
                    })
            })
        {
            return Err(NpcRuleError::MissingNativeResolution(
                "Tereban guard".into(),
            ));
        }
        let plan = evaluate::plan(self, input, &BTreeMap::new(), false)?;
        if plan.as_ref().is_some_and(|plan| {
            plan.quest != "oteryn:quest.a_father_s_burden_quest"
                || plan.effects.iter().any(|effect| {
                    !matches!(
                        effect,
                        NpcRuleEffect::Set { .. }
                            | NpcRuleEffect::Add { .. }
                            | NpcRuleEffect::SetTopic { .. }
                            | NpcRuleEffect::TakeItem { .. }
                            | NpcRuleEffect::GiveItem { .. }
                            | NpcRuleEffect::Experience { .. }
                    )
                })
        }) {
            return Err(NpcRuleError::MissingNativeResolution(
                "Tereban effect".into(),
            ));
        }
        Ok(plan)
    }

    /// Native effects require a bounded result from the encounter/RNG owner. The
    /// key is the full selected rule key. Missing or malformed results refuse the
    /// whole plan; no unresolved obligation reaches the durable writer.
    #[cfg(test)]
    fn plan_with_native(
        &self,
        input: &NpcRuleInput<'_>,
        resolutions: &BTreeMap<String, Vec<NpcRuleEffect>>,
    ) -> Result<Option<NpcRulePlan>, NpcRuleError> {
        evaluate::plan(self, input, resolutions, false)
    }
}

/// Extend native bounds only for source writes; preserve existing absent values.
pub(super) fn extend(
    catalogue: super::QuestStateCatalogue,
    source_tracks: &BTreeMap<String, String>,
) -> Result<super::QuestStateCatalogue, String> {
    let source = NpcRuleCatalog::runtime().map_err(|e| e.to_string())?;
    let mut legacy = catalogue.npc_legacy.clone();
    for program in source.programs() {
        let quest = &program.document.quest;
        if quest != "oteryn:quest.the_order_of_the_stag_quest"
            && let Some(hash) = catalogue.definition_hash(quest)
        {
            legacy.insert(
                quest.clone(),
                (
                    hash,
                    catalogue
                        .tracks
                        .values()
                        .filter(|track| &track.quest == quest)
                        .map(|track| (track.key.clone(), track.initial))
                        .collect(),
                ),
            );
        }
    }
    let mut tracks = catalogue.tracks.clone();
    let mut transitions = catalogue.transitions.clone();
    for program in source.programs() {
        let writes = bounds::literal_write_bounds(program);
        for track in &program.document.tracks {
            if let Some(native) = tracks.get_mut(&track.key) {
                let original = track
                    .key
                    .strip_prefix("oteryn:")
                    .map(|tail| format!("canary:{tail}"));
                if native.quest != track.quest
                    || (native.initial != track.initial
                        && !(native.initial == 0
                            && source_tracks.get(&track.key) == original.as_ref()))
                {
                    return Err(format!("NPC source track conflict: {}", track.key));
                }
                // Keep the old catalogue's absent value for every existing native
                // consumer. Only source NPC planning uses its donor absent value.
                if let Some(&(min, max)) = writes.get(&track.key) {
                    native.min = native.min.min(min);
                    native.max = native.max.max(max);
                }
            } else {
                tracks.insert(
                    track.key.clone(),
                    super::QuestTrack {
                        key: track.key.clone(),
                        quest: track.quest.clone(),
                        initial: track.initial,
                        min: track.min,
                        max: track.max,
                    },
                );
            }
        }
        // An unexecutable definition marker binds every guard, reply, effect and
        // source reference into the existing quest definition hash.
        let first = program
            .document
            .tracks
            .first()
            .ok_or("NPC program has no tracks")?;
        let key = admission::binding_key(program);
        if transitions
            .insert(
                key.clone(),
                super::QuestTransition {
                    key,
                    quest: program.document.quest.clone(),
                    effects: vec![super::QuestEffect {
                        track: first.key.clone(),
                        from: super::QuestComparison::Any,
                        effect: super::QuestEffectKind::Computed,
                    }],
                    completes: false,
                    experience: None,
                    world_change: None,
                },
            )
            .is_some()
        {
            return Err("duplicate NPC source pin".into());
        }
    }
    let mut active = super::QuestStateCatalogue::new(
        catalogue.content_revision,
        tracks.into_values().collect(),
        transitions.into_values().collect(),
    )
    .map_err(|e| format!("{e:?}"))?;
    active.npc_legacy = legacy;
    Ok(active)
}

/// Only an exact registered predecessor retains its old absent-track values.
/// Existing rows, their values, pins and receipts are never rewritten.
pub fn legacy_defaults<'a>(
    catalogue: &'a super::QuestStateCatalogue,
    quest: &str,
    stored_hash: &[u8; 32],
) -> Option<&'a BTreeMap<String, i64>> {
    catalogue
        .npc_legacy
        .get(quest)
        .filter(|(hash, _)| hash == stored_hash)
        .map(|(_, defaults)| defaults)
        .or_else(|| {
            catalogue
                .predecessors
                .get(quest)?
                .iter()
                .find(|entry| &entry.hash == stored_hash)
                .map(|entry| &entry.defaults)
        })
}

#[cfg(test)]
mod multiple_item_rewards_tests;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod calendar_window_tests;

#[cfg(test)]
mod draconia_source_tests {
    use super::*;
    const DATA: &str =
        include_str!("../../../../../content/dialogues/quest-rules-supplemental/draconia.json");
    #[test]
    fn draconia_programme_rejects_source_substitution() -> Result<(), String> {
        NpcRuleCatalog::from_json_documents(&[DATA]).map_err(|e| format!("{e:?}"))?;
        assert!(
            NpcRuleCatalog::from_json_documents(&[&DATA.replace(
                "85016cc08856f49d94c507f43bc84f54801d3a2844946ae391015a0d8c84a052",
                "0000000000000000000000000000000000000000000000000000000000000000"
            )])
            .is_err()
        );
        Ok(())
    }
}
