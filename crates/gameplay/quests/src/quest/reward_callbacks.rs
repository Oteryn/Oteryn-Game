//! Bounded NPC exchanges composed by the fenced quest writer, never by dialogue text.
use super::world_callbacks::Storage;
use super::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack, QuestTransition,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const PROVENANCE: &str = "crystal-9f5a72c-quest-rewards-v1";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ItemAmount {
    pub item: String,
    pub count: u16,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DecayTarget {
    pub family: String,
    pub key: String,
    pub revision: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RewardDecay {
    pub duration_millis: u32,
    pub target: Option<DecayTarget>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ItemReward {
    pub item: String,
    pub count: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decay: Option<RewardDecay>,
    #[serde(default)]
    pub contents: Vec<ItemAmount>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub backpack_outputs: Vec<ItemAmount>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OutfitReward {
    pub outfit: String,
    /// Base entitlement is always included. Bits 1 and 2 earn the respective addons.
    pub addons: u8,
}

/// Trade every carried matching input, rolling once per input on the server.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BulkExchange {
    pub item: String,
    pub output: String,
    pub base: u16,
    pub bonus: u16,
    pub chance_percent: u8,
}

/// Exact source price; only the durable fee owner may consume the funds.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MonetaryFee {
    pub gold: u64,
    pub include_bank: bool,
}

/// Database Unix-second clock effects, executed by the existing QuestState writer.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RewardClock {
    pub track: String,
    pub elapsed_seconds: Option<i64>,
    pub set_now: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RewardCooldown {
    pub track: String,
    pub seconds: u32,
}

/// A read-only prerequisite: its original quest retains ownership and writes.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TrackRequirement {
    pub track: String,
    pub minimum: i64,
    pub maximum: i64,
}

impl TrackRequirement {
    pub fn holds(&self, catalogue: &QuestStateCatalogue, stored: &BTreeMap<String, i64>) -> bool {
        catalogue.track(&self.track).is_some_and(|track| {
            QuestComparison::Between(self.minimum, self.maximum)
                .holds(stored.get(&self.track).copied().unwrap_or(track.initial), 0)
        })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RewardCallback {
    pub key: String,
    pub quest: String,
    pub npc: String,
    pub keywords: Vec<String>,
    pub topic: u16,
    pub next_topic: u16,
    pub storage: Vec<Storage>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub clocks: Vec<RewardClock>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cooldown: Option<RewardCooldown>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requires: Vec<TrackRequirement>,
    pub take: Vec<ItemAmount>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fee: Option<MonetaryFee>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bulk: Option<BulkExchange>,
    /// Carried evidence checked under the same locks, without consuming it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub present: Vec<ItemAmount>,
    pub give: Option<ItemReward>,
    pub outfit: Option<OutfitReward>,
    pub achievement: Option<String>,
    /// A costed item exchange may repeat under a fresh command occurrence.
    #[serde(default, skip_serializing_if = "is_false")]
    pub repeatable: bool,
    /// Costed repeats may retain one idempotent account achievement occurrence.
    #[serde(default, skip_serializing_if = "is_false")]
    pub achievement_once: bool,
    pub reply: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cooldown_reply: Vec<String>,
    pub sources: Vec<String>,
}

impl RewardCallback {
    pub fn clock_effects(&self) -> impl Iterator<Item = RewardClock> + '_ {
        self.clocks
            .iter()
            .cloned()
            .chain(self.cooldown.iter().map(|c| RewardClock {
                track: c.track.clone(),
                elapsed_seconds: Some(i64::from(c.seconds)),
                set_now: true,
            }))
    }
}

fn is_false(value: &bool) -> bool {
    !*value
}

/// Source-bound catalogue input only; this declaration never writes progress.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TrackDeclaration {
    key: String,
    quest: String,
    initial: i64,
    min: i64,
    max: i64,
    sources: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    schema: u32,
    #[serde(default)]
    tracks: Vec<TrackDeclaration>,
    callbacks: Vec<RewardCallback>,
}

pub fn load() -> Result<Vec<RewardCallback>, String> {
    parse(include_str!(
        "../../../../../content/quests/shared-reward-callbacks.json"
    ))
}

fn parse(text: &str) -> Result<Vec<RewardCallback>, String> {
    parse_document(text).map(|document| document.callbacks)
}

fn parse_document(text: &str) -> Result<Document, String> {
    let document: Document = serde_json::from_str(text).map_err(|e| e.to_string())?;
    let mut keys = BTreeSet::new();
    let slug = |value: &str, prefix: &str, alphabet: &[u8], maximum: usize| {
        value.len() <= maximum
            && value.strip_prefix(prefix).is_some_and(|tail| {
                !tail.is_empty()
                    && tail.bytes().all(|b| {
                        b.is_ascii_lowercase() || b.is_ascii_digit() || alphabet.contains(&b)
                    })
            })
    };
    let item =
        |item: &str, count| slug(item, "oteryn:item.", b"._/:-", 256) && (1..=100).contains(&count);
    if document.schema != 1 || document.callbacks.len() > 256 || document.tracks.len() > 256 {
        return Err("reward callback bounds".into());
    }
    let mut declarations = BTreeSet::new();
    for track in &document.tracks {
        if !slug(&track.key, "oteryn:", b"._/:-", 128)
            || !slug(&track.quest, "oteryn:", b"._/:-", 128)
            || !declarations.insert(&track.key)
            || track.min > track.initial
            || track.initial > track.max
            || track.sources.is_empty()
            || track.sources.len() > 8
            || track.sources.iter().any(|s| s.is_empty() || s.len() > 1024)
            || !document.callbacks.iter().any(|c| {
                c.requires.iter().any(|r| r.track == track.key)
                    || c.clock_effects().any(|clock| clock.track == track.key)
            })
            || document
                .callbacks
                .iter()
                .any(|c| c.storage.iter().any(|r| r.track == track.key))
        {
            return Err(format!(
                "invalid reward prerequisite declaration {}",
                track.key
            ));
        }
    }
    for c in &document.callbacks {
        let tracks: BTreeSet<_> = c.storage.iter().map(|s| &s.track).collect();
        let clocks: Vec<_> = c.clock_effects().collect();
        let clock_keys: BTreeSet<_> = clocks.iter().map(|c| &c.track).collect();
        let inputs: BTreeSet<_> = c.take.iter().map(|s| &s.item).collect();
        let evidence: BTreeSet<_> = c.present.iter().map(|s| &s.item).collect();
        let requires: BTreeSet<_> = c.requires.iter().map(|s| &s.track).collect();
        if !slug(&c.key, "oteryn:quest-reward/", b"_/-", 128)
            || !slug(&c.quest, "oteryn:", b"._/:-", 128)
            || !keys.insert(&c.key)
            || !slug(&c.npc, "oteryn:npc.", b"_", 128)
            || c.storage.is_empty()
            || c.storage.len() + clocks.len() > super::QUESTSTATE0_RL_02
            || clocks.len() > 8
            || clock_keys.len() != clocks.len()
            || !clock_keys.is_disjoint(&tracks)
            || !clock_keys.is_disjoint(&requires)
            || clocks.iter().any(|clock| {
                !slug(&clock.track, "oteryn:", b"._/:-", 128)
                    || clock
                        .elapsed_seconds
                        .is_some_and(|seconds| seconds <= 0 || seconds > 31_536_000)
                    || (!clock.set_now && clock.elapsed_seconds.is_none())
            })
            || tracks.len() != c.storage.len()
            || c.requires.len() > 8
            || requires.len() != c.requires.len()
            || !requires.is_disjoint(&tracks)
            || c.requires
                .iter()
                .any(|r| !slug(&r.track, "oteryn:", b"._/:-", 128) || r.minimum > r.maximum)
            || c.storage
                .iter()
                .any(|s| !slug(&s.track, "oteryn:", b"._/:-", 128) || s.minimum > s.maximum)
            || (!c.repeatable
                && c.storage
                    .iter()
                    .all(|s| s.preserve || (s.minimum..=s.maximum).contains(&s.set)))
            || (c.repeatable
                && ((c.take.is_empty() && c.fee.is_none() && c.cooldown.is_none())
                    || c.give.is_none()
                    || c.outfit.is_some()
                    || (c.achievement.is_some() && !c.achievement_once)))
            || (c.achievement_once && (!c.repeatable || c.achievement.is_none()))
            || (c.cooldown.is_some() && !c.repeatable)
            || c.present.len() > 8
            || evidence.len() != c.present.len()
            || c.present
                .iter()
                .any(|s| !slug(&s.item, "oteryn:item.", b"._/:-", 256) || s.count == 0)
            || c.bulk.as_ref().is_some_and(|bulk| {
                !item(&bulk.item, 1)
                    || !item(&bulk.output, bulk.base)
                    || bulk.item == bulk.output
                    || bulk.bonus == 0
                    || !(1..=100).contains(&bulk.chance_percent)
                    || !c.repeatable
                    || c.take.len() != 1
                    || c.take
                        .first()
                        .is_none_or(|take| take.item != bulk.item || take.count != 1)
                    || c.give.as_ref().is_none_or(|give| {
                        give.item != bulk.output
                            || give.count != bulk.base
                            || !give.contents.is_empty()
                            || !give.backpack_outputs.is_empty()
                            || give.decay.is_some()
                    })
                    || c.fee.is_some()
                    || !c.present.is_empty()
                    || c.outfit.is_some()
                    || c.achievement.is_some()
            })
            || c.fee.as_ref().is_some_and(|fee| {
                fee.gold == 0
                    || fee.gold > 1_000_000_000_000
                    || c.take.iter().any(|item| {
                        matches!(
                            item.item.as_str(),
                            "oteryn:item.tibia.i3031"
                                | "oteryn:item.tibia.i3035"
                                | "oteryn:item.tibia.i3043"
                        )
                    })
            })
            || c.take.len() > 8
            || inputs.len() != c.take.len()
            || c.take
                .iter()
                .any(|s| !slug(&s.item, "oteryn:item.", b"._/:-", 256) || s.count == 0)
            || c.give.as_ref().is_some_and(|r| {
                !item(&r.item, r.count)
                    || r.decay.as_ref().is_some_and(|decay| {
                        r.count != 1
                            || decay.duration_millis == 0
                            || !r.backpack_outputs.is_empty()
                            || decay.target.as_ref().is_some_and(|target| {
                                target.family != "Item"
                                    || !slug(&target.key, "oteryn:item.", b"._/:-", 256)
                                    || target.revision.is_empty()
                                    || target.revision.len() > 128
                            })
                    })
                    || (!r.contents.is_empty() && !r.backpack_outputs.is_empty())
                    || r.backpack_outputs.len() > 19
                    || r.backpack_outputs.iter().any(|s| !item(&s.item, s.count))
                    || r.contents.len() > 20
                    || r.contents.iter().any(|s| !item(&s.item, s.count))
            })
            || c.outfit
                .as_ref()
                .is_some_and(|r| !slug(&r.outfit, "oteryn:outfit.", b"_", 160) || r.addons > 3)
            || c.keywords.is_empty()
            || c.keywords.len() > 8
            || c.keywords.iter().collect::<BTreeSet<_>>().len() != c.keywords.len()
            || c.keywords.iter().any(|s| {
                s.is_empty() || s.len() > 64 || !s.is_ascii() || s != &s.to_ascii_lowercase()
            })
            || (!c.cooldown_reply.is_empty()
                && !clocks.iter().any(|clock| clock.elapsed_seconds.is_some()))
            || c.cooldown_reply.len() > 8
            || c.reply.is_empty()
            || c.reply.len() > 8
            || c.reply
                .iter()
                .chain(&c.cooldown_reply)
                .any(|s| s.is_empty() || s.len() > 512 || s.chars().any(char::is_control))
            || c.achievement
                .as_ref()
                .is_some_and(|key| !slug(key, "oteryn:achievement/", b"_/-", 128))
            || c.sources.len() > 8
            || c.sources.iter().any(|s| s.is_empty() || s.len() > 1024)
            || c.sources.is_empty()
        {
            return Err(format!("invalid reward callback {}", c.key));
        }
    }
    Ok(document)
}

pub fn extend(catalogue: QuestStateCatalogue) -> Result<QuestStateCatalogue, String> {
    let mut tracks: Vec<_> = catalogue.tracks.values().cloned().collect();
    let mut transitions: Vec<_> = catalogue.transitions.values().cloned().collect();
    let document = parse_document(include_str!(
        "../../../../../content/quests/shared-reward-callbacks.json"
    ))?;
    for declaration in document.tracks {
        let declared = QuestTrack {
            key: declaration.key,
            quest: declaration.quest,
            initial: declaration.initial,
            min: declaration.min,
            max: declaration.max,
        };
        if let Some(existing) = tracks.iter().find(|t| t.key == declared.key) {
            if existing != &declared {
                return Err(format!(
                    "conflicting reward prerequisite declaration {}",
                    declared.key
                ));
            }
        } else {
            tracks.push(declared);
        }
    }
    let callbacks = document.callbacks;
    let requires: Vec<_> = callbacks.iter().flat_map(|c| &c.requires).collect();
    for callback in &callbacks {
        for s in &callback.storage {
            if let Some(track) = tracks.iter_mut().find(|t| t.key == s.track) {
                if track.quest != callback.quest {
                    return Err(format!("foreign reward track {}", s.track));
                }
                track.max = track.max.max(s.maximum).max(s.set);
                track.min = track.min.min(s.minimum);
            } else {
                tracks.push(QuestTrack {
                    key: s.track.clone(),
                    quest: callback.quest.clone(),
                    initial: -1,
                    min: s.minimum.min(-1).min(s.set),
                    max: s.maximum.max(s.set),
                });
            }
        }
        for clock in callback.clock_effects() {
            if let Some(track) = tracks.iter().find(|track| track.key == clock.track) {
                if track.quest != callback.quest
                    || track.initial != -1
                    || track.min != -1
                    || track.max != i64::MAX
                {
                    return Err(format!("foreign reward clock {}", clock.track));
                }
            } else {
                tracks.push(QuestTrack {
                    key: clock.track,
                    quest: callback.quest.clone(),
                    initial: -1,
                    min: -1,
                    max: i64::MAX,
                });
            }
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
                    effect: if s.preserve {
                        QuestEffectKind::Add(0)
                    } else {
                        QuestEffectKind::Set(s.set)
                    },
                })
                .chain(callback.clock_effects().map(|clock| {
                    QuestEffect {
                        track: clock.track,
                        from: clock
                            .elapsed_seconds
                            .map_or(QuestComparison::Any, QuestComparison::ElapsedAtLeast),
                        effect: if clock.set_now {
                            QuestEffectKind::SetNow
                        } else {
                            QuestEffectKind::Add(0)
                        },
                    }
                }))
                .collect(),
            completes: false,
            experience: None,
        });
    }
    if let Some(required) = requires
        .iter()
        .find(|r| !tracks.iter().any(|t| t.key == r.track))
    {
        return Err(format!("undeclared reward prerequisite {}", required.track));
    }
    QuestStateCatalogue::new(catalogue.content_revision, tracks, transitions)
        .map_err(|e| format!("{e:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workshop_clocks_are_bounded_disjoint_and_required_for_free_repeats() -> Result<(), String> {
        let callback = load()?
            .into_iter()
            .find(|c| c.key == "oteryn:quest-reward/npcfix-ghost-tear")
            .ok_or("tear")?;
        let original = serde_json::to_value(callback).map_err(|e| e.to_string())?;
        let document = |value: &serde_json::Value| {
            serde_json::json!({"schema":1,"callbacks":[value]}).to_string()
        };
        assert!(parse(&document(&original)).is_ok());
        for mutation in 0..6 {
            let mut value = original.clone();
            match mutation {
                0 => {
                    value.as_object_mut().ok_or("object")?.remove("cooldown");
                }
                1 => value["cooldown"]["seconds"] = 0.into(),
                2 => value["cooldown"]["seconds"] = 31_536_001_i64.into(),
                3 => value["cooldown"]["track"] = value["storage"][0]["track"].clone(),
                4 => {
                    value["clocks"] = serde_json::json!([{"track":value["cooldown"]["track"],"elapsed_seconds":72000,"set_now":true}])
                }
                _ => {
                    value["requires"] = serde_json::json!([{"track":value["cooldown"]["track"],"minimum":0,"maximum":1}])
                }
            }
            assert!(parse(&document(&value)).is_err(), "mutation {mutation}");
        }
        Ok(())
    }

    #[test]
    fn repeatable_item_exchange_requires_an_explicit_cost() -> Result<(), String> {
        let callback = load()?.into_iter().next().ok_or("callback")?;
        let mut value = serde_json::to_value(callback).map_err(|e| e.to_string())?;
        value["repeatable"] = true.into();
        value["take"] = serde_json::json!([{"item": "oteryn:item.tibia.i3031", "count": 20}]);
        value["give"] = serde_json::json!({"item": "oteryn:item.tibia.i875", "count": 1});
        value["outfit"] = serde_json::Value::Null;
        value["achievement"] = serde_json::Value::Null;
        for state in value["storage"].as_array_mut().ok_or("storage")? {
            state["preserve"] = true.into();
        }
        let document = |callback: &serde_json::Value| {
            serde_json::json!({"schema": 1, "callbacks": [callback]}).to_string()
        };
        assert!(parse(&document(&value)).is_ok());
        value["take"] = serde_json::json!([]);
        assert!(
            parse(&document(&value)).is_err(),
            "a repeat must consume value"
        );
        Ok(())
    }

    #[test]
    fn costed_achievement_and_deadline_require_explicit_single_item_authority() -> Result<(), String>
    {
        let callback = load()?
            .into_iter()
            .find(|callback| callback.key == "oteryn:quest-reward/npcfix/wandering-crystal")
            .ok_or("crystal callback")?;
        let original = serde_json::to_value(callback).map_err(|error| error.to_string())?;
        let document = |value: &serde_json::Value| {
            serde_json::json!({"schema":1,"callbacks":[value]}).to_string()
        };
        assert!(parse(&document(&original)).is_ok());
        for mutation in 0..7 {
            let mut value = original.clone();
            match mutation {
                0 => value["take"] = serde_json::json!([]),
                1 => value["achievement_once"] = false.into(),
                2 => value["repeatable"] = false.into(),
                3 => {
                    value["outfit"] =
                        serde_json::json!({"outfit":"oteryn:outfit.pirate","addons":0})
                }
                4 => value["give"]["decay"]["duration_millis"] = 0.into(),
                5 => value["give"]["count"] = 2.into(),
                _ => value["give"]["decay"]["target"]["key"] = "not-an-item".into(),
            }
            assert!(parse(&document(&value)).is_err(), "mutation {mutation}");
        }
        Ok(())
    }

    #[test]
    fn reward_storage_supports_one_atomic_sixteen_track_plan() -> Result<(), String> {
        let mut callback = load()?.into_iter().next().ok_or("callback")?;
        callback.storage = (0..16)
            .map(|i| Storage {
                track: format!("oteryn:quest-progress/atomic/{i}"),
                minimum: 0,
                maximum: 0,
                set: 1,
                preserve: false,
            })
            .collect();
        let document = |callback: &RewardCallback| {
            serde_json::json!({"schema": 1, "callbacks": [callback]}).to_string()
        };
        assert!(parse(&document(&callback)).is_ok());
        callback.storage.push(Storage {
            track: "oteryn:quest-progress/atomic/16".into(),
            minimum: 0,
            maximum: 0,
            set: 1,
            preserve: false,
        });
        assert!(parse(&document(&callback)).is_err());
        Ok(())
    }

    #[test]
    fn readonly_prerequisite_uses_declared_initial_and_fresh_stored_value() -> Result<(), String> {
        let catalogue = QuestStateCatalogue::new(
            "test",
            vec![QuestTrack {
                key: "oteryn:parent".into(),
                quest: "oteryn:quest.parent".into(),
                initial: 7,
                min: 0,
                max: 8,
            }],
            vec![],
        )
        .map_err(|e| format!("{e:?}"))?;
        let requirement = TrackRequirement {
            track: "oteryn:parent".into(),
            minimum: 7,
            maximum: 7,
        };
        let mut stored = BTreeMap::new();
        assert!(requirement.holds(&catalogue, &stored));
        stored.insert(requirement.track.clone(), 6);
        assert!(!requirement.holds(&catalogue, &stored));
        stored.insert(requirement.track.clone(), 7);
        assert!(requirement.holds(&catalogue, &stored));
        assert!(
            !TrackRequirement {
                track: "oteryn:missing".into(),
                ..requirement
            }
            .holds(&catalogue, &stored)
        );
        Ok(())
    }

    #[test]
    fn qengine_reward_callbacks_are_bounded_and_never_repeatable_value_grants() -> Result<(), String>
    {
        let source = include_str!("../../../../../content/quests/shared-reward-callbacks.json");
        let callbacks = load()?;
        assert!(callbacks.len() >= 6);
        for edit in 0..6 {
            let mut value: serde_json::Value =
                serde_json::from_str(source).map_err(|e| e.to_string())?;
            match edit {
                0 => {
                    value["callbacks"][0]["take"] =
                        serde_json::json!([{"item":"oteryn:item.tibia.i139","count":0}])
                }
                1 => {
                    value["callbacks"][0]["outfit"] =
                        serde_json::json!({"outfit":"oteryn:outfit.pirate","addons":4})
                }
                2 => value["callbacks"][0]["storage"] = serde_json::json!([]),
                3 => {
                    value["callbacks"][0]["storage"][0]["set"] =
                        value["callbacks"][0]["storage"][0]["minimum"].clone()
                }
                4 => value["callbacks"][0]["unknown"] = true.into(),
                _ => {
                    value["callbacks"][0]["take"] = serde_json::json!([{"item":"oteryn:item.tibia.i139","count":1},{"item":"oteryn:item.tibia.i139","count":1}])
                }
            }
            assert!(parse(&value.to_string()).is_err(), "case {edit}");
        }
        for (field, invalid) in [
            ("npc", "oteryn:npc."),
            ("npc", "oteryn:npc.morgan?"),
            ("quest", "oteryn:quest.BAD"),
            ("key", "oteryn:quest-reward/bad.key"),
        ] {
            let mut value: serde_json::Value =
                serde_json::from_str(source).map_err(|e| e.to_string())?;
            value["callbacks"][0][field] = invalid.into();
            assert!(parse(&value.to_string()).is_err(), "{field}={invalid}");
        }
        let mut value: serde_json::Value =
            serde_json::from_str(source).map_err(|e| e.to_string())?;
        value["callbacks"][0]["take"] =
            serde_json::json!([{"item":"oteryn:item.tibia.i3031","count":1000}]);
        assert!(
            parse(&value.to_string()).is_ok(),
            "inputs may span many stacks"
        );
        for present in [
            serde_json::json!([{"item":"oteryn:item.tibia.i9041","count":0}]),
            serde_json::json!([{"item":"oteryn:item.tibia.i9041","count":1},{"item":"oteryn:item.tibia.i9041","count":1}]),
        ] {
            let mut invalid: serde_json::Value =
                serde_json::from_str(source).map_err(|e| e.to_string())?;
            invalid["callbacks"][0]["present"] = present;
            assert!(parse(&invalid.to_string()).is_err());
        }
        let track = value["callbacks"][0]["storage"][0]["track"].clone();
        for required in [
            serde_json::json!([{"track":track,"minimum":0,"maximum":1}]),
            serde_json::json!([{"track":"oteryn:parent","minimum":2,"maximum":1}]),
            serde_json::json!([{"track":"oteryn:parent","minimum":0,"maximum":1},{"track":"oteryn:parent","minimum":0,"maximum":1}]),
        ] {
            let mut invalid: serde_json::Value =
                serde_json::from_str(source).map_err(|e| e.to_string())?;
            invalid["callbacks"][0]["requires"] = required;
            assert!(parse(&invalid.to_string()).is_err());
        }
        super::super::loader::load_embedded_quest_state("content-1")
            .map_err(|e| format!("{e:?}"))?;
        Ok(())
    }
}

#[cfg(test)]
mod npcfix_tests {
    use super::*;

    #[test]
    fn repeatable_callbacks_require_a_cost_or_a_locked_database_cooldown() -> Result<(), String> {
        let source = include_str!("../../../../../content/quests/shared-reward-callbacks.json");
        for invalid in 0..4 {
            let mut document: serde_json::Value =
                serde_json::from_str(source).map_err(|e| e.to_string())?;
            let callback = document["callbacks"]
                .as_array_mut()
                .ok_or("callbacks")?
                .iter_mut()
                .find(|row| row["key"] == "oteryn:quest-reward/npcfix-ghost-tear")
                .ok_or("cooldown callback")?;
            match invalid {
                0 => {
                    callback["cooldown"] = serde_json::Value::Null;
                }
                1 => {
                    callback["cooldown"] = serde_json::json!({"track":"oteryn:clock","seconds":0});
                }
                2 => {
                    callback["outfit"] =
                        serde_json::json!({"outfit":"oteryn:outfit.pirate","addons":0});
                }
                _ => {
                    callback["give"] = serde_json::Value::Null;
                }
            }
            assert!(parse(&serde_json::to_string(&document).map_err(|e| e.to_string())?).is_err());
        }
        Ok(())
    }

    #[test]
    fn source_cooldown_uses_the_locked_timestamp_and_exact_twenty_hour_boundary()
    -> Result<(), String> {
        let loaded = super::super::loader::load_embedded_quest_state("content-1")
            .map_err(|e| format!("{e:?}"))?;
        let catalogue = loaded.catalogue();
        let transition = catalogue
            .transition("oteryn:quest-reward/npcfix-ghost-tear")
            .ok_or("tear transition")?;
        let clock = transition
            .effects
            .iter()
            .find(|effect| effect.effect == QuestEffectKind::SetNow)
            .ok_or("clock effect")?;
        let mut stored = BTreeMap::from([(clock.track.clone(), 28_001)]);
        assert_eq!(
            catalogue.evaluate(transition, &stored, 100_000),
            Err(super::super::QuestRefusal::StageMismatch)
        );
        stored.insert(clock.track.clone(), 28_000);
        let changes = catalogue
            .evaluate(transition, &stored, 100_000)
            .map_err(|e| format!("{e:?}"))?;
        assert!(
            changes
                .iter()
                .any(|change| change.track == clock.track && change.after == 100_000)
        );
        Ok(())
    }
}
