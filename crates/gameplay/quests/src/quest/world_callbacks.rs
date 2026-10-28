//! QUESTCB2: bounded, declarative world callbacks. Coordinates use native negative floors.
use super::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack, QuestTransition,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const EMBEDDED: &str = include_str!("../../../../../content/quests/world-callbacks.json");
pub const MAX_CALLBACKS: usize = 128;
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
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Selector {
    pub position: Option<Position>,
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
}

/// One carried item applied to the selected world object.  This deliberately is not a
/// recipe language: the source is consumed or changed exactly once by the receipt owner.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ItemOnTarget {
    pub item: String,
    pub count: u16,
    pub replacement: Option<String>,
    pub target_replacement: Option<u16>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Callback {
    pub key: String,
    pub quest: String,
    pub trigger: Trigger,
    pub selector: Selector,
    /// Both lever appearances. None for a step tile. One-way levers refuse their second use.
    pub lever: Option<[u16; 2]>,
    pub reversible: bool,
    pub passages: Vec<Passage>,
    pub teleport: Option<Position>,
    pub storage: Vec<Storage>,
    pub offerings: Vec<Offering>,
    #[serde(default)]
    pub item_on_target: Option<ItemOnTarget>,
    pub return_on_failure: bool,
    pub failure_message: Option<String>,
    pub sources: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    schema: u32,
    callbacks: Vec<Callback>,
}

/// Read the same closed content shape at boot and at the durable mutation boundary.
pub fn load() -> Result<Vec<Callback>, String> {
    parse(EMBEDDED)
}

fn parse(text: &str) -> Result<Vec<Callback>, String> {
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
            || (selector.position.is_none()
                && selector.unique.is_none()
                && selector.action.is_none())
            || selector.position.is_some_and(|p| !valid_pos(p))
            || callback.teleport.is_some_and(|p| !valid_pos(p))
            || unique_offerings.len() != callback.offerings.len()
            || unique_passages.len() != callback.passages.len()
            || unique_storage.len() != callback.storage.len()
            || callback.passages.len() > MAX_EFFECTS
            || callback.storage.len() > MAX_EFFECTS
            || callback.offerings.len() > MAX_EFFECTS
            || (!callback.offerings.is_empty() && callback.storage.is_empty())
            || callback.sources.is_empty()
            || !callback.sources.iter().any(|source| {
                source.starts_with("canary:47dfd51f45280a59a1d3e50ba7edd573d7234446:")
            })
            || callback.item_on_target.as_ref().is_some_and(|item| {
                item.target_replacement == Some(0)
                    || (item.replacement.is_some() && item.count != 1)
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
    let mut tracks: Vec<_> = catalogue.tracks.values().cloned().collect();
    let mut transitions: Vec<_> = catalogue.transitions.values().cloned().collect();
    for callback in callbacks.iter().filter(|c| !c.storage.is_empty()) {
        for storage in &callback.storage {
            if let Some(track) = tracks.iter_mut().find(|track| track.key == storage.track) {
                track.max = track.max.max(storage.set).max(storage.maximum);
            } else {
                tracks.push(QuestTrack {
                    key: storage.track.clone(),
                    quest: callback.quest.clone(),
                    initial: -1,
                    min: -1,
                    max: storage.set.max(storage.maximum),
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
                    effect: QuestEffectKind::Set(s.set),
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
        for case in 0..10 {
            let mut bad = source.clone();
            match case {
                0 => {
                    bad["callbacks"][0]["selector"] = serde_json::json!({"position":null,"unique":null,"action":null,"appearance":null})
                }
                1 => bad["callbacks"][0]["extra"] = true.into(),
                2 => {
                    bad["callbacks"][3]["offerings"][1] =
                        bad["callbacks"][3]["offerings"][0].clone()
                }
                3 => bad["callbacks"][3]["offerings"][0]["count"] = 101.into(),
                4 => bad["callbacks"][6]["item_on_target"]["target_replacement"] = 0.into(),
                5 => bad["callbacks"][0]["sources"] = serde_json::json!(["crystal:unverified"]),
                6 => bad["callbacks"][6]["item_on_target"]["replacement"] = "bad".into(),
                7 => {
                    bad["callbacks"][6]["item_on_target"]["replacement"] =
                        "oteryn:item.tibia.i3114".into();
                    bad["callbacks"][6]["item_on_target"]["count"] = 2.into();
                }
                8 => bad["callbacks"][6]["storage"] = serde_json::json!([]),
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
            bounded["callbacks"][3][field] = serde_json::json!(entries);
            assert!(parse(&bounded.to_string()).is_ok(), "{field} at its bound");
            let mut extra = bounded["callbacks"][3][field][0].clone();
            if field == "storage" {
                extra["track"] = "oteryn:quest-progress/overflow".into();
            } else {
                extra["position"]["x"] = 100.into();
            }
            bounded["callbacks"][3][field]
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
}
