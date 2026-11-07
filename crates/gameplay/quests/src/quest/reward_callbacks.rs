//! Bounded NPC exchanges composed by the fenced quest writer, never by dialogue text.
use super::world_callbacks::Storage;
use super::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack, QuestTransition,
};
use serde::{Deserialize, Serialize};
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key_number: Option<u16>,
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
    pub take: Vec<ItemAmount>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fee: Option<MonetaryFee>,
    /// Paid item sales may preserve their guards; unpaid rewards remain once-only.
    #[serde(default, skip_serializing_if = "is_false")]
    pub repeatable: bool,
    /// Carried evidence checked under the same locks, without consuming it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub present: Vec<ItemAmount>,
    pub give: Option<ItemReward>,
    pub outfit: Option<OutfitReward>,
    pub achievement: Option<String>,
    pub reply: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub failure_reply: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure_next_topic: Option<u16>,
    pub sources: Vec<String>,
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
            || (c.storage.is_empty() && !(c.repeatable && c.fee.is_some()))
            || c.storage.len() > 8
            || tracks.len() != c.storage.len()
            || c.storage.iter().any(|s| {
                !slug(&s.track, "oteryn:", b"._/:-", 128)
                    || s.minimum > s.maximum
                    || s.read_only
                    || s.set_if.is_some()
            })
            || (c
                .storage
                .iter()
                .all(|s| s.preserve || (s.minimum..=s.maximum).contains(&s.set))
                && !c.repeatable)
            || (c.repeatable
                && (c.fee.is_none()
                    || c.give.is_none()
                    || c.outfit.is_some()
                    || c.achievement.is_some()
                    || c.storage.iter().any(|s| !s.preserve)))
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
                    || r.key_number
                        .is_some_and(|number| number == 0 || r.count != 1 || !r.contents.is_empty())
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
            completes: false,
            experience: None,
        });
    }
    let extended =
        QuestStateCatalogue::new(catalogue.content_revision.clone(), tracks, transitions)
            .map_err(|e| format!("{e:?}"))?;
    catalogue.with_additive_extension(extended)
}

/// Empty graphs are executable only for a currently source-bound paid sale.
/// The durable writer separately requires its admitted fee/MINT participant.
pub(super) fn stateless_sale(key: &str, quest: &str) -> bool {
    load().is_ok_and(|rows| {
        rows.iter().any(|c| {
            c.key == key
                && c.quest == quest
                && c.storage.is_empty()
                && c.repeatable
                && c.fee.is_some()
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stateless_paid_source_is_exact_and_preserves_predecessor_defaults() -> Result<(), String> {
        let callback = load()?
            .into_iter()
            .find(|c| c.storage.is_empty())
            .ok_or("stateless source sale")?;
        assert!(stateless_sale(&callback.key, &callback.quest));
        assert!(!stateless_sale(
            "oteryn:quest-reward/forged-sale",
            &callback.quest
        ));
        assert!(!stateless_sale(&callback.key, "oteryn:quest.other"));
        let track = "oteryn:quest-progress/test/paid-predecessor";
        let baseline = QuestStateCatalogue::new(
            "content-test",
            vec![QuestTrack {
                key: track.into(),
                quest: callback.quest.clone(),
                initial: -1,
                min: -1,
                max: 1,
            }],
            vec![QuestTransition {
                key: "oteryn:quest-transition/test/original".into(),
                quest: callback.quest.clone(),
                effects: vec![QuestEffect {
                    track: track.into(),
                    from: QuestComparison::Between(-1, 1),
                    effect: QuestEffectKind::Set(1),
                }],
                completes: false,
                experience: None,
            }],
        )
        .map_err(|e| format!("{e:?}"))?;
        let old = baseline
            .definition_hash(&callback.quest)
            .ok_or("original hash")?;
        let extended = extend(baseline)?;
        assert_ne!(extended.definition_hash(&callback.quest), Some(old));
        let defaults = super::super::npc_rules::legacy_defaults(&extended, &callback.quest, &old)
            .ok_or("exact predecessor")?;
        assert_eq!(defaults.get(track), Some(&-1));
        assert!(
            super::super::npc_rules::legacy_defaults(&extended, &callback.quest, &[0; 32])
                .is_none()
        );
        let forged = QuestTransition {
            key: "oteryn:quest-reward/forged-sale".into(),
            quest: callback.quest,
            effects: vec![],
            completes: false,
            experience: None,
        };
        assert!(QuestStateCatalogue::new("content-test", vec![], vec![forged]).is_err());
        Ok(())
    }

    fn fixture_text() -> Result<String, String> {
        let row = serde_json::to_value(load()?.into_iter().next().ok_or("source sale")?)
            .map_err(|e| e.to_string())?;
        let mut callbacks = Vec::new();
        for i in 0..6 {
            let mut c = row.clone();
            c["key"] = format!("oteryn:quest-reward/test-reward-{i}").into();
            c["storage"] = serde_json::json!([{"track":"oteryn:quest-progress/test/reward","minimum":-1,"maximum":0,"set":1,"preserve":false}]);
            c["repeatable"] = false.into();
            c["fee"] = serde_json::Value::Null;
            c["give"] =
                serde_json::json!({"item":"oteryn:item.tibia.i139","count":1,"contents":[]});
            if i == 1 {
                c = row.clone();
                c["key"] = "oteryn:quest-reward/fibula-key-repeat".into();
                c["storage"] = serde_json::json!([{"track":"oteryn:quest-progress/test/reward","minimum":1,"maximum":1,"set":1,"preserve":true}]);
            }
            callbacks.push(c);
        }
        Ok(serde_json::json!({"schema":1,"callbacks":callbacks}).to_string())
    }

    #[test]
    fn paid_repeatable_key_sales_require_price_preserved_guards_and_one_key() -> Result<(), String>
    {
        let source = fixture_text()?;
        let mut value: serde_json::Value =
            serde_json::from_str(&source).map_err(|e| e.to_string())?;
        let index = value["callbacks"]
            .as_array()
            .ok_or("callbacks")?
            .iter()
            .position(|c| c["key"] == "oteryn:quest-reward/fibula-key-repeat")
            .ok_or("key sale")?;
        assert!(parse(&value.to_string()).is_ok());
        for edit in 0..7 {
            let mut invalid = value.clone();
            let c = &mut invalid["callbacks"][index];
            match edit {
                0 => c["fee"] = serde_json::Value::Null,
                1 => c["fee"]["gold"] = 0.into(),
                2 => c["give"]["key_number"] = 0.into(),
                3 => c["give"]["count"] = 2.into(),
                4 => {
                    c["give"]["contents"] =
                        serde_json::json!([{"item":"oteryn:item.tibia.i3031","count":1}])
                }
                5 => c["storage"][0]["preserve"] = false.into(),
                _ => c["achievement"] = "oteryn:achievement/backpack_tourist".into(),
            }
            assert!(
                parse(&invalid.to_string()).is_err(),
                "invalid paid sale {edit}"
            );
        }
        value["callbacks"][index]["repeatable"] = false.into();
        assert!(
            parse(&value.to_string()).is_err(),
            "ordinary rewards still need a once-only guard"
        );
        Ok(())
    }

    #[test]
    fn qengine_reward_callbacks_are_bounded_and_never_repeatable_value_grants() -> Result<(), String>
    {
        let source = fixture_text()?;
        let callbacks = parse(&source)?;
        assert!(callbacks.len() >= 6);
        for edit in 0..6 {
            let mut value: serde_json::Value =
                serde_json::from_str(&source).map_err(|e| e.to_string())?;
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
                serde_json::from_str(&source).map_err(|e| e.to_string())?;
            value["callbacks"][0][field] = invalid.into();
            assert!(parse(&value.to_string()).is_err(), "{field}={invalid}");
        }
        let mut value: serde_json::Value =
            serde_json::from_str(&source).map_err(|e| e.to_string())?;
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
                serde_json::from_str(&source).map_err(|e| e.to_string())?;
            invalid["callbacks"][0]["present"] = present;
            assert!(parse(&invalid.to_string()).is_err());
        }
        super::super::loader::load_embedded_quest_state("content-1")
            .map_err(|e| format!("{e:?}"))?;
        Ok(())
    }
}
