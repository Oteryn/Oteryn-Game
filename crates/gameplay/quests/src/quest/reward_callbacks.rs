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
pub struct ItemReward {
    pub item: String,
    pub count: u16,
    #[serde(default)]
    pub contents: Vec<ItemAmount>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OutfitReward {
    pub outfit: String,
    /// Base entitlement is always included. Bits 1 and 2 earn the respective addons.
    pub addons: u8,
}

/// Exact source price settled by the durable fee owner.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MonetaryFee {
    pub gold: u64,
    pub include_bank: bool,
}

/// A bounded, monotone receipt counter for an unlocked repeatable exchange.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RewardCounter {
    pub track: String,
    pub maximum: i64,
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
    pub take: Vec<ItemAmount>,
    /// Gold units settled by the existing carried-coin and Bank fee owner.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fee: Option<MonetaryFee>,
    #[serde(default)]
    pub completes: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub counter: Option<RewardCounter>,
    /// Carried evidence checked under the same locks, without consuming it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub present: Vec<ItemAmount>,
    pub give: Option<ItemReward>,
    pub outfit: Option<OutfitReward>,
    pub achievement: Option<String>,
    pub reply: Vec<String>,
    pub sources: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    schema: u32,
    callbacks: Vec<RewardCallback>,
}

pub fn load() -> Result<Vec<RewardCallback>, String> {
    parse(include_str!(
        "../../../../../content/quests/shared-reward-callbacks.json"
    ))
}

fn parse(text: &str) -> Result<Vec<RewardCallback>, String> {
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
    if document.schema != 1 || document.callbacks.len() > 256 {
        return Err("reward callback bounds".into());
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
            || c.storage.len() > 8
            || c.storage.iter().any(|storage| storage.read_only)
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
            || (c.counter.is_none()
                && c.storage
                    .iter()
                    .all(|s| s.preserve || (s.minimum..=s.maximum).contains(&s.set)))
            || c.counter.as_ref().is_some_and(|counter| {
                !slug(&counter.track, "oteryn:quest-progress/", b"._/:-", 256)
                    || !(1..=1_000_000_000_000).contains(&counter.maximum)
                    || c.completes
                    || tracks.contains(&counter.track)
                    || requires.contains(&counter.track)
            })
            || c.present.len() > 8
            || evidence.len() != c.present.len()
            || c.present
                .iter()
                .any(|s| !slug(&s.item, "oteryn:item.", b"._/:-", 256) || s.count == 0)
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
    Ok(document.callbacks)
}

pub fn extend(catalogue: QuestStateCatalogue) -> Result<QuestStateCatalogue, String> {
    extend_callbacks(catalogue, &load()?)
}

fn extend_callbacks(
    catalogue: QuestStateCatalogue,
    callbacks: &[RewardCallback],
) -> Result<QuestStateCatalogue, String> {
    let mut tracks: Vec<_> = catalogue.tracks.values().cloned().collect();
    let mut transitions: Vec<_> = catalogue.transitions.values().cloned().collect();
    let requires: Vec<_> = callbacks.iter().flat_map(|c| &c.requires).collect();
    for callback in callbacks {
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
        let mut effects: Vec<_> = callback
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
            .collect();
        if let Some(counter) = &callback.counter {
            if !(1..=1_000_000_000_000).contains(&counter.maximum) {
                return Err(format!("invalid reward counter {}", counter.track));
            }
            if let Some(track) = tracks.iter().find(|t| t.key == counter.track) {
                if track.quest != callback.quest
                    || track.initial != 0
                    || track.min != 0
                    || track.max != counter.maximum
                {
                    return Err(format!("foreign reward counter {}", counter.track));
                }
            } else {
                tracks.push(QuestTrack {
                    key: counter.track.clone(),
                    quest: callback.quest.clone(),
                    initial: 0,
                    min: 0,
                    max: counter.maximum,
                });
            }
            effects.push(QuestEffect {
                track: counter.track.clone(),
                from: QuestComparison::Between(0, counter.maximum - 1),
                effect: QuestEffectKind::Add(1),
            });
        }
        transitions.push(QuestTransition {
            key: callback.key.clone(),
            quest: callback.quest.clone(),
            effects,
            completes: callback.completes,
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
    fn service_counter_uses_real_add_owner_and_refuses_at_its_bound() -> Result<(), String> {
        let mut callback = load()?
            .into_iter()
            .find(|c| c.key.ends_with("questpc7-friends-royal-steel"))
            .ok_or("service")?;
        callback.counter.as_mut().ok_or("counter")?.maximum = 2;
        let counter = callback.counter.as_ref().ok_or("counter")?.track.clone();
        let parent = callback.storage[0].track.clone();
        let catalogue = extend_callbacks(
            QuestStateCatalogue::new("counter-test", vec![], vec![])
                .map_err(|e| format!("{e:?}"))?,
            std::slice::from_ref(&callback),
        )?;
        let transition = catalogue.transition(&callback.key).ok_or("transition")?;
        let mut stored = BTreeMap::from([(parent.clone(), 1)]);
        for value in 1..=2 {
            let changes = catalogue
                .evaluate(transition, &stored, 0)
                .map_err(|e| format!("{e:?}"))?;
            let change = changes
                .iter()
                .find(|c| c.track == counter)
                .ok_or("missing counted receipt")?;
            assert_eq!((change.before, change.after), (value - 1, value));
            assert!(
                changes
                    .iter()
                    .any(|c| c.track == parent && c.before == 1 && c.after == 1)
            );
            stored.insert(counter.clone(), value);
        }
        assert!(
            catalogue.evaluate(transition, &stored, 0).is_err(),
            "exhausted counter cannot overflow or consume another input"
        );
        for maximum in [0, 1_000_000_000_001] {
            let mut invalid = callback.clone();
            invalid.counter.as_mut().ok_or("counter")?.maximum = maximum;
            assert!(
                parse(&serde_json::json!({"schema":1,"callbacks":[invalid]}).to_string()).is_err()
            );
        }
        callback.counter.as_mut().ok_or("counter")?.track = parent;
        assert!(
            parse(&serde_json::json!({"schema":1,"callbacks":[callback]}).to_string()).is_err(),
            "counter cannot take over its prerequisite track"
        );
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
        let mut read_only = value.clone();
        read_only["callbacks"][0]["storage"][0]["read_only"] = true.into();
        assert!(
            parse(&read_only.to_string()).is_err(),
            "read-only parent guards belong in requires, never in storage writes"
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
