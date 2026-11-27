//! The QuestState content loader (QUEST-LOWER-1; QUEST-STATE-0 §13.2).
//!
//! `content/quests/missions/quest-state.json` is generated from the Quest definitions by
//! `tools/content-schema/quest-authoring/quest_state_lowering.py`: per quest, its tracks and
//! transitions under Oteryn keys, each with its source key kept as a source binding. It is
//! embedded at build time and [`load_embedded_quest_state`] parses it into a
//! [`QuestStateCatalogue`] for the server's content revision. Any unknown field, effect kind or
//! comparison, a record naming another quest, or a catalogue `QuestStateCatalogue::new`
//! refuses fails the whole load closed; nothing is skipped.
//!
//! An effect whose source `from` was not lowered exactly (`from_exact: false`: the generator
//! kept one comparison of a compound source condition) is not executable: it loads as
//! [`QuestEffectKind::Computed`], so its transition is refused `NOT_SUPPORTED` until the omitted
//! predicates are lowered, and never runs on a partial guard.
//!
//! The source keys and the NPC `requested_by` bindings stay beside the catalogue
//! ([`LoweredQuestState`]): they are content evidence for the callers that bind them
//! (NPC-QUEST-CONTENT-1) and never reach the store or the wire.

use std::collections::BTreeMap;

use serde::Deserialize;
use serde::de::IgnoredAny;

use super::{
    QuestCatalogueError, QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue,
    QuestTrack, QuestTransition,
};

const EMBEDDED: &str = include_str!("../../../../../content/quests/missions/quest-state.json");
const SCHEMA: &str = "OTERYN_QUEST_STATE_LOWERING/v1";

/// The NPC dialogue a source transition was written from (format §3.2): the source NPC bundle
/// key, the player keywords and the dialogue topics around the write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestRequestedBy {
    pub npc: String,
    pub keywords: Vec<QuestKeyword>,
    pub topics: Vec<i64>,
}

/// A player keyword: the word, or a text reference where the source phrase is not committed.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(untagged)]
pub enum QuestKeyword {
    Word(String),
    TextRef(QuestTextRef),
}

/// The SHA-256 and byte length of a source phrase (LICENSE-ASSETS.md: no source text).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuestTextRef {
    pub sha256: String,
    pub length: u64,
    pub placeholders: Vec<String>,
}

/// The loaded catalogue with its content-side bindings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoweredQuestState {
    catalogue: QuestStateCatalogue,
    source_tracks: BTreeMap<String, String>,
    requested_by: BTreeMap<String, QuestRequestedBy>,
    counts: QuestStateCounts,
}

/// What a load lowered: the quests and transitions, and the transitions refused
/// `NOT_SUPPORTED` (a `Computed` effect), with their explicit `COMPUTED` and inexact
/// (`from_exact: false`) effects counted apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct QuestStateCounts {
    pub quests: usize,
    pub transitions: usize,
    pub not_supported: usize,
    pub explicit_computed: usize,
    pub inexact: usize,
}

impl LoweredQuestState {
    #[must_use]
    pub fn catalogue(&self) -> &QuestStateCatalogue {
        &self.catalogue
    }

    /// The source key a track was lowered from.
    #[must_use]
    pub fn source_track(&self, track: &str) -> Option<&str> {
        self.source_tracks.get(track).map(String::as_str)
    }

    /// The NPC dialogue that requests a transition, when its source write was an NPC's.
    #[must_use]
    pub fn requested_by(&self, transition: &str) -> Option<&QuestRequestedBy> {
        self.requested_by.get(transition)
    }

    #[must_use]
    pub fn counts(&self) -> QuestStateCounts {
        self.counts
    }
}

/// Why the lowered quest content does not load.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuestLoadError {
    Json(String),
    Schema(String),
    ForeignRecord(String),
    Comparison(String),
    Effect(String),
    Catalogue(QuestCatalogueError),
}

// Every field is named, the unread ones as `IgnoredAny`, so an unknown field fails the load.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    schema: String,
    #[serde(rename = "classification")]
    _classification: IgnoredAny,
    #[serde(rename = "family")]
    _family: IgnoredAny,
    #[serde(rename = "contract")]
    _contract: IgnoredAny,
    #[serde(rename = "authoring_sources")]
    _authoring_sources: IgnoredAny,
    #[serde(rename = "counts")]
    _counts: IgnoredAny,
    quests: Vec<Quest>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Quest {
    quest: String,
    #[serde(rename = "source_quest")]
    _source_quest: IgnoredAny,
    #[serde(rename = "completion")]
    _completion: IgnoredAny,
    tracks: Vec<Track>,
    transitions: Vec<Transition>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Track {
    key: String,
    quest: String,
    initial: i64,
    min: i64,
    max: i64,
    #[serde(rename = "bounds_basis")]
    _bounds_basis: IgnoredAny,
    source_key: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Transition {
    #[serde(default)]
    world_change: Option<String>,
    key: String,
    quest: String,
    completes: bool,
    effects: Vec<Effect>,
    requested_by: Option<RequestedBy>,
    #[serde(rename = "source")]
    _source: IgnoredAny,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Effect {
    track: String,
    from: Tagged,
    from_exact: bool,
    effect: Tagged,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Tagged {
    #[serde(alias = "kind")]
    op: String,
    value: Option<i64>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RequestedBy {
    npc: String,
    keywords: Vec<QuestKeyword>,
    topics: Vec<i64>,
}

/// Load the embedded lowered quest content for `content_revision`.
pub fn load_embedded_quest_state(
    content_revision: &str,
) -> Result<LoweredQuestState, QuestLoadError> {
    let lowered = lower_embedded_quest_state(content_revision)?;
    let bindings = super::authored_bindings::embedded::load().map_err(QuestLoadError::Json)?;
    let mut lowered = admit_native_bindings(lowered, bindings.as_ref())?;
    super::published_quest_pins::retain_authored_predecessors(&mut lowered.catalogue);
    super::reward_callbacks::retain_qc_reward_predecessors(&mut lowered.catalogue);
    Ok(lowered)
}

/// Admit a complete explicit binding snapshot instead of the compiled binding registry.
/// Both routes use the same canonical recipe evidence and existing native quest owners.
pub fn load_embedded_quest_state_with_bindings(
    content_revision: &str,
    bindings: &super::authored_bindings::Bindings,
) -> Result<LoweredQuestState, QuestLoadError> {
    admit_native_bindings(
        lower_embedded_quest_state(content_revision)?,
        Some(bindings),
    )
}

fn admit_native_bindings(
    mut lowered: LoweredQuestState,
    bindings: Option<&super::authored_bindings::Bindings>,
) -> Result<LoweredQuestState, QuestLoadError> {
    lowered.catalogue.native_recipe_evidence = Some(std::sync::Arc::new(
        super::authored_bindings::embedded::RecipeEvidence::embedded()
            .map_err(QuestLoadError::Json)?,
    ));
    if let Some(bindings) = bindings {
        bindings.validate().map_err(QuestLoadError::Json)?;
        if !bindings.quests.is_empty() {
            let predecessors =
                super::completion_sets::journey_predecessors(&lowered.catalogue, bindings)
                    .map_err(QuestLoadError::Json)?;
            lowered.catalogue = bindings
                .attach(&lowered.catalogue)
                .map_err(QuestLoadError::Json)?;
            for (quest, predecessor) in predecessors {
                lowered
                    .catalogue
                    .predecessors
                    .entry(quest)
                    .or_default()
                    .push(predecessor);
            }
            for quest in &bindings.quests {
                let source = quest.sources.join(";");
                for track in &quest.tracks {
                    // A shared native counter keeps its donor witness (shared_tracks.rs).
                    lowered
                        .source_tracks
                        .entry(track.key.clone())
                        .or_insert_with(|| source.clone());
                }
            }
        }
    }
    lowered.catalogue =
        super::native_kills::extend(lowered.catalogue).map_err(QuestLoadError::Json)?;
    lowered.catalogue =
        super::completion_sets::extend(lowered.catalogue).map_err(QuestLoadError::Json)?;
    lowered.catalogue =
        super::arena_completion::extend(lowered.catalogue).map_err(QuestLoadError::Json)?;
    // An empty binding snapshot keeps the predecessor set from the legacy loader.
    super::published_quest_pins::retain_predecessors(&mut lowered.catalogue);
    Ok(lowered)
}

pub(super) fn lower_embedded_quest_state(
    content_revision: &str,
) -> Result<LoweredQuestState, QuestLoadError> {
    let mut lowered = parse_quest_state(EMBEDDED, content_revision)?;
    // Native goals own their zero-based counters before callbacks add read-only guards.
    lowered.catalogue =
        super::native_kill_goals::extend(lowered.catalogue).map_err(QuestLoadError::Json)?;
    lowered.catalogue =
        super::kill_objectives::extend(lowered.catalogue).map_err(QuestLoadError::Json)?;
    lowered.catalogue =
        super::lions_rock::extend(lowered.catalogue).map_err(QuestLoadError::Json)?;
    lowered.catalogue =
        super::world_change_journals::extend(lowered.catalogue).map_err(QuestLoadError::Json)?;
    lowered.catalogue =
        super::last_creep_native::extend(lowered.catalogue).map_err(QuestLoadError::Json)?;
    lowered.catalogue =
        super::elvenbane::extend(lowered.catalogue).map_err(QuestLoadError::Json)?;
    lowered.catalogue =
        super::classic_dialogue::extend_rottin(lowered.catalogue).map_err(QuestLoadError::Json)?;
    // Zelos's passage advances the donor questline to Jack's report stage.
    lowered.catalogue =
        super::grave_danger::extend(lowered.catalogue).map_err(QuestLoadError::Json)?;
    lowered.catalogue = super::world_callbacks::extend(
        super::reward_callbacks::declare_tracks(lowered.catalogue).map_err(QuestLoadError::Json)?,
    )
    .map_err(QuestLoadError::Json)?;
    lowered.catalogue =
        super::fire_feathered_native::extend(lowered.catalogue).map_err(QuestLoadError::Json)?;
    lowered.catalogue =
        super::shared_world_progress::extend(lowered.catalogue).map_err(QuestLoadError::Json)?;
    lowered.catalogue =
        super::monk_shrines::extend(lowered.catalogue).map_err(QuestLoadError::Json)?;
    lowered.catalogue =
        super::threatened_dreams::extend(lowered.catalogue).map_err(QuestLoadError::Json)?;
    lowered.catalogue =
        super::classic_dialogue::extend(lowered.catalogue).map_err(QuestLoadError::Json)?;
    lowered.catalogue =
        super::liquid_black::extend(lowered.catalogue).map_err(QuestLoadError::Json)?;
    lowered.catalogue =
        super::wrath_sleeping_dragon::extend(lowered.catalogue).map_err(QuestLoadError::Json)?;
    lowered.catalogue =
        super::grizzly_adams::extend(lowered.catalogue).map_err(QuestLoadError::Json)?;
    lowered.catalogue =
        super::rotten_blood::extend(lowered.catalogue).map_err(QuestLoadError::Json)?;
    lowered.catalogue =
        super::rotten_blood::area_icons::extend(lowered.catalogue).map_err(QuestLoadError::Json)?;
    lowered.catalogue =
        super::soul_war::source_items::extend(lowered.catalogue).map_err(QuestLoadError::Json)?;
    lowered.catalogue =
        super::dark_trails::extend(lowered.catalogue).map_err(QuestLoadError::Json)?;
    lowered.catalogue =
        super::isle_of_evil::extend(lowered.catalogue).map_err(QuestLoadError::Json)?;
    lowered.catalogue =
        super::source_deaths::extend(lowered.catalogue).map_err(QuestLoadError::Json)?;
    lowered.catalogue = super::timers::extend(lowered.catalogue).map_err(QuestLoadError::Json)?;
    lowered.source_tracks.insert(
        super::rotten_blood::ACCESS_TRACK.into(),
        "kv:rotten-blood-quest:access".into(),
    );
    lowered.source_tracks.insert(
        super::rotten_blood::ACCESS_DOOR_TRACK.into(),
        "storage:Quest.U13_20.RottenBlood.AccessDoor".into(),
    );
    // These explicitly authored native adaptations carry their own source witnesses;
    // their progress tracks are absent from the donor-lowered mission catalogue.
    for callback in super::fire_feathered_native::callbacks()
        .into_iter()
        .chain(super::shared_world_progress::callbacks())
        .chain(super::last_creep_native::callbacks().map_err(QuestLoadError::Json)?)
        .chain(super::world_callbacks::load().map_err(QuestLoadError::Json)?)
        .filter(|callback| {
            !callback
                .sources
                .iter()
                .any(|source| source.starts_with("oteryn-authored:"))
        })
    {
        for storage in callback.storage {
            lowered
                .source_tracks
                .entry(storage.track)
                .or_insert_with(|| callback.sources.join(";"));
        }
    }
    let before_npc_rewards = lowered.catalogue.clone();
    lowered.catalogue = super::npc_rules::extend(lowered.catalogue, &lowered.source_tracks)
        .map_err(QuestLoadError::Json)?;
    super::reward_callbacks::retain_outfit_reward_predecessor(
        before_npc_rewards,
        &mut lowered.catalogue,
    )
    .map_err(QuestLoadError::Json)?;
    // Complete NPC reward activations and counters before validating native goals.
    lowered.catalogue =
        super::reward_callbacks::extend(lowered.catalogue).map_err(QuestLoadError::Json)?;
    lowered.catalogue =
        super::death_objectives::extend(lowered.catalogue).map_err(QuestLoadError::Json)?;
    super::reward_callbacks::retain_warrior_reward_predecessor(&mut lowered.catalogue)
        .map_err(QuestLoadError::Json)?;
    super::native_kill_goals::validate_closed(&lowered.catalogue).map_err(QuestLoadError::Json)?;
    // Crystal 201189f, data-global/scripts/lib/register_actions.lua:921-926.
    // This tool-owned auxiliary flag is outside the captured NPC track inventory.
    lowered.source_tracks.insert(
        "oteryn:quest-progress/quest/u8_4/in_service_of_yalahar/sewer_pipe03".into(),
        "storage:Quest.U8_4.InServiceOfYalahar.SewerPipe03".into(),
    );
    super::native_kill_goals::validate_closed(&lowered.catalogue).map_err(QuestLoadError::Json)?;
    // Authored world programmes lower their tracks from the pinned canonical recipe.
    // Register that evidence after legacy NPC alias compilation, retaining every
    // existing donor storage binding and never presenting a new track as Lua storage.
    for callback in super::world_callbacks::load().map_err(QuestLoadError::Json)? {
        if let Some(source) = callback
            .sources
            .iter()
            .find(|source| source.starts_with("canonical:"))
        {
            for storage in &callback.storage {
                if lowered
                    .catalogue
                    .track(&storage.track)
                    .is_some_and(|track| track.quest == callback.quest)
                {
                    lowered
                        .source_tracks
                        .entry(storage.track.clone())
                        .or_insert_with(|| source.clone());
                }
            }
        }
    }
    // The additive death callback owns this predecessor independently of
    // whether native bindings are imported by the caller.
    super::source_deaths::retain_overlord_predecessor(&mut lowered.catalogue)
        .map_err(QuestLoadError::Json)?;
    super::rottin_walls::retain_predecessor(&mut lowered.catalogue)
        .map_err(QuestLoadError::Json)?;
    let mut lowered = restrict_runtime_scope(lowered, content_revision)?;
    super::published_quest_pins::retain_predecessors(&mut lowered.catalogue);
    Ok(lowered)
}

fn restrict_runtime_scope(
    mut lowered: LoweredQuestState,
    content_revision: &str,
) -> Result<LoweredQuestState, QuestLoadError> {
    // Preserve source captures and their counts; only the production catalogue
    // and its executable bindings are filtered by the owner's target scope.
    let npc_legacy = std::mem::take(&mut lowered.catalogue.npc_legacy);
    let mut predecessors = std::mem::take(&mut lowered.catalogue.predecessors);
    predecessors.retain(|quest, _| super::in_runtime_scope(quest));
    lowered.catalogue = QuestStateCatalogue::new(
        content_revision,
        lowered
            .catalogue
            .tracks
            .values()
            .filter(|track| super::in_runtime_scope(&track.quest))
            .cloned()
            .collect(),
        lowered
            .catalogue
            .transitions
            .values()
            .filter(|transition| super::in_runtime_scope(&transition.quest))
            .cloned()
            .collect(),
    )
    .map_err(QuestLoadError::Catalogue)?;
    lowered.catalogue.npc_legacy = npc_legacy;
    lowered.catalogue.predecessors = predecessors;
    lowered
        .source_tracks
        .retain(|key, _| lowered.catalogue.track(key).is_some());
    lowered
        .requested_by
        .retain(|key, _| lowered.catalogue.transition(key).is_some());
    Ok(lowered)
}

/// Parse one lowered quest content document for `content_revision`.
pub fn parse_quest_state(
    text: &str,
    content_revision: &str,
) -> Result<LoweredQuestState, QuestLoadError> {
    let document: Document =
        serde_json::from_str(text).map_err(|error| QuestLoadError::Json(error.to_string()))?;
    if document.schema != SCHEMA {
        return Err(QuestLoadError::Schema(document.schema));
    }
    let mut tracks = Vec::new();
    let mut transitions = Vec::new();
    let mut source_tracks = BTreeMap::new();
    let mut requested_by = BTreeMap::new();
    let mut counts = QuestStateCounts {
        quests: document.quests.len(),
        ..QuestStateCounts::default()
    };
    for quest in document.quests {
        for track in quest.tracks {
            if track.quest != quest.quest {
                return Err(QuestLoadError::ForeignRecord(track.key));
            }
            source_tracks.insert(track.key.clone(), track.source_key);
            tracks.push(QuestTrack {
                key: track.key,
                quest: track.quest,
                initial: track.initial,
                min: track.min,
                max: track.max,
            });
        }
        for transition in quest.transitions {
            if transition.quest != quest.quest {
                return Err(QuestLoadError::ForeignRecord(transition.key));
            }
            let effects = transition
                .effects
                .into_iter()
                .map(|effect| {
                    counts.explicit_computed += usize::from(effect.effect.op == "COMPUTED");
                    counts.inexact += usize::from(!effect.from_exact);
                    let from = comparison(&effect.from)
                        .ok_or_else(|| QuestLoadError::Comparison(transition.key.clone()))?;
                    let kind = effect_kind(&effect.effect)
                        .ok_or_else(|| QuestLoadError::Effect(transition.key.clone()))?;
                    Ok(QuestEffect {
                        from,
                        effect: if effect.from_exact {
                            kind
                        } else {
                            QuestEffectKind::Computed
                        },
                        track: effect.track,
                    })
                })
                .collect::<Result<Vec<_>, QuestLoadError>>()?;
            if let Some(binding) = transition.requested_by {
                requested_by.insert(
                    transition.key.clone(),
                    QuestRequestedBy {
                        npc: binding.npc,
                        keywords: binding.keywords,
                        topics: binding.topics,
                    },
                );
            }
            counts.transitions += 1;
            counts.not_supported += usize::from(
                effects
                    .iter()
                    .any(|effect| effect.effect == QuestEffectKind::Computed),
            );
            transitions.push(QuestTransition {
                world_change: transition.world_change,
                key: transition.key,
                quest: transition.quest,
                effects,
                completes: transition.completes,
                experience: None,
            });
        }
    }
    let catalogue = QuestStateCatalogue::new(content_revision, tracks, transitions)
        .map_err(QuestLoadError::Catalogue)?;
    Ok(LoweredQuestState {
        catalogue,
        source_tracks,
        requested_by,
        counts,
    })
}

fn comparison(from: &Tagged) -> Option<QuestComparison> {
    Some(match (from.op.as_str(), from.value) {
        ("ANY", None) => QuestComparison::Any,
        ("EQ", Some(value)) => QuestComparison::Eq(value),
        ("NE", Some(value)) => QuestComparison::Ne(value),
        ("LT", Some(value)) => QuestComparison::Lt(value),
        ("LE", Some(value)) => QuestComparison::Le(value),
        ("GT", Some(value)) => QuestComparison::Gt(value),
        ("GE", Some(value)) => QuestComparison::Ge(value),
        _ => return None,
    })
}

fn effect_kind(effect: &Tagged) -> Option<QuestEffectKind> {
    Some(match (effect.op.as_str(), effect.value) {
        ("SET", Some(value)) => QuestEffectKind::Set(value),
        ("ADD", Some(value)) => QuestEffectKind::Add(value),
        ("SET_NOW", None) => QuestEffectKind::SetNow,
        ("COMPUTED", None) => QuestEffectKind::Computed,
        _ => return None,
    })
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    mod donor_quest_tests;
    use super::super::{QuestRefusal, valid_quest_key};
    use super::*;

    const QUEST: &str = "oteryn:quest.a_father_s_burden_quest";
    const QUEST_LOG: &str = "oteryn:quest-progress/quest/u8_6/afathers_burden/quest_log";
    const QUEST_LOG_NPC_1: &str =
        "oteryn:quest-transition/quest/u8_6/afathers_burden/quest_log/npc_1";

    fn embedded() -> LoweredQuestState {
        load_embedded_quest_state("content-1").expect("embedded quest state loads")
    }

    #[test]
    fn quest_gap_loader_billy_pan_exchange() -> Result<(), String> {
        let loaded = embedded();
        let offered = super::super::reward_callbacks::offered_for(loaded.catalogue())?;
        assert!(offered.iter().any(|row| row.npc == "oteryn:npc.billy"
            && row.key == "oteryn:quest-reward/billy-pan-first"
            && row.completes));
        Ok(())
    }

    #[test]
    fn quest_gap_loader_lee_delle_studded_legs_exchange() -> Result<(), String> {
        let loaded = embedded();
        let offered = super::super::reward_callbacks::offered_for(loaded.catalogue())?;
        assert!(offered.iter().any(|row| row.npc == "oteryn:npc.lee_delle"
            && row.key == "oteryn:quest-reward/lee-delle-flower-first"
            && row.completes));
        Ok(())
    }

    #[test]
    fn quest_gap_loader_ahmet_oasis_box() -> Result<(), String> {
        let loaded = embedded();
        let callbacks = super::super::world_callbacks::load_for(loaded.catalogue())?;
        let chest = callbacks
            .iter()
            .find(|row| row.key == "oteryn:quest-callback/qd-steal-from-thieves-oasis-box")
            .ok_or("oasis box must be in the live callback catalogue")?;
        assert_eq!(chest.selector.action, Some(19910));
        assert_eq!(
            chest
                .item_reward()
                .as_ref()
                .map(|reward| reward.item.as_str()),
            Some("oteryn:item.tibia.i235")
        );
        Ok(())
    }

    #[test]
    fn runtime_scope_preserves_only_admitted_predecessors() -> Result<(), String> {
        const MARLIN: &str = "oteryn:quest.marlin_trophy_quest";
        const DAWNPORT: &str = "oteryn:quest.dawnport_quest";
        let mut lowered = parse_quest_state(EMBEDDED, "scope-predecessors")
            .map_err(|error| format!("{error:?}"))?;
        let cases = [
            (QUEST, [1u8; 32]),
            (MARLIN, [2u8; 32]),
            (DAWNPORT, [3u8; 32]),
        ];
        for (quest, hash) in cases {
            assert!(lowered.catalogue.definition_hash(quest).is_some());
            lowered.catalogue.predecessors.insert(
                quest.into(),
                vec![super::super::NpcDefinitionPredecessor {
                    hash,
                    defaults: BTreeMap::new(),
                }],
            );
        }
        let expected = lowered.catalogue.predecessors.clone();
        let scoped = restrict_runtime_scope(lowered, "scope-predecessors")
            .map_err(|error| format!("{error:?}"))?;
        assert_eq!(scoped.catalogue.predecessors.len(), 2);
        for (quest, hash) in cases {
            let defaults =
                super::super::npc_rules::legacy_defaults(scoped.catalogue(), quest, &hash);
            if quest == DAWNPORT {
                assert!(defaults.is_none());
                assert!(!scoped.catalogue.predecessors.contains_key(quest));
                assert!(scoped.catalogue.definition_hash(quest).is_none());
                assert!(
                    scoped
                        .catalogue
                        .tracks
                        .values()
                        .all(|track| track.quest != quest)
                );
                assert!(
                    scoped
                        .catalogue
                        .transitions
                        .values()
                        .all(|rule| rule.quest != quest)
                );
            } else {
                assert!(defaults.ok_or("retained predecessor")?.is_empty());
                assert_eq!(
                    scoped.catalogue.predecessors.get(quest),
                    expected.get(quest)
                );
                let mut invalid = hash;
                invalid[0] ^= 1;
                assert!(
                    super::super::npc_rules::legacy_defaults(scoped.catalogue(), quest, &invalid)
                        .is_none()
                );
            }
        }
        Ok(())
    }

    #[test]
    fn quest_parity_outlaw_camp_treasure_loads_as_one_terminal_container_reward()
    -> Result<(), String> {
        use crate::quest::world_callbacks::{self, Position, Trigger};

        let key = "oteryn:quest-callback/outlaw-camp-treasure";
        let lowered =
            load_embedded_quest_state("outlaw-camp").map_err(|error| format!("{error:?}"))?;
        let transition = lowered
            .catalogue()
            .transition(key)
            .ok_or("missing treasure transition")?;
        assert!(transition.completes);
        assert_eq!(transition.quest, "oteryn:quest.the_outlaw_camp_quest");
        let callbacks = world_callbacks::load_for(lowered.catalogue())?;
        let callback = callbacks
            .iter()
            .find(|callback| callback.key == key)
            .ok_or("missing treasure callback")?;
        assert_eq!(callback.trigger, Trigger::Use);
        assert_eq!(
            callback.selector.position,
            Some(Position {
                x: 32620,
                y: 32198,
                floor: -10
            })
        );
        assert_eq!(callback.selector.appearance, Some(2472));
        assert_eq!(callback.selector.unique, Some(14091));
        let reward = callback.item_reward().as_ref().ok_or("missing bag")?;
        assert_eq!(
            (reward.item.as_str(), reward.count),
            ("oteryn:item.tibia.i2853", 1)
        );
        assert_eq!(
            reward
                .contents
                .iter()
                .map(|item| (item.item.as_str(), item.count))
                .collect::<Vec<_>>(),
            [
                ("oteryn:item.tibia.i3295", 1),
                ("oteryn:item.tibia.i3039", 1)
            ],
        );
        assert_eq!(
            callback.storage[0].track,
            "oteryn:quest-progress/quest/u6_4/outlaw_camp_quest"
        );
        assert_eq!(
            (
                callback.storage[0].minimum,
                callback.storage[0].maximum,
                callback.storage[0].set
            ),
            (0, 0, 1)
        );
        Ok(())
    }

    #[test]
    fn quest_parity_explorer_society_loads_ordered_enrollment_and_powder_report()
    -> Result<(), String> {
        use crate::quest::{reward_callbacks, world_callbacks};

        let lowered =
            load_embedded_quest_state("explorer-society").map_err(|error| format!("{error:?}"))?;
        let catalogue = lowered.catalogue();
        let callbacks = reward_callbacks::offered_for(catalogue)?;
        let world = world_callbacks::load_for(catalogue)?;
        let line = "oteryn:quest-progress/quest/u7_6/explorer_society/quest_line";
        for (name, minimum, maximum, after) in [
            ("angus/enroll", 0, 0, 1),
            ("mortimer/enroll", 0, 0, 1),
            ("uzgod/brooch-errand", 1, 1, 2),
            ("uzgod/pickaxe", 3, 3, 4),
            ("angus/pickaxe-report", 1, 4, 5),
            ("mortimer/pickaxe-report", 1, 4, 5),
            ("angus/orc-powder-report", 33, 33, 35),
            ("mortimer/orc-powder-report", 33, 33, 35),
        ] {
            let key = format!("oteryn:quest-reward/explorer-society/{name}");
            let row = callbacks
                .iter()
                .find(|row| row.key == key)
                .ok_or_else(|| key.clone())?;
            let gate = row
                .storage
                .iter()
                .find(|gate| gate.track == line)
                .ok_or("missing quest-line guard")?;
            assert_eq!(
                (gate.minimum, gate.maximum, gate.set),
                (minimum, maximum, after),
                "{name}"
            );
            assert!(catalogue.transition(&key).is_some(), "{name}");
        }
        for (name, uid, reward, before, after) in [
            ("family-brooch", 14029, 4834, 2, 3),
            ("bonelord-parchment", 14030, 173, 30, 31),
            ("orc-powder", 14031, 13974, 33, 34),
            ("elven-poetry", 14032, 4844, 36, 37),
            ("memory-stone", 14033, 4841, 39, 40),
            ("spectral-dress", 14034, 4836, 48, 49),
            ("lizard-urn", 40041, 4847, 27, 28),
            ("frozen-dragon", 40042, 7314, 57, 58),
        ] {
            let key = format!("oteryn:quest-callback/explorer-society-{name}");
            let row = world
                .iter()
                .find(|row| row.key == key)
                .ok_or_else(|| key.clone())?;
            assert_eq!(row.selector.unique, Some(uid), "{name}");
            assert!(row.selector.native.is_some(), "{name}");
            assert_eq!(
                row.item_reward().as_ref().ok_or("missing pickup")?.item,
                format!("oteryn:item.tibia.i{reward}")
            );
            let gate = row.storage.first().ok_or("missing mission guard")?;
            assert_eq!(
                (gate.minimum, gate.maximum, gate.set),
                (before, before, after),
                "{name}"
            );
            assert!(catalogue.transition(&key).is_some(), "{name}");
        }
        let final_report = "oteryn:quest-reward/explorer-society/lurik/ice-music-report";
        let row = callbacks
            .iter()
            .find(|row| row.key == final_report)
            .ok_or("missing final Explorer Society report")?;
        assert_eq!(row.take.len(), 1);
        assert_eq!(
            (row.take[0].item.as_str(), row.take[0].count),
            ("oteryn:item.tibia.i7315", 1)
        );
        assert!(
            catalogue
                .transition(final_report)
                .is_some_and(|transition| transition.completes)
        );
        Ok(())
    }

    #[test]
    fn explorer_legacy_pickaxe_saves_enter_ice_delivery_and_can_retry() -> Result<(), String> {
        let loaded =
            load_embedded_quest_state("content-1").map_err(|error| format!("{error:?}"))?;
        let catalogue = loaded.catalogue();
        let prefix = "oteryn:quest-progress/quest/u7_6/explorer_society/";
        let line = format!("{prefix}quest_line");
        let field = format!("{prefix}field_course");
        let ice = format!("{prefix}the_ice_delivery");
        let callbacks = crate::quest::world_callbacks::load_for(catalogue)?;
        let authored = catalogue.authored_bindings().ok_or("authored bindings")?;
        let explorer = authored
            .quests
            .iter()
            .find(|quest| quest.quest == "oteryn:quest.the_explorer_society_quest")
            .ok_or("Explorer Society binding")?;
        for stage in &explorer.stages {
            let world = matches!(
                stage.trigger,
                crate::quest::authored_bindings::Trigger::MapUse { .. }
                    | crate::quest::authored_bindings::Trigger::StepIn { .. }
            );
            for operation in stage.transitions(&explorer.quest)? {
                let registered = catalogue.transition(&operation.key).ok_or(&operation.key)?;
                crate::quest::admission_binding::validate_resolved_world(
                    catalogue, registered, world, &callbacks,
                )
                .map_err(|error| format!("{}: {error}", operation.key))?;
            }
        }
        for npc in ["angus", "mortimer"] {
            let before = BTreeMap::from([(field.clone(), 5), (line.clone(), 1), (ice.clone(), 0)]);
            let start = format!("oteryn:quest-reward/explorer-society/assign-ice-{npc}");
            let transition = catalogue.transition(&start).ok_or(start)?;
            let changes = catalogue
                .evaluate(transition, &before, 0)
                .map_err(|e| format!("{e:?}"))?;
            let after = |track: &str| {
                changes
                    .iter()
                    .find(|change| change.track == track)
                    .map(|change| change.after)
            };
            assert_eq!(after(&line), Some(6));
            assert_eq!(after(&field), Some(6));
            assert_eq!(after(&ice), Some(6));

            let before = BTreeMap::from([(field.clone(), 7), (line.clone(), 7), (ice.clone(), 7)]);
            let retry = format!("oteryn:quest-reward/explorer-society/retry-ice-{npc}");
            let transition = catalogue.transition(&retry).ok_or(retry)?;
            let changes = catalogue
                .evaluate(transition, &before, 0)
                .map_err(|e| format!("{e:?}"))?;
            assert_eq!(changes.len(), 3);
            assert!(changes.iter().all(|change| change.after == 6));
        }

        for suffix in ["/cut-ice", "-ice-delivery-1", "-ice-delivery-2"] {
            let key = format!("oteryn:quest-callback/explorer-society{suffix}");
            let callback = callbacks
                .iter()
                .find(|callback| callback.key == key)
                .ok_or(key)?;
            let sample = callback
                .item_reward
                .as_ref()
                .ok_or("scheduled icicle source")?;
            assert_eq!(
                (sample.item.as_str(), sample.count),
                ("oteryn:item.tibia.i4837", 1)
            );
            assert!(
                callback
                    .storage
                    .iter()
                    .any(|storage| storage.add == Some(1))
            );
        }
        Ok(())
    }

    #[test]
    fn quest_parity_explorer_tools_and_portals_bind_exact_assets() -> Result<(), String> {
        use crate::quest::world_callbacks::{self, Position, Trigger};

        let callbacks = world_callbacks::load()?;
        for (name, target, tool, replacement) in [
            ("ice-delivery-1", 4994, 4872, None),
            ("ice-delivery-2", 4994, 4872, None),
            ("jungle-bells", 3874, 4867, Some(4868)),
            ("witches-cauldron", 3885, 4867, Some(4869)),
            ("giant-jungle-rose", 3878, 4867, Some(4870)),
            ("rune-tracing", 2199, 4842, Some(4843)),
            ("ice-music", 6966, 7242, Some(7315)),
        ] {
            let key = format!("oteryn:quest-callback/explorer-society-{name}");
            let row = callbacks
                .iter()
                .find(|row| row.key == key)
                .ok_or_else(|| key.clone())?;
            let input = row.item_on_target.as_ref().ok_or("missing selected tool")?;
            assert_eq!(row.trigger, Trigger::Use);
            assert_eq!(
                row.selector
                    .native
                    .as_ref()
                    .ok_or("missing target")?
                    .appearance,
                target
            );
            assert_eq!(input.item, format!("oteryn:item.tibia.i{tool}"));
            assert_eq!(
                input.replacement,
                replacement.map(|id| format!("oteryn:item.tibia.i{id}"))
            );
            assert!(input.selected_instance && row.requires_world_admission);
            if tool == 4872 {
                assert!(input.reusable);
                assert_eq!(
                    row.item_reward().as_ref().ok_or("missing ice")?.item,
                    "oteryn:item.tibia.i4837"
                );
            }
        }
        for (uid, landing) in [
            (25018, (32498, 31622)),
            (25019, (32665, 32735)),
            (25020, (32320, 31137)),
            (25021, (32359, 32807)),
        ] {
            let row = callbacks
                .iter()
                .find(|row| row.trigger == Trigger::StepIn && row.selector.unique == Some(uid))
                .ok_or("missing astral portal")?;
            assert_eq!(
                row.teleport,
                Some(Position {
                    x: landing.0,
                    y: landing.1,
                    floor: -6
                })
            );
            assert!(row.return_on_failure);
            let fee = row.item_on_target.as_ref().ok_or("missing pearl cost")?;
            assert_eq!(
                (fee.item.as_str(), fee.count),
                ("oteryn:item.tibia.i5021", 1)
            );
            assert!(fee.on_step && !fee.reusable);
        }
        Ok(())
    }

    #[test]
    fn quest_parity_outlaw_oven_reopens_and_restores_the_source_tiles() -> Result<(), String> {
        use crate::quest::world_callbacks::{self, Position};

        let callbacks = world_callbacks::load()?;
        let callback = callbacks
            .iter()
            .find(|row| row.key == "oteryn:quest-callback/outlaw-camp-oven")
            .ok_or("missing oven callback")?;
        assert_eq!(callback.selector.unique, Some(30031));
        assert_eq!(callback.lever, Some([2772, 2773]));
        assert!(callback.reversible);
        let scene = callback
            .effects
            .scene
            .as_ref()
            .ok_or("missing oven movement")?;
        let origin = Position {
            x: 32623,
            y: 32188,
            floor: -9,
        };
        let destination = Position { y: 32189, ..origin };
        assert_eq!(
            (scene.open[0].position, scene.open[0].remove),
            (origin, Some(2535))
        );
        assert_eq!(
            (scene.open[1].position, scene.open[1].create),
            (destination, Some(2535))
        );
        assert_eq!(
            (scene.close[0].position, scene.close[0].remove),
            (destination, Some(2535))
        );
        assert_eq!(
            (scene.close[1].position, scene.close[1].create),
            (origin, Some(2535))
        );
        Ok(())
    }

    #[test]
    fn owner_scope_excludes_dawnport_from_runtime_state_and_callbacks() -> Result<(), String> {
        let loaded =
            load_embedded_quest_state("owner-scope").map_err(|error| format!("{error:?}"))?;
        assert!(
            loaded
                .catalogue()
                .definition_hash("oteryn:quest.dawnport_quest")
                .is_none()
        );
        assert!(
            loaded
                .catalogue()
                .transition("oteryn:quest-transition/classic/dawnport/start-the-rare-herb")
                .is_none()
        );
        let callbacks = crate::quest::world_callbacks::load()?;
        assert!(
            callbacks
                .iter()
                .all(|callback| callback.quest != "oteryn:quest.dawnport_quest")
        );
        let registries = [
            crate::quest::world_callbacks::EMBEDDED,
            // QMECH source cohorts retain their separate registration owners.
            include_str!("../../../../../content/quests/world-callbacks/children-grease.json"),
            include_str!("../../../../../content/quests/world-callbacks/cwdoor-world-routes.json"),
            include_str!("../../../../../content/quests/world-callbacks/cursed-crystal.json"),
            include_str!("../../../../../content/quests/world-callbacks/dream-courts.json"),
            include_str!("../../../../../content/quests/world-callbacks/engenc2-cults.json"),
            include_str!("../../../../../content/quests/world-callbacks/hot-cuisine-books.json"),
            include_str!("../../../../../content/quests/world-callbacks/ice-islands-seals.json"),
            include_str!("../../../../../content/quests/world-callbacks/m07-dreamer-levers.json"),
            include_str!("../../../../../content/quests/world-callbacks/m07-lion-offerings.json"),
            include_str!("../../../../../content/quests/world-callbacks/m07-machinery-burn.json"),
            include_str!(
                "../../../../../content/quests/world-callbacks/m07-machinery-catapults.json"
            ),
            include_str!(
                "../../../../../content/quests/world-callbacks/m07-machinery-repairs.json"
            ),
            include_str!("../../../../../content/quests/world-callbacks/m07-machinery-wheels.json"),
            include_str!("../../../../../content/quests/world-callbacks/m07-realm-trial.json"),
            include_str!("../../../../../content/quests/world-callbacks/m07-too-hot-columns.json"),
            include_str!("../../../../../content/quests/world-callbacks/m09-treasure.json"),
            include_str!("../../../../../content/quests/world-callbacks/m39-stories.json"),
            include_str!("../../../../../content/quests/world-callbacks/oramond-native-roots.json"),
            include_str!("../../../../../content/quests/world-callbacks/postman.json"),
            include_str!("../../../../../content/quests/world-callbacks/qc-chests.json"),
            include_str!("../../../../../content/quests/world-callbacks/qc-lizard-sanctuary.json"),
            include_str!("../../../../../content/quests/world-callbacks/qm48-mammoth.json"),
            include_str!("../../../../../content/quests/world-callbacks/qready1-clocks.json"),
            include_str!("../../../../../content/quests/world-callbacks/qready1-crafts.json"),
            include_str!("../../../../../content/quests/world-callbacks/quest-contacts.json"),
            include_str!("../../../../../content/quests/world-callbacks/source-outfits.json"),
            include_str!("../../../../../content/quests/world-callbacks/nomads-land.json"),
            include_str!(
                "../../../../../content/quests/world-callbacks/questpc5-nomads-cabinet.json"
            ),
            // Accepted main83acbdac3 source cohorts remain independently enumerated.
            include_str!(
                "../../../../../content/quests/world-callbacks/l6-ancient-instruments.json"
            ),
            include_str!("../../../../../content/quests/world-callbacks/l6-item-rewards.json"),
            include_str!("../../../../../content/quests/world-callbacks/l6-tools.json"),
            include_str!("../../../../../content/quests/world-callbacks/l6-world-puzzles.json"),
            include_str!("../../../../../content/quests/world-callbacks/l6-tomb-rooms.json"),
            include_str!("../../../../../content/quests/shared-access-callbacks.json"),
            include_str!("../../../../../content/quests/native-scene-callbacks.json"),
            include_str!("../../../../../content/quests/shared-outfit-callbacks.json"),
            include_str!("../../../../../content/quests/authored-world-callbacks.json"),
            include_str!("../../../../../content/quests/questpc7-world-callbacks.json"),
            include_str!("../../../../../content/quests/world-callbacks/feaster-phase-routes.json"),
            include_str!(
                "../../../../../content/quests/world-callbacks/threatened-dreams-mosaic-treasure.json"
            ),
            include_str!("../../../../../content/quests/world-callbacks/tibia-tales.json"),
            include_str!("../../../../../content/quests/world-callbacks/too-hot-repairs.json"),
            include_str!("../../../../../content/quests/questl2-djinn-world.json"),
        ];
        let mut expected = std::collections::BTreeSet::new();
        for text in registries {
            let source: serde_json::Value =
                serde_json::from_str(text).map_err(|e| e.to_string())?;
            for callback in source["callbacks"].as_array().ok_or("source callbacks")? {
                if callback["quest"] != "oteryn:quest.dawnport_quest" {
                    expected.insert(callback["key"].as_str().ok_or("callback key")?.to_owned());
                }
            }
        }
        assert!(expected.insert(crate::quest::schrodinger_island::callback().key));
        expected.insert(crate::quest::rotten_blood::altar_callback().key);
        assert!(expected.insert("oteryn:quest-callback/cwarlock/lever".to_owned()));
        // Collector main adds these exact native World cohorts; no donor
        // exclusion becomes an automatic admission through a loaded key.
        let before_native = expected.len();
        for index in 0..48 {
            expected.insert(format!("oteryn:quest-callback/hive-native-gate-{index}"));
        }
        for index in [0, 1, 2, 3, 5] {
            expected.insert(format!(
                "oteryn:quest-callback/hive-native-objective-{index}"
            ));
        }
        for suffix in [
            "fire-feathered-native-darkness",
            "fire-feathered-native-life",
            "fire-feathered-native-light",
            "hive-native-hatchlings",
            "hive-native-fluid-preparation",
            "deeplings-native-coral",
            "deeplings-native-heart",
            "deeplings-native-soul-net",
            "deeplings-native-stair",
            "deeplings-native-boss-28574",
            "deeplings-native-boss-28575",
            "deeplings-native-boss-28576",
        ] {
            assert!(expected.insert(format!("oteryn:quest-callback/{suffix}")));
        }
        assert_eq!(expected.len() - before_native, 65);
        // Crystal source carried tomb keys and shared switch/gate identities
        // are fixed independently of the runtime callback constructors.
        let before_main_native = expected.len();
        for unique in 3950..=3956 {
            assert!(expected.insert(format!("oteryn:quest-callback/ancient-tombs-key-{unique}")));
        }
        for suffix in [
            "thalas-switch",
            "diprath-switch",
            "ashmunrah-switch",
            "thalas-switch-gate",
            "diprath-switch-gate",
            "ashmunrah-switch-gate",
            "diprath-room",
        ] {
            assert!(expected.insert(format!("oteryn:quest-callback/ancient-tombs-{suffix}")));
        }
        assert_eq!(expected.len() - before_main_native, 14);
        let before_elvenbane = expected.len();
        for suffix in ["poison", "dragon-floor"] {
            assert!(expected.insert(format!("oteryn:quest-callback/elvenbane/{suffix}")));
        }
        assert_eq!(expected.len() - before_elvenbane, 2);
        let before_qmech_native = expected.len();
        for suffix in [
            "barbarian-test-mead",
            "dreamers-challenge-carrot-entry",
            "dreamers-challenge-death-ring-reset",
            "dreamers-challenge-fire-bug-ignite",
            "engenc2/nomads-land-window",
            "engitem-ice-seal-0-0",
            "engitem-ice-seal-0-1",
            "engitem-ice-seal-0-2",
            "engitem-ice-seal-1-0",
            "engitem-ice-seal-1-1",
            "engitem-ice-seal-1-2",
            "engitem-ice-seal-2-0",
            "engitem-ice-seal-2-1",
            "engitem-ice-seal-2-2",
            "goblin-merchant-elevator-lower",
            "goblin-merchant-elevator-upper",
            "last-creep-native-blightwalker",
            "last-creep-native-carniphila",
            "last-creep-native-exit",
            "last-creep-native-lost-soul",
            "last-creep-native-mutated-rat",
            "last-creep-native-nightstalker",
            "last-creep-native-pharaoh",
            "last-creep-native-quara-hydromancer",
            "last-creep-native-return",
            "last-creep-native-start",
            "last-creep-native-zombie",
            "m02/nomads-land/window",
            "m44-lions-amphora-0",
            "m44-lions-amphora-1",
            "m44-lions-amphora-2",
            "m44-lions-amphora-3",
            "m44-lions-amphora-4",
            "m44-lions-amphora-5",
            "m44-lions-amphora-6",
            "m44-lions-amphora-7",
            "m44-lions-flower-0",
            "m44-lions-flower-1",
            "m44-lions-flower-2",
            "m44-lions-flower-3",
            "m44-lions-flower-4",
            "m44-lions-flower-5",
            "nrwicked/arbaziloth",
            "qn1-children-lever-0/ordinary",
            "qn1-children-lever-0/solved",
            "qn1-children-lever-1/ordinary",
            "qn1-children-lever-1/solved",
            "qn1-children-lever-2/ordinary",
            "qn1-children-lever-2/solved",
            "qn1-children-lever-3/ordinary",
            "qn1-children-lever-3/solved",
            "qn1-children-lever-4/ordinary",
            "qn1-children-lever-4/solved",
            "rotten-blood/prepare-chagorz",
            "rotten-blood/prepare-ichgahal",
            "rotten-blood/prepare-murcion",
            "rotten-blood/prepare-vemiath",
            "rottin-repair-42501",
            "rottin-repair-42502",
            "rottin-repair-42503",
            "rottin-repair-42504",
            "rottin-repair-42505",
            "rottin-repair-42506",
            "rottin-wood/final-net",
            "rottin-wood/loot-merchant",
            "rottin-wood/place-net",
            "srv7-reliable-ram",
            "uneasy-alliance-crystal",
            "worldchg-m31/fire_from_the_earth/volcano",
            "worldchg-m31/fury_gates/dungeon",
            "worldchg-m31/fury_gates/egress",
            "worldchg-m31/fury_gates/entry",
            "worldchg-m31/fury_gates/seal",
            "worldchg-m31/grimvale/island",
            "worldchg-m31/grimvale/survey",
            "worldchg-m31/hive_born/eastern-inner",
            "worldchg-m31/hive_born/western-inner",
            "worldchg-m31/hive_outpost/outpost",
            "worldchg-m31/hive_outpost/survey",
            "worldchg-m31/insectoid_invasion/incursion",
            "worldchg-m31/kingsday/royal-visit",
        ] {
            assert!(expected.insert(format!("oteryn:quest-callback/{suffix}")));
        }
        assert_eq!(expected.len() - before_qmech_native, 81);
        let before_ape_city_and_beginning = expected.len();
        for suffix in [
            "ape-city-amphora-40092",
            "ape-city-amphora-40093",
            "ape-city-amphora-40094",
            "ape-city-amphora-40095",
            "ape-city-cask-32495",
            "ape-city-cask-32497",
            "ape-city-cask-32499",
            "ape-city-cask-32501",
            "ape-city-cask-32503",
            "ape-city-cask-32505",
            "ape-city-cask-32507",
            "ape-city-catacombs-amphora-gate",
            "ape-city-catacombs-entry",
            "ape-city-decipher",
            "ape-city-destroy-monument",
            "ape-city-holy-hair",
            "ape-city-lever-1",
            "ape-city-lever-2",
            "ape-city-lever-3",
            "ape-city-lever-4",
            "ape-city-lever-5",
            "ape-city-lever-6",
            "ape-city-wall-1040",
            "ape-city-wall-1041",
            "ape-city-witches-cap",
            "beginning-santiago-cellar",
            "beginning-santiago-coat",
            "beginning-zirella-rope",
            "beginning-zirella-shovel",
        ] {
            assert!(expected.insert(format!("oteryn:quest-callback/{suffix}")));
        }
        assert_eq!(expected.len() - before_ape_city_and_beginning, 29);
        assert!(expected.insert("oteryn:quest-callback/qd-steal-from-thieves-oasis-box".into()));
        assert_eq!(
            callbacks
                .into_iter()
                .map(|callback| callback.key)
                .collect::<std::collections::BTreeSet<_>>(),
            expected
        );
        Ok(())
    }

    fn document() -> serde_json::Value {
        serde_json::from_str(EMBEDDED).expect("embedded quest state is JSON")
    }

    fn reparse(document: &serde_json::Value) -> Result<LoweredQuestState, QuestLoadError> {
        parse_quest_state(&document.to_string(), "content-1")
    }

    #[test]
    fn every_lowered_record_loads_under_an_oteryn_key() {
        // Source transcription remains complete, including owner-excluded
        // donor quests. The owner_scope test separately checks runtime admission.
        let lowered = parse_quest_state(EMBEDDED, "content-1").expect("source catalogue");
        let document = document();
        let quests = document["quests"].as_array().expect("quests");
        let (mut tracks, mut transitions) = (0, 0);
        for quest in quests {
            let key = quest["quest"].as_str().expect("quest key");
            assert!(valid_quest_key(key), "{key}");
            assert!(lowered.catalogue().definition_hash(key).is_some(), "{key}");
            for track in quest["tracks"].as_array().expect("tracks") {
                let key = track["key"].as_str().expect("track key");
                let loaded = lowered.catalogue().track(key).expect("track loads");
                assert!(valid_quest_key(&loaded.key) && valid_quest_key(&loaded.quest));
                assert_eq!(lowered.source_track(key), track["source_key"].as_str());
                tracks += 1;
            }
            for transition in quest["transitions"].as_array().expect("transitions") {
                let key = transition["key"].as_str().expect("transition key");
                assert!(lowered.catalogue().transition(key).is_some(), "{key}");
                assert_eq!(
                    lowered.requested_by(key).is_some(),
                    !transition["requested_by"].is_null()
                );
                transitions += 1;
            }
        }
        assert_eq!(tracks, document["counts"]["tracks"]);
        assert_eq!(transitions, document["counts"]["transitions"]);
        assert!(tracks > 0 && transitions > 0);
    }

    #[test]
    fn a_lowered_npc_transition_keeps_its_source_and_dialogue_bindings() {
        // Retain the exact native domain; only actual source writes may widen it.
        // QuestLog only receives1, so a donor sentinel must not broaden its range.
        let lowered = parse_quest_state(EMBEDDED, "content-1").expect("source catalogue");
        let track = lowered.catalogue().track(QUEST_LOG).expect("track");
        assert_eq!(
            (track.quest.as_str(), track.initial, track.min, track.max),
            (QUEST, 0, 0, 1)
        );
        // Keep the observed completion domain and the pre-existing wider
        // integer counter domain; the source sentinel must not lower either min.
        for (suffix, max) in [("status", 2), ("progress", i64::from(i32::MAX))] {
            let key = format!("oteryn:quest-progress/quest/u8_6/afathers_burden/{suffix}");
            let track = lowered.catalogue().track(&key).expect("Father track");
            assert_eq!((track.initial, track.min, track.max), (0, 0, max));
        }
        let native = embedded();
        let track = native.catalogue().track(QUEST_LOG).expect("native track");
        assert_eq!(
            (track.quest.as_str(), track.initial, track.min, track.max),
            (QUEST, 0, 0, 1)
        );
        for (suffix, max) in [("status", 2), ("progress", i64::from(i32::MAX))] {
            let key = format!("oteryn:quest-progress/quest/u8_6/afathers_burden/{suffix}");
            let track = native.catalogue().track(&key).expect("native Father track");
            assert_eq!((track.initial, track.min, track.max), (0, 0, max));
        }
        for lowered in [&lowered, &native] {
            assert_eq!(
                lowered.source_track(QUEST_LOG),
                Some("canary:quest-progress/quest/u8_6/afathers_burden/quest_log")
            );
            let transition = lowered
                .catalogue()
                .transition(QUEST_LOG_NPC_1)
                .expect("transition");
            assert_eq!(
                transition.effects,
                vec![QuestEffect {
                    track: QUEST_LOG.into(),
                    from: QuestComparison::Any,
                    effect: QuestEffectKind::Set(1),
                }]
            );
            assert!(!transition.completes);
            assert_eq!(
                lowered.requested_by(QUEST_LOG_NPC_1),
                Some(&QuestRequestedBy {
                    npc: "canary:npc/tereban_functions".into(),
                    keywords: vec![QuestKeyword::Word("yes".into())],
                    topics: vec![1],
                })
            );
            let changes = lowered
                .catalogue()
                .evaluate(transition, &BTreeMap::new(), 0)
                .expect("first write");
            assert_eq!((changes[0].before, changes[0].after), (0, 1));
        }
    }

    #[test]
    fn computed_expressions_load_and_are_refused_not_supported() {
        let lowered = embedded();
        let document = document();
        let computed = document["quests"]
            .as_array()
            .expect("quests")
            .iter()
            .flat_map(|quest| quest["transitions"].as_array().expect("transitions"))
            .find(|transition| transition["effects"][0]["effect"]["kind"] == "COMPUTED")
            .expect("a computed transition");
        let transition = lowered
            .catalogue()
            .transition(computed["key"].as_str().expect("key"))
            .expect("loaded");
        assert_eq!(
            lowered
                .catalogue()
                .evaluate(transition, &BTreeMap::new(), 0),
            Err(QuestRefusal::NotSupported)
        );
    }

    #[test]
    fn an_inexact_source_guard_is_refused_not_supported() {
        let document = document();
        let (quest, index) = document["quests"]
            .as_array()
            .expect("quests")
            .iter()
            .enumerate()
            .flat_map(|(quest, entry)| {
                let transitions = entry["transitions"].as_array().expect("transitions");
                (0..transitions.len()).map(move |index| (quest, index))
            })
            .find(|&(quest, index)| {
                let effects = document["quests"][quest]["transitions"][index]["effects"]
                    .as_array()
                    .expect("effects");
                effects.iter().any(|effect| effect["from_exact"] == false)
                    && effects
                        .iter()
                        .all(|effect| effect["effect"]["kind"] != "COMPUTED")
            })
            .expect("an inexact transition with closed effects");
        let key = document["quests"][quest]["transitions"][index]["key"]
            .as_str()
            .expect("key")
            .to_owned();
        let evaluate = |lowered: &LoweredQuestState| {
            let transition = lowered.catalogue().transition(&key).expect("loaded");
            lowered
                .catalogue()
                .evaluate(transition, &BTreeMap::new(), 0)
                .err()
        };
        assert_eq!(evaluate(&embedded()), Some(QuestRefusal::NotSupported));

        // The same transition with its guard marked exact is executable again.
        let mut exact = document.clone();
        for effect in exact["quests"][quest]["transitions"][index]["effects"]
            .as_array_mut()
            .expect("effects")
        {
            effect["from_exact"] = true.into();
        }
        let exact = reparse(&exact).expect("exact guard loads");
        assert_ne!(evaluate(&exact), Some(QuestRefusal::NotSupported));
    }

    #[test]
    fn malformed_content_fails_the_whole_load_closed() {
        let mut bad = document();
        bad["schema"] = "OTHER/v1".into();
        assert!(matches!(reparse(&bad), Err(QuestLoadError::Schema(_))));

        let mut bad = document();
        bad["quests"][0]["tracks"][0]["extra"] = 1.into();
        assert!(matches!(reparse(&bad), Err(QuestLoadError::Json(_))));

        let mut bad = document();
        bad["quests"][0]["transitions"][0]["effects"][0]["from"] =
            serde_json::json!({"op": "ELAPSED", "value": 1});
        assert!(matches!(reparse(&bad), Err(QuestLoadError::Comparison(_))));

        let mut bad = document();
        bad["quests"][0]["transitions"][0]["effects"][0]["effect"] =
            serde_json::json!({"kind": "SET"});
        assert!(matches!(reparse(&bad), Err(QuestLoadError::Effect(_))));

        let mut bad = document();
        bad["quests"][0]["tracks"][0]["quest"] = "oteryn:quest.other".into();
        assert!(matches!(
            reparse(&bad),
            Err(QuestLoadError::ForeignRecord(_))
        ));

        let mut bad = document();
        bad["quests"][0]["tracks"][0]["key"] = "canary:quest-progress/leak".into();
        assert!(matches!(
            reparse(&bad),
            Err(QuestLoadError::Catalogue(QuestCatalogueError::InvalidKey(
                _
            )))
        ));

        assert!(matches!(
            parse_quest_state(EMBEDDED, "bad revision"),
            Err(QuestLoadError::Catalogue(
                QuestCatalogueError::InvalidContentRevision
            ))
        ));
    }
    #[test]
    fn liquid_black_native_start_has_both_source_storage_effects() -> Result<(), String> {
        let lowered =
            load_embedded_quest_state("liquid-black-red").map_err(|error| format!("{error:?}"))?;
        let catalogue = lowered.catalogue();
        let transition = catalogue
            .transition("oteryn:quest-transition/liquid-black/visitor-start")
            .ok_or("missing source-qualified Liquid Black start")?;
        let changes = catalogue
            .evaluate(transition, &BTreeMap::new(), 0)
            .map_err(|error| format!("{error:?}"))?;
        assert_eq!(changes.len(), 2);
        assert!(
            changes
                .iter()
                .any(|change| change.track.ends_with("/questline") && change.after == 1)
        );
        assert!(
            changes
                .iter()
                .any(|change| change.track.ends_with("/visitor") && change.after == 1)
        );
        Ok(())
    }
    #[test]
    fn authored_world_tracks_retain_canonical_witness_without_replacing_donor_storage()
    -> Result<(), String> {
        let original = parse_quest_state(EMBEDDED, "content-1").map_err(|e| format!("{e:?}"))?;
        let lowered = load_embedded_quest_state("content-1").map_err(|e| format!("{e:?}"))?;
        for (key, source) in original.source_tracks {
            if lowered.catalogue.track(&key).is_some() {
                assert_eq!(
                    lowered.source_track(&key),
                    Some(source.as_str()),
                    "donor binding {key}"
                );
            }
        }
        for key in [
            "oteryn:quest-progress/questpc7/grimvale/programme",
            "oteryn:quest-progress/questpc7/grimvale/feroxa",
        ] {
            let source = lowered
                .source_track(key)
                .ok_or("canonical source witness")?;
            assert!(source.starts_with("canonical:content/quests/definitions/"));
            assert!(source.contains("#/records/55/definition:sha256:"));
            assert_eq!(
                lowered.catalogue.track(key).ok_or("track")?.quest,
                "oteryn:quest.grimvale_quest"
            );
        }
        Ok(())
    }
}
