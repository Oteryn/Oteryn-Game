//! Bounded NPC exchanges composed by the fenced quest writer, never by dialogue text.
use super::world_callbacks::Storage;
use super::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack, QuestTransition,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

mod inventory;
pub use inventory::InventoryRequirement;

pub const PROVENANCE: &str = "crystal-9f5a72c-quest-rewards-v1";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ItemAmount {
    pub item: String,
    pub count: u16,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ItemReward {
    pub item: String,
    pub count: u16,
    #[serde(default)]
    pub contents: Vec<ItemAmount>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub backpack_outputs: Vec<ItemAmount>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decay: Option<ItemDecay>,
}

/// Full source lifetime; initialization uses the existing absolute-deadline owner.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ItemDecay {
    pub duration_millis: u32,
    pub target: Option<DecayTarget>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DecayTarget {
    pub key: String,
    pub revision: String,
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
    pub requires: Vec<TrackRequirement>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inventory: Vec<InventoryRequirement>,
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
    /// Source greetings can conclude without retaining NPC focus.
    #[serde(default, skip_serializing_if = "is_false")]
    pub close_after: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub failure_reply: Vec<String>,
    /// Rechecked under the same character lock as the exchange.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minimum_level: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub experience: Option<i64>,
    pub reply: Vec<String>,
    pub sources: Vec<String>,
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

/// One bounded complete output layout, shared by NPC and world participants.
pub(crate) fn valid_item_reward(r: &ItemReward) -> bool {
    let item = |key: &str, count: u16| {
        key.len() <= 256
            && key.strip_prefix("oteryn:item.").is_some_and(|tail| {
                !tail.is_empty()
                    && tail.bytes().all(|b| {
                        b.is_ascii_lowercase() || b.is_ascii_digit() || b"._/:-".contains(&b)
                    })
            })
            && (1..=100).contains(&count)
    };
    item(&r.item, r.count)
        && r.contents.len() <= 20
        && (r.contents.is_empty() || r.backpack_outputs.is_empty())
        && r.backpack_outputs.len() <= 19
        && r.contents
            .iter()
            .chain(&r.backpack_outputs)
            .all(|s| item(&s.item, s.count))
        && r.decay.as_ref().is_none_or(|decay| {
            decay.duration_millis > 0
                && r.count == 1
                && r.contents.is_empty()
                && r.backpack_outputs.is_empty()
                && decay.target.as_ref().is_none_or(|target| {
                    item(&target.key, 1)
                        && !target.revision.is_empty()
                        && target.revision.len() <= 512
                })
        })
}

fn is_false(value: &bool) -> bool {
    !*value
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
            || !document
                .callbacks
                .iter()
                .any(|c| c.requires.iter().any(|r| r.track == track.key))
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
        let inputs: BTreeSet<_> = c.take.iter().map(|s| &s.item).collect();
        let evidence: BTreeSet<_> = c.present.iter().map(|s| &s.item).collect();
        let requires: BTreeSet<_> = c.requires.iter().map(|s| &s.track).collect();
        if !slug(&c.key, "oteryn:quest-reward/", b"_/-", 128)
            || !slug(&c.quest, "oteryn:", b"._/:-", 128)
            || !keys.insert(&c.key)
            || !slug(&c.npc, "oteryn:npc.", b"_", 128)
            || c.storage.is_empty()
            || c.storage.len() > super::QUESTSTATE0_RL_02
            || tracks.len() != c.storage.len()
            || c.requires.len() > 8
            || requires.len() != c.requires.len()
            || !requires.is_disjoint(&tracks)
            || c.requires
                .iter()
                .any(|r| !slug(&r.track, "oteryn:", b"._/:-", 128) || r.minimum > r.maximum)
            || c.storage.iter().any(|s| {
                !slug(&s.track, "oteryn:", b"._/:-", 128)
                    || s.minimum > s.maximum
                    || s.read_only
                    || s.set_if.is_some_and(|value| {
                        !(s.minimum..=s.maximum).contains(&value) || s.preserve
                    })
            })
            || (!c.repeatable
                && c.storage
                    .iter()
                    .all(|s| s.preserve || (s.minimum..=s.maximum).contains(&s.set)))
            || (c.repeatable
                && ((c.take.is_empty() && c.fee.is_none())
                    || c.give.is_none()
                    || c.outfit.is_some()
                    || c.achievement.is_some()))
            || c.inventory.len() > 8
            || c.inventory
                .iter()
                .map(|r| &r.item)
                .collect::<BTreeSet<_>>()
                .len()
                != c.inventory.len()
            || c.inventory
                .iter()
                .any(|r| !slug(&r.item, "oteryn:item.", b"._/:-", 256) || r.minimum > r.maximum)
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
                    || c.experience.is_some()
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
            || c.give.as_ref().is_some_and(|r| !valid_item_reward(r))
            || c.outfit
                .as_ref()
                .is_some_and(|r| !slug(&r.outfit, "oteryn:outfit.", b"_", 160) || r.addons > 3)
            || c.keywords.is_empty()
            || c.keywords.len() > 8
            || c.keywords.iter().collect::<BTreeSet<_>>().len() != c.keywords.len()
            || c.keywords.iter().any(|s| {
                s.is_empty() || s.len() > 64 || !s.is_ascii() || s != &s.to_ascii_lowercase()
            })
            || c.minimum_level == Some(0)
            || c.experience
                .is_some_and(|xp| !(1..=super::QUESTGATE0_RL_05).contains(&xp))
            || c.failure_reply.len() > 8
            || c.failure_reply
                .iter()
                .any(|s| s.is_empty() || s.len() > 512 || s.chars().any(char::is_control))
            || c.reply.is_empty()
            || c.reply.len() > 8
            || c.reply
                .iter()
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
                        s.set_if.map_or(QuestEffectKind::Set(s.set), |equals| {
                            QuestEffectKind::SetIf {
                                equals,
                                value: s.set,
                            }
                        })
                    },
                })
                .collect(),
            completes: false,
            experience: callback.experience,
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
    fn reward_storage_supports_one_atomic_sixteen_track_plan() -> Result<(), String> {
        let mut callback = load()?.into_iter().next().ok_or("callback")?;
        callback.storage = (0..16)
            .map(|i| Storage {
                track: format!("oteryn:quest-progress/atomic/{i}"),
                minimum: 0,
                maximum: 0,
                set: 1,
                set_if: None,
                preserve: false,
                read_only: false,
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
            set_if: None,
            preserve: false,
            read_only: false,
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
        for edit in 0..8 {
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
                6 => value["callbacks"][0]["storage"][0]["read_only"] = true.into(),
                7 => value["callbacks"][0]["storage"][0]["set_if"] = 1.into(),
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
        for edit in 0..3 {
            let mut invalid: serde_json::Value =
                serde_json::from_str(source).map_err(|e| e.to_string())?;
            match edit {
                0 => {
                    let duplicate = invalid["tracks"][0].clone();
                    invalid["tracks"]
                        .as_array_mut()
                        .ok_or("tracks")?
                        .push(duplicate);
                }
                1 => invalid["tracks"][0]["initial"] = serde_json::json!(i64::MAX),
                _ => invalid["tracks"][0]["key"] = serde_json::json!("oteryn:unused"),
            }
            assert!(
                parse(&invalid.to_string()).is_err(),
                "invalid declaration {edit}"
            );
        }
        super::super::loader::load_embedded_quest_state("content-1")
            .map_err(|e| format!("{e:?}"))?;
        Ok(())
    }
}
