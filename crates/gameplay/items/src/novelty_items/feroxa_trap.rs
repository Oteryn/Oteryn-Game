//! Crystal 9f5a72c actions_closed_silvered_trap.lua:3-15 and position_functions.cpp.
use super::{
    GroundEffect, NativeGameplayState, NoveltyCue, NoveltyEffects, NoveltyKind, NoveltyPlan,
    TypedDefinitionRef,
};
use oteryn_gameplay_content::content::native_gameplay::{
    NativeItemDestination, NativeItemStackClass,
};

pub(super) fn is_closed(key: &str) -> bool {
    key == "oteryn:item.tibia.i22074"
}

fn in_room(actor: &GroundEffect) -> bool {
    (i64::from(actor.x) - 33390)
        .abs()
        .max((i64::from(actor.y) - 31540).abs())
        .max((i64::from(actor.floor) - 11).abs())
        < 30
}

pub(super) fn plan(
    native: &NativeGameplayState,
    used: &TypedDefinitionRef,
    ground: GroundEffect,
    actor: GroundEffect,
) -> Option<NoveltyPlan> {
    if !is_closed(&used.production_key) || used.family != "Item" || !in_room(&actor) {
        return None;
    }
    let source = native
        .item_policy(&used.production_key, &used.revision_ref)?
        .record();
    let output = native.item_policy_for_source_id(22059)?.record();
    if !source.admission.materializable
        || !output.admission.materializable
        || output.admission.stack_class != NativeItemStackClass::NonStackable
        || output.admission.legal_destinations.as_slice() != [NativeItemDestination::Ground]
    {
        return None;
    }
    let definition = &output.production_definition;
    let effects = NoveltyEffects {
        kind: NoveltyKind::Transform, source_item_id: 22074,
        donor: "crystalserver:9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d:data-global/scripts/quests/grimvale/actions_closed_silvered_trap.lua".into(),
        ground: Some(ground), cues: vec![NoveltyCue { binding: "canary.appearance:effect/poff".into(), effect_id: 3 }],
        speech: None, private_speech: false, require_head: false, require_player_target: false,
        dice_value: None, costume: None, damage: None, achievement: None,
        achievement_progress: Vec::new(), timed_output: None,
    };
    Some(NoveltyPlan {
        recipe: "oteryn:feroxa-silvered-trap.v1".into(),
        outputs: vec![(
            TypedDefinitionRef {
                family: definition.family.clone(),
                production_key: definition.production_key.clone(),
                revision_ref: definition.revision_ref.clone(),
            },
            1,
        )],
        consume_source: true,
        effects: serde_json::to_value(effects).ok()?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn qn5_feroxa_trap_uses_actor_chebyshev_distance_without_overflow() {
        assert!(in_room(&GroundEffect {
            x: 33419,
            y: 31540,
            floor: 11
        }));
        assert!(!in_room(&GroundEffect {
            x: 33420,
            y: 31540,
            floor: 11
        }));
        assert!(in_room(&GroundEffect {
            x: 33390,
            y: 31569,
            floor: 11
        }));
        assert!(!in_room(&GroundEffect {
            x: 33390,
            y: 31570,
            floor: 11
        }));
        assert!(in_room(&GroundEffect {
            x: 33390,
            y: 31540,
            floor: 0
        }));
        assert!(!in_room(&GroundEffect {
            x: i32::MIN,
            y: i32::MAX,
            floor: i16::MIN
        }));
    }
}
