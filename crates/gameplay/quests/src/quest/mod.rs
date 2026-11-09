//! The quest state catalogue (QUEST-STATE-0 §3, §4, §6 and §9; QUEST-STATE-1).
//!
//! [`QuestStateCatalogue`] holds, for one content revision, every quest's tracks (Oteryn key,
//! owner quest, initial value, `[min, max]`) and transitions (key, quest, at most
//! [`QUESTSTATE0_RL_02`] effects on that quest's own tracks, `completes`), and each quest's
//! `definition_hash` over its tracks and transitions only (a transition's `experience` reward
//! included), never its journal text (§6; QUEST-GATE-0 §5.5). Tests build it in code; [`loader`]
//! loads the lowered quest content into it (QUEST-LOWER-1).
//!
//! [`QuestStateCatalogue::evaluate`] is the pure §4 validation of one transition against the
//! values the writer read under lock: the closed `from` comparisons, then `SET`, checked `ADD`
//! and `SET_NOW` (database transaction time, never a node clock), each within the track's
//! bounds. It writes nothing; `durability::quest_state` owns the transaction.
//!
//! [`predicate`] is the read-only §7 predicate API over the session's copy (QUEST-PRED-1).
//!
//! This module depends on `std` and `sha2` only, and [`loader`] on `serde` and `serde_json`.

pub mod admission_binding;

// Source qualification only: the coordinator retained new NPC execution in
// Left out. These evaluators are not a second production conversation adapter.
pub mod authored_bindings;
pub mod barbarian_mead;
#[cfg(test)]
pub mod batch3_a;
pub mod carried_step;
pub mod children_puzzle;
pub mod classic_dialogue;
pub mod classic_talk;
pub mod completion_sets;
pub mod cults_of_tibia;
pub mod dark_trails;
pub mod deepling_boss_access;
pub mod elemental_spheres;
pub mod fire_feathered_native;
pub mod grave_danger;
pub mod grizzly_adams;
pub mod ground_stack;
pub mod hero_of_bounac;
pub mod hive_access;
pub mod isle_of_evil;
pub mod lions_rock;
pub mod liquid_black;
pub mod loader;
pub mod log;
pub mod map_marks;
pub mod monk_shrines;
pub mod native_kills;
pub mod native_rules;
pub mod npc_rules;
pub mod planegazer_finale;
mod quests_2_focus;
pub mod reward_callbacks;
pub mod rotten_blood;
pub mod schrodinger_island;
pub mod shared_switches;
pub mod shared_world_progress;
pub mod soul_war;
pub mod soul_war_dialogue;
pub mod source_deaths;
pub mod source_world_objects;
pub mod targuna_ram;
pub mod task_board;
pub mod threatened_dreams;
pub mod timers;
pub mod unnatural_selection;
pub mod world_callbacks;
pub mod wrath_sleeping_dragon;

use std::collections::{BTreeMap, BTreeSet};

use sha2::{Digest, Sha256};

pub mod predicate;

/// `QUESTSTATE0-RL-01`: tracks per Character.
pub const QUESTSTATE0_RL_01: usize = 4096;
/// `QUESTSTATE0-RL-02`: effects per atomic transition (migration 1600).
pub const QUESTSTATE0_RL_02: usize = 16;
/// `QUESTSTATE0-RL-04`: request binding bytes.
pub const QUESTSTATE0_RL_04: usize = 1024;
/// `QUESTSTATE0-RL-05`: quest states per Character.
pub const QUESTSTATE0_RL_05: usize = 1024;
/// `QUESTSTATE0-RL-06`: bytes per track, quest or transition key.
pub const QUESTSTATE0_RL_06: usize = 128;
/// `QUESTSTATE0-RL-07`: pending obligations per Character (`PENDING` and `WAITING_MIGRATION`).
pub const QUESTSTATE0_RL_07: usize = 64;
/// `QUESTSTATE0-RL-08`: the runtime copy of one online Character.
pub const QUESTSTATE0_RL_08_TRACKS: usize = 4096;
pub const QUESTSTATE0_RL_08_STATES: usize = 1024;
/// `QUESTGATE0-RL-05`: experience per transition (QUEST-GATE-0 §5.5, §12).
pub const QUESTGATE0_RL_05: i64 = 100_000_000;
/// `QUESTGATE0-RL-10`: pending quest XP obligations per Character (QUEST-GATE-0 §5.5).
pub const QUESTGATE0_RL_10: usize = 16;

const KEY_PREFIX: &str = "oteryn:";
const DEFINITION_HASH_VERSION: u8 = 2;

/// An Oteryn key (§13.2): `oteryn:` and a non-empty tail of `[A-Za-z0-9._:/-]`, at most
/// [`QUESTSTATE0_RL_06`] bytes. A source key never reaches the store.
#[must_use]
pub fn valid_quest_key(key: &str) -> bool {
    key.len() <= QUESTSTATE0_RL_06
        && key.strip_prefix(KEY_PREFIX).is_some_and(|tail| {
            !tail.is_empty()
                && tail
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"._:/-".contains(&byte))
        })
}

/// One quest-owned integer track (D34, §3). No stored row reads as `initial`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestTrack {
    pub key: String,
    pub quest: String,
    pub initial: i64,
    pub min: i64,
    pub max: i64,
}

/// The closed `from` comparisons of §4, on the locked value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestComparison {
    Any,
    Eq(i64),
    Ne(i64),
    Lt(i64),
    Le(i64),
    Gt(i64),
    Ge(i64),
    /// `in [a, b]`, both ends included.
    Between(i64, i64),
    /// `elapsed >= s`: database time minus the value, in seconds.
    ElapsedAtLeast(i64),
}

impl QuestComparison {
    fn holds(self, value: i64, now: i64) -> bool {
        match self {
            Self::Any => true,
            Self::Eq(other) => value == other,
            Self::Ne(other) => value != other,
            Self::Lt(other) => value < other,
            Self::Le(other) => value <= other,
            Self::Gt(other) => value > other,
            Self::Ge(other) => value >= other,
            Self::Between(low, high) => (low..=high).contains(&value),
            Self::ElapsedAtLeast(seconds) => {
                i128::from(now) - i128::from(value) >= i128::from(seconds)
            }
        }
    }

    fn encode(self, out: &mut Vec<u8>) {
        let (tag, values): (u8, &[i64]) = match &self {
            Self::Any => (0, &[]),
            Self::Eq(value) => (1, std::slice::from_ref(value)),
            Self::Ne(value) => (2, std::slice::from_ref(value)),
            Self::Lt(value) => (3, std::slice::from_ref(value)),
            Self::Le(value) => (4, std::slice::from_ref(value)),
            Self::Gt(value) => (5, std::slice::from_ref(value)),
            Self::Ge(value) => (6, std::slice::from_ref(value)),
            Self::Between(low, high) => {
                out.push(7);
                out.extend_from_slice(&low.to_be_bytes());
                out.extend_from_slice(&high.to_be_bytes());
                return;
            }
            Self::ElapsedAtLeast(value) => (8, std::slice::from_ref(value)),
        };
        out.push(tag);
        for value in values {
            out.extend_from_slice(&value.to_be_bytes());
        }
    }
}

/// The closed effect kinds of §4.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestEffectKind {
    Set(i64),
    /// Raise a locked value to this floor, preserving already higher progress.
    RaiseTo(i64),
    /// The same monotone operation pinned to its physical admission terms.
    RaiseToWithBinding {
        value: i64,
        binding: [u8; 32],
    },
    /// Set a value while pinning cross-owner admission terms (PC3 f673adfeb).
    SetWithBinding {
        value: i64,
        binding: [u8; 32],
    },
    /// Checked `i64` addition.
    Add(i64),
    /// Checked addition while pinning physical admission terms.
    AddWithBinding {
        delta: i64,
        binding: [u8; 32],
    },
    /// The database transaction time, in Unix seconds.
    SetNow,
    /// A source `computed: expression` effect with no closed kind yet: its transition is
    /// `NOT_SUPPORTED` until an amendment adds one (§4).
    Computed,
    /// A source-specific bounded rule over its complete locked snapshot.
    Native(native_rules::NativeRule),
    /// Preserve this track until another effect's original locked value reaches a threshold.
    /// Reused from questpc3 262618b3; the index remains inside this transition.
    SetWhenAtLeast {
        other_effect: u8,
        minimum: i64,
        value: i64,
    },
}

impl QuestEffectKind {
    fn encode(self, out: &mut Vec<u8>) {
        match self {
            Self::Set(value) => {
                out.push(1);
                out.extend_from_slice(&value.to_be_bytes());
            }
            Self::RaiseTo(value) => {
                out.push(9);
                out.extend_from_slice(&value.to_be_bytes());
            }
            Self::RaiseToWithBinding { value, binding } => {
                out.push(10);
                out.extend_from_slice(&value.to_be_bytes());
                out.extend_from_slice(&binding);
            }
            Self::SetWithBinding { value, binding } => {
                out.push(7);
                out.extend_from_slice(&value.to_be_bytes());
                out.extend_from_slice(&binding);
            }
            Self::Add(value) => {
                out.push(2);
                out.extend_from_slice(&value.to_be_bytes());
            }
            Self::AddWithBinding { delta, binding } => {
                out.push(8);
                out.extend_from_slice(&delta.to_be_bytes());
                out.extend_from_slice(&binding);
            }
            Self::SetNow => out.push(3),
            Self::Computed => out.push(4),
            Self::Native(rule) => {
                out.push(5);
                rule.encode(out);
            }
            Self::SetWhenAtLeast {
                other_effect,
                minimum,
                value,
            } => {
                out.extend_from_slice(&[11, other_effect]);
                out.extend_from_slice(&minimum.to_be_bytes());
                out.extend_from_slice(&value.to_be_bytes());
            }
        }
    }
}

/// One effect of a transition: `{track_key, from, effect}` (§4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestEffect {
    pub track: String,
    pub from: QuestComparison,
    pub effect: QuestEffectKind,
}

/// One named transition (D35, §4): effects on tracks of its own quest only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestTransition {
    pub key: String,
    pub quest: String,
    pub effects: Vec<QuestEffect>,
    pub completes: bool,
    /// `experience: n` (QUEST-GATE-0 §5.5): `1 <= n <=` [`QUESTGATE0_RL_05`], awarded through a
    /// quest XP obligation written by the committing transition. In the definition hash: a reward
    /// edit is a `REVISION_MISMATCH` for players in progress, like any transition edit (§6).
    pub experience: Option<i64>,
    /// Current global occurrence required by this transition. Rechecked under the durable fence.
    pub world_change: Option<String>,
}

/// Why a transition writes nothing (§4 "Validation").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestRefusal {
    /// A `from` comparison does not hold on the locked value.
    StageMismatch,
    /// The quest state is pinned to another definition hash (§6).
    RevisionMismatch,
    /// A result leaves the track's bounds, or `ADD` overflows.
    OutOfRange,
    /// A `computed` effect has no closed kind yet.
    NotSupported,
    /// A first track or first quest state over `QUESTSTATE0-RL-01` or `-05`.
    CapacityExceeded,
}

impl QuestRefusal {
    /// The result code an obligation keeps (§5.4).
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::StageMismatch => "STAGE_MISMATCH",
            Self::RevisionMismatch => "REVISION_MISMATCH",
            Self::OutOfRange => "OUT_OF_RANGE",
            Self::NotSupported => "NOT_SUPPORTED",
            Self::CapacityExceeded => "CAPACITY_EXCEEDED",
        }
    }
}

/// One effect's committed change: the value before and after.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestTrackChange {
    pub track: String,
    pub before: i64,
    pub after: i64,
}

/// A catalogue that breaks a §4 or §9 rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuestCatalogueError {
    InvalidKey(String),
    DuplicateKey(String),
    InvalidBounds(String),
    UnknownTrack(String),
    ForeignTrack(String),
    TooManyEffects(String),
    DuplicateEffectTrack(String),
    InvalidEffect(String),
    /// `experience` is zero, negative or over `QUESTGATE0-RL-05`: the quest is not admitted.
    InvalidExperience(String),
    InvalidContentRevision,
}

/// The tracks and transitions of one content revision, with each quest's definition hash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestStateCatalogue {
    completion_sets: BTreeMap<String, completion_sets::CompletionSet>,
    content_revision: String,
    tracks: BTreeMap<String, QuestTrack>,
    transitions: BTreeMap<String, QuestTransition>,
    hashes: BTreeMap<String, [u8; 32]>,
    authored_bindings: Option<std::sync::Arc<authored_bindings::RuntimeBindings>>,
    npc_legacy: BTreeMap<String, ([u8; 32], BTreeMap<String, i64>)>,
    predecessors: BTreeMap<String, Vec<NpcDefinitionPredecessor>>,
    native_recipe_evidence: Option<std::sync::Arc<authored_bindings::embedded::RecipeEvidence>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NpcDefinitionPredecessor {
    hash: [u8; 32],
    defaults: BTreeMap<String, i64>,
}

impl QuestStateCatalogue {
    /// Check every key, bound and effect, and compute each quest's definition hash.
    pub fn new(
        content_revision: impl Into<String>,
        tracks: Vec<QuestTrack>,
        transitions: Vec<QuestTransition>,
    ) -> Result<Self, QuestCatalogueError> {
        use QuestCatalogueError as E;
        let content_revision = content_revision.into();
        if !valid_revision(&content_revision) {
            return Err(E::InvalidContentRevision);
        }
        let mut track_map = BTreeMap::new();
        for track in tracks {
            for key in [&track.key, &track.quest] {
                if !valid_quest_key(key) {
                    return Err(E::InvalidKey(key.clone()));
                }
            }
            if !(track.min <= track.initial && track.initial <= track.max) {
                return Err(E::InvalidBounds(track.key));
            }
            if track_map.contains_key(&track.key) {
                return Err(E::DuplicateKey(track.key));
            }
            track_map.insert(track.key.clone(), track);
        }
        let mut transition_map = BTreeMap::new();
        for transition in transitions {
            for key in [&transition.key, &transition.quest] {
                if !valid_quest_key(key) {
                    return Err(E::InvalidKey(key.clone()));
                }
            }
            if transition.effects.len() > QUESTSTATE0_RL_02 {
                return Err(E::TooManyEffects(transition.key));
            }
            if transition.effects.is_empty()
                && !transition.completes
                && !(((transition.key == "oteryn:quest-transition/worldchange/benevola/handover"
                    && transition.quest == "oteryn:quest/worldchange/benevola")
                    || transition.key.starts_with("oteryn:quest-reward/"))
                    && transition.experience.is_none())
            {
                return Err(E::InvalidEffect(transition.key));
            }
            if transition.world_change.as_ref().is_some_and(|key| {
                key.is_empty()
                    || key.len() > 100
                    || !key
                        .bytes()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_')
            }) {
                return Err(E::InvalidEffect(transition.key));
            }
            if transition
                .experience
                .is_some_and(|experience| !(1..=QUESTGATE0_RL_05).contains(&experience))
            {
                return Err(E::InvalidExperience(transition.key));
            }
            if transition.effects.iter().any(|effect| {
                matches!(effect.effect, QuestEffectKind::Native(rule)
                    if !rule.valid_effects(&transition.quest, &transition.effects))
            }) {
                return Err(E::InvalidEffect(transition.key));
            }
            let mut seen = BTreeSet::new();
            for (index, effect) in transition.effects.iter().enumerate() {
                let Some(track) = track_map.get(&effect.track) else {
                    return Err(E::UnknownTrack(effect.track.clone()));
                };
                if track.quest != transition.quest {
                    return Err(E::ForeignTrack(effect.track.clone()));
                }
                if !seen.insert(effect.track.as_str()) {
                    return Err(E::DuplicateEffectTrack(effect.track.clone()));
                }
                let comparison_ok = match effect.from {
                    QuestComparison::Between(low, high) => low <= high,
                    QuestComparison::ElapsedAtLeast(seconds) => seconds >= 0,
                    _ => true,
                };
                let effect_ok = match effect.effect {
                    QuestEffectKind::Set(value)
                    | QuestEffectKind::SetWithBinding { value, .. }
                    | QuestEffectKind::RaiseTo(value)
                    | QuestEffectKind::RaiseToWithBinding { value, .. } => {
                        (track.min..=track.max).contains(&value)
                    }
                    QuestEffectKind::SetWhenAtLeast {
                        other_effect,
                        value,
                        ..
                    } => {
                        usize::from(other_effect) < transition.effects.len()
                            && usize::from(other_effect) != index
                            && (track.min..=track.max).contains(&value)
                    }

                    _ => true,
                };
                if !comparison_ok || !effect_ok {
                    return Err(E::InvalidEffect(transition.key.clone()));
                }
            }
            if transition_map.contains_key(&transition.key) {
                return Err(E::DuplicateKey(transition.key));
            }
            transition_map.insert(transition.key.clone(), transition);
        }
        let quests: BTreeSet<&String> = track_map
            .values()
            .map(|track| &track.quest)
            .chain(transition_map.values().map(|transition| &transition.quest))
            .collect();
        let hashes = quests
            .into_iter()
            .map(|quest| {
                (
                    quest.clone(),
                    definition_hash(quest, &track_map, &transition_map),
                )
            })
            .collect();
        Ok(Self {
            content_revision,
            tracks: track_map,
            transitions: transition_map,
            hashes,
            completion_sets: BTreeMap::new(),
            npc_legacy: BTreeMap::new(),
            predecessors: BTreeMap::new(),
            authored_bindings: None,
            native_recipe_evidence: None,
        })
    }

    /// This catalogue with one more code-built quest's tracks and transitions, every rule and
    /// hash checked again. A key or quest this catalogue already holds is a `DuplicateKey`, so
    /// the hash of every quest already present is unchanged.
    pub fn with_quest(
        &self,
        tracks: Vec<QuestTrack>,
        transitions: Vec<QuestTransition>,
    ) -> Result<Self, QuestCatalogueError> {
        let quests = tracks.iter().map(|track| &track.quest);
        let mut quests = quests.chain(transitions.iter().map(|transition| &transition.quest));
        if let Some(quest) = quests.find(|quest| self.hashes.contains_key(*quest)) {
            return Err(QuestCatalogueError::DuplicateKey(quest.clone()));
        }
        let mut extended = Self::new(
            self.content_revision.clone(),
            self.tracks.values().cloned().chain(tracks).collect(),
            self.transitions
                .values()
                .cloned()
                .chain(transitions)
                .collect(),
        )?;
        extended.hashes.extend(self.hashes.clone());
        extended.completion_sets = self.completion_sets.clone();
        extended.npc_legacy = self.npc_legacy.clone();
        extended.predecessors = self.predecessors.clone();
        extended.authored_bindings = self.authored_bindings.clone();
        extended.native_recipe_evidence = self.native_recipe_evidence.clone();
        Ok(extended)
    }

    /// A catalogue with no quest: every transition request is unknown.
    #[must_use]
    pub fn empty(content_revision: impl Into<String>) -> Option<Self> {
        Self::new(content_revision, Vec::new(), Vec::new()).ok()
    }

    pub fn authored_world_callbacks(&self) -> Option<&[world_callbacks::Callback]> {
        self.authored_bindings
            .as_ref()
            .map(|runtime| runtime.world.as_slice())
    }

    pub fn authored_bindings(&self) -> Option<&authored_bindings::Bindings> {
        self.authored_bindings
            .as_ref()
            .map(|runtime| &runtime.source)
    }

    pub fn content_revision(&self) -> &str {
        &self.content_revision
    }

    #[must_use]
    pub fn track(&self, key: &str) -> Option<&QuestTrack> {
        self.tracks.get(key)
    }

    #[must_use]
    pub fn transition(&self, key: &str) -> Option<&QuestTransition> {
        self.transitions.get(key)
    }

    /// The hash over the quest's tracks and transitions only (§6).
    #[must_use]
    pub fn definition_hash(&self, quest: &str) -> Option<[u8; 32]> {
        self.hashes.get(quest).copied()
    }

    /// Validate `transition` against the stored values (`stored` lacks a track that has no
    /// row, which then reads as its initial value) at database time `now` (Unix seconds). On
    /// success, one change per effect in effect order. Order of refusal: `NOT_SUPPORTED`, then
    /// `STAGE_MISMATCH`, then `OUT_OF_RANGE`; every `from` reads the value before any effect.
    pub fn evaluate(
        &self,
        transition: &QuestTransition,
        stored: &BTreeMap<String, i64>,
        now: i64,
    ) -> Result<Vec<QuestTrackChange>, QuestRefusal> {
        self.evaluate_with_roll(transition, stored, now, None)
    }

    /// Only the fenced writer supplies a fresh database roll after receipt lookup.
    pub fn evaluate_with_roll(
        &self,
        transition: &QuestTransition,
        stored: &BTreeMap<String, i64>,
        now: i64,
        roll: Option<u8>,
    ) -> Result<Vec<QuestTrackChange>, QuestRefusal> {
        if transition.effects.iter().enumerate().any(|(index,effect)| {
            effect.effect==QuestEffectKind::Computed || matches!(effect.effect,
                QuestEffectKind::SetWhenAtLeast {other_effect,..}
                    if usize::from(other_effect)==index || usize::from(other_effect)>=transition.effects.len())
        })
        {
            return Err(QuestRefusal::NotSupported);
        }
        let mut current = Vec::with_capacity(transition.effects.len());
        for effect in &transition.effects {
            let track = self
                .tracks
                .get(&effect.track)
                .ok_or(QuestRefusal::NotSupported)?;
            let value = stored.get(&effect.track).copied().unwrap_or(track.initial);
            if !effect.from.holds(value, now) {
                return Err(QuestRefusal::StageMismatch);
            }
            current.push((track, value));
        }
        let native = transition
            .effects
            .iter()
            .find_map(|effect| match effect.effect {
                QuestEffectKind::Native(rule) => Some(rule),
                _ => None,
            });
        let native_after = if let Some(rule) = native {
            if !rule.valid_effects(&transition.quest, &transition.effects) {
                return Err(QuestRefusal::NotSupported);
            }
            let snapshot = current
                .iter()
                .map(|(track, value)| (track.key.clone(), *value))
                .collect();
            Some(rule.evaluate_with_roll(&snapshot, roll)?)
        } else {
            None
        };
        transition
            .effects
            .iter()
            .zip(current.iter().copied())
            .map(|(effect, (track, before))| {
                let after = match effect.effect {
                    QuestEffectKind::RaiseTo(value)
                    | QuestEffectKind::RaiseToWithBinding { value, .. } => Some(before.max(value)),
                    QuestEffectKind::Set(value) | QuestEffectKind::SetWithBinding { value, .. } => {
                        Some(value)
                    }
                    QuestEffectKind::Add(delta) | QuestEffectKind::AddWithBinding { delta, .. } => {
                        before.checked_add(delta)
                    }
                    QuestEffectKind::SetNow => Some(now),
                    QuestEffectKind::Computed => None,
                    QuestEffectKind::SetWhenAtLeast {
                        other_effect,
                        minimum,
                        value,
                    } => current
                        .get(usize::from(other_effect))
                        .map(|(_, observed)| if *observed >= minimum { value } else { before }),
                    QuestEffectKind::Native(_) => native_after
                        .as_ref()
                        .and_then(|after| after.get(&effect.track).copied()),
                }
                .filter(|after| (track.min..=track.max).contains(after))
                .ok_or(QuestRefusal::OutOfRange)?;
                Ok(QuestTrackChange {
                    track: effect.track.clone(),
                    before,
                    after,
                })
            })
            .collect()
    }
}

/// A content revision spelling, as the Character interpretation revisions (`0005`).
fn valid_revision(value: &str) -> bool {
    let mut bytes = value.bytes();
    value.len() <= 128
        && bytes
            .next()
            .is_some_and(|byte| byte.is_ascii_alphanumeric())
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || b"._:-".contains(&byte))
}

fn push_text(out: &mut Vec<u8>, text: &str) {
    // Keys are at most 128 bytes (RL-06), so a u16 length frames them exactly.
    out.extend_from_slice(&u16::try_from(text.len()).unwrap_or(u16::MAX).to_be_bytes());
    out.extend_from_slice(text.as_bytes());
}

/// SHA-256 over the quest key, its tracks and its transitions in key order, each with its
/// `experience` reward (§6; version 2): never journal text, so only text edits never block players
/// in progress.
fn definition_hash(
    quest: &str,
    tracks: &BTreeMap<String, QuestTrack>,
    transitions: &BTreeMap<String, QuestTransition>,
) -> [u8; 32] {
    let mut out = b"oteryn:quest-definition\0".to_vec();
    out.push(DEFINITION_HASH_VERSION);
    push_text(&mut out, quest);
    let owned: Vec<&QuestTrack> = tracks.values().filter(|t| t.quest == quest).collect();
    out.extend_from_slice(&u32::try_from(owned.len()).unwrap_or(u32::MAX).to_be_bytes());
    for track in owned {
        push_text(&mut out, &track.key);
        for value in [track.initial, track.min, track.max] {
            out.extend_from_slice(&value.to_be_bytes());
        }
    }
    let owned: Vec<&QuestTransition> = transitions
        .values()
        .filter(|transition| transition.quest == quest)
        .collect();
    out.extend_from_slice(&u32::try_from(owned.len()).unwrap_or(u32::MAX).to_be_bytes());
    for transition in owned {
        push_text(&mut out, &transition.key);
        out.push(u8::from(transition.completes));
        out.push(u8::try_from(transition.effects.len()).unwrap_or(u8::MAX));
        for effect in &transition.effects {
            push_text(&mut out, &effect.track);
            effect.from.encode(&mut out);
            effect.effect.encode(&mut out);
        }
        match transition.experience {
            None => out.push(0),
            Some(experience) => {
                out.push(1);
                out.extend_from_slice(&experience.to_be_bytes());
            }
        }
    }
    for transition in transitions
        .values()
        .filter(|transition| transition.quest == quest && transition.world_change.is_some())
    {
        out.extend_from_slice(b"world-change-gate/v1");
        push_text(&mut out, &transition.key);
        if let Some(change) = &transition.world_change {
            push_text(&mut out, change);
        }
    }
    Sha256::digest(&out).into()
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    const QUEST: &str = "oteryn:quest/test.rats";
    const STAGE: &str = "oteryn:quest-progress/test.rats.stage";
    const TIMER: &str = "oteryn:quest-progress/test.rats.timer";

    fn track(key: &str, initial: i64, min: i64, max: i64) -> QuestTrack {
        QuestTrack {
            key: key.into(),
            quest: QUEST.into(),
            initial,
            min,
            max,
        }
    }

    fn effect(track: &str, from: QuestComparison, effect: QuestEffectKind) -> QuestEffect {
        QuestEffect {
            track: track.into(),
            from,
            effect,
        }
    }

    fn transition(key: &str, effects: Vec<QuestEffect>, completes: bool) -> QuestTransition {
        QuestTransition {
            world_change: None,
            key: key.into(),
            quest: QUEST.into(),
            effects,
            completes,
            experience: None,
        }
    }

    fn catalogue(transitions: Vec<QuestTransition>) -> QuestStateCatalogue {
        QuestStateCatalogue::new(
            "content-1",
            vec![track(STAGE, -1, -1, 10), track(TIMER, 0, 0, i64::MAX)],
            transitions,
        )
        .expect("catalogue")
    }

    fn run(
        catalogue: &QuestStateCatalogue,
        key: &str,
        stored: &[(&str, i64)],
        now: i64,
    ) -> Result<Vec<QuestTrackChange>, QuestRefusal> {
        let stored = stored
            .iter()
            .map(|(key, value)| ((*key).to_owned(), *value))
            .collect();
        let transition = catalogue.transition(key).expect("transition");
        catalogue.evaluate(transition, &stored, now)
    }

    #[test]
    fn keys_are_oteryn_keys_within_rl_06() {
        assert!(valid_quest_key(STAGE));
        assert!(valid_quest_key(&format!("oteryn:{}", "a".repeat(121))));
        for key in [
            "oteryn:".to_owned(),
            "canary:quest-progress/1".to_owned(),
            "oteryn:bad key".to_owned(),
            format!("oteryn:{}", "a".repeat(122)),
        ] {
            assert!(!valid_quest_key(&key), "{key}");
        }
    }

    #[test]
    fn the_catalogue_refuses_foreign_unknown_duplicate_and_oversized_effects() {
        use QuestCatalogueError as E;
        let set = |track: &str| effect(track, QuestComparison::Any, QuestEffectKind::Set(1));
        let other = QuestTrack {
            quest: "oteryn:quest/test.other".into(),
            ..track("oteryn:quest-progress/other", 0, 0, 1)
        };
        let build = |transition: QuestTransition| {
            QuestStateCatalogue::new(
                "content-1",
                vec![track(STAGE, 0, -1, 10), other.clone()],
                vec![transition],
            )
        };
        assert!(matches!(
            build(transition(
                "oteryn:t/1",
                vec![set("oteryn:quest-progress/other")],
                false
            )),
            Err(E::ForeignTrack(_))
        ));
        assert!(matches!(
            build(transition(
                "oteryn:t/1",
                vec![set("oteryn:quest-progress/none")],
                false
            )),
            Err(E::UnknownTrack(_))
        ));
        assert!(matches!(
            build(transition(
                "oteryn:t/1",
                vec![set(STAGE), set(STAGE)],
                false
            )),
            Err(E::DuplicateEffectTrack(_))
        ));
        assert!(matches!(
            build(transition(
                "oteryn:t/1",
                vec![set(STAGE); QUESTSTATE0_RL_02 + 1],
                false
            )),
            Err(E::TooManyEffects(_))
        ));
        assert!(matches!(
            build(transition(
                "oteryn:t/1",
                vec![effect(
                    STAGE,
                    QuestComparison::Any,
                    QuestEffectKind::Set(11)
                )],
                false
            )),
            Err(E::InvalidEffect(_))
        ));
        assert!(matches!(
            build(transition("oteryn:t/1", vec![], false)),
            Err(E::InvalidEffect(_))
        ));
        assert!(matches!(
            QuestStateCatalogue::new("content-1", vec![track(STAGE, 11, -1, 10)], vec![]),
            Err(E::InvalidBounds(_))
        ));
        assert!(build(transition("oteryn:t/1", vec![], true)).is_ok());
        // QUESTGATE0-RL-05: experience is 1 to 100,000,000; anything else is not admitted.
        let rewarded = |experience| QuestTransition {
            world_change: None,
            experience: Some(experience),
            ..transition("oteryn:t/1", vec![set(STAGE)], false)
        };
        for experience in [1, QUESTGATE0_RL_05] {
            assert!(build(rewarded(experience)).is_ok(), "{experience}");
        }
        for experience in [0, -1, QUESTGATE0_RL_05 + 1, i64::MIN, i64::MAX] {
            assert!(
                matches!(build(rewarded(experience)), Err(E::InvalidExperience(_))),
                "{experience}"
            );
        }
    }

    #[test]
    fn every_comparison_and_effect_kind() {
        let catalogue = catalogue(vec![
            transition(
                "oteryn:t/start",
                vec![
                    effect(STAGE, QuestComparison::Eq(-1), QuestEffectKind::Set(1)),
                    effect(TIMER, QuestComparison::Any, QuestEffectKind::SetNow),
                ],
                false,
            ),
            transition(
                "oteryn:t/step",
                vec![
                    effect(
                        STAGE,
                        QuestComparison::Between(1, 3),
                        QuestEffectKind::Add(2),
                    ),
                    effect(
                        TIMER,
                        QuestComparison::ElapsedAtLeast(60),
                        QuestEffectKind::SetNow,
                    ),
                ],
                true,
            ),
        ]);
        let start = run(&catalogue, "oteryn:t/start", &[], 1_000).expect("start");
        assert_eq!(
            start,
            vec![
                QuestTrackChange {
                    track: STAGE.into(),
                    before: -1,
                    after: 1
                },
                QuestTrackChange {
                    track: TIMER.into(),
                    before: 0,
                    after: 1_000
                },
            ]
        );
        assert_eq!(
            run(&catalogue, "oteryn:t/start", &[(STAGE, 1)], 1_000),
            Err(QuestRefusal::StageMismatch)
        );
        let stored = [(STAGE, 1), (TIMER, 1_000)];
        assert_eq!(
            run(&catalogue, "oteryn:t/step", &stored, 1_059),
            Err(QuestRefusal::StageMismatch),
            "59 seconds have not elapsed"
        );
        assert_eq!(
            run(&catalogue, "oteryn:t/step", &stored, 1_060).expect("elapsed")[0].after,
            3
        );
        assert_eq!(
            run(
                &catalogue,
                "oteryn:t/step",
                &[(STAGE, 9), (TIMER, 0)],
                1_060
            ),
            Err(QuestRefusal::StageMismatch)
        );
        for (comparison, value, holds) in [
            (QuestComparison::Ne(1), 1, false),
            (QuestComparison::Ne(1), 2, true),
            (QuestComparison::Lt(1), 0, true),
            (QuestComparison::Lt(1), 1, false),
            (QuestComparison::Le(1), 1, true),
            (QuestComparison::Gt(1), 1, false),
            (QuestComparison::Gt(1), 2, true),
            (QuestComparison::Ge(1), 1, true),
            (QuestComparison::Ge(1), 0, false),
            (QuestComparison::ElapsedAtLeast(0), i64::MIN, true),
            (QuestComparison::ElapsedAtLeast(i64::MAX), i64::MIN, true),
        ] {
            assert_eq!(comparison.holds(value, 0), holds, "{comparison:?} {value}");
        }
    }

    #[test]
    fn results_stay_in_bounds_and_add_is_checked() {
        let catalogue = catalogue(vec![
            transition(
                "oteryn:t/add",
                vec![effect(STAGE, QuestComparison::Any, QuestEffectKind::Add(5))],
                false,
            ),
            transition(
                "oteryn:t/overflow",
                vec![effect(TIMER, QuestComparison::Any, QuestEffectKind::Add(1))],
                false,
            ),
            transition(
                "oteryn:t/computed",
                vec![
                    effect(STAGE, QuestComparison::Eq(7), QuestEffectKind::Set(1)),
                    effect(TIMER, QuestComparison::Any, QuestEffectKind::Computed),
                ],
                false,
            ),
            transition(
                "oteryn:t/now",
                vec![effect(STAGE, QuestComparison::Any, QuestEffectKind::SetNow)],
                false,
            ),
        ]);
        assert_eq!(
            run(&catalogue, "oteryn:t/add", &[(STAGE, 5)], 0).expect("at max")[0].after,
            10
        );
        assert_eq!(
            run(&catalogue, "oteryn:t/add", &[(STAGE, 6)], 0),
            Err(QuestRefusal::OutOfRange)
        );
        assert_eq!(
            run(&catalogue, "oteryn:t/overflow", &[(TIMER, i64::MAX)], 0),
            Err(QuestRefusal::OutOfRange)
        );
        assert_eq!(
            run(&catalogue, "oteryn:t/computed", &[], 0),
            Err(QuestRefusal::NotSupported),
            "NOT_SUPPORTED comes before the stage check"
        );
        assert_eq!(
            run(&catalogue, "oteryn:t/now", &[], 1_522_018_605),
            Err(QuestRefusal::OutOfRange)
        );
    }

    #[test]
    fn the_definition_hash_covers_tracks_and_transitions_of_its_quest_only() {
        let base = catalogue(vec![transition(
            "oteryn:t/start",
            vec![effect(
                STAGE,
                QuestComparison::Eq(-1),
                QuestEffectKind::Set(1),
            )],
            false,
        )]);
        let hash = base.definition_hash(QUEST).expect("hash");
        assert_eq!(
            catalogue(vec![transition(
                "oteryn:t/start",
                vec![effect(
                    STAGE,
                    QuestComparison::Eq(-1),
                    QuestEffectKind::Set(1)
                )],
                false,
            )])
            .definition_hash(QUEST),
            Some(hash),
            "deterministic"
        );
        for changed in [
            transition(
                "oteryn:t/start",
                vec![effect(
                    STAGE,
                    QuestComparison::Eq(0),
                    QuestEffectKind::Set(1),
                )],
                false,
            ),
            transition(
                "oteryn:t/start",
                vec![effect(
                    STAGE,
                    QuestComparison::Eq(-1),
                    QuestEffectKind::Set(2),
                )],
                false,
            ),
            transition(
                "oteryn:t/start",
                vec![effect(
                    STAGE,
                    QuestComparison::Eq(-1),
                    QuestEffectKind::Set(1),
                )],
                true,
            ),
            transition(
                "oteryn:t/begin",
                vec![effect(
                    STAGE,
                    QuestComparison::Eq(-1),
                    QuestEffectKind::Set(1),
                )],
                false,
            ),
        ] {
            assert_ne!(catalogue(vec![changed]).definition_hash(QUEST), Some(hash));
        }
        let bounds = QuestStateCatalogue::new(
            "content-1",
            vec![track(STAGE, -1, -1, 11), track(TIMER, 0, 0, i64::MAX)],
            vec![],
        )
        .expect("catalogue");
        assert_ne!(bounds.definition_hash(QUEST), Some(hash));
        // Another quest and another content revision leave this quest's hash unchanged.
        let other = QuestStateCatalogue::new(
            "content-2",
            vec![
                track(STAGE, -1, -1, 10),
                track(TIMER, 0, 0, i64::MAX),
                QuestTrack {
                    quest: "oteryn:quest/test.other".into(),
                    ..track("oteryn:quest-progress/other", 0, 0, 1)
                },
            ],
            vec![transition(
                "oteryn:t/start",
                vec![effect(
                    STAGE,
                    QuestComparison::Eq(-1),
                    QuestEffectKind::Set(1),
                )],
                false,
            )],
        )
        .expect("catalogue");
        assert_eq!(other.definition_hash(QUEST), Some(hash));
        assert_eq!(other.content_revision(), "content-2");
        // A reward is part of the transition (§6; QUEST-GATE-0 §5.5): adding or changing one
        // changes the hash, so an in-progress Character is refused, never awarded the new amount.
        let rewarded = catalogue(vec![QuestTransition {
            world_change: None,
            experience: Some(500),
            ..transition(
                "oteryn:t/start",
                vec![effect(
                    STAGE,
                    QuestComparison::Eq(-1),
                    QuestEffectKind::Set(1),
                )],
                false,
            )
        }]);
        let rewarded_hash = rewarded.definition_hash(QUEST).expect("hash");
        assert_ne!(rewarded_hash, hash);
        let rewarded_more = catalogue(vec![QuestTransition {
            world_change: None,
            experience: Some(501),
            ..transition(
                "oteryn:t/start",
                vec![effect(
                    STAGE,
                    QuestComparison::Eq(-1),
                    QuestEffectKind::Set(1),
                )],
                false,
            )
        }]);
        assert_ne!(rewarded_more.definition_hash(QUEST), Some(rewarded_hash));
        assert_ne!(rewarded_more.definition_hash(QUEST), Some(hash));
        assert_eq!(
            rewarded
                .transition("oteryn:t/start")
                .and_then(|transition| transition.experience),
            Some(500)
        );
        assert_eq!(QuestRefusal::RevisionMismatch.code(), "REVISION_MISMATCH");
        assert!(QuestStateCatalogue::empty("content-1").is_some());
        assert!(QuestStateCatalogue::empty("bad revision").is_none());
    }
}
