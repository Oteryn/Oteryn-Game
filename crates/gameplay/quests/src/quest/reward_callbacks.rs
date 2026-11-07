//! Bounded NPC exchanges composed by the fenced quest writer, never by dialogue text.
use super::world_callbacks::Storage;
use super::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack, QuestTransition,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

mod state_ops;
pub use state_ops::{StateCondition, StateMutation, StateOperation};

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

/// Exact source price; only the durable fee owner may consume the funds.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MonetaryFee {
    pub gold: u64,
    pub include_bank: bool,
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
    pub state_ops: Vec<StateOperation>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub requires_premium: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_receipts: Vec<ReceiptPredicate>,
    pub take: Vec<ItemAmount>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fee: Option<MonetaryFee>,
    /// Carried evidence checked under the same locks, without consuming it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub present: Vec<ItemAmount>,
    pub give: Option<ItemReward>,
    /// Portable recurring rewards use the original MINT owner with an occurrence key.
    #[serde(default, skip_serializing_if = "is_false")]
    pub per_occurrence_mint: bool,
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
        && (reward.contents.is_empty() || reward.backpack_outputs.is_empty())
        && reward.backpack_outputs.len() <= 19
        && reward
            .backpack_outputs
            .iter()
            .all(|item| valid(&item.item, item.count))
        && reward.contents.len() <= 20
        && reward
            .contents
            .iter()
            .all(|item| valid(&item.item, item.count))
}

fn is_false(value: &bool) -> bool {
    !*value
}

/// Bounded writable metadata, naming the original quest owner explicitly.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TrackDeclaration {
    writable: bool,
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
    Ok(load_document()?.callbacks)
}

fn load_document() -> Result<Document, String> {
    let mut document = parse_document(include_str!(
        "../../../../../content/quests/shared-reward-callbacks.json"
    ))?;
    let chosen = parse_document(include_str!(
        "../../../../../content/quests/chosen-reward-callbacks.json"
    ))?;
    document.callbacks.extend(chosen.callbacks);
    document.tracks.extend(chosen.tracks);
    let keys: BTreeSet<_> = document
        .callbacks
        .iter()
        .map(|callback| &callback.key)
        .collect();
    let tracks: BTreeSet<_> = document.tracks.iter().map(|track| &track.key).collect();
    if keys.len() != document.callbacks.len()
        || document.callbacks.len() > 256
        || tracks.len() != document.tracks.len()
        || document.tracks.len() > 256
    {
        return Err("duplicate or excessive reward callbacks/tracks".into());
    }
    Ok(document)
}

#[cfg(test)]
fn parse(text: &str) -> Result<Vec<RewardCallback>, String> {
    Ok(parse_document(text)?.callbacks)
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
    if document.schema != 1 || document.callbacks.len() > 256 || document.tracks.len() > 256 {
        return Err("reward callback bounds".into());
    }
    let mut declared = BTreeSet::new();
    for track in &document.tracks {
        let owners = document
            .callbacks
            .iter()
            .flat_map(|callback| {
                callback
                    .storage
                    .iter()
                    .map(|storage| (&callback.quest, &storage.track))
                    .chain(
                        callback
                            .state_ops
                            .iter()
                            .map(|operation| (&callback.quest, &operation.track)),
                    )
            })
            .filter(|(_, key)| key.as_str() == track.key.as_str())
            .map(|(quest, _)| quest)
            .collect::<Vec<_>>();
        if !track.writable
            || !slug(&track.key, "oteryn:", b"._/:-", 128)
            || !slug(&track.quest, "oteryn:", b"._/:-", 128)
            || !declared.insert(&track.key)
            || track.min > track.initial
            || track.initial > track.max
            || owners.is_empty()
            || owners
                .iter()
                .any(|quest| quest.as_str() != track.quest.as_str())
            || track.sources.is_empty()
            || track.sources.len() > 8
            || track
                .sources
                .iter()
                .any(|source| source.is_empty() || source.len() > 1024)
        {
            return Err(format!("invalid writable reward track {}", track.key));
        }
    }
    for c in &document.callbacks {
        let tracks: BTreeSet<_> = c
            .storage
            .iter()
            .map(|s| &s.track)
            .chain(c.state_ops.iter().map(|s| &s.track))
            .collect();
        let inputs: BTreeSet<_> = c.take.iter().map(|s| &s.item).collect();
        let evidence: BTreeSet<_> = c.present.iter().map(|s| &s.item).collect();
        if !slug(&c.key, "oteryn:quest-reward/", b"_/-", 128)
            || !slug(&c.quest, "oteryn:", b"._/:-", 128)
            || !keys.insert(&c.key)
            || !slug(&c.npc, "oteryn:npc.", b"_", 128)
            || c.storage.len() + c.state_ops.len() == 0
            || c.storage.len() + c.state_ops.len() > 8
            || tracks.len() != c.storage.len() + c.state_ops.len()
            || c.state_ops.iter().any(|operation| !operation.valid())
            || c.storage
                .iter()
                .any(|s| !slug(&s.track, "oteryn:", b"._/:-", 128) || s.minimum > s.maximum)
            || c.storage
                .iter()
                .all(|s| s.preserve || (s.minimum..=s.maximum).contains(&s.set))
                && c.state_ops
                    .iter()
                    .all(|operation| matches!(operation.effect, StateMutation::Add { value: 0 }))
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
            || (c.per_occurrence_mint
                && (c.give.is_none() || c.outfit.is_some() || c.achievement.is_some()))
            || c.take.len() > 8
            || inputs.len() != c.take.len()
            || c.take
                .iter()
                .any(|s| !slug(&s.item, "oteryn:item.", b"._/:-", 256) || s.count == 0)
            || c.give
                .as_ref()
                .is_some_and(|reward| !valid_item_reward(reward))
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
    Ok(document)
}

fn value_plan(callback: &RewardCallback) -> Result<Vec<u8>, String> {
    let legacy = serde_json::to_vec(&(
        &callback.key,
        &callback.quest,
        &callback.required_receipts,
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
        callback.completes,
        callback.experience,
    ))
    .map_err(|e| e.to_string())?;
    let current =
        if callback.state_ops.is_empty() && callback.fee.is_none() && !callback.requires_premium {
            legacy
        } else {
            serde_json::to_vec(&(
                "oteryn:quest-reward-extended-v1",
                legacy,
                &callback.state_ops,
                &callback.fee,
                callback.requires_premium,
            ))
            .map_err(|error| error.to_string())?
        };
    if callback.per_occurrence_mint {
        serde_json::to_vec(&("oteryn:quest-reward-occurrence-mint-v1", current))
            .map_err(|error| error.to_string())
    } else {
        Ok(current)
    }
}

/// Declare authored bounds before world and death adapters validate their stages.
/// Their complete rewards and provenance are still installed by `extend` afterward.
pub(crate) fn declare_tracks(
    catalogue: QuestStateCatalogue,
) -> Result<QuestStateCatalogue, String> {
    extend_callbacks(catalogue, Vec::new(), load_document()?.tracks)
}

pub fn extend(catalogue: QuestStateCatalogue) -> Result<QuestStateCatalogue, String> {
    let document = load_document()?;
    extend_callbacks(catalogue, document.callbacks, document.tracks)
}

fn extend_callbacks(
    catalogue: QuestStateCatalogue,
    callbacks: Vec<RewardCallback>,
    declarations: Vec<TrackDeclaration>,
) -> Result<QuestStateCatalogue, String> {
    let npc_legacy = catalogue.npc_legacy.clone();
    let mut tracks: Vec<_> = catalogue.tracks.values().cloned().collect();
    let mut transitions: Vec<_> = catalogue.transitions.values().cloned().collect();
    let explicit: BTreeSet<_> = declarations
        .iter()
        .map(|declaration| declaration.key.clone())
        .collect();
    for declaration in declarations {
        let track = QuestTrack {
            key: declaration.key,
            quest: declaration.quest,
            initial: declaration.initial,
            min: declaration.min,
            max: declaration.max,
        };
        if let Some(existing) = tracks.iter().find(|existing| existing.key == track.key) {
            if existing != &track {
                return Err(format!(
                    "conflicting writable reward declaration {}",
                    track.key
                ));
            }
        } else {
            tracks.push(track);
        }
    }
    for callback in callbacks {
        for operation in &callback.state_ops {
            let track = tracks
                .iter()
                .find(|track| track.key == operation.track)
                .ok_or_else(|| format!("undeclared reward operation {}", operation.track))?;
            if track.quest != callback.quest
                || !operation.valid()
                || ((matches!(operation.effect, StateMutation::SetNow)
                    || matches!(
                        operation.from,
                        StateCondition::ElapsedAtLeast { .. }
                            | StateCondition::ElapsedLessThan { .. }
                    ))
                    && (track.initial != 0 || track.min != 0 || track.max != i64::MAX))
            {
                return Err(format!(
                    "foreign or invalid reward operation {}",
                    operation.track
                ));
            }
        }
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

        // Value participants are part of the pinned quest contract. Reply copy and
        // source citations remain journal metadata and do not invalidate progress.
        let value_plan = value_plan(&callback)?;
        let digest: String = Sha256::digest(value_plan)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        transitions.push(QuestTransition {
            key: format!("oteryn:quest-reward-rule/{digest}"),
            quest: callback.quest.clone(),
            effects: vec![QuestEffect {
                track: callback
                    .storage
                    .first()
                    .map(|storage| storage.track.clone())
                    .or_else(|| {
                        callback
                            .state_ops
                            .first()
                            .map(|operation| operation.track.clone())
                    })
                    .ok_or("reward storage/operation")?,
                from: QuestComparison::Any,
                effect: QuestEffectKind::Computed,
            }],
            completes: false,
            experience: None,
        });
        for s in &callback.storage {
            if let Some(track) = tracks.iter_mut().find(|t| t.key == s.track) {
                if track.quest != callback.quest {
                    return Err(format!("foreign reward track {}", s.track));
                }
                if explicit.contains(&s.track)
                    && (s.minimum < track.min
                        || s.maximum > track.max
                        || !(track.min..=track.max).contains(&s.set))
                {
                    return Err(format!(
                        "unbounded explicitly declared reward track {}",
                        s.track
                    ));
                }
                if !explicit.contains(&s.track) {
                    track.max = track.max.max(s.maximum).max(s.set);
                    track.min = track.min.min(s.minimum);
                }
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
                .map(|s| QuestEffect {
                    track: s.track.clone(),
                    from: QuestComparison::Between(s.minimum, s.maximum),
                    effect: if s.preserve {
                        QuestEffectKind::Add(0)
                    } else {
                        QuestEffectKind::Set(s.set)
                    },
                })
                .chain(callback.state_ops.iter().map(StateOperation::quest_effect))
                .collect(),
            completes: callback.completes,
            experience: callback.experience,
        });
    }
    let mut updated = QuestStateCatalogue::new(catalogue.content_revision, tracks, transitions)
        .map_err(|e| format!("{e:?}"))?;
    updated.npc_legacy = npc_legacy;
    Ok(updated)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reward_extension_preserves_native_npc_legacy_bindings() -> Result<(), String> {
        let loaded = super::super::loader::load_embedded_quest_state("content-1")
            .map_err(|e| format!("{e:?}"))?;
        assert!(
            !loaded.catalogue().npc_legacy.is_empty(),
            "adding rewards must preserve the native NPC source binding owner"
        );
        Ok(())
    }

    #[test]
    fn reward_amount_is_part_of_the_quest_definition_hash() -> Result<(), String> {
        let before = parse(include_str!(
            "../../../../../content/quests/chosen-reward-callbacks.json"
        ))?;
        let mut changed = before.clone();
        changed
            .first_mut()
            .and_then(|c| c.give.as_mut())
            .ok_or("reward")?
            .count = 51;
        let empty = || {
            super::super::grizzly_adams::extend(
                QuestStateCatalogue::empty("content-1").ok_or("empty")?,
            )
        };
        let before = extend_callbacks(
            empty()?,
            before,
            parse_document(include_str!(
                "../../../../../content/quests/chosen-reward-callbacks.json"
            ))?
            .tracks,
        )?;
        let changed = extend_callbacks(
            empty()?,
            changed,
            parse_document(include_str!(
                "../../../../../content/quests/chosen-reward-callbacks.json"
            ))?
            .tracks,
        )?;
        assert_ne!(
            before.definition_hash(super::super::grizzly_adams::QUEST),
            changed.definition_hash(super::super::grizzly_adams::QUEST),
            "a changed payout must not reuse a running quest's pinned value plan"
        );
        Ok(())
    }

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
    #[test]
    fn npc_rewards_reject_mixed_or_unbounded_backpack_outputs() -> Result<(), String> {
        let mut document: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../content/quests/shared-reward-callbacks.json"
        ))
        .map_err(|error| error.to_string())?;
        document["callbacks"][0]["outfit"] = serde_json::Value::Null;
        let valid = serde_json::json!({"item":"oteryn:item.tibia.i2853","count":1,
            "contents":[],"backpack_outputs":[{"item":"oteryn:item.tibia.i3034","count":1}]});
        document["callbacks"][0]["give"] = valid.clone();
        assert!(parse(&document.to_string()).is_ok());
        let mut mixed = valid.clone();
        mixed["contents"] = serde_json::json!([{"item":"oteryn:item.tibia.i3031","count":1}]);
        let mut excessive = valid.clone();
        excessive["backpack_outputs"] = serde_json::json!(vec![
            serde_json::json!({"item":"oteryn:item.tibia.i3034","count":1});
            20
        ]);
        let mut empty_quantity = valid;
        empty_quantity["backpack_outputs"][0]["count"] = serde_json::json!(0);
        for invalid in [mixed, excessive, empty_quantity] {
            document["callbacks"][0]["give"] = invalid;
            assert!(parse(&document.to_string()).is_err());
        }
        Ok(())
    }
    #[test]
    fn quest_fees_preserve_free_callback_identity_and_reject_coin_double_charge()
    -> Result<(), String> {
        let source = include_str!("../../../../../content/quests/shared-reward-callbacks.json");
        let mut document: serde_json::Value =
            serde_json::from_str(source).map_err(|error| error.to_string())?;
        let free = parse(source)?.remove(0);
        assert!(free.fee.is_none());
        assert!(
            serde_json::to_value(&free)
                .map_err(|error| error.to_string())?
                .get("fee")
                .is_none()
        );
        document["callbacks"][0]["fee"] = serde_json::json!({"gold":100,"include_bank":true});
        assert!(parse(&document.to_string()).is_ok());
        for gold in [0_u64, 1_000_000_000_001] {
            let mut invalid = document.clone();
            invalid["callbacks"][0]["fee"]["gold"] = gold.into();
            assert!(parse(&invalid.to_string()).is_err());
        }
        for coin in [3031, 3035, 3043] {
            let mut invalid = document.clone();
            invalid["callbacks"][0]["take"] = serde_json::json!([
                {"item":format!("oteryn:item.tibia.i{coin}"),"count":1}
            ]);
            assert!(parse(&invalid.to_string()).is_err());
        }
        Ok(())
    }

    fn state_ops_callback() -> Result<RewardCallback, String> {
        serde_json::from_value(serde_json::json!({
            "key":"oteryn:quest-reward/state_ops_test/start","quest":"oteryn:quest.state_ops_test",
            "npc":"oteryn:npc.test","keywords":["start"],"topic":0,"next_topic":0,
            "storage":[{"track":"oteryn:quest-progress/state_ops_test/stage","minimum":-1,"maximum":-1,"set":1}],
            "take":[],"give":null,"outfit":null,"achievement":null,"reply":["Start."],"sources":["test:state-ops"]
        })).map_err(|error|error.to_string())
    }

    fn clock_declaration() -> TrackDeclaration {
        TrackDeclaration {
            writable: true,
            key: "oteryn:quest-progress/state_ops_test/clock".into(),
            quest: "oteryn:quest.state_ops_test".into(),
            initial: 0,
            min: 0,
            max: i64::MAX,
            sources: vec!["test:clock".into()],
        }
    }

    fn callback_hash(
        callback: RewardCallback,
        declarations: Vec<TrackDeclaration>,
    ) -> Result<String, String> {
        let quest = callback.quest.clone();
        let catalogue = extend_callbacks(
            QuestStateCatalogue::empty("content-1").ok_or("empty catalogue")?,
            vec![callback],
            declarations,
        )?;
        Ok(catalogue
            .definition_hash(&quest)
            .ok_or("definition hash")?
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect())
    }

    #[test]
    fn free_legacy_hash_is_stable_and_fees_premium_operations_and_guards_are_bound()
    -> Result<(), String> {
        let legacy = state_ops_callback()?;
        // Snapshot computed from the pre-port reward tuple and version2 quest encoding.
        let baseline = callback_hash(legacy.clone(), Vec::new())?;
        assert_eq!(
            baseline,
            "4a17ff7834109f75619e0aac06f022d0485ef3a7f9ca89973479c244c4d7ffea"
        );
        let encoded = serde_json::to_value(&legacy).map_err(|error| error.to_string())?;
        for omitted in ["state_ops", "fee", "requires_premium"] {
            assert!(encoded.get(omitted).is_none());
        }
        let mut paid = legacy.clone();
        paid.fee = Some(MonetaryFee {
            gold: 50,
            include_bank: false,
        });
        let first = callback_hash(paid.clone(), Vec::new())?;
        assert_ne!(first, baseline);
        paid.fee.as_mut().ok_or("fee")?.gold = 51;
        assert_ne!(callback_hash(paid.clone(), Vec::new())?, first);
        paid.fee = Some(MonetaryFee {
            gold: 50,
            include_bank: true,
        });
        assert_ne!(callback_hash(paid, Vec::new())?, first);
        let mut premium = legacy.clone();
        premium.requires_premium = true;
        assert_ne!(callback_hash(premium, Vec::new())?, baseline);
        let clock = clock_declaration();
        let mut operation = legacy;
        operation.state_ops.push(StateOperation {
            track: clock.key.clone(),
            from: StateCondition::ElapsedLessThan { seconds: 1800 },
            effect: StateMutation::Add { value: 0 },
        });
        let with_guard = callback_hash(operation.clone(), vec![clock.clone()])?;
        operation.state_ops[0].from = StateCondition::ElapsedLessThan { seconds: 1801 };
        assert_ne!(
            callback_hash(operation.clone(), vec![clock.clone()])?,
            with_guard
        );
        operation.state_ops[0].from = StateCondition::ElapsedLessThan { seconds: 1800 };
        operation.state_ops[0].effect = StateMutation::SetNow;
        assert_ne!(callback_hash(operation, vec![clock])?, with_guard);
        Ok(())
    }

    #[test]
    fn state_operations_require_exact_writable_owned_declarations() -> Result<(), String> {
        let mut callback = state_ops_callback()?;
        let clock = clock_declaration();
        callback.state_ops.push(StateOperation {
            track: clock.key.clone(),
            from: StateCondition::Any,
            effect: StateMutation::SetNow,
        });
        assert!(callback_hash(callback.clone(), Vec::new()).is_err());
        callback_hash(callback.clone(), vec![clock.clone()])?;
        let mut foreign = clock.clone();
        foreign.quest = "oteryn:quest.other".into();
        assert!(callback_hash(callback.clone(), vec![foreign]).is_err());
        let mut untyped = clock.clone();
        untyped.initial = -1;
        untyped.min = -1;
        assert!(callback_hash(callback.clone(), vec![untyped]).is_err());
        let serialized = serde_json::to_value(&callback).map_err(|error| error.to_string())?;
        let mut document = serde_json::json!({"schema":1,"tracks":[{"writable":true,"key":clock.key,"quest":clock.quest,"initial":0,"min":0,"max":i64::MAX,"sources":["test:clock"]}],"callbacks":[serialized]});
        parse_document(&document.to_string())?;
        for edit in 0..4 {
            let mut invalid = document.clone();
            match edit {
                0 => invalid["tracks"][0]["writable"] = false.into(),
                1 => invalid["tracks"][0]["quest"] = "oteryn:quest.other".into(),
                2 => invalid["tracks"][0]["initial"] = (-1).into(),
                _ => {
                    invalid["callbacks"][0]["state_ops"][0]["track"] =
                        invalid["callbacks"][0]["storage"][0]["track"].clone()
                }
            }
            assert!(
                parse_document(&invalid.to_string()).is_err(),
                "invalid declaration/operator case {edit}"
            );
        }
        document["tracks"] = serde_json::json!([]);
        let undeclared = parse_document(&document.to_string())?;
        assert!(
            extend_callbacks(
                QuestStateCatalogue::empty("content-1").ok_or("empty")?,
                undeclared.callbacks,
                undeclared.tracks
            )
            .is_err()
        );
        Ok(())
    }
    #[test]
    fn occurrence_mint_is_explicit_portable_and_part_of_the_value_hash() -> Result<(), String> {
        let mut callback = load()?
            .into_iter()
            .find(|c| c.key.ends_with("/venore-daily/accept"))
            .ok_or("accept")?;
        let legacy = value_plan(&callback)?;
        let encoded = serde_json::to_value(&callback).map_err(|e| e.to_string())?;
        assert!(encoded.get("per_occurrence_mint").is_none());
        callback.per_occurrence_mint = true;
        assert_ne!(legacy, value_plan(&callback)?);
        let document = serde_json::json!({"schema":1,"callbacks":[callback]});
        assert!(parse(&document.to_string()).is_ok());
        for case in 0..3 {
            let mut bad = document.clone();
            match case {
                0 => bad["callbacks"][0]["give"] = serde_json::Value::Null,
                1 => {
                    bad["callbacks"][0]["outfit"] =
                        serde_json::json!({"outfit":"oteryn:outfit.revenant","addons":0})
                }
                _ => bad["callbacks"][0]["per_occurrence_mint"] = "true".into(),
            }
            assert!(
                parse(&bad.to_string()).is_err(),
                "occurrence option case {case}"
            );
        }
        callback.per_occurrence_mint = false;
        assert_eq!(legacy, value_plan(&callback)?);
        Ok(())
    }
}
