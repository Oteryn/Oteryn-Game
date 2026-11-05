//! W19 ground-source and presentation adapter for the existing reward-bag rules.
//! Chance tables and conversion stay in reward_bag::event_containers; Ground
//! custody is committed by the existing consume/transform owner.
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
    Some(NoveltyPlan {
        recipe: conversion.recipe,
        outputs: conversion.outputs,
        consume_source: true,
        effects: serde_json::to_value(effects(&used.production_key, ground)?).ok()?,
    })
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn sp04_bags_have_no_w19_route_or_presentation() {
        for rule in crate::reward_bag::event_containers::event_container_rules().expect("catalogue")
        {
            let key = format!("oteryn:item.tibia.i{}", rule.item_id);
            assert!(crate::reward_bag::is_reward_bag(&key));
            assert!(!is_event_reward(&key));
            assert!(!ground_output_at_source(&key));
            assert!(effects(&key, None).is_none());
            assert!(
                effects(
                    &key,
                    Some(GroundEffect {
                        x: 6,
                        y: 0,
                        floor: 7
                    })
                )
                .is_none()
            );
        }
        assert!(!is_event_reward("oteryn:item.tibia.i3600"));
    }
}
