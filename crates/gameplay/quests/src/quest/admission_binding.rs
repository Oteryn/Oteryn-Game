//! Registered bound transition ownership, including rejection of stripped/renamed aliases.
use super::{QuestEffectKind, QuestTransition, reward_callbacks, world_callbacks};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

mod shared_world;

struct Owner {
    transition: QuestTransition,
    world: bool,
}
struct Registry {
    owners: BTreeMap<String, Owner>,
    protected_tracks: BTreeSet<String>,
}

fn changes(kind: QuestEffectKind) -> bool {
    !matches!(kind, QuestEffectKind::Add(0))
}

fn registry() -> Result<&'static Registry, String> {
    static REGISTRY: OnceLock<Result<Registry, String>> = OnceLock::new();
    REGISTRY
        .get_or_init(|| {
            let mut registry = Registry {
                owners: BTreeMap::new(),
                protected_tracks: BTreeSet::new(),
            };
            let mut add = |transition: QuestTransition, world, protect| -> Result<(), String> {
                if protect {
                    registry.protected_tracks.extend(
                        transition
                            .effects
                            .iter()
                            .filter(|effect| changes(effect.effect))
                            .map(|effect| effect.track.clone()),
                    );
                }
                if registry
                    .owners
                    .insert(transition.key.clone(), Owner { transition, world })
                    .is_some()
                {
                    return Err("duplicate bound owner".into());
                }
                Ok(())
            };
            for callback in reward_callbacks::load()? {
                let protect = reward_callbacks::gated_binding(&callback)?.is_some();
                add(
                    reward_callbacks::primary_transition(&callback)?,
                    callback.world_claim.is_some(),
                    protect,
                )?;
            }
            for transition in super::completion_sets::mint_transitions()? {
                add(transition, false, true)?;
            }
            let shared_world = shared_world::catalogue()?;
            for callback in world_callbacks::load()? {
                let protect = world_callbacks::gated_binding(&callback)?.is_some()
                    || !callback.state_ops.is_empty();
                let transition = if callback.key == super::rotten_blood::SACRIFICE {
                    super::rotten_blood::altar_transition()
                } else if let Some(operation) = shared_world.transition(&callback.key) {
                    operation.clone()
                } else {
                    world_callbacks::primary_transition(&callback)?
                };
                add(transition, true, protect)?;
            }
            // Native death writes share tracks with bound NPC trial stages.
            // Their specialised writer already requires an exact native-death proof.
            for counter in super::native_kills::registered_counters()? {
                add(counter.transition(), false, true)?;
            }
            // M24 source deaths may share questline tracks with existing NPC
            // stages. Register the exact canonical operation; the writer also
            // requires its independently fenced native-death context.
            for rule in super::source_deaths::rules().map_err(str::to_owned)? {
                add(rule.transition()?, false, false)?;
            }
            for recipe in super::timers::RECIPES {
                add(recipe.transition(), false, true)?;
            }
            Ok(registry)
        })
        .as_ref()
        .map_err(Clone::clone)
}

/// Bound owners must use their exact canonical operation. Removing the binding
/// does not release ownership of a protected track to the weaker generic writer.
pub fn validate(transition: &QuestTransition, world_context: bool) -> Result<(), String> {
    let registry = registry()?;
    if let Some(owner) = registry.owners.get(&transition.key) {
        if transition != &owner.transition || (owner.world && !world_context) {
            return Err("bound operation or physical context mismatch".into());
        }
    } else if transition.effects.iter().any(|effect| {
        matches!(
            effect.effect,
            QuestEffectKind::SetWithBinding { .. }
                | QuestEffectKind::AddWithBinding { .. }
                | QuestEffectKind::RaiseToWithBinding { .. }
        ) || registry.protected_tracks.contains(&effect.track)
    }) {
        return Err("unregistered write to bound track".into());
    }
    Ok(())
}

/// Catalogue-scoped authored world owners remain immutable and cannot seize a
/// global owner's tracks or downgrade a registered native operation.
pub fn validate_for_catalogue(
    catalogue: &super::QuestStateCatalogue,
    transition: &QuestTransition,
    world_context: bool,
) -> Result<(), String> {
    let global = registry()?;
    if global.owners.contains_key(&transition.key)
        || transition
            .effects
            .iter()
            .any(|effect| global.protected_tracks.contains(&effect.track))
    {
        return validate(transition, world_context);
    }
    let mut protected = BTreeSet::new();
    if let Some(bindings) = &catalogue.authored_bindings {
        for quest in &bindings.source.quests {
            for stage in &quest.stages {
                let expected = stage.transition(&quest.quest)?;
                if expected.key == transition.key {
                    let world = matches!(
                        stage.trigger,
                        super::authored_bindings::Trigger::MapUse { .. }
                            | super::authored_bindings::Trigger::StepIn { .. }
                    );
                    return if world == world_context && transition == &expected {
                        Ok(())
                    } else {
                        Err("authored operation or physical context mismatch".into())
                    };
                }
            }
            protected.extend(quest.tracks.iter().map(|track| track.key.clone()));
        }
    }
    if transition
        .effects
        .iter()
        .any(|effect| protected.contains(effect.track.as_str()))
    {
        return Err("unregistered write to authored bound track".into());
    }
    validate(transition, world_context)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn specialized_altar_owner_preserves_the_registered_increment() -> Result<(), String> {
        let mut transition =
            world_callbacks::primary_transition(&super::super::rotten_blood::altar_callback())?;
        transition.effects[0].effect = QuestEffectKind::Add(1);
        validate(&transition, true)?;
        assert!(validate(&transition, false).is_err());
        transition.effects[0].effect = QuestEffectKind::Set(1);
        assert!(validate(&transition, true).is_err());
        Ok(())
    }

    #[test]
    fn canonical_trial_stages_can_reset_protected_native_counters() -> Result<(), String> {
        let callbacks = reward_callbacks::load()?;
        let mut checked = 0;
        for callback in callbacks
            .iter()
            .filter(|callback| callback.key.contains("l12-"))
        {
            let original = reward_callbacks::primary_transition(callback)?;
            validate(&original, false)?;
            let mut changed = original;
            changed.completes = !changed.completes;
            assert!(validate(&changed, false).is_err());
            checked += 1;
        }
        assert!(
            checked >= 3,
            "all chosen trial programmes retain exact owners"
        );
        Ok(())
    }

    #[test]
    fn native_deaths_keep_their_exact_registered_counter_operation() -> Result<(), String> {
        for counter in super::super::native_kills::COUNTERS {
            let original = counter.transition();
            validate(&original, false)?;
            let mut changed = original.clone();
            changed.effects[1].effect = QuestEffectKind::Add(2);
            assert!(validate(&changed, false).is_err());
            let mut alias = original;
            alias.key.push_str("-shadow");
            assert!(validate(&alias, false).is_err());
        }
        Ok(())
    }

    #[test]
    fn monotone_effect_cannot_replace_or_strip_a_registered_physical_operation()
    -> Result<(), String> {
        let callback = world_callbacks::load()?
            .into_iter()
            .find(|c| c.key.ends_with("/paradox-food-four"))
            .ok_or("physical consumer")?;
        let original = world_callbacks::primary_transition(&callback)?;
        let QuestEffectKind::AddWithBinding { binding, .. } = original.effects[0].effect else {
            return Err("bound source operation".into());
        };
        let mut changed = original.clone();
        changed.effects[0].effect = QuestEffectKind::RaiseToWithBinding { value: 1, binding };
        assert!(
            validate(&changed, true).is_err(),
            "copied binding does not authorize a different effect"
        );
        changed.key.push_str("-shadow");
        assert!(
            validate(&changed, true).is_err(),
            "unregistered bound raise is refused"
        );
        changed.effects[0].effect = QuestEffectKind::RaiseTo(1);
        assert!(
            validate(&changed, true).is_err(),
            "stripping the raise binding cannot release track ownership"
        );
        Ok(())
    }

    #[test]
    fn physical_owner_checks_context_and_complete_operation() -> Result<(), String> {
        let callback = world_callbacks::load()?
            .into_iter()
            .find(|callback| callback.key.ends_with("/paradox-food-four"))
            .ok_or("physical consumer")?;
        let original = world_callbacks::primary_transition(&callback)?;
        assert!(matches!(
            original.effects[0].effect,
            QuestEffectKind::AddWithBinding { delta: 1, .. }
        ));
        assert!(validate(&original, false).is_err());
        validate(&original, true)?;
        let mut changed = original.clone();
        changed.completes = true;
        assert!(validate(&changed, true).is_err());
        let mut changed = original;
        changed.key.push_str("-shadow");
        changed.effects[0].effect = QuestEffectKind::Add(1);
        assert!(validate(&changed, true).is_err());
        Ok(())
    }

    #[test]
    fn registered_track_cannot_be_renamed_and_stripped() -> Result<(), String> {
        let callback = reward_callbacks::load()?
            .into_iter()
            .find(|callback| callback.key == "oteryn:quest-reward/l6-paradox-oldrak")
            .ok_or("gated consumer")?;
        let original = reward_callbacks::primary_transition(&callback)?;
        validate(&original, false)?;
        let mut changed = original.clone();
        changed.key = "oteryn:quest-transition/fixture-shadow".into();
        for effect in &mut changed.effects {
            if let QuestEffectKind::SetWithBinding { value, .. } = effect.effect {
                effect.effect = QuestEffectKind::Set(value);
            }
        }
        assert!(validate(&changed, false).is_err());
        changed.effects[0].effect = QuestEffectKind::Add(0);
        changed.completes = true;
        assert!(
            validate(&changed, false).is_err(),
            "an alias cannot pin or complete bound state through a no-op"
        );
        let mut changed = original;
        changed.effects[0].from = super::super::QuestComparison::Between(-10, 10);
        assert!(
            validate(&changed, false).is_err(),
            "bound bytes alone do not authorize altered conditions"
        );
        Ok(())
    }
    #[test]
    fn room_progress_is_bound_while_source_timer_keeps_its_owner() -> Result<(), String> {
        let callback = world_callbacks::load()?
            .into_iter()
            .find(|c| c.key == "oteryn:quest-callback/ancient-tombs-diprath-room")
            .ok_or("room owner")?;
        let room = world_callbacks::primary_transition(&callback)?;
        assert!(matches!(
            room.effects[0].effect,
            QuestEffectKind::RaiseToWithBinding { value: 3, .. }
        ));
        assert!(
            validate(&room, false).is_err(),
            "room progress requires physical context"
        );
        validate(&room, true)?;
        let mut shadow = room.clone();
        shadow.key.push_str("-shadow");
        shadow.effects[0].effect = QuestEffectKind::RaiseTo(3);
        assert!(
            validate(&shadow, true).is_err(),
            "renaming and stripping cannot release timer track ownership"
        );
        let timer =
            super::super::timers::recipe("oteryn:quest-timer/ancient-tombs-diprath-treasure")
                .ok_or("timer owner")?
                .transition();
        assert_eq!(room.effects[0].track, timer.effects[0].track);
        validate(&timer, false)?;
        Ok(())
    }
    #[test]
    fn collector_shared_world_operations_keep_their_canonical_owner() -> Result<(), String> {
        let empty =
            super::super::QuestStateCatalogue::new("collector-owner", Vec::new(), Vec::new())
                .map_err(|error| format!("{error:?}"))?;
        let source = super::super::shared_world_progress::extend(empty)?;
        let mut checked = 0;
        for callback in super::super::shared_world_progress::callbacks() {
            if let Some(operation) = source.transition(&callback.key) {
                checked += 1;
                validate(operation, true).map_err(|error| format!("{}: {error}", callback.key))?;
                assert!(validate(operation, false).is_err());
                let mut shadow = operation.clone();
                shadow.effects[0].effect = QuestEffectKind::Set(999);
                assert!(validate(&shadow, true).is_err());
            }
        }
        assert_eq!(
            checked, 6,
            "five tool objectives plus the hatchling counter"
        );
        let fluid = super::super::shared_world_progress::callbacks()
            .into_iter()
            .find(|callback| callback.key == "oteryn:quest-callback/hive-native-fluid-preparation")
            .ok_or("fluid source callback")?;
        // Fluid preparation keeps its ordinary Set(0), rather than a specialized increment.
        validate(&world_callbacks::primary_transition(&fluid)?, true)?;
        Ok(())
    }
}
