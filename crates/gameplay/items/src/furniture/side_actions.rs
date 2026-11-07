//! Donor-authored House decoration effects, distinct from USE transforms and Rotate.
use super::*;
use crate::novelty_items::{GroundEffect, NoveltyCue, NoveltyEffects, NoveltyKind};
use crate::reward_bag::reward_bag_bounded_draw;

#[derive(Clone, Debug, Deserialize)]
pub struct SideRule {
    pub id: u32,
    pub effect: u16,
    pub success_effect: Option<u16>,
    pub chance: Option<u32>,
    pub output: Option<u32>,
    pub speech: Vec<String>,
    pub exhaustion_seconds: Option<u32>,
    pub donor: String,
}
pub fn side_rule(key: &str) -> Option<&'static SideRule> {
    let id = key
        .strip_prefix("oteryn:item.tibia.i")?
        .parse::<u32>()
        .ok()?;
    let rules = &catalogue()?.side_actions;
    rules
        .binary_search_by_key(&id, |r| r.id)
        .ok()
        .map(|at| &rules[at])
}

pub fn plan_interactive(
    native: &NativeGameplayState,
    used: &TypedDefinitionRef,
    session: [u8; 16],
    command: u64,
    instance: [u8; 16],
    ground: GroundEffect,
    exhausted: bool,
) -> Option<NoveltyPlan> {
    plan_interactive_at(
        native,
        used,
        session,
        command,
        instance,
        Some(ground),
        exhausted,
    )
}

pub fn plan_portable_effect(
    native: &NativeGameplayState,
    used: &TypedDefinitionRef,
    session: [u8; 16],
    command: u64,
    instance: [u8; 16],
) -> Option<NoveltyPlan> {
    if !super::is_portable_effect(&used.production_key) {
        return None;
    }
    let mut plan = plan_interactive_at(native, used, session, command, instance, None, false)?;
    if plan.outputs.as_slice() != [(used.clone(), 1)] {
        return None;
    }
    plan.outputs.clear();
    plan.effects["kind"] = json!("consume_effect");
    plan.recipe = format!(
        "oteryn:novelty-item.furniture-effect.{}.v1",
        used.production_key
    );
    Some(plan)
}

fn plan_interactive_at(
    native: &NativeGameplayState,
    used: &TypedDefinitionRef,
    session: [u8; 16],
    command: u64,
    instance: [u8; 16],
    ground: Option<GroundEffect>,
    exhausted: bool,
) -> Option<NoveltyPlan> {
    let rule = side_rule(&used.production_key)?;
    let roll = reward_bag_bounded_draw(session, command, instance, 81, 100)?;
    let succeeded = rule.chance.is_none_or(|chance| roll < u64::from(chance));
    let target = if succeeded {
        rule.output.unwrap_or(rule.id)
    } else {
        rule.id
    };
    let source = native
        .item_policy(&used.production_key, &used.revision_ref)?
        .record();
    let after = native.item_policy_for_source_id(target)?.record();
    if !source.admission.materializable || !after.admission.materializable {
        return None;
    }
    let effect = if succeeded {
        rule.success_effect.unwrap_or(rule.effect)
    } else {
        rule.effect
    };
    let speech = if rule.exhaustion_seconds.is_some() && exhausted {
        Some("Don't be so vain about your appearance.".into())
    } else if !rule.speech.is_empty() {
        let choice = reward_bag_bounded_draw(
            session,
            command,
            instance,
            82,
            u64::try_from(rule.speech.len()).ok()?,
        )?;
        Some(rule.speech[usize::try_from(choice).ok()?].clone())
    } else {
        None
    };
    let effects = NoveltyEffects {
        kind: NoveltyKind::HouseTransform,
        source_item_id: rule.id,
        donor: rule.donor.clone(),
        ground,
        cues: if effect == 0 {
            Vec::new()
        } else {
            vec![NoveltyCue {
                binding: format!("appearance:effect/donor_{effect}"),
                effect_id: effect,
            }]
        },
        speech,
        private_speech: false,
        require_head: false,
        require_player_target: false,
        dice_value: None,
        costume: None,
        damage: None,
        achievement: None,
        achievement_progress: Vec::new(),
        timed_output: None,
    };
    Some(NoveltyPlan {
        recipe: format!("oteryn:house-furniture.interactive.{}.v1", rule.id),
        outputs: vec![(
            TypedDefinitionRef {
                family: "Item".into(),
                production_key: after.production_definition.production_key.clone(),
                revision_ref: after.production_definition.revision_ref.clone(),
            },
            1,
        )],
        consume_source: false,
        effects: serde_json::to_value(effects).ok()?,
    })
}
