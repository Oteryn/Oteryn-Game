//! Source world-object lifecycle bindings. OTS_HYPOTHESIS_ONLY, retained donor corpus.
use super::world_callbacks::{
    Callback, CallbackEffectSet, NativeSelector, Position, Selector, Storage, Trigger,
};

pub const UNEASY_CRYSTAL: &str = "oteryn:quest-callback/uneasy-alliance-crystal";

pub(crate) fn uneasy_crystal() -> Callback {
    let mut effects = CallbackEffectSet::default();
    effects.transform = Some(10187);
    effects.reset_after_ms = Some(10_000);
    Callback { track_initials: Default::default(), required_completed_quests: Vec::new(), admission: None, requires_premium: false,
        world_change: None,
        actor_kind: Default::default(),
        opening_only: false,
        requires_world_admission: false,
        key: UNEASY_CRYSTAL.into(),
        quest: "oteryn:quest.an_uneasy_alliance_quest".into(),
        trigger: Trigger::Use,
        selector: Selector {
            // The retained B3 counterpart has no legacy action attribute.
            native: Some(NativeSelector {
                position: Position { x: 33155, y: 31297, floor: -3 },
                ordinal: 2,
                appearance: 10186,
            }),
            position: None,
            unique: None,
            action: Some(40030),
            appearance: Some(10186),
        },
        lever: None,
        reversible: false,
        passages: Vec::new(),
        teleport: None,
        facing: None,
        state_ops: Vec::new(),
        world_requirements: Vec::new(),
        storage: vec![Storage {
            track: "oteryn:quest-progress/quest/u8_54/an_uneasy_alliance/questline".into(),
            minimum: 3,
            maximum: 3,
            set: 4,
            preserve: false,
            increment: false,
            at_least: false,
        }],
        minimum_level: None, character_requirements: None,
        experience: None,
        monk_reward: None,
        offerings: Vec::new(),
        item_on_target: None,
        return_on_failure: false,
        failure_message: None,
        success_message: None,
        native_rule: None,
        effects,
        sources: vec![
            "crystalserver:9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d:data-global/scripts/quests/an_uneasy_alliance/actions_crystal_ball.lua:1-24".into(),
        ],
    }
}

/// Source readiness is re-proven by the Channel owner immediately before projection.
pub fn source_is_ready(callback: &Callback, appearance: u16) -> bool {
    if callback.has_fenced_regrowth_reward() {
        return appearance == 21388;
    }
    (match callback.key.as_str() {
        UNEASY_CRYSTAL => appearance == 10186,
        // Crystal 201189f movement_fire_bug_teleport.lua:9-14.
        "oteryn:quest-callback/dreamers-challenge-fire-bug-entry" => appearance == 1949,
        super::dreamer_fire_bug::IGNITE => appearance == 3134,
        _ => true,
    }) && (!callback
        .item_on_target
        .as_ref()
        .is_some_and(|item| item.selected_instance)
        || (callback.effects.transform.is_none()
            && callback
                .item_on_target
                .as_ref()
                .is_none_or(|item| item.target_replacement.is_none()))
        || callback
            .selector
            .native
            .as_ref()
            .map(|binding| binding.appearance)
            .or(callback.selector.appearance)
            .is_none_or(|expected| expected == appearance))
}

/// Crystal 201189f movements_lions_rock.lua:54-108, OTS_HYPOTHESIS_ONLY.
pub(crate) fn lions_amphora() -> Vec<Callback> {
    use super::reward_callbacks::ItemReward;
    use super::world_callbacks::TargetTransform;
    [(33118, 32246, 520), (33119, 32246, 521), (33120, 32246, 522),
        (33118, 32247, 523), (33120, 32247, 525), (33118, 32248, 526),
        (33119, 32248, 527), (33120, 32248, 528)]
        .into_iter().enumerate().map(|(index, (x, y, appearance))| {
            let mut callback = uneasy_crystal();
            callback.key = format!("oteryn:quest-callback/m44-lions-amphora-{index}");
            callback.quest = "oteryn:quest.tibia_tales".into();
            callback.trigger = Trigger::StepIn;
            callback.requires_world_admission = true;
            callback.selector = Selector {
                native: Some(NativeSelector { position: Position { x, y, floor: -9 }, ordinal: 0, appearance }),
                position: None, unique: None, action: None, appearance: Some(appearance),
            };
            callback.effects = CallbackEffectSet::default();
            callback.effects.target_transform = Some(TargetTransform {
                position: Position { x: 33119, y: 32247, floor: -9 }, from: 21945, to: 21946, reset_after_ms: 15_000,
            });
            callback.effects.item_reward = Some(ItemReward {
                premium: false, item: "oteryn:item.tibia.i21467".into(), count: 1,
                contents: Vec::new(), text: None, additional: Vec::new(),
            });
            callback.storage = vec![Storage {
                track: "oteryn:quest-progress/quest/u10_70/lions_rock/questline".into(),
                minimum: 4, maximum: 4, set: 5, preserve: false, increment: false, at_least: false,
            }];
            callback.success_message = Some("As you pass incautiously, the ancient amphora crumbles to shards and dust. Amidst the debris you discover an old scroll.".into());
            callback.sources = vec!["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/lions_rock/movements_lions_rock.lua:54-108".into()];
            callback
        }).collect()
}

/// Crystal 201189f actions_lions_rock.lua:86-104, OTS_HYPOTHESIS_ONLY.
pub(crate) fn lions_flowers() -> Vec<Callback> {
    use super::reward_callbacks::ItemReward;
    [
        (33138, 32352, -5, 1), (33140, 32352, -5, 1),
        (33137, 32355, -5, 1), (33139, 32363, -5, 2),
        (33154, 32359, -5, 1), (33153, 32361, -5, 3),
        (33133, 32359, -6, 2), (33157, 32364, -6, 2),
    ].into_iter().enumerate().map(|(index, (x,y,floor,ordinal))| {
        let mut callback = uneasy_crystal();
        callback.key = format!("oteryn:quest-callback/m44-lions-flower-{index}");
        callback.quest = "oteryn:quest.tibia_tales".into();
        callback.requires_world_admission = true;
        callback.selector = Selector {
            native: Some(NativeSelector { position: Position { x,y,floor }, ordinal, appearance: 21388 }),
            position: None, unique: None, action: None, appearance: Some(21388),
        };
        callback.effects = CallbackEffectSet::default();
        callback.effects.transform = Some(21935);
        callback.effects.reset_after_ms = Some(60_000);
        callback.effects.item_reward = Some(ItemReward { premium: false,
            item: "oteryn:item.tibia.i21389".into(), count: 1, contents: Vec::new(),
            text: None, additional: Vec::new(),
        });
        callback.storage = [
            ("oteryn:quest-progress/quest/u10_70/lions_rock/questline", i64::from(i32::MAX)),
            ("oteryn:quest-progress/quest/u8_1/tibia_tales/default_start", 1),
        ].into_iter().map(|(track,maximum)| Storage {
            track: track.into(), minimum: 0, maximum, set: 1,
            preserve: false, increment: false, at_least: true,
        }).collect();
        callback.success_message = Some("You picked a beautiful lion's mane flower.".into());
        callback.sources = vec!["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/lions_rock/actions_lions_rock.lua:86-104".into()];
        callback
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lions_flower_regrowth_preserves_progress_and_requires_a_fenced_source() -> Result<(), String>
    {
        let callbacks = super::super::world_callbacks::load()?;
        let flowers: Vec<_> = callbacks
            .iter()
            .filter(|c| c.key.contains("m44-lions-flower-"))
            .collect();
        assert_eq!(flowers.len(), 8);
        for callback in flowers {
            assert!(source_is_ready(callback, 21388));
            assert!(!source_is_ready(callback, 21935));
            assert_eq!(callback.effects.transform, Some(21935));
            assert_eq!(callback.effects.reset_after_ms, Some(60_000));
            assert_eq!(
                callback
                    .effects
                    .item_reward
                    .as_ref()
                    .ok_or("flower reward")?
                    .item,
                "oteryn:item.tibia.i21389"
            );
            assert!(
                callback
                    .storage
                    .iter()
                    .all(|state| state.at_least && state.set == 1)
            );
            let document =
                |c: &Callback| serde_json::json!({"schema":1,"callbacks":[c]}).to_string();
            super::super::world_callbacks::parse(&document(callback))?;
            for mutation in 0..3 {
                let mut invalid = callback.clone();
                match mutation {
                    0 => invalid.key = "oteryn:quest-callback/unfenced-flower".into(),
                    1 => invalid.effects.reset_after_ms = None,
                    _ => invalid.selector.native = None,
                }
                assert!(super::super::world_callbacks::parse(&document(&invalid)).is_err());
            }
        }
        Ok(())
    }
    #[test]
    fn lions_amphora_step_ring_has_eight_bound_sources_and_one_scroll_reward() -> Result<(), String>
    {
        let callbacks = super::super::world_callbacks::load()?;
        let ring: Vec<_> = callbacks
            .iter()
            .filter(|c| c.key.contains("m44-lions-amphora-"))
            .collect();
        assert_eq!(ring.len(), 8);
        for callback in ring {
            assert_eq!(callback.trigger, Trigger::StepIn);
            assert!(callback.has_fenced_step_reward());
            let value = serde_json::to_value(&callback.effects).map_err(|e| e.to_string())?;
            assert_eq!(value["target_transform"]["from"], 21945);
            assert_eq!(value["target_transform"]["to"], 21946);
            assert_eq!(value["target_transform"]["reset_after_ms"], 15000);
            assert_eq!(value["item_reward"]["item"], "oteryn:item.tibia.i21467");
            assert_eq!(value["item_reward"]["count"], 1);
            assert_eq!(
                (
                    callback.storage[0].minimum,
                    callback.storage[0].maximum,
                    callback.storage[0].set
                ),
                (4, 4, 5)
            );
            super::super::world_callbacks::parse(
                &serde_json::json!({"schema":1,"callbacks":[callback]}).to_string(),
            )?;
        }
        Ok(())
    }
    #[test]
    fn lions_rock_pagodas_bind_each_offering_and_both_questline_phases() -> Result<(), String> {
        let callbacks = super::super::world_callbacks::load()?;
        let trials: Vec<_> = callbacks
            .iter()
            .filter(|c| {
                matches!(
                    c.key.as_str(),
                    "oteryn:quest-callback/m07/lion-strength"
                        | "oteryn:quest-callback/m07/lion-beauty"
                        | "oteryn:quest-callback/m07/lion-tears"
                )
            })
            .collect();
        assert_eq!(trials.len(), 3);
        let loaded = super::super::loader::load_embedded_quest_state("lion-owner")
            .map_err(|error| format!("{error:?}"))?;
        let catalogue = loaded.catalogue();
        for callback in trials {
            let native = callback.selector.native.as_ref().ok_or("native trial")?;
            assert_eq!(
                callbacks
                    .iter()
                    .filter(|other| other.trigger == callback.trigger
                        && other.selector.native.as_ref() == Some(native))
                    .count(),
                1,
                "each native offering has exactly one dispatch owner"
            );
            assert!(
                !callbacks
                    .iter()
                    .any(
                        |other| ["strength", "beauty", "tears"].iter().any(|name| other
                            .key
                            .starts_with(&format!("oteryn:quest-callback/m44-lions-{name}/")))
                            || super::super::lions_rock::binding(&other.key).is_some()
                    )
            );
            super::super::world_callbacks::parse(
                &serde_json::json!({"schema":1,"callbacks":[callback]}).to_string(),
            )?;
            let passage = callback.passages.first().ok_or("pagoda")?;
            assert_eq!(passage.appearance, 2074);
            assert_eq!(passage.replacement, Some(2075));
            assert_eq!(callback.effects.passages_reset_after_ms, Some(15000));
            assert_eq!(callback.storage[0].set, 1);
            assert!(!callback.item_on_target.as_ref().ok_or("offering")?.reusable);
            let mut invalid = callback.clone();
            invalid.effects.passages_reset_after_ms = Some(0);
            assert!(
                super::super::world_callbacks::parse(
                    &serde_json::json!({"schema":1,"callbacks":[invalid]}).to_string()
                )
                .is_err()
            );
            // Crystal actions_lions_rock.lua:61-66: both first and later phases.
            assert_eq!(callback.state_ops.len(), 1);
            assert_eq!(
                callback.state_ops[0].track,
                super::super::lions_rock::QUESTLINE
            );
            let flag = &callback.storage[0].track;
            let counter = &callback.state_ops[0].track;
            let transition = catalogue
                .transition(&callback.key)
                .ok_or("trial transition")?;
            for (before, after) in [
                (0, 2),
                (1, 2),
                (2, 3),
                (3, 4),
                (i64::from(i32::MAX) - 1, i64::from(i32::MAX)),
            ] {
                let values = std::collections::BTreeMap::from([
                    (flag.clone(), 0),
                    (counter.clone(), before),
                ]);
                let changes = catalogue
                    .evaluate(transition, &values, 0)
                    .map_err(|error| format!("{error:?}"))?;
                assert_eq!(
                    changes
                        .iter()
                        .find(|change| &change.track == counter)
                        .ok_or("counter change")?
                        .after,
                    after
                );
                assert_eq!(
                    changes
                        .iter()
                        .find(|change| &change.track == flag)
                        .ok_or("flag change")?
                        .after,
                    1
                );
            }
            for (progress, completed) in [(4, 1), (i64::from(i32::MAX), 0)] {
                let values = std::collections::BTreeMap::from([
                    (flag.clone(), completed),
                    (counter.clone(), progress),
                ]);
                assert!(catalogue.evaluate(transition, &values, 0).is_err());
            }
        }
        Ok(())
    }
    #[test]
    fn dreamer_fire_bug_entry_requires_the_shared_lit_glyph() {
        let mut callback = uneasy_crystal();
        callback.key = "oteryn:quest-callback/dreamers-challenge-fire-bug-entry".into();
        assert!(source_is_ready(&callback, 1949));
        assert!(!source_is_ready(&callback, 3134));
        assert!(!source_is_ready(&callback, 1948));
    }

    #[test]
    fn uneasy_native_counterpart_matches_the_retained_bare_map_object() -> Result<(), String> {
        let callback = uneasy_crystal();
        let native = callback
            .selector
            .native
            .as_ref()
            .ok_or("missing native counterpart")?;
        assert_eq!(
            (
                native.position.x,
                native.position.y,
                native.position.floor,
                native.ordinal,
                native.appearance
            ),
            (33155, 31297, -3, 2, 10186)
        );
        assert_eq!(callback.selector.action, Some(40030));
        Ok(())
    }
    #[test]
    fn uneasy_busy_crystal_cannot_advance_another_players_quest() {
        let callback = uneasy_crystal();
        assert!(source_is_ready(&callback, 10186));
        assert!(!source_is_ready(&callback, 10187));
        assert!(!source_is_ready(&callback, 10185));
        let mut unrelated = callback;
        unrelated.key = "oteryn:quest-callback/other-source".into();
        assert!(source_is_ready(&unrelated, 10187));
    }
}
