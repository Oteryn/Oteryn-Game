//! Bounded NPC exchanges composed by the fenced quest writer, never by dialogue text.
use super::world_callbacks::Storage;
use super::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack, QuestTransition,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

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
}

/// Shared bounded item shape used by NPC and literal world reward participants.
pub(crate) fn valid_item_reward(reward: &ItemReward) -> bool {
    let valid = |item: &str, count: u16| {
        super::valid_quest_key(item)
            && item.starts_with("oteryn:item.")
            && (1..=100).contains(&count)
    };
    valid(&reward.item, reward.count)
        && (reward.contents.is_empty() || reward.backpack_outputs.is_empty())
        && reward.backpack_outputs.len() <= 19
        && reward
            .backpack_outputs
            .iter()
            .all(|item| valid(&item.item, item.count))
        && reward.contents.len() <= 20
        && (reward.contents.is_empty() || reward.count == 1)
        && reward
            .contents
            .iter()
            .all(|item| valid(&item.item, item.count))
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OutfitReward {
    pub outfit: String,
    /// Base entitlement is always included. Bits 1 and 2 earn the respective addons.
    pub addons: u8,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RewardCallback {
    pub key: String,
    pub quest: String,
    pub npc: String,
    /// Shared QENGINE2 selected-instance USE binding; never executable by NPC talk.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub carried_item: Option<String>,
    /// USE must name this callback's live canonical NPC in immediate reach.
    #[serde(default, skip_serializing_if = "is_false")]
    pub carried_target_npc: bool,
    pub keywords: Vec<String>,
    pub topic: u16,
    pub next_topic: u16,
    pub storage: Vec<Storage>,
    pub take: Vec<ItemAmount>,
    /// Carried evidence checked under the same locks, without consuming it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub present: Vec<ItemAmount>,
    pub give: Option<ItemReward>,
    pub outfit: Option<OutfitReward>,
    pub achievement: Option<String>,
    /// Checked from the locked durable progression row before any value effect.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minimum_level: Option<u32>,
    /// Current account Premium is checked by its existing transaction participant.
    #[serde(default, skip_serializing_if = "is_false")]
    pub premium_required: bool,
    /// Uses the existing quest XP obligation, never a separate reward writer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub experience: Option<i64>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub completes: bool,
    pub reply: Vec<String>,
    pub sources: Vec<String>,
}

fn is_false(value: &bool) -> bool {
    !value
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
        if !slug(&c.key, "oteryn:quest-reward/", b"_/-", 128)
            || !slug(&c.quest, "oteryn:", b"._/:-", 128)
            || !keys.insert(&c.key)
            || !slug(&c.npc, "oteryn:npc.", b"_", 128)
            || (c.carried_target_npc && c.carried_item.is_none())
            || c.carried_item.as_ref().is_some_and(|key| {
                c.take
                    != [ItemAmount {
                        item: key.clone(),
                        count: 1,
                    }]
                    || c.storage.is_empty()
                    || c.give.is_some()
            })
            || c.storage.is_empty()
            || c.storage.len() > 8
            || tracks.len() != c.storage.len()
            || c.storage
                .iter()
                .any(|s| !slug(&s.track, "oteryn:", b"._/:-", 128) || s.minimum > s.maximum)
            || c.storage
                .iter()
                .all(|s| s.preserve || (s.minimum..=s.maximum).contains(&s.set))
            || c.present.len() > 8
            || evidence.len() != c.present.len()
            || c.present
                .iter()
                .any(|s| !slug(&s.item, "oteryn:item.", b"._/:-", 256) || s.count == 0)
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
            || c.minimum_level
                .is_some_and(|level| !(1..=65535).contains(&level))
            || c.experience
                .is_some_and(|amount| !(1..=super::QUESTGATE0_RL_05).contains(&amount))
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
    let mut tracks: Vec<_> = catalogue.tracks.values().cloned().collect();
    let mut transitions: Vec<_> = catalogue.transitions.values().cloned().collect();
    for callback in load()? {
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
        if callback.premium_required || callback.carried_item.is_some() {
            let digest: String =
                Sha256::digest(serde_json::to_vec(&callback).map_err(|e| e.to_string())?)
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect();
            transitions.push(QuestTransition {
                key: format!("oteryn:quest-reward-plan/{digest}"),
                quest: callback.quest.clone(),
                effects: vec![QuestEffect {
                    track: callback
                        .storage
                        .first()
                        .ok_or("missing reward progress guard")?
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
            key: callback.key,
            quest: callback.quest,
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
                .collect(),
            completes: callback.completes,
            experience: callback.experience,
        });
    }
    QuestStateCatalogue::new(catalogue.content_revision, tracks, transitions)
        .map_err(|e| format!("{e:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn carried_npc_tools_require_exact_input_and_cannot_be_value_exchanges() -> Result<(), String> {
        let original = load()?
            .into_iter()
            .find(|c| c.key == "oteryn:quest-reward/noodles-recovered")
            .ok_or("carried callback")?;
        for edit in 0..5 {
            let mut row = serde_json::to_value(&original).map_err(|e| e.to_string())?;
            match edit {
                0 => row["carried_item"] = "oteryn:item.tibia.i14157".into(),
                1 => row["take"] = serde_json::json!([]),
                2 => row["take"][0]["count"] = 2.into(),
                3 => {
                    row["give"] = serde_json::json!({"item":"oteryn:item.tibia.i3035","count":1,"contents":[]})
                }
                _ => {
                    row.as_object_mut()
                        .ok_or("callback")?
                        .remove("carried_item");
                }
            }
            assert!(
                parse(&serde_json::json!({"schema":1,"callbacks":[row]}).to_string()).is_err(),
                "edit{edit}"
            );
        }
        Ok(())
    }

    #[test]
    fn reward_experience_and_level_bounds_preserve_legacy_serialization() -> Result<(), String> {
        let source = include_str!("../../../../../content/quests/shared-reward-callbacks.json");
        let mut value: serde_json::Value =
            serde_json::from_str(source).map_err(|e| e.to_string())?;
        for (field, bad) in [
            ("experience", 0),
            ("experience", -1),
            ("experience", super::super::QUESTGATE0_RL_05 + 1),
            ("minimum_level", 0),
            ("minimum_level", 65536),
        ] {
            let mut invalid = value.clone();
            invalid["callbacks"][0][field] = bad.into();
            assert!(parse(&invalid.to_string()).is_err(), "{field}={bad}");
        }
        for amount in [1, super::super::QUESTGATE0_RL_05] {
            value["callbacks"][0]["experience"] = amount.into();
            assert!(parse(&value.to_string()).is_ok());
        }
        let legacy: serde_json::Value = serde_json::from_str(source).map_err(|e| e.to_string())?;
        for row in legacy["callbacks"].as_array().ok_or("callbacks")? {
            if row["key"]
                .as_str()
                .is_some_and(|key| key.starts_with("oteryn:quest-reward/newhaven-"))
            {
                continue;
            }
            let parsed: RewardCallback =
                serde_json::from_value(row.clone()).map_err(|e| e.to_string())?;
            assert_eq!(
                serde_json::to_value(parsed).map_err(|e| e.to_string())?,
                *row,
                "defaulted fields must not change old durable reward plans"
            );
        }
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
        super::super::loader::load_embedded_quest_state("content-1")
            .map_err(|e| format!("{e:?}"))?;
        Ok(())
    }
}
