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

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OutfitReward {
    pub outfit: String,
    /// Base entitlement is always included. Bits 1 and 2 earn the respective addons.
    pub addons: u8,
}

/// Exact source price, consumed only by the existing durable coin fee owner.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MonetaryFee {
    pub gold: u64,
    pub include_bank: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RewardCallback {
    pub key: String,
    pub quest: String,
    pub npc: String,
    /// Native USE must select this exact carried definition; NPC text cannot invoke it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub carried_item: Option<String>,
    /// Positive carried costs permit a fresh command occurrence of this exchange.
    #[serde(default, skip_serializing_if = "is_false")]
    pub repeatable: bool,
    pub keywords: Vec<String>,
    pub topic: u16,
    pub next_topic: u16,
    pub storage: Vec<Storage>,
    /// Awarded by the existing fenced Quest XP obligation owner.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub experience: Option<i64>,
    pub take: Vec<ItemAmount>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fee: Option<MonetaryFee>,
    /// Carried evidence checked under the same locks, without consuming it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub present: Vec<ItemAmount>,
    /// At least one complete carried requirement must hold; inputs remain untouched.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub present_any: Vec<ItemAmount>,
    /// Read-only canonical durable equipment slots, checked under the Character root lock.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub equipped_slots: Vec<u8>,
    /// A repeated source check may never grant or consume durable value.
    #[serde(default, skip_serializing_if = "is_false")]
    pub repeatable_check: bool,
    pub give: Option<ItemReward>,
    pub outfit: Option<OutfitReward>,
    pub achievement: Option<String>,
    /// A canonical blessing Service, granted through the existing blessing owner.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blessing: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub premium: bool,
    /// Current locked progression must satisfy this source requirement.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minimum_level: Option<u32>,
    /// Source health loss, staged and installed by the existing live vitals owner.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub health_loss: Option<u32>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub completes: bool,
    pub reply: Vec<String>,
    pub sources: Vec<String>,
}

/// The same bounded complete Item output is admitted for NPC and world rewards.
pub(crate) fn valid_item_reward(reward: &ItemReward) -> bool {
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
    item(&reward.item, reward.count)
        && reward.contents.len() <= 20
        && reward.backpack_outputs.len() <= 19
        && (reward.contents.is_empty() || reward.backpack_outputs.is_empty())
        && reward
            .contents
            .iter()
            .chain(&reward.backpack_outputs)
            .all(|entry| item(&entry.item, entry.count))
}

fn is_false(value: &bool) -> bool {
    !*value
}

fn reward_binding(callback: &RewardCallback) -> Result<[u8; 32], String> {
    // Speech and source citations are not durable reward terms. Every value and
    // admission participant is: changing one must invalidate an old quest pin.
    let mut bytes = serde_json::to_vec(&(
        &callback.key,
        &callback.quest,
        &callback.npc,
        &callback.keywords,
        callback.topic,
        callback.next_topic,
        &callback.storage,
        &callback.take,
        &callback.present,
        &callback.give,
        &callback.outfit,
        &callback.achievement,
        &callback.blessing,
        callback.experience,
        callback.premium,
        callback.completes,
    ))
    .map_err(|error| error.to_string())?;
    // Preserve every published binding for callbacks without a level gate.
    if let Some(level) = callback.minimum_level {
        bytes.extend_from_slice(b"\0minimum_level\0");
        bytes.extend_from_slice(&level.to_be_bytes());
    }
    let mut digest = Sha256::new();
    digest.update(bytes);
    // Empty optional predicates retain the original binding for unrelated
    // callbacks. Nonempty terms extend it in a disjoint domain (JSON contains
    // no literal NUL), so old in-flight receipts keep their exact definitions.
    if !callback.equipped_slots.is_empty() || callback.repeatable_check {
        digest.update(b"\0quest-reward-equipment-check-v1\0");
        digest.update(
            serde_json::to_vec(&(&callback.equipped_slots, callback.repeatable_check))
                .map_err(|error| error.to_string())?,
        );
    }
    if let Some(amount) = callback.health_loss {
        digest.update(b"\0quest-reward-health-loss-v1\0");
        digest.update(amount.to_be_bytes());
    }
    if !callback.present_any.is_empty() {
        digest.update(b"\0quest-reward-present-any-v1\0");
        digest
            .update(serde_json::to_vec(&callback.present_any).map_err(|error| error.to_string())?);
    }
    if let Some(fee) = &callback.fee {
        digest.update(b"\0quest-reward-fee-v1\0");
        digest.update(serde_json::to_vec(fee).map_err(|error| error.to_string())?);
    }
    if let Some(item) = &callback.carried_item {
        digest.update(b"\0quest-reward-carried-use-v1\0");
        digest.update(item.as_bytes());
    }
    if callback.repeatable {
        digest.update(b"\0quest-reward-repeatable-v1\0");
    }
    Ok(digest.finalize().into())
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
    if document.schema != 1 || document.callbacks.len() > 256 {
        return Err("reward callback bounds".into());
    }
    for c in &document.callbacks {
        let tracks: BTreeSet<_> = c.storage.iter().map(|s| &s.track).collect();
        let inputs: BTreeSet<_> = c.take.iter().map(|s| &s.item).collect();
        let evidence: BTreeSet<_> = c.present.iter().map(|s| &s.item).collect();
        let alternatives: BTreeSet<_> = c.present_any.iter().map(|s| &s.item).collect();
        if !slug(&c.key, "oteryn:quest-reward/", b"_/-", 128)
            || !slug(&c.quest, "oteryn:", b"._/:-", 128)
            || !keys.insert(&c.key)
            || !slug(&c.npc, "oteryn:npc.", b"_", 128)
            || c.carried_item.as_ref().is_some_and(|item| {
                !c.take
                    .iter()
                    .any(|take| &take.item == item && take.count == 1)
                    || c.fee.is_some()
                    || c.health_loss.is_some()
            })
            || c.storage.is_empty()
            || c.storage.len() > 8
            || tracks.len() != c.storage.len()
            || c.storage
                .iter()
                .any(|s| !slug(&s.track, "oteryn:", b"._/:-", 128) || s.minimum > s.maximum)
            || (!c.repeatable_check
                && !c.repeatable
                && c.storage
                    .iter()
                    .all(|s| s.preserve || (s.minimum..=s.maximum).contains(&s.set)))
            || (c.repeatable
                && (c.take.is_empty()
                    || c.give.is_none()
                    || c.fee.is_some()
                    || c.outfit.is_some()
                    || c.achievement.is_some()
                    || c.blessing.is_some()
                    || c.experience.is_some()
                    || c.health_loss.is_some()
                    || c.completes
                    || c.repeatable_check))
            || (c.repeatable_check
                && (!c.take.is_empty()
                    || c.fee.is_some()
                    || c.give.is_some()
                    || c.outfit.is_some()
                    || c.achievement.is_some()
                    || c.blessing.is_some()
                    || c.experience.is_some()
                    || c.health_loss.is_some()
                    || c.completes))
            || (c.repeatable_check
                && c.storage
                    .iter()
                    .any(|s| !s.preserve && (s.minimum != s.maximum || s.set != s.minimum)))
            || c.equipped_slots.len() > 9
            || c.equipped_slots.iter().collect::<BTreeSet<_>>().len() != c.equipped_slots.len()
            || c.equipped_slots
                .iter()
                .any(|slot| !(1..=10).contains(slot) || *slot == 9)
            || c.minimum_level == Some(0)
            || c.health_loss == Some(0)
            || c.experience
                .is_some_and(|value| !(1..=super::QUESTGATE0_RL_05).contains(&value))
            || c.present.len() > 8
            || evidence.len() != c.present.len()
            || c.present
                .iter()
                .any(|s| !slug(&s.item, "oteryn:item.", b"._/:-", 256) || s.count == 0)
            || c.present_any.len() > 8
            || alternatives.len() != c.present_any.len()
            || c.present_any
                .iter()
                .any(|s| !slug(&s.item, "oteryn:item.", b"._/:-", 256) || s.count == 0)
            || c.take.len() > 8
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
            || c.reply.is_empty()
            || c.reply.len() > 8
            || c.reply
                .iter()
                .any(|s| s.is_empty() || s.len() > 512 || s.chars().any(char::is_control))
            || c.achievement
                .as_ref()
                .is_some_and(|key| !slug(key, "oteryn:achievement/", b"_/-", 128))
            || c.blessing
                .as_ref()
                .is_some_and(|key| !slug(key, "oteryn:service.blessing.", b"_", 128))
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
        let binding = reward_binding(&callback)?;
        let bound_effect = callback
            .storage
            .iter()
            .position(|s| !s.preserve)
            .ok_or("reward needs a changed state")?;
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
            key: callback.key,
            quest: callback.quest,
            effects: callback
                .storage
                .iter()
                .enumerate()
                .map(|(index, s)| QuestEffect {
                    track: s.track.clone(),
                    from: QuestComparison::Between(s.minimum, s.maximum),
                    effect: if s.preserve {
                        QuestEffectKind::Add(0)
                    } else if index == bound_effect {
                        QuestEffectKind::SetWithBinding {
                            value: s.set,
                            binding,
                        }
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
    fn carried_exchanges_bind_selection_and_refuse_free_or_unselected_inputs() -> Result<(), String>
    {
        let callback = load()?
            .into_iter()
            .find(|c| c.carried_item.is_some())
            .ok_or("carried exchange")?;
        let document =
            |c: &RewardCallback| serde_json::json!({"schema":1,"callbacks":[c]}).to_string();
        assert!(parse(&document(&callback)).is_ok());
        let mut other = callback.clone();
        other.carried_item = callback
            .take
            .iter()
            .find(|take| Some(&take.item) != callback.carried_item.as_ref())
            .map(|take| take.item.clone());
        assert!(other.carried_item.is_some());
        assert!(parse(&document(&other)).is_ok());
        assert_ne!(reward_binding(&callback)?, reward_binding(&other)?);
        for mutation in 0..5 {
            let mut invalid = callback.clone();
            match mutation {
                0 => invalid.take.clear(),
                1 => invalid.give = None,
                2 => invalid.carried_item = Some("oteryn:item.tibia.i3270".into()),
                3 => invalid.take.iter_mut().for_each(|take| take.count = 2),
                _ => {
                    invalid.fee = Some(MonetaryFee {
                        gold: 1,
                        include_bank: false,
                    })
                }
            }
            assert!(
                parse(&document(&invalid)).is_err(),
                "carried exchange mutation {mutation}"
            );
        }
        Ok(())
    }
    #[test]
    fn legacy_callbacks_keep_their_existing_reward_binding() -> Result<(), String> {
        // Frozen before equipment/repeatable-check fields existed. Empty defaults
        // must not invalidate unrelated in-flight reward definitions and receipts.
        let text = r#"{"schema":1,"callbacks":[{"key":"oteryn:quest-reward/test","quest":"oteryn:quest.test","npc":"oteryn:npc.test","keywords":["yes"],"topic":0,"next_topic":0,"storage":[{"track":"oteryn:track","minimum":3,"maximum":3,"set":4}],"take":[],"give":null,"outfit":null,"achievement":null,"reply":["Reply."],"sources":["fixture"]}]}"#;
        let callback = parse(text)?.into_iter().next().ok_or("callback")?;
        assert_eq!(
            reward_binding(&callback)?,
            [
                207, 41, 110, 39, 135, 189, 244, 75, 82, 120, 174, 202, 42, 96, 136, 154, 182, 250,
                189, 203, 104, 136, 202, 26, 238, 6, 117, 167, 127, 82, 231, 202
            ]
        );
        Ok(())
    }

    #[test]
    fn monetary_fees_are_bounded_source_bound_and_excluded_from_value_free_checks()
    -> Result<(), String> {
        let callback = load()?
            .into_iter()
            .find(|c| c.fee.is_some())
            .ok_or("paid callback")?;
        let document =
            |c: &RewardCallback| serde_json::json!({"schema":1,"callbacks":[c]}).to_string();
        let original = reward_binding(&callback)?;
        let mut changed = callback.clone();
        changed.fee.as_mut().ok_or("fee")?.gold += 1;
        assert_ne!(reward_binding(&changed)?, original);
        assert!(parse(&document(&changed)).is_ok());
        changed = callback.clone();
        changed.fee.as_mut().ok_or("fee")?.include_bank = true;
        assert_ne!(reward_binding(&changed)?, original);
        changed = callback.clone();
        changed.fee = None;
        assert_ne!(reward_binding(&changed)?, original);
        for gold in [0, 1_000_000_000_001] {
            changed = callback.clone();
            changed.fee.as_mut().ok_or("fee")?.gold = gold;
            assert!(parse(&document(&changed)).is_err());
        }
        changed = callback.clone();
        changed.take.push(ItemAmount {
            item: "oteryn:item.tibia.i3031".into(),
            count: 1,
        });
        assert!(parse(&document(&changed)).is_err());
        changed = callback;
        changed.repeatable_check = true;
        changed.give = None;
        changed.outfit = None;
        changed.achievement = None;
        changed.blessing = None;
        changed.experience = None;
        changed.health_loss = None;
        changed.completes = false;
        changed.take.clear();
        for storage in &mut changed.storage {
            storage.preserve = true;
        }
        assert!(parse(&document(&changed)).is_err());
        changed.fee = None;
        assert!(parse(&document(&changed)).is_ok());
        Ok(())
    }
    #[test]
    fn alternative_evidence_is_bounded_distinct_positive_and_content_bound() -> Result<(), String> {
        let callback = load()?
            .into_iter()
            .find(|callback| !callback.present_any.is_empty())
            .ok_or("alternative callback")?;
        let document = |callback: &RewardCallback| {
            serde_json::json!({"schema": 1, "callbacks": [callback]}).to_string()
        };
        let original = reward_binding(&callback)?;
        let mut changed = callback.clone();
        changed.present_any[0].count += 1;
        assert_ne!(reward_binding(&changed)?, original);
        assert!(parse(&document(&changed)).is_ok());
        changed.present_any[0].count = 0;
        assert!(parse(&document(&changed)).is_err());
        changed = callback.clone();
        changed.present_any.push(changed.present_any[0].clone());
        assert!(parse(&document(&changed)).is_err());
        changed.present_any = (1..=9)
            .map(|id| ItemAmount {
                item: format!("oteryn:item.tibia.i{id}"),
                count: 1,
            })
            .collect();
        assert!(parse(&document(&changed)).is_err());
        changed = callback.clone();
        changed.present_any[0].item = "unqualified-input".into();
        assert!(parse(&document(&changed)).is_err());
        changed.present_any.clear();
        assert_ne!(reward_binding(&changed)?, original);
        assert!(parse(&document(&changed)).is_ok());
        Ok(())
    }

    #[test]
    fn health_loss_is_positive_once_only_and_content_bound() -> Result<(), String> {
        let mut callback = load()?
            .into_iter()
            .find(|callback| callback.health_loss.is_some())
            .ok_or("health callback")?;
        let original = reward_binding(&callback)?;
        callback.health_loss = Some(21);
        assert_ne!(reward_binding(&callback)?, original);
        let document = |callback: &RewardCallback| {
            serde_json::json!({"schema": 1, "callbacks": [callback]}).to_string()
        };
        callback.health_loss = Some(0);
        assert!(parse(&document(&callback)).is_err());
        callback.health_loss = Some(20);
        callback.repeatable_check = true;
        for storage in &mut callback.storage {
            storage.maximum = storage.minimum;
            storage.set = storage.minimum;
        }
        assert!(parse(&document(&callback)).is_err());
        callback.health_loss = None;
        assert!(parse(&document(&callback)).is_ok());
        Ok(())
    }

    #[test]
    fn repeatable_equipment_checks_are_bounded_value_free_and_content_bound() -> Result<(), String>
    {
        let mut callback = load()?.into_iter().next().ok_or("callback")?;
        callback.take.clear();
        callback.give = None;
        callback.outfit = None;
        callback.achievement = None;
        callback.experience = None;
        callback.blessing = None;
        callback.completes = false;
        callback.repeatable_check = true;
        callback.equipped_slots = vec![4];
        for storage in &mut callback.storage {
            storage.preserve = false;
            storage.set = storage.minimum;
            storage.maximum = storage.minimum;
        }
        let accepts = |c: &RewardCallback| {
            let value = serde_json::json!({"schema":1, "callbacks":[c]});
            parse(&value.to_string()).is_ok()
        };
        assert!(accepts(&callback));
        let mut level_bound = callback.clone();
        level_bound.minimum_level = Some(0);
        assert!(!accepts(&level_bound));
        level_bound.minimum_level = Some(30);
        assert!(accepts(&level_bound));
        assert_ne!(reward_binding(&level_bound)?, reward_binding(&callback)?);
        let mut mutating = callback.clone();
        mutating.storage[0].maximum = mutating.storage[0].minimum + 1;
        assert!(!accepts(&mutating));
        let original_binding = reward_binding(&callback)?;
        for slots in [vec![0], vec![9], vec![11], vec![4, 4]] {
            let mut invalid = callback.clone();
            invalid.equipped_slots = slots;
            assert!(!accepts(&invalid));
        }
        for field in [
            "take",
            "give",
            "outfit",
            "achievement",
            "experience",
            "blessing",
            "completes",
        ] {
            let mut invalid = serde_json::to_value(&callback).map_err(|e| e.to_string())?;
            invalid[field] = match field {
                "take" => serde_json::json!([{"item":"oteryn:item.tibia.i3562","count":1}]),
                "give" => serde_json::json!({"item":"oteryn:item.tibia.i3270","count":1}),
                "outfit" => serde_json::json!({"outfit":"oteryn:outfit.citizen","addons":0}),
                "achievement" => serde_json::json!("oteryn:achievement/test"),
                "experience" => serde_json::json!(100),
                "blessing" => serde_json::json!("oteryn:service.blessing.test"),
                "completes" => serde_json::json!(true),
                _ => return Err("unexpected test field".into()),
            };
            let text = serde_json::json!({"schema":1,"callbacks":[invalid]}).to_string();
            assert!(
                parse(&text).is_err(),
                "repeated checks cannot include {field}"
            );
        }
        callback.equipped_slots = vec![1];
        assert_ne!(reward_binding(&callback)?, original_binding);
        callback.equipped_slots = vec![4];
        callback.repeatable_check = false;
        assert_ne!(reward_binding(&callback)?, original_binding);
        assert!(!accepts(&callback));
        Ok(())
    }

    #[test]
    fn reward_value_and_premium_terms_change_the_quest_definition_hash() -> Result<(), String> {
        let callback = load()?.into_iter().next().ok_or("callback")?;
        let make = |callback: RewardCallback| -> Result<QuestStateCatalogue, String> {
            let binding = reward_binding(&callback)?;
            let tracks = callback
                .storage
                .iter()
                .map(|s| QuestTrack {
                    key: s.track.clone(),
                    quest: callback.quest.clone(),
                    initial: -1,
                    min: s.minimum.min(-1).min(s.set),
                    max: s.maximum.max(s.set),
                })
                .collect();
            let effects = callback
                .storage
                .iter()
                .map(|s| QuestEffect {
                    track: s.track.clone(),
                    from: QuestComparison::Between(s.minimum, s.maximum),
                    effect: QuestEffectKind::SetWithBinding {
                        value: s.set,
                        binding,
                    },
                })
                .collect();
            QuestStateCatalogue::new(
                "reward-binding",
                tracks,
                vec![QuestTransition {
                    key: callback.key,
                    quest: callback.quest,
                    effects,
                    completes: false,
                    experience: None,
                }],
            )
            .map_err(|e| format!("{e:?}"))
        };
        let original = make(callback.clone())?.definition_hash(&callback.quest);
        let mut changed = callback.clone();
        changed.blessing = Some("oteryn:service.blessing.the_spiritual_shielding".into());
        assert_ne!(make(changed)?.definition_hash(&callback.quest), original);
        let mut changed = callback.clone();
        changed.premium = !changed.premium;
        assert_ne!(make(changed)?.definition_hash(&callback.quest), original);
        let mut changed = callback.clone();
        changed.minimum_level = Some(30);
        assert_ne!(make(changed)?.definition_hash(&callback.quest), original);
        Ok(())
    }

    #[test]
    fn qengine_reward_callbacks_are_bounded_and_never_repeatable_value_grants() -> Result<(), String>
    {
        let source = include_str!("../../../../../content/quests/shared-reward-callbacks.json");
        let callbacks = load()?;
        assert!(callbacks.len() >= 6);
        for amount in [0, -1, super::super::QUESTGATE0_RL_05 + 1] {
            let mut invalid: serde_json::Value =
                serde_json::from_str(source).map_err(|error| error.to_string())?;
            invalid["callbacks"][0]["experience"] = amount.into();
            assert!(parse(&invalid.to_string()).is_err(), "invalid XP {amount}");
        }
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
                    let storage = value["callbacks"][0]["storage"]
                        .as_array_mut()
                        .ok_or("storage fixture")?;
                    for field in storage {
                        field["set"] = field["minimum"].clone();
                    }
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
