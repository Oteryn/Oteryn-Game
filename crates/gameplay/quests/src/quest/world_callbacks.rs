//! QUESTCB2: bounded, declarative world callbacks. Coordinates use native negative floors.
pub use super::native_rules::NativeRule;
use super::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack, QuestTransition,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub const EMBEDDED: &str = include_str!("../../../../../content/quests/world-callbacks.json");
// Static callbacks and the dynamic altar share the same boot bound.
pub const MAX_CALLBACKS: usize = 512;
pub const MAX_EFFECTS: usize = 8;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub struct Position {
    pub x: u16,
    pub y: u16,
    pub floor: i8,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Trigger {
    Use,
    StepIn,
    StepOut,
}

mod effects;
pub use effects::{
    CallbackEffects, Encounter, EncounterMonster, EncounterPlayer, Facing, ItemCycle, MagicEffect,
    Portal, PortalPair,
};

mod scene;
pub use scene::{SceneEdit, SceneEdits, SceneEvacuation, SceneRelocation};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Selector {
    pub position: Option<Position>,
    /// Exact native top-level stack ordinal; never a fabricated source UID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ordinal: Option<u16>,
    pub unique: Option<u16>,
    pub action: Option<u16>,
    pub appearance: Option<u16>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Passage {
    pub position: Position,
    pub appearance: u16,
    #[serde(default)]
    pub replacement: Option<u16>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Offering {
    pub position: Position,
    pub item: String,
    pub count: u16,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Storage {
    pub track: String,
    pub minimum: i64,
    pub maximum: i64,
    pub set: i64,
    /// Preserve the track value within a receipt-bearing quest transition.
    #[serde(default, skip_serializing_if = "is_false")]
    pub preserve: bool,
    /// Validate quest progress without writing tracks, state, receipts or revisions.
    #[serde(default, skip_serializing_if = "is_false")]
    pub read_only: bool,
}

fn is_false(value: &bool) -> bool {
    !*value
}

mod native;
use native::valid_native_callback;

/// Creature appearance projected only while this committed Quest track is in range.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Disguise {
    pub track: String,
    pub minimum: i64,
    pub maximum: i64,
    pub creature: String,
}

impl Disguise {
    fn valid_for(&self, callback: &Callback) -> bool {
        self.minimum > 0
            && self.maximum >= self.minimum
            && self.maximum <= i64::from(u16::MAX)
            && self
                .creature
                .strip_prefix("oteryn:creature.")
                .is_some_and(|tail| {
                    !tail.is_empty()
                        && tail
                            .bytes()
                            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_')
                })
            && callback.trigger == Trigger::Use
            && callback.selector.position.is_some()
            && !callback.is_read_only()
            && callback.storage.iter().any(|s| {
                s.track == self.track
                    && !s.preserve
                    && !s.read_only
                    && s.maximum < self.minimum
                    && s.set == self.minimum
            })
    }

    pub fn active(&self, tracks: &std::collections::BTreeMap<String, i64>) -> bool {
        tracks
            .get(&self.track)
            .is_some_and(|value| (self.minimum..=self.maximum).contains(value))
    }
}

/// Ambiguous concurrent disguises fail closed; a registry order never chooses authority.
pub fn active_disguise(
    tracks: &std::collections::BTreeMap<String, i64>,
) -> Result<Option<&'static Disguise>, String> {
    static RULES: std::sync::OnceLock<Result<Vec<Disguise>, String>> = std::sync::OnceLock::new();
    let rules = RULES
        .get_or_init(|| {
            let rules = load()?
                .into_iter()
                .filter_map(|callback| callback.disguise)
                .collect::<Vec<_>>();
            if rules.len() > 32 {
                return Err("quest disguise bounds".into());
            }
            Ok(rules)
        })
        .as_ref()
        .map_err(Clone::clone)?;
    let mut active = rules.iter().filter(|rule| rule.active(tracks));
    let first = active.next();
    if active.next().is_some() {
        return Err("ambiguous quest disguise".into());
    }
    Ok(first)
}

/// One carried item applied to the selected world object.  This deliberately is not a
/// recipe language: the receipt owner locks the source and consumes, changes or retains it.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ItemOnTarget {
    pub item: String,
    pub count: u16,
    pub replacement: Option<String>,
    pub target_replacement: Option<u16>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub reusable: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MonkReward {
    pub item_source_id: Option<u32>,
    pub learn_spell: Option<String>,
    #[serde(default)]
    pub final_shrine: bool,
}

/// A native paid transport receipt, optionally later than a named earned milestone.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TravelRequirement {
    pub route: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after_transition: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Callback {
    pub key: String,
    pub quest: String,
    pub trigger: Trigger,
    pub selector: Selector,
    /// Bounded same-floor tool reach for large authored world targets.
    #[serde(
        default = "default_use_range",
        skip_serializing_if = "is_default_use_range"
    )]
    pub use_range: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disguise: Option<Disguise>,
    /// Both lever appearances. None for a step tile. One-way levers refuse their second use.
    pub lever: Option<[u16; 2]>,
    pub reversible: bool,
    pub passages: Vec<Passage>,
    pub teleport: Option<Position>,
    pub storage: Vec<Storage>,
    /// Checked against the progression owner in the same fenced quest transaction.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minimum_level: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub experience: Option<i64>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub completes: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub premium_required: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub monk_reward: Option<MonkReward>,
    pub offerings: Vec<Offering>,
    /// Carried prerequisites retained under the same Character/custody locks.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub present: Vec<super::reward_callbacks::ItemAmount>,
    /// Completed native transport receipts; no caller can substitute a location for payment.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_travel: Vec<TravelRequirement>,
    #[serde(default)]
    pub item_on_target: Option<ItemOnTarget>,
    pub return_on_failure: bool,
    pub failure_message: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub success_message: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_rule: Option<NativeRule>,
    #[serde(default, skip_serializing_if = "CallbackEffects::is_empty")]
    pub effects: CallbackEffects,
    pub sources: Vec<String>,
}

fn default_use_range() -> u8 {
    1
}
fn is_default_use_range(value: &u8) -> bool {
    *value == 1
}

impl Callback {
    /// Uses the same-floor Chebyshev rule; legacy callbacks retain one-cell reach.
    #[must_use]
    pub fn reaches(&self, actor: Position, target: Position) -> bool {
        actor.floor == target.floor
            && actor.x.abs_diff(target.x) <= u16::from(self.use_range)
            && actor.y.abs_diff(target.y) <= u16::from(self.use_range)
    }

    /// A pure quest-state predicate, with no Quest mutation or receipt.
    #[must_use]
    pub fn is_read_only(&self) -> bool {
        !self.storage.is_empty() && self.storage.iter().all(|storage| storage.read_only)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    schema: u32,
    callbacks: Vec<Callback>,
}

/// Read the same closed content shape at boot and at the durable mutation boundary.
pub fn load() -> Result<Vec<Callback>, String> {
    let mut callbacks = parse(EMBEDDED)?;
    callbacks.extend(parse(include_str!(
        "../../../../../content/quests/shared-access-callbacks.json"
    ))?);
    callbacks.extend(parse(include_str!(
        "../../../../../content/quests/native-scene-callbacks.json"
    ))?);
    callbacks.extend(parse(include_str!(
        "../../../../../content/quests/shared-outfit-callbacks.json"
    ))?);
    callbacks.extend(parse_with_provenance(
        include_str!("../../../../../content/quests/authored-world-callbacks.json"),
        true,
    )?);
    callbacks.retain(|callback| super::in_runtime_scope(&callback.quest));
    if callbacks.len() > MAX_CALLBACKS.saturating_sub(2) {
        return Err("callback bounds including Rotten Blood altar".into());
    }
    callbacks.push(super::rotten_blood::altar_callback());
    callbacks.push(super::schrodinger_island::callback());
    let keys: BTreeSet<_> = callbacks.iter().map(|c| &c.key).collect();
    if keys.len() != callbacks.len() {
        return Err("duplicate callback across registries".into());
    }
    Ok(callbacks)
}

fn parse(text: &str) -> Result<Vec<Callback>, String> {
    parse_with_provenance(text, false)
}

fn parse_with_provenance(text: &str, authored: bool) -> Result<Vec<Callback>, String> {
    let document: Document = serde_json::from_str(text).map_err(|e| e.to_string())?;
    if document.schema != 1 || document.callbacks.len() > MAX_CALLBACKS {
        return Err("callback bounds".into());
    }
    let mut keys = BTreeSet::new();
    for callback in &document.callbacks {
        let selector = &callback.selector;
        let valid_pos = |p: Position| (-15..=0).contains(&p.floor);
        let unique_offerings: BTreeSet<_> = callback.offerings.iter().map(|o| o.position).collect();
        let unique_passages: BTreeSet<_> = callback.passages.iter().map(|p| p.position).collect();
        let unique_storage: BTreeSet<_> = callback.storage.iter().map(|s| &s.track).collect();
        if !super::valid_quest_key(&callback.key)
            || !super::valid_quest_key(&callback.quest)
            || !keys.insert(&callback.key)
            || !(1..=2).contains(&callback.use_range)
            || (callback.use_range > 1
                && (callback.trigger != Trigger::Use
                    || selector.position.is_none()
                    || callback.item_on_target.is_none()
                    || callback.storage.is_empty()
                    || callback.is_read_only()))
            || (selector.position.is_none()
                && selector.unique.is_none()
                && selector.action.is_none())
            || selector.position.is_some_and(|p| !valid_pos(p))
            || (selector.ordinal.is_some() && selector.position.is_none())
            || (callback.trigger != Trigger::Use
                && selector.ordinal.is_some_and(|ordinal| ordinal != 0))
            || callback.teleport.is_some_and(|p| !valid_pos(p))
            || !effects::valid(callback)
            || callback
                .disguise
                .as_ref()
                .is_some_and(|rule| !rule.valid_for(callback))
            || !effects::valid(callback)
            || unique_offerings.len() != callback.offerings.len()
            || unique_passages.len() != callback.passages.len()
            || unique_storage.len() != callback.storage.len()
            || (callback.storage.iter().any(|storage| storage.read_only)
                && (!callback.is_read_only()
                    || !matches!(callback.trigger, Trigger::Use | Trigger::StepIn)
                    || callback.teleport.is_none()
                    || callback.lever.is_some()
                    || !callback.passages.is_empty()
                    || !callback.offerings.is_empty()
                    || callback.item_on_target.is_some()
                    || callback.minimum_level.is_some()
                    || callback.premium_required
                    || callback.experience.is_some()
                    || callback.monk_reward.is_some()
                    || callback.native_rule.is_some()
                    || callback.storage.iter().any(|storage| storage.preserve)))
            || callback.passages.len() > MAX_EFFECTS
            || callback.storage.len() > MAX_EFFECTS
            || callback
                .minimum_level
                .is_some_and(|level| level == 0 || callback.storage.is_empty())
            || callback.present.len() > MAX_EFFECTS
            || callback
                .present
                .iter()
                .map(|item| &item.item)
                .collect::<BTreeSet<_>>()
                .len()
                != callback.present.len()
            || callback.present.iter().any(|item| {
                !item.item.starts_with("oteryn:item.")
                    || !super::valid_quest_key(&item.item)
                    || !(1..=100).contains(&item.count)
            })
            || callback.required_travel.len() > 4
            || callback
                .required_travel
                .iter()
                .map(|requirement| &requirement.route)
                .collect::<BTreeSet<_>>()
                .len()
                != callback.required_travel.len()
            || callback.required_travel.iter().any(|requirement| {
                let route = &requirement.route;
                !route.starts_with("oteryn:service.travel.")
                    || !super::valid_quest_key(route)
                    || !route.contains('/')
                    || route.len() > 192
                    || requirement.after_transition.as_ref().is_some_and(|key| {
                        !key.starts_with("oteryn:quest-callback/") || !super::valid_quest_key(key)
                    })
            })
            || ((!callback.present.is_empty() || !callback.required_travel.is_empty())
                && (callback.storage.is_empty() || callback.is_read_only()))
            || callback.offerings.len() > MAX_EFFECTS
            || callback
                .experience
                .is_some_and(|xp| !(1..=super::QUESTGATE0_RL_05).contains(&xp))
            || callback.monk_reward.as_ref().is_some_and(|reward| {
                callback.quest != super::monk_shrines::QUEST
                    || callback.trigger != Trigger::Use
                    || callback.storage.is_empty()
                    || reward.item_source_id.is_some_and(|id| {
                        ![50267, 50271, 50269, 50273, 50274, 50272, 50268].contains(&id)
                    })
                    || reward.learn_spell.as_deref().is_some_and(|key| {
                        ![
                            "candidate:spell/mystic_repulse",
                            "candidate:spell/forceful_uppercut",
                            "candidate:spell/focus_harmony",
                        ]
                        .contains(&key)
                    })
            })
            || (!callback.offerings.is_empty() && callback.storage.is_empty())
            || callback.sources.is_empty()
            || (authored
                && (!callback.key.starts_with("oteryn:quest-callback/authored-")
                    || !callback
                        .sources
                        .iter()
                        .any(|source| source.starts_with("oteryn-authored:"))))
            || !callback.sources.iter().any(|source| {
                // Dryad Garden uses the pinned Crystal donor; the earlier callbacks use Canary.
                (authored && source.starts_with("oteryn-authored:"))
                    || source.starts_with("canary:47dfd51f45280a59a1d3e50ba7edd573d7234446:")
                    || source.starts_with("crystal:201189f871d23597b82caf88fd611454a2ddefb8:")
                    || (callback.effects.outfit_reward.is_some()
                        && source.starts_with("crystal:9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d:"))
            })
            || callback.item_on_target.as_ref().is_some_and(|item| {
                item.target_replacement == Some(0)
                    || (item.replacement.is_some() && item.count != 1)
                    || (item.reusable && (item.count != 1 || item.replacement.is_some()))
                    || callback.storage.is_empty()
                    || callback.trigger != Trigger::Use
                    || item.count == 0
                    || item.count > 100
                    || !super::valid_quest_key(&item.item)
                    || item
                        .replacement
                        .as_ref()
                        .is_some_and(|key| !super::valid_quest_key(key))
            })
            || callback
                .failure_message
                .as_ref()
                .is_some_and(|s| s.is_empty() || s.len() > 256 || s.contains(['\n', '\r', '\0']))
            || callback
                .success_message
                .as_ref()
                .is_some_and(|s| s.is_empty() || s.len() > 256 || s.contains(['\n', '\r', '\0']))
            || callback
                .native_rule
                .is_some_and(|rule| !valid_native_callback(callback, rule))
            || callback
                .lever
                .is_some_and(|p| p[0] == p[1] || callback.trigger != Trigger::Use)
            || callback.passages.iter().any(|p| !valid_pos(p.position))
            || callback.offerings.iter().any(|o| {
                !valid_pos(o.position)
                    || o.count == 0
                    || o.count > 100
                    || !super::valid_quest_key(&o.item)
            })
            || callback
                .storage
                .iter()
                .any(|s| !super::valid_quest_key(&s.track) || s.minimum > s.maximum)
        {
            return Err(format!("invalid callback {}", callback.key));
        }
    }
    Ok(document.callbacks)
}

pub(super) fn extend(catalogue: QuestStateCatalogue) -> Result<QuestStateCatalogue, String> {
    let callbacks = load()?;
    extend_callbacks(catalogue, &callbacks)
}

fn extend_callbacks(
    catalogue: QuestStateCatalogue,
    callbacks: &[Callback],
) -> Result<QuestStateCatalogue, String> {
    let mut tracks: Vec<_> = catalogue.tracks.values().cloned().collect();
    let mut transitions: Vec<_> = catalogue.transitions.values().cloned().collect();
    for callback in callbacks.iter().filter(|c| !c.storage.is_empty()) {
        for storage in &callback.storage {
            if let Some(track) = tracks.iter_mut().find(|track| track.key == storage.track) {
                if callback.native_rule.is_none() && !storage.preserve && !storage.read_only {
                    track.max = track.max.max(storage.set).max(storage.maximum);
                }
            } else {
                if callback.native_rule.is_some() {
                    return Err(format!(
                        "native callback {} requires an existing track {}",
                        callback.key, storage.track
                    ));
                }
                tracks.push(QuestTrack {
                    key: storage.track.clone(),
                    quest: callback.quest.clone(),
                    initial: -1,
                    min: -1,
                    max: if storage.preserve || storage.read_only {
                        storage.maximum
                    } else {
                        storage.set.max(storage.maximum)
                    },
                });
            }
        }
        // Pure gates must not acquire a generic transition: even ADD(0)
        // creates a Character revision, quest state and receipt.
        if callback.is_read_only() {
            continue;
        }
        if callback.effects.item_reward.is_some() || callback.completes || callback.premium_required
        {
            let digest: String =
                Sha256::digest(serde_json::to_vec(&callback).map_err(|e| e.to_string())?)
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect();
            transitions.push(QuestTransition {
                key: format!("oteryn:quest-world-plan/{digest}"),
                quest: callback.quest.clone(),
                effects: vec![QuestEffect {
                    track: callback
                        .storage
                        .first()
                        .ok_or("missing value guard")?
                        .track
                        .clone(),
                    from: QuestComparison::Any,
                    effect: QuestEffectKind::Computed,
                }],
                completes: false,
                experience: None,
            });
        }
        transitions.push(QuestTransition {
            key: callback.key.clone(),
            quest: callback.quest.clone(),
            effects: callback
                .storage
                .iter()
                .map(|s| QuestEffect {
                    track: s.track.clone(),
                    from: QuestComparison::Between(s.minimum, s.maximum),
                    effect: match callback.native_rule {
                        Some(rule) => QuestEffectKind::Native(rule),
                        None if s.preserve => QuestEffectKind::Add(0),
                        None => QuestEffectKind::Set(s.set),
                    },
                })
                .collect(),
            completes: callback.completes,
            experience: callback.experience,
        });
    }
    QuestStateCatalogue::new(catalogue.content_revision, tracks, transitions)
        .map_err(|e| format!("{e:?}"))
}

#[cfg(test)]
mod batch4_tests;

#[cfg(test)]
mod scene_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qengine_shared_access_rejects_ambiguous_or_unbounded_effects() -> Result<(), String> {
        let source: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../content/quests/shared-access-callbacks.json"
        ))
        .map_err(|e| e.to_string())?;
        parse(&source.to_string())?;
        let callbacks = source["callbacks"].as_array().ok_or("callbacks")?;
        let index = |name: &str| {
            callbacks
                .iter()
                .position(|c| c["key"].as_str().is_some_and(|k| k.ends_with(name)))
                .ok_or(name.to_owned())
        };
        let portals = index("thais-lighthouse-portals")?;
        let encounter = index("annihilator-entry")?;
        let cycle = index("triangle-tower-lever")?;
        for case in 0..8 {
            let mut bad = source.clone();
            match case {
                0 => {
                    bad["callbacks"][portals]["effects"]["portals"]["portals"][1]["position"] = bad
                        ["callbacks"][portals]["effects"]["portals"]["portals"][0]["position"]
                        .clone()
                }
                1 => bad["callbacks"][portals]["effects"]["portals"]["lifetime_us"] = 0.into(),
                2 => {
                    bad["callbacks"][encounter]["effects"]["encounter"]["players"] =
                        serde_json::json!([])
                }
                3 => {
                    bad["callbacks"][encounter]["effects"]["encounter"]["players"][1]["to"] =
                        bad["callbacks"][encounter]["effects"]["encounter"]["players"][0]["to"]
                            .clone()
                }
                4 => {
                    bad["callbacks"][encounter]["effects"]["encounter"]["monsters"][0]["position"]
                        ["floor"] = 1.into()
                }
                5 => bad["callbacks"][cycle]["effects"]["item_cycle"]["create"] = 0.into(),
                6 => bad["callbacks"][cycle]["effects"]["facing"] = "north".into(),
                _ => bad["callbacks"][cycle]["effects"]["unknown"] = true.into(),
            }
            assert!(
                parse(&bad.to_string()).is_err(),
                "invalid effect case {case}"
            );
        }
        Ok(())
    }

    #[test]
    fn authored_world_provenance_cannot_enter_the_pinned_source_registry() -> Result<(), String> {
        let source = include_str!("../../../../../content/quests/authored-world-callbacks.json");
        assert!(parse(source).is_err());
        parse_with_provenance(source, true)?;
        let mut value: serde_json::Value =
            serde_json::from_str(source).map_err(|e| e.to_string())?;
        value["callbacks"][0]["key"] = "oteryn:quest-callback/source-masquerade".into();
        assert!(parse_with_provenance(&value.to_string(), true).is_err());
        Ok(())
    }

    #[test]
    fn lane_qengine_shared_access_rejects_ambiguous_or_unbounded_effects() -> Result<(), String> {
        let source: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../content/quests/shared-access-callbacks.json"
        ))
        .map_err(|e| e.to_string())?;
        parse(&source.to_string())?;
        let callbacks = source["callbacks"].as_array().ok_or("callbacks")?;
        let index = |name: &str| {
            callbacks
                .iter()
                .position(|c| c["key"].as_str().is_some_and(|k| k.ends_with(name)))
                .ok_or(name.to_owned())
        };
        let portals = index("thais-lighthouse-portals")?;
        let encounter = index("annihilator-entry")?;
        let cycle = index("triangle-tower-lever")?;
        for case in 0..8 {
            let mut bad = source.clone();
            match case {
                0 => {
                    bad["callbacks"][portals]["effects"]["portals"]["portals"][1]["position"] = bad
                        ["callbacks"][portals]["effects"]["portals"]["portals"][0]["position"]
                        .clone()
                }
                1 => bad["callbacks"][portals]["effects"]["portals"]["lifetime_us"] = 0.into(),
                2 => {
                    bad["callbacks"][encounter]["effects"]["encounter"]["players"] =
                        serde_json::json!([])
                }
                3 => {
                    bad["callbacks"][encounter]["effects"]["encounter"]["players"][1]["to"] =
                        bad["callbacks"][encounter]["effects"]["encounter"]["players"][0]["to"]
                            .clone()
                }
                4 => {
                    bad["callbacks"][encounter]["effects"]["encounter"]["monsters"][0]["position"]
                        ["floor"] = 1.into()
                }
                5 => bad["callbacks"][cycle]["effects"]["item_cycle"]["create"] = 0.into(),
                6 => bad["callbacks"][cycle]["effects"]["facing"] = "north".into(),
                _ => bad["callbacks"][cycle]["effects"]["unknown"] = true.into(),
            }
            assert!(
                parse(&bad.to_string()).is_err(),
                "invalid effect case {case}"
            );
        }
        Ok(())
    }

    #[test]
    fn read_only_world_callback_has_no_quest_write_transition() -> Result<(), String> {
        let held: serde_json::Value = serde_json::from_str(include_str!("../../../../../tools/content-schema/quest-authoring/batch3_c/held-world/the_order_of_the_stag.json")).map_err(|e| e.to_string())?;
        let callbacks = parse(&serde_json::json!({"schema": 1, "callbacks": [held["callbacks"][1].clone(), held["callbacks"][3].clone()]}).to_string())?;
        let catalogue = extend_callbacks(
            QuestStateCatalogue::empty("content-1").ok_or("catalogue")?,
            &callbacks,
        )?;
        for callback in callbacks {
            assert!(
                catalogue.transition(&callback.key).is_none(),
                "read-only USE acquired a write transition"
            );
            assert!(catalogue.track(&callback.storage[0].track).is_some());
        }
        Ok(())
    }

    #[test]
    fn read_only_world_callbacks_reject_mixed_mutations() -> Result<(), String> {
        let held: serde_json::Value = serde_json::from_str(include_str!("../../../../../tools/content-schema/quest-authoring/batch3_c/held-world/the_order_of_the_stag.json")).map_err(|e| e.to_string())?;
        for case in 0..4 {
            let mut callback = held["callbacks"][1].clone();
            match case {
                0 => callback["experience"] = 1.into(),
                1 => callback["storage"][0]["preserve"] = true.into(),
                2 => callback["trigger"] = "step_in".into(),
                _ => callback["teleport"] = serde_json::Value::Null,
            }
            assert!(
                parse(&serde_json::json!({"schema": 1, "callbacks": [callback]}).to_string())
                    .is_err()
            );
        }
        Ok(())
    }

    #[test]
    fn dryad_garden_uses_source_unique_ids_in_both_directions() -> Result<(), String> {
        let callbacks = load()?;
        for (suffix, unique, destination) in [
            (
                "enter",
                33331,
                Position {
                    x: 33202,
                    y: 32012,
                    floor: -11,
                },
            ),
            (
                "leave",
                33332,
                Position {
                    x: 33264,
                    y: 32012,
                    floor: -7,
                },
            ),
        ] {
            let key = format!("oteryn:quest-callback/dryad-garden-{suffix}");
            let callback = callbacks.iter().find(|c| c.key == key).ok_or(key)?;
            assert_eq!(callback.trigger, Trigger::Use);
            assert_eq!(callback.selector.unique, Some(unique));
            assert_eq!(callback.teleport, Some(destination));
            assert!(callback.storage.is_empty());
            assert!(callback.sources.iter().all(|source| {
                source.starts_with("crystal:201189f871d23597b82caf88fd611454a2ddefb8:")
            }));
        }
        Ok(())
    }

    #[test]
    fn quest_world_callbacks_accept_only_pinned_donor_revisions() -> Result<(), String> {
        let mut source: serde_json::Value =
            serde_json::from_str(EMBEDDED).map_err(|e| e.to_string())?;
        source["callbacks"]
            .as_array_mut()
            .ok_or("callbacks")?
            .truncate(1);
        for (provenance, accepted) in [
            (
                "canary:47dfd51f45280a59a1d3e50ba7edd573d7234446:script.lua:1",
                true,
            ),
            (
                "crystal:201189f871d23597b82caf88fd611454a2ddefb8:script.lua:1",
                true,
            ),
            ("crystal:unverified:script.lua:1", false),
            (
                "canary:201189f871d23597b82caf88fd611454a2ddefb8:script.lua:1",
                false,
            ),
            (
                "crystal:47dfd51f45280a59a1d3e50ba7edd573d7234446:script.lua:1",
                false,
            ),
            (
                "crystal:201189f871d23597b82caf88fd611454a2ddefb80:script.lua:1",
                false,
            ),
            ("tibiawiki:1203780", false),
        ] {
            source["callbacks"][0]["sources"] = serde_json::json!([provenance]);
            assert_eq!(parse(&source.to_string()).is_ok(), accepted, "{provenance}");
        }
        Ok(())
    }

    #[test]
    fn quest_world_callbacks_shortcut_preserves_the_source_storage_gate() -> Result<(), String> {
        let lowered = super::super::loader::load_embedded_quest_state("content-1")
            .map_err(|e| format!("{e:?}"))?;
        let c = load()?
            .into_iter()
            .find(|c| c.key.ends_with("liquid-black-shortcut"))
            .ok_or("shortcut")?;
        let catalogue = lowered.catalogue();
        let transition = catalogue.transition(&c.key).ok_or("transition")?;
        assert_eq!(
            catalogue.evaluate(transition, &Default::default(), 0),
            Err(super::super::QuestRefusal::StageMismatch)
        );
        for value in [4, 5] {
            let changes = catalogue
                .evaluate(
                    transition,
                    &std::collections::BTreeMap::from([(c.storage[0].track.clone(), value)]),
                    0,
                )
                .map_err(|e| format!("{e:?}"))?;
            assert_eq!((changes[0].before, changes[0].after), (value, 5));
        }
        Ok(())
    }

    #[test]
    fn quest_world_callbacks_reject_ambiguous_and_unbounded_authoring() -> Result<(), String> {
        let source: serde_json::Value =
            serde_json::from_str(EMBEDDED).map_err(|e| e.to_string())?;
        let callbacks = source["callbacks"].as_array().ok_or("callbacks")?;
        let offerings = callbacks
            .iter()
            .position(|c| c["key"] == "oteryn:quest-callback/paradox-skulls")
            .ok_or("offerings callback")?;
        let item = callbacks
            .iter()
            .position(|c| c["key"] == "oteryn:quest-callback/fish-napping-key")
            .ok_or("item callback")?;
        for case in 0..10 {
            let mut bad = source.clone();
            match case {
                0 => {
                    bad["callbacks"][0]["selector"] = serde_json::json!({"position":null,"unique":null,"action":null,"appearance":null})
                }
                1 => bad["callbacks"][0]["extra"] = true.into(),
                2 => {
                    bad["callbacks"][offerings]["offerings"][1] =
                        bad["callbacks"][offerings]["offerings"][0].clone()
                }
                3 => bad["callbacks"][offerings]["offerings"][0]["count"] = 101.into(),
                4 => bad["callbacks"][item]["item_on_target"]["target_replacement"] = 0.into(),
                5 => bad["callbacks"][0]["sources"] = serde_json::json!(["crystal:unverified"]),
                6 => bad["callbacks"][item]["item_on_target"]["replacement"] = "bad".into(),
                7 => {
                    bad["callbacks"][item]["item_on_target"]["replacement"] =
                        "oteryn:item.tibia.i3114".into();
                    bad["callbacks"][item]["item_on_target"]["count"] = 2.into();
                }
                8 => bad["callbacks"][item]["storage"] = serde_json::json!([]),
                _ => bad["callbacks"][0]["failure_message"] = "x".repeat(257).into(),
            }
            assert!(parse(&bad.to_string()).is_err(), "case {case}");
        }
        let mut bounded = source.clone();
        bounded["callbacks"] = serde_json::json!([]);
        for index in 0..MAX_CALLBACKS {
            let mut callback = source["callbacks"][0].clone();
            callback["key"] = format!("oteryn:quest-callback/bounded-{index}").into();
            bounded["callbacks"]
                .as_array_mut()
                .ok_or("callbacks")?
                .push(callback);
        }
        assert!(parse(&bounded.to_string()).is_ok());
        let mut extra = source["callbacks"][0].clone();
        extra["key"] = "oteryn:quest-callback/overflow".into();
        bounded["callbacks"]
            .as_array_mut()
            .ok_or("callbacks")?
            .push(extra);
        assert!(parse(&bounded.to_string()).is_err());
        for field in ["passages", "storage", "offerings"] {
            let mut bounded = source.clone();
            let entries = (0..MAX_EFFECTS).map(|index| match field {
                "passages" => serde_json::json!({"position":{"x":index,"y":1,"floor":-1},"appearance":1791}),
                "storage" => serde_json::json!({"track":format!("oteryn:quest-progress/bounded-{index}"),"minimum":-1,"maximum":1,"set":1}),
                _ => serde_json::json!({"position":{"x":index,"y":1,"floor":-1},"item":"oteryn:item.tibia.i3114","count":100}),
            }).collect::<Vec<_>>();
            bounded["callbacks"][offerings][field] = serde_json::json!(entries);
            assert!(parse(&bounded.to_string()).is_ok(), "{field} at its bound");
            let mut extra = bounded["callbacks"][offerings][field][0].clone();
            if field == "storage" {
                extra["track"] = "oteryn:quest-progress/overflow".into();
            } else {
                extra["position"]["x"] = 100.into();
            }
            bounded["callbacks"][offerings][field]
                .as_array_mut()
                .ok_or("entries")?
                .push(extra);
            assert!(
                parse(&bounded.to_string()).is_err(),
                "{field} past its bound"
            );
        }
        let mut bounded = source;
        bounded["callbacks"][0]["failure_message"] = "x".repeat(256).into();
        assert!(parse(&bounded.to_string()).is_ok());
        Ok(())
    }

    #[test]
    fn quest_world_callbacks_three_bounded_action_classes_are_source_witnessed()
    -> Result<(), String> {
        let callbacks = load()?;
        let lowered = super::super::loader::load_embedded_quest_state("content-1")
            .map_err(|e| format!("{e:?}"))?;
        for callback in callbacks.iter().filter(|c| {
            c.item_on_target.is_some()
                || c.key.ends_with("outlaw-camp-door")
                || c.key.ends_with("travelling-trader-delivery")
        }) {
            for storage in &callback.storage {
                assert!(
                    lowered.source_track(&storage.track).is_some(),
                    "{}",
                    storage.track
                );
                assert_eq!(
                    lowered
                        .catalogue()
                        .track(&storage.track)
                        .ok_or("track")?
                        .quest,
                    callback.quest
                );
            }
        }
        let outlaw = callbacks
            .iter()
            .find(|callback| callback.key.ends_with("outlaw-camp-door"))
            .ok_or("outlaw door")?;
        assert_eq!(
            (outlaw.selector.action, outlaw.lever),
            (Some(1108), Some([1642, 1643]))
        );
        assert_eq!(
            (outlaw.storage[0].minimum, outlaw.storage[0].maximum),
            (3, 3)
        );

        let key = callbacks
            .iter()
            .find(|callback| callback.key.ends_with("fish-napping-key"))
            .ok_or("fish key")?;
        let item = key.item_on_target.as_ref().ok_or("item target")?;
        assert_eq!((&*item.item, item.count), ("oteryn:item.tibia.i7934", 1));
        assert_eq!(key.selector.action, Some(51394));

        let door = callbacks
            .iter()
            .find(|callback| callback.key.ends_with("travelling-trader-delivery"))
            .ok_or("trader delivery")?;
        assert_eq!(
            (
                door.storage[0].minimum,
                door.storage[0].maximum,
                door.storage[0].set
            ),
            (3, 3, 4)
        );
        assert_eq!(door.selector.unique, Some(3020));
        Ok(())
    }

    /// PAR-W15: lever/step callbacks against Crystal `201189f8`
    /// (`quests/behemoth/action-lever.lua`, `others/actions_hellgate_lever.lua`,
    /// `liquid_black/movements_shortcut.lua`). Oteryn floors are stored as `-z`.
    #[test]
    fn w15_crystal_world_callback_parity() -> Result<(), String> {
        let callbacks = load()?;
        let find = |suffix: &str| {
            callbacks
                .iter()
                .find(|c| c.key.ends_with(suffix))
                .ok_or(suffix.to_string())
        };
        let row = |x: u16, y: u16, floor: i8, n: u16| -> Vec<Position> {
            (0..n).map(|i| Position { x: x + i, y, floor }).collect()
        };
        // Behemoth: uid 30009, one-way lever 2772 -> 2773, removes wall 1791 at
        // 33295..=33299, 31677, 15. MATCH.
        let behemoth = find("behemoth")?;
        assert_eq!(behemoth.trigger, Trigger::Use);
        assert_eq!(behemoth.selector.unique, Some(30009));
        assert_eq!(
            (behemoth.lever, behemoth.reversible),
            (Some([2772, 2773]), false)
        );
        assert_eq!(
            behemoth
                .passages
                .iter()
                .map(|p| p.position)
                .collect::<Vec<_>>(),
            row(33295, 31677, -15, 5)
        );
        assert!(
            behemoth
                .passages
                .iter()
                .all(|p| p.appearance == 1791 && p.replacement.is_none())
        );
        // Hellgate bridge: aid 50027, reversible lever, water 622 <-> bridge 5770 at
        // 32627..=32629, 31699, 10. MATCH. DIFF: Crystal also moves a creature standing on
        // the bridge to 32630, 31699, 10; the Oteryn record has no creature push.
        let hellgate = find("hellgate-bridge")?;
        assert_eq!(hellgate.selector.action, Some(50027));
        assert_eq!(
            (hellgate.lever, hellgate.reversible),
            (Some([2772, 2773]), true)
        );
        assert_eq!(
            hellgate
                .passages
                .iter()
                .map(|p| p.position)
                .collect::<Vec<_>>(),
            row(32627, 31699, -10, 3)
        );
        assert!(
            hellgate
                .passages
                .iter()
                .all(|p| p.appearance == 622 && p.replacement == Some(5770))
        );
        assert_eq!(hellgate.teleport, None);
        // Liquid Black shortcut: aid 57746 step-in, Visitor >= 4 sets 5 and teleports to
        // 33478, 31314, 7, otherwise pushes back. MATCH (the track's maximum is 5).
        let shortcut = find("liquid-black-shortcut")?;
        assert_eq!(shortcut.trigger, Trigger::StepIn);
        assert_eq!(shortcut.selector.action, Some(57746));
        assert_eq!(
            shortcut.teleport,
            Some(Position {
                x: 33478,
                y: 31314,
                floor: -7
            })
        );
        assert_eq!(
            (
                shortcut.storage[0].minimum,
                shortcut.storage[0].set,
                shortcut.return_on_failure
            ),
            (4, 5, true)
        );
        // MISSING: the Annihilator lever (Crystal `annihilator.lua`, uid 30015).
        assert!(callbacks.iter().all(|c| c.selector.unique != Some(30015)));
        Ok(())
    }
    #[test]
    fn qengine_l1_world_outfit_sources_are_narrowly_admitted() -> Result<(), String> {
        let source = include_str!("../../../../../content/quests/shared-outfit-callbacks.json");
        assert_eq!(parse(source)?.len(), 2);
        for edit in 0..3 {
            let mut value: serde_json::Value =
                serde_json::from_str(source).map_err(|e| e.to_string())?;
            match edit {
                0 => value["callbacks"][0]["effects"]["outfit_reward"] = serde_json::Value::Null,
                1 => value["callbacks"][0]["effects"]["outfit_reward"]["addons"] = 4.into(),
                _ => {
                    for state in value["callbacks"][0]["storage"]
                        .as_array_mut()
                        .ok_or("storage")?
                    {
                        state["set"] = state["minimum"].clone();
                    }
                }
            }
            assert!(
                parse(&value.to_string()).is_err(),
                "invalid wardrobe edit {edit}"
            );
        }
        Ok(())
    }
    #[test]
    fn lane_qengine_l1_world_outfit_sources_are_narrowly_admitted() -> Result<(), String> {
        let source = include_str!("../../../../../content/quests/shared-outfit-callbacks.json");
        assert_eq!(parse(source)?.len(), 2);
        for edit in 0..3 {
            let mut value: serde_json::Value =
                serde_json::from_str(source).map_err(|e| e.to_string())?;
            match edit {
                0 => value["callbacks"][0]["effects"]["outfit_reward"] = serde_json::Value::Null,
                1 => value["callbacks"][0]["effects"]["outfit_reward"]["addons"] = 4.into(),
                _ => {
                    for state in value["callbacks"][0]["storage"]
                        .as_array_mut()
                        .ok_or("storage")?
                    {
                        state["set"] = state["minimum"].clone();
                    }
                }
            }
            assert!(
                parse(&value.to_string()).is_err(),
                "invalid wardrobe edit {edit}"
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod reusable_tool_tests {
    use super::*;

    #[test]
    fn reusable_tools_require_one_item_and_forbid_inventory_replacement() -> Result<(), String> {
        let source = include_str!("../../../../../content/quests/authored-world-callbacks.json");
        let valid: serde_json::Value = serde_json::from_str(source).map_err(|e| e.to_string())?;
        let callbacks = parse_with_provenance(source, true)?;
        let index = callbacks
            .iter()
            .position(|callback| callback.key == "oteryn:quest-callback/authored-life-ring-pick")
            .ok_or("pick")?;
        assert!(
            callbacks[index]
                .item_on_target
                .as_ref()
                .ok_or("tool")?
                .reusable
        );
        for case in 0..3 {
            let mut invalid = valid.clone();
            let tool = &mut invalid["callbacks"][index]["item_on_target"];
            match case {
                0 => tool["count"] = 0.into(),
                1 => tool["count"] = 2.into(),
                _ => tool["replacement"] = "oteryn:item.tibia.i3031".into(),
            }
            assert!(
                parse_with_provenance(&invalid.to_string(), true).is_err(),
                "invalid reusable tool case {case}"
            );
        }
        // The absent default remains absent in old callback receipt hashes.
        let mut legacy = valid;
        legacy["callbacks"][index]["item_on_target"]["reusable"] = false.into();
        let parsed = parse_with_provenance(&legacy.to_string(), true)?;
        let encoded = serde_json::to_value(parsed[index].item_on_target.as_ref().ok_or("tool")?)
            .map_err(|e| e.to_string())?;
        assert!(encoded.get("reusable").is_none());
        Ok(())
    }
}

#[cfg(test)]
mod tool_tests;

#[cfg(test)]
mod disguise_tests {
    use super::*;
    #[test]
    fn quest_disguise_requires_an_exact_earning_transition_and_bounded_native_identity()
    -> Result<(), String> {
        let mut callback = load()?
            .into_iter()
            .find(|c| c.key == "oteryn:quest-callback/authored-venore-machine-9")
            .ok_or("fixture callback")?;
        let rule = Disguise {
            track: callback.storage[0].track.clone(),
            minimum: 10,
            maximum: 12,
            creature: "oteryn:creature.percht".into(),
        };
        assert!(rule.valid_for(&callback));
        let legacy = serde_json::to_vec(&callback).map_err(|e| e.to_string())?;
        callback.disguise = None;
        assert_eq!(
            serde_json::to_vec(&callback).map_err(|e| e.to_string())?,
            legacy
        );
        for mut bad in [rule.clone(), rule.clone(), rule.clone(), rule.clone()]
            .into_iter()
            .enumerate()
        {
            match bad.0 {
                0 => bad.1.minimum = 0,
                1 => bad.1.maximum = 9,
                2 => bad.1.maximum = 65536,
                _ => bad.1.creature = "client:creature/percht".into(),
            }
            assert!(!bad.1.valid_for(&callback));
        }
        callback.storage[0].preserve = true;
        assert!(!rule.valid_for(&callback));
        let mut tracks = std::collections::BTreeMap::new();
        for value in [9, 10, 12, 13] {
            tracks.insert(rule.track.clone(), value);
            assert_eq!(rule.active(&tracks), (10..=12).contains(&value));
        }
        Ok(())
    }
}
