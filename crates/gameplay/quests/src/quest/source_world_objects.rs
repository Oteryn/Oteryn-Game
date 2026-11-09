//! Source world-object lifecycle bindings. OTS_HYPOTHESIS_ONLY, retained donor corpus.
use super::world_callbacks::{
    Callback, CallbackEffectSet, NativeSelector, Position, Selector, Storage, Trigger,
};

pub const UNEASY_CRYSTAL: &str = "oteryn:quest-callback/uneasy-alliance-crystal";

pub(crate) fn uneasy_crystal() -> Callback {
    let mut effects = CallbackEffectSet::default();
    effects.transform = Some(10187);
    effects.reset_after_ms = Some(10_000);
    Callback { admission: None, requires_premium: false,
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
        minimum_level: None,
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
    callback.key != UNEASY_CRYSTAL || appearance == 10186
}

/// Crystal 201189f actions_lions_rock.lua:14-84, OTS_HYPOTHESIS_ONLY.
/// First progress is max(old, 1) + 1; later trials add one without regressing it.
pub(crate) fn lions_pagodas() -> Vec<Callback> {
    use super::world_callbacks::{ItemOnTarget, TargetTransform};
    let trials = [
        (
            "strength",
            (33137, 32291, 1, 21390),
            (33134, 32289),
            9634,
            "You have sacrificed a cobra tongue at an ancient statue. The light in the small pyramid nearby begins to shine.",
        ),
        (
            "beauty",
            (33138, 32369, 3, 2110),
            (33136, 32369),
            21389,
            "You burnt a lion's mane flower. The light in the small pyramid nearby begins to shine.",
        ),
        (
            "tears",
            (33154, 32279, 2, 21391),
            (33156, 32279),
            21466,
            "You have purified a sacret pedestal with holy water. You have now passed the last testto enter thge inner sanctum.",
        ),
    ];
    let mut callbacks = Vec::new();
    for (name, (x, y, ordinal, appearance), (px, py), item, message) in trials {
        for first in [true, false] {
            let mut callback = uneasy_crystal();
            callback.key = format!(
                "oteryn:quest-callback/m44-lions-{name}/{}",
                if first { "first" } else { "later" }
            );
            callback.quest = "oteryn:quest.tibia_tales".into();
            callback.selector = Selector {
                native: Some(NativeSelector {
                    position: Position { x, y, floor: -8 },
                    ordinal,
                    appearance,
                }),
                position: None,
                unique: None,
                action: None,
                appearance: Some(appearance),
            };
            callback.effects = CallbackEffectSet::default();
            callback.effects.target_transform = Some(TargetTransform {
                position: Position {
                    x: px,
                    y: py,
                    floor: -8,
                },
                from: 2074,
                to: 2075,
                reset_after_ms: 15_000,
            });
            callback.item_on_target = Some(ItemOnTarget {
                item: format!("oteryn:item.tibia.i{item}"),
                alternatives: Vec::new(),
                count: 1,
                replacement: None,
                target_replacement: None,
                reusable: false,
            });
            callback.storage = vec![
                Storage {
                    track: format!(
                        "oteryn:quest-progress/quest/u10_70/lions_rock/outer_sanctum/lions_{name}"
                    ),
                    minimum: -1,
                    maximum: 0,
                    set: 1,
                    preserve: false,
                    increment: false,
                    at_least: false,
                },
                Storage {
                    track: "oteryn:quest-progress/quest/u10_70/lions_rock/questline".into(),
                    minimum: if first { -1 } else { 1 },
                    maximum: if first { 0 } else { 2_147_483_646 },
                    set: if first { 2 } else { 1 },
                    preserve: false,
                    increment: !first,
                    at_least: false,
                },
            ];
            callback.success_message = Some(message.into());
            callback.sources = vec!["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/lions_rock/actions_lions_rock.lua:14-84".into()];
            callbacks.push(callback);
        }
    }
    callbacks
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lions_rock_pagodas_bind_each_offering_and_both_questline_phases() -> Result<(), String> {
        let callbacks = super::super::world_callbacks::load()?;
        let trials: Vec<_> = callbacks
            .iter()
            .filter(|c| c.key.contains("m44-lions-"))
            .collect();
        assert_eq!(trials.len(), 6);
        for callback in trials {
            super::super::world_callbacks::parse(
                &serde_json::json!({"schema":1,"callbacks":[callback]}).to_string(),
            )?;
            let value = serde_json::to_value(&callback.effects).map_err(|e| e.to_string())?;
            assert_eq!(value["target_transform"]["from"], 2074);
            assert_eq!(value["target_transform"]["to"], 2075);
            assert_eq!(value["target_transform"]["reset_after_ms"], 15000);
            assert_eq!(callback.storage[0].set, 1);
            assert!(!callback.item_on_target.as_ref().ok_or("offering")?.reusable);
            let mut invalid = callback.clone();
            invalid
                .effects
                .target_transform
                .as_mut()
                .ok_or("target")?
                .reset_after_ms = 0;
            assert!(
                super::super::world_callbacks::parse(
                    &serde_json::json!({"schema":1,"callbacks":[invalid]}).to_string()
                )
                .is_err()
            );
        }
        Ok(())
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
