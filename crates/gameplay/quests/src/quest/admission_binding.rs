//! Registered bound transition ownership, including rejection of stripped/renamed aliases.
use super::{QuestEffectKind, QuestTransition, reward_callbacks, world_callbacks};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

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
            let mut add = |transition: QuestTransition, world| -> Result<(), String> {
                registry.protected_tracks.extend(
                    transition
                        .effects
                        .iter()
                        .filter(|effect| changes(effect.effect))
                        .map(|effect| effect.track.clone()),
                );
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
                if reward_callbacks::gated_binding(&callback)?.is_some() {
                    add(reward_callbacks::primary_transition(&callback)?, false)?;
                }
            }
            for callback in world_callbacks::load()? {
                if world_callbacks::gated_binding(&callback)?.is_some() {
                    add(world_callbacks::primary_transition(&callback)?, true)?;
                }
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
            QuestEffectKind::SetWithBinding { .. } | QuestEffectKind::AddWithBinding { .. }
        ) || registry.protected_tracks.contains(&effect.track)
    }) {
        return Err("unregistered write to bound track".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
