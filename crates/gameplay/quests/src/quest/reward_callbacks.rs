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
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OutfitReward {
    pub outfit: String,
    /// Base entitlement is always included. Bits 1 and 2 earn the respective addons.
    pub addons: u8,
}

/// Immutable evidence survives a source track being reset for another task run.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub struct ReceiptPredicate {
    pub transition: String,
    pub track: String,
    pub value: i64,
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
    pub required_receipts: Vec<ReceiptPredicate>,
    pub take: Vec<ItemAmount>,
    /// Carried evidence checked under the same locks, without consuming it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub present: Vec<ItemAmount>,
    pub give: Option<ItemReward>,
    pub outfit: Option<OutfitReward>,
    pub achievement: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub experience: Option<i64>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub completes: bool,
    pub reply: Vec<String>,
    pub sources: Vec<String>,
}

pub(crate) fn valid_item_reward(reward: &ItemReward) -> bool {
    let valid = |item: &str, count: u16| {
        super::valid_quest_key(item)
            && item
                .strip_prefix("oteryn:item.")
                .is_some_and(|tail| !tail.is_empty())
            && (1..=100).contains(&count)
    };
    valid(&reward.item, reward.count)
        && reward.contents.len() <= 20
        && reward
            .contents
            .iter()
            .all(|item| valid(&item.item, item.count))
}

fn is_false(value: &bool) -> bool {
    !*value
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    schema: u32,
    callbacks: Vec<RewardCallback>,
}

pub fn load() -> Result<Vec<RewardCallback>, String> {
    let mut callbacks = parse(include_str!(
        "../../../../../content/quests/shared-reward-callbacks.json"
    ))?;
    callbacks.extend(parse(include_str!(
        "../../../../../content/quests/authored-reward-callbacks.json"
    ))?);
    let keys: BTreeSet<_> = callbacks.iter().map(|callback| &callback.key).collect();
    if keys.len() != callbacks.len() || callbacks.len() > 256 {
        return Err("duplicate or excessive reward callbacks".into());
    }
    Ok(callbacks)
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
            || c.storage.is_empty()
            || c.storage.len() > 8
            || tracks.len() != c.storage.len()
            || c.storage
                .iter()
                .any(|s| !slug(&s.track, "oteryn:", b"._/:-", 128) || s.minimum > s.maximum)
            || c.storage
                .iter()
                .all(|s| s.preserve || (s.minimum..=s.maximum).contains(&s.set))
            || c.required_receipts.len() > 8
            || c.required_receipts.iter().collect::<BTreeSet<_>>().len()
                != c.required_receipts.len()
            || c.required_receipts.iter().any(|requirement| {
                !super::valid_quest_key(&requirement.transition)
                    || !super::valid_quest_key(&requirement.track)
                    || requirement.transition == c.key
            })
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
        for requirement in &callback.required_receipts {
            if !catalogue
                .transition(&requirement.transition)
                .is_some_and(|transition| transition.quest == callback.quest)
                || !catalogue.track(&requirement.track).is_some_and(|track| {
                    track.quest == callback.quest
                        && (track.min..=track.max).contains(&requirement.value)
                })
            {
                return Err(format!(
                    "foreign or invalid receipt predicate {}",
                    callback.key
                ));
            }
        }
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
        // Seal every value participant and historical guard into the quest pin,
        // using the same refused-marker pattern as the Grizzly source rules.
        let digest: String =
            Sha256::digest(serde_json::to_vec(&callback).map_err(|e| e.to_string())?)
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect();
        let marker = format!("oteryn:quest-reward-plan/{digest}");
        transitions.push(QuestTransition {
            key: marker,
            quest: callback.quest.clone(),
            effects: vec![QuestEffect {
                track: callback
                    .storage
                    .first()
                    .ok_or("missing reward guard")?
                    .track
                    .clone(),
                from: QuestComparison::Any,
                effect: QuestEffectKind::Computed,
            }],
            completes: false,
            experience: None,
        });
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
    fn reward_receipt_predicates_require_the_same_quest_and_declared_value() -> Result<(), String> {
        let empty =
            QuestStateCatalogue::empty("receipt-predicate-test").ok_or("empty catalogue")?;
        let base = super::super::grizzly_adams::extend(empty)?;
        extend(base.clone())?;
        let transition = "oteryn:quest-transition/grizzly/report-boss-1";
        let track = super::super::grizzly_adams::storage_track(34100);
        let mut foreign_transition = base.clone();
        foreign_transition
            .transitions
            .get_mut(transition)
            .ok_or("report")?
            .quest = "oteryn:quest.foreign".into();
        assert!(extend(foreign_transition).is_err());
        let mut foreign_track = base.clone();
        foreign_track
            .tracks
            .get_mut(&track)
            .ok_or("boss track")?
            .quest = "oteryn:quest.foreign".into();
        assert!(extend(foreign_track).is_err());
        let mut bounded = base;
        bounded.tracks.get_mut(&track).ok_or("boss track")?.max = 2;
        assert!(extend(bounded).is_err());
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
