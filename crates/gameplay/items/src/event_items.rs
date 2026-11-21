//! W19 ground-source and presentation adapter for the existing reward-bag rules.
//! Reward conversion stays with reward_bag; captured source metadata stays in
//! reward_bag::event_containers. Ground custody uses the consume/transform owner.
use crate::item_craft::CookingConversion;
use crate::novelty_items::{GroundEffect, NoveltyCue, NoveltyEffects, NoveltyKind, NoveltyPlan};
use crate::reward_bag::{
    event_containers::event_container_rule, reward_bag_conversion, reward_bag_draw,
};
use oteryn_gameplay_content::content::native_gameplay::NativeGameplayState;
use oteryn_gameplay_domain::domain::item_definition::TypedDefinitionRef;

pub fn is_event_reward(key: &str) -> bool {
    event_container_rule(key).is_some()
}

pub fn ground_output_at_source(key: &str) -> bool {
    event_container_rule(key).is_some_and(|rule| rule.ground_output == "source")
}

/// Source fromPosition is a container address for carried surprise bags, so
/// their donor effect has no map location. item:getPosition follows its owner.
pub fn effects(key: &str, source_position: Option<GroundEffect>) -> Option<NoveltyEffects> {
    let rule = event_container_rule(key)?;
    let cue = &rule.presentation;
    let repetitions = if cue.origin == "wire_source" && source_position.is_none() {
        0
    } else if cue.per_reward {
        rule.rewards.len()
    } else {
        1
    };
    Some(NoveltyEffects {
        kind: NoveltyKind::Reward,
        source_item_id: rule.item_id,
        donor: rule.donor.clone(),
        ground: if cue.origin == "player" {
            None
        } else {
            source_position
        },
        cues: vec![
            NoveltyCue {
                binding: cue.cue.binding.clone(),
                effect_id: cue.cue.effect_id
            };
            repetitions
        ],
        speech: None,
        private_speech: false,
        require_head: false,
        require_player_target: false,
        dice_value: None,
        costume: None,
        damage: None,
        achievement: None,
        achievement_progress: Vec::new(),
        timed_output: None,
    })
}

pub fn conversion(
    native: &NativeGameplayState,
    used: &TypedDefinitionRef,
    session: [u8; 16],
    command: u64,
    source: [u8; 16],
) -> Option<CookingConversion> {
    event_container_rule(&used.production_key)?;
    reward_bag_conversion(native, used, reward_bag_draw(session, command, source))
}

pub fn plan(
    native: &NativeGameplayState,
    used: &TypedDefinitionRef,
    session: [u8; 16],
    command: u64,
    source: [u8; 16],
    ground: Option<GroundEffect>,
) -> Option<NoveltyPlan> {
    let conversion = conversion(native, used, session, command, source)?;
    let effects = effects(&used.production_key, ground)?;
    Some(NoveltyPlan {
        recipe: format!("oteryn:event-container.{}", effects.source_item_id),
        outputs: conversion.outputs,
        consume_source: true,
        effects: serde_json::to_value(effects).ok()?,
    })
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn generated_bags_retain_source_eligibility_and_presentation() {
        let key = "oteryn:item.tibia.i6570";
        assert!(is_event_reward(key));
        assert!(crate::novelty_items::is_ground_action(key));
        let rule = event_container_rule(key).expect("source metadata");
        assert_eq!(ground_output_at_source(key), rule.ground_output == "source");
        let ground = GroundEffect {
            x: 6,
            y: 0,
            floor: 7,
        };
        let effects = effects(key, Some(ground)).expect("source presentation");
        assert_eq!(effects.source_item_id, 6570);
        assert_eq!(effects.donor, rule.donor);
        assert_eq!(effects.cues.len(), 1);
        assert_eq!(effects.cues[0].effect_id, rule.presentation.cue.effect_id);
    }

    #[test]
    fn sp04_bags_keep_metadata_without_a_second_reward_converter() {
        for rule in crate::reward_bag::event_containers::event_container_rules().expect("catalogue")
        {
            let key = format!("oteryn:item.tibia.i{}", rule.item_id);
            assert!(crate::reward_bag::is_reward_bag(&key));
            assert!(crate::reward_bag::event_containers::legacy_conversion_rule(&key).is_none());
            assert!(is_event_reward(&key));
            assert_eq!(
                ground_output_at_source(&key),
                rule.ground_output == "source"
            );
            for ground in [
                None,
                Some(GroundEffect {
                    x: 6,
                    y: 0,
                    floor: 7,
                }),
            ] {
                let expected = if rule.presentation.origin == "wire_source" && ground.is_none() {
                    0
                } else if rule.presentation.per_reward {
                    rule.rewards.len()
                } else {
                    1
                };
                let effects = effects(&key, ground).expect("captured presentation");
                assert_eq!(effects.cues.len(), expected);
                assert_eq!(effects.source_item_id, rule.item_id);
                assert_eq!(effects.donor, rule.donor);
            }
        }
        assert!(!is_event_reward("oteryn:item.tibia.i3600"));
    }
}
