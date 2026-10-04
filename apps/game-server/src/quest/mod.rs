//! The quest state catalogue (QUEST-STATE-0 §3, §4, §6 and §9; QUEST-STATE-1).
//!
//! [`QuestStateCatalogue`] holds, for one content revision, every quest's tracks (Oteryn key,
//! owner quest, initial value, `[min, max]`) and transitions (key, quest, at most
//! [`QUESTSTATE0_RL_02`] effects on that quest's own tracks, `completes`), and each quest's
//! `definition_hash` over its tracks and transitions only, never its journal text (§6). Tests
//! build it in code; QUEST-LOWER-1 adds the content loader.
//!
//! [`QuestStateCatalogue::evaluate`] is the pure §4 validation of one transition against the
//! values the writer read under lock: the closed `from` comparisons, then `SET`, checked `ADD`
//! and `SET_NOW` (database transaction time, never a node clock), each within the track's
//! bounds. It writes nothing; `durability::quest_state` owns the transaction.
//!
//! [`predicate`] is the read-only §7 predicate API over the session's copy (QUEST-PRED-1).
//!
//! This module depends on `std` and `sha2` only, so every crate that path-loads `durability`
//! also compiles it unchanged.

use std::collections::{BTreeMap, BTreeSet};

use sha2::{Digest, Sha256};

pub mod predicate;

/// `QUESTSTATE0-RL-01`: tracks per Character.
pub const QUESTSTATE0_RL_01: usize = 4096;
/// `QUESTSTATE0-RL-02`: effects per transition.
pub const QUESTSTATE0_RL_02: usize = 8;
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

const KEY_PREFIX: &str = "oteryn:";
const DEFINITION_HASH_VERSION: u8 = 1;

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
    /// Checked `i64` addition.
    Add(i64),
    /// The database transaction time, in Unix seconds.
    SetNow,
    /// A source `computed: expression` effect with no closed kind yet: its transition is
    /// `NOT_SUPPORTED` until an amendment adds one (§4).
    Computed,
}

impl QuestEffectKind {
    fn encode(self, out: &mut Vec<u8>) {
        match self {
            Self::Set(value) => {
                out.push(1);
                out.extend_from_slice(&value.to_be_bytes());
            }
            Self::Add(value) => {
                out.push(2);
                out.extend_from_slice(&value.to_be_bytes());
            }
            Self::SetNow => out.push(3),
            Self::Computed => out.push(4),
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
    InvalidContentRevision,
}

/// The tracks and transitions of one content revision, with each quest's definition hash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestStateCatalogue {
    content_revision: String,
    tracks: BTreeMap<String, QuestTrack>,
    transitions: BTreeMap<String, QuestTransition>,
    hashes: BTreeMap<String, [u8; 32]>,
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
            if transition.effects.is_empty() && !transition.completes {
                return Err(E::InvalidEffect(transition.key));
            }
            let mut seen = BTreeSet::new();
            for effect in &transition.effects {
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
                    QuestEffectKind::Set(value) => (track.min..=track.max).contains(&value),
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
        })
    }

    /// A catalogue with no quest: every transition request is unknown.
    #[must_use]
    pub fn empty(content_revision: impl Into<String>) -> Option<Self> {
        Self::new(content_revision, Vec::new(), Vec::new()).ok()
    }

    #[must_use]
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
        if transition
            .effects
            .iter()
            .any(|effect| effect.effect == QuestEffectKind::Computed)
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
        transition
            .effects
            .iter()
            .zip(current)
            .map(|(effect, (track, before))| {
                let after = match effect.effect {
                    QuestEffectKind::Set(value) => Some(value),
                    QuestEffectKind::Add(delta) => before.checked_add(delta),
                    QuestEffectKind::SetNow => Some(now),
                    QuestEffectKind::Computed => None,
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

/// SHA-256 over the quest key, its tracks and its transitions in key order (§6): never journal
/// text, so text edits never block players in progress.
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
            key: key.into(),
            quest: QUEST.into(),
            effects,
            completes,
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
            build(transition("oteryn:t/1", vec![set(STAGE); 9], false)),
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
        assert_eq!(QuestRefusal::RevisionMismatch.code(), "REVISION_MISMATCH");
        assert!(QuestStateCatalogue::empty("content-1").is_some());
        assert!(QuestStateCatalogue::empty("bad revision").is_none());
    }
}
