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
pub struct TrackDelta {
    pub track: String,
    pub amount: i64,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClaimScope {
    #[default]
    Once,
    PerCommand,
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub increments: Vec<TrackDelta>,
    /// Carried evidence checked under the same locks, without consuming it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub present: Vec<ItemAmount>,
    #[serde(default)]
    pub claim_scope: ClaimScope,
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
    let mut callbacks = parse(include_str!(
        "../../../../../content/quests/shared-reward-callbacks.json"
    ))?;
    callbacks.extend(parse(include_str!(
        "../../../../../content/quests/world-contribution-rewards.json"
    ))?);
    if callbacks.len() > 256
        || callbacks
            .iter()
            .map(|c| &c.key)
            .collect::<BTreeSet<_>>()
            .len()
            != callbacks.len()
    {
        return Err("combined reward callback bounds".into());
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
            || (c.increments.is_empty()
                && c.storage
                    .iter()
                    .all(|s| s.preserve || (s.minimum..=s.maximum).contains(&s.set)))
            || c.increments.len() > 8
            || c.increments
                .iter()
                .map(|d| &d.track)
                .collect::<BTreeSet<_>>()
                .len()
                != c.increments.len()
            || c.increments.iter().any(|d| {
                d.amount == 0
                    || !c.storage.iter().any(|s| {
                        s.track == d.track
                            && s.preserve
                            && s.minimum.checked_add(d.amount).is_some()
                            && s.maximum.checked_add(d.amount).is_some()
                    })
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
            || (c.claim_scope == ClaimScope::PerCommand && (c.give.is_none() || c.take.is_empty()))
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
    let mut tracks: Vec<_> = catalogue.tracks.values().cloned().collect();
    let mut transitions: Vec<_> = catalogue.transitions.values().cloned().collect();
    for callback in load()? {
        for s in &callback.storage {
            if let Some(track) = tracks.iter_mut().find(|t| t.key == s.track) {
                if track.quest != callback.quest {
                    return Err(format!("foreign reward track {}", s.track));
                }
                track.max = track.max.max(s.maximum).max(s.set).max(
                    s.maximum.saturating_add(
                        callback
                            .increments
                            .iter()
                            .find(|d| d.track == s.track)
                            .map_or(0, |d| d.amount),
                    ),
                );
                track.min = track.min.min(s.minimum);
            } else {
                tracks.push(QuestTrack {
                    key: s.track.clone(),
                    quest: callback.quest.clone(),
                    initial: -1,
                    min: s.minimum.min(-1).min(s.set),
                    max: s.maximum.max(s.set).max(
                        s.maximum.saturating_add(
                            callback
                                .increments
                                .iter()
                                .find(|d| d.track == s.track)
                                .map_or(0, |d| d.amount),
                        ),
                    ),
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
                        QuestEffectKind::Add(
                            callback
                                .increments
                                .iter()
                                .find(|d| d.track == s.track)
                                .map_or(0, |d| d.amount),
                        )
                    } else {
                        QuestEffectKind::Set(s.set)
                    },
                })
                .collect(),
            completes: false,
            experience: None,
        });
    }
    QuestStateCatalogue::new(catalogue.content_revision, tracks, transitions)
        .map_err(|e| format!("{e:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn repeatable_rewards_require_custody_and_disjoint_ottokar_counts() -> Result<(), String> {
        let callbacks = load()?;
        let rows: Vec<_> = callbacks
            .iter()
            .filter(|c| c.key.starts_with("oteryn:quest-reward/ottokar-medicine-"))
            .collect();
        assert_eq!(rows.len(), 4);
        for count in [-1, 0, 1, 98, 99, 100, 1000] {
            let selected: Vec<_> = rows
                .iter()
                .filter(|c| {
                    c.storage
                        .iter()
                        .all(|s| (s.minimum..=s.maximum).contains(&count))
                })
                .collect();
            assert_eq!(selected.len(), 1, "counter {count}");
            assert_eq!(selected[0].claim_scope, ClaimScope::PerCommand);
            assert_eq!(
                selected[0].achievement.as_deref(),
                (count == 99).then_some("oteryn:achievement/doctor_doctor")
            );
        }
        let mut document = serde_json::json!({"schema":1,"callbacks":[rows[0]]});
        document["callbacks"][0]["take"] = serde_json::json!([]);
        assert!(parse(&document.to_string()).is_err());
        document["callbacks"][0]["take"] =
            serde_json::json!([{"item":"oteryn:item.tibia.i12517","count":1}]);
        document["callbacks"][0]["give"] = serde_json::Value::Null;
        assert!(parse(&document.to_string()).is_err());
        Ok(())
    }

    #[test]
    fn qengine_increment_requires_preserved_guard_and_checked_endpoints() -> Result<(), String> {
        let source = include_str!("../../../../../content/quests/world-contribution-rewards.json");
        let callbacks = parse(source)?;
        assert_eq!(
            callbacks
                .iter()
                .filter(|c| c.key.starts_with("oteryn:quest-reward/chavis-food-"))
                .count(),
            2
        );
        let repeat = callbacks
            .iter()
            .find(|callback| callback.key == "oteryn:quest-reward/chavis-food-repeat")
            .ok_or("repeat callback")?;
        assert_eq!(
            repeat.take,
            vec![ItemAmount {
                item: "oteryn:item.tibia.i21291".into(),
                count: 5
            }]
        );
        assert_eq!(
            repeat.increments,
            vec![TrackDelta {
                track: super::super::rathleton::POINTS_TRACK.into(),
                amount: 1
            }]
        );
        for edit in 0..8 {
            let mut value: serde_json::Value =
                serde_json::from_str(source).map_err(|error| error.to_string())?;
            let repeat = &mut value["callbacks"][1];
            match edit {
                0 => repeat["storage"] = serde_json::json!([]),
                1 => repeat["storage"][0]["preserve"] = false.into(),
                2 => {
                    let delta = repeat["increments"][0].clone();
                    repeat["increments"] = serde_json::json!([delta.clone(), delta]);
                }
                3 => repeat["increments"][0]["amount"] = 0.into(),
                4 => repeat["increments"][0]["track"] = "oteryn:quest-progress/foreign".into(),
                5 => repeat["storage"][0]["maximum"] = i64::MAX.into(),
                6 => {
                    repeat["storage"][0]["minimum"] = i64::MIN.into();
                    repeat["increments"][0]["amount"] = (-1).into();
                }
                _ => repeat["increments"][0]["amount"] = 2.into(),
            }
            assert!(
                parse(&value.to_string()).is_err(),
                "increment mutation {edit}"
            );
        }
        Ok(())
    }

    #[test]
    fn qengine_chavis_first_and_repeat_add_rights_without_overflow() -> Result<(), String> {
        use super::super::rathleton::POINTS_TRACK;
        use std::collections::BTreeMap;
        let empty = QuestStateCatalogue::empty("increments-r1").ok_or("empty catalogue")?;
        let catalogue = extend(empty)?;
        let first = catalogue
            .transition("oteryn:quest-reward/chavis-food-first")
            .ok_or("first transition")?;
        let repeat = catalogue
            .transition("oteryn:quest-reward/chavis-food-repeat")
            .ok_or("repeat transition")?;
        let mut stored = BTreeMap::new();
        let first_change = catalogue
            .evaluate(first, &stored, 0)
            .map_err(|error| format!("{error:?}"))?;
        assert_eq!(first_change.len(), 1);
        assert_eq!((first_change[0].before, first_change[0].after), (-1, 1));
        stored.insert(POINTS_TRACK.into(), first_change[0].after);
        let repeated = catalogue
            .evaluate(repeat, &stored, 0)
            .map_err(|error| format!("{error:?}"))?;
        assert_eq!((repeated[0].before, repeated[0].after), (1, 2));
        stored.insert(POINTS_TRACK.into(), i64::MAX - 1);
        let last = catalogue
            .evaluate(repeat, &stored, 0)
            .map_err(|error| format!("{error:?}"))?;
        assert_eq!(last[0].after, i64::MAX);
        stored.insert(POINTS_TRACK.into(), i64::MAX);
        assert_eq!(
            catalogue.evaluate(repeat, &stored, 0),
            Err(super::super::QuestRefusal::StageMismatch)
        );
        stored.insert(POINTS_TRACK.into(), -1);
        assert_eq!(
            catalogue.evaluate(repeat, &stored, 0),
            Err(super::super::QuestRefusal::StageMismatch)
        );
        Ok(())
    }
}
