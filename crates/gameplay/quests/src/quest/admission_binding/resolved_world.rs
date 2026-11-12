//! Extend admission with an immutable resolved World source, preserving published owners.
use super::*;

pub fn validate_resolved_world(
    catalogue: &super::super::QuestStateCatalogue,
    transition: &QuestTransition,
    world_context: bool,
    callbacks: &[world_callbacks::Callback],
) -> Result<(), String> {
    let published = registry()?;
    if published.owners.contains_key(&transition.key) {
        return validate_for_catalogue(catalogue, transition, world_context);
    }
    let mut authored_tracks = BTreeSet::new();
    if let Some(bindings) = &catalogue.authored_bindings {
        for quest in &bindings.source.quests {
            authored_tracks.extend(quest.tracks.iter().map(|track| &track.key));
            for stage in &quest.stages {
                if stage.transition(&quest.quest)?.key == transition.key {
                    return validate_for_catalogue(catalogue, transition, world_context);
                }
            }
        }
    }
    // The published cohorts share the boot bound; only additional resolved
    // owners consume the closed per-source admission bound.
    if callbacks.len() > world_callbacks::MAX_RUNTIME_CALLBACKS
        || callbacks
            .iter()
            .filter(|callback| !published.owners.contains_key(&callback.key))
            .count()
            > world_callbacks::MAX_CALLBACKS
    {
        return Err("resolved World source bounds".into());
    }
    let mut owned = None;
    let mut protected = BTreeSet::new();
    for callback in callbacks {
        if published.owners.contains_key(&callback.key)
            || world_callbacks::gated_binding(callback)?.is_none()
        {
            continue;
        }
        let operation = world_callbacks::primary_transition(callback)?;
        protected.extend(
            operation
                .effects
                .iter()
                .filter(|e| changes(e.effect))
                .map(|e| e.track.clone()),
        );
        if operation.key == transition.key && owned.replace(operation).is_some() {
            return Err("ambiguous resolved World owner".into());
        }
    }
    if let Some(operation) = owned {
        if !world_context
            || operation != *transition
            || transition.effects.iter().any(|e| {
                published.protected_tracks.contains(&e.track) || authored_tracks.contains(&e.track)
            })
        {
            return Err("resolved World operation or context mismatch".into());
        }
        return Ok(());
    }
    if transition
        .effects
        .iter()
        .any(|e| changes(e.effect) && protected.contains(&e.track))
    {
        return Err("renamed resolved World owner".into());
    }
    validate_for_catalogue(catalogue, transition, world_context)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn published_registry_does_not_exhaust_the_resolved_source_bound() -> Result<(), String> {
        let source = super::super::super::loader::load_embedded_quest_state("content-1")
            .map_err(|error| format!("{error:?}"))?;
        let catalogue = source.catalogue();
        let callbacks = world_callbacks::load_for(catalogue)?;
        assert!(callbacks.len() > world_callbacks::MAX_CALLBACKS);
        let transition = catalogue
            .transition("oteryn:quest-transition/kv/bear_chest3/chest")
            .ok_or("bear claim transition")?;
        validate_resolved_world(catalogue, transition, false, &callbacks)
    }

    #[test]
    fn resolved_source_and_combined_registry_keep_their_separate_bounds() -> Result<(), String> {
        let source = super::super::super::loader::load_embedded_quest_state("content-1")
            .map_err(|error| format!("{error:?}"))?;
        let catalogue = source.catalogue();
        let callbacks = world_callbacks::load_for(catalogue)?;
        let transition = catalogue
            .transition("oteryn:quest-transition/kv/bear_chest3/chest")
            .ok_or("bear claim transition")?;
        let callback = callbacks.first().ok_or("published callback")?;
        let additional: Vec<_> = (0..=world_callbacks::MAX_CALLBACKS)
            .map(|index| {
                let mut callback = callback.clone();
                callback.key = format!("oteryn:quest-callback/resolved-bound-{index}");
                callback
            })
            .collect();
        assert_eq!(
            validate_resolved_world(catalogue, transition, false, &additional),
            Err("resolved World source bounds".into())
        );
        let oversized = vec![callback.clone(); world_callbacks::MAX_RUNTIME_CALLBACKS + 1];
        assert_eq!(
            validate_resolved_world(catalogue, transition, false, &oversized),
            Err("resolved World source bounds".into())
        );
        Ok(())
    }

    #[test]
    fn resolved_world_owner_preserves_exact_binding_and_rejects_weaker_aliases()
    -> Result<(), String> {
        let catalogue =
            super::super::super::QuestStateCatalogue::new("resolved-fixture", vec![], vec![])
                .map_err(|e| format!("{e:?}"))?;
        let mut source = world_callbacks::load()?
            .into_iter()
            .find(|c| !c.storage.is_empty())
            .ok_or("fixture")?;
        source.key = "oteryn:quest-callback/m02-resolved-owner".into();
        source.quest = "oteryn:quest.m02_resolved_owner".into();
        source.native_rule = None;
        source.state_ops.clear();
        source.requires_world_admission = true;
        source.storage = vec![world_callbacks::Storage {
            add: None,
            read_only: false,
            set_if: None,
            track: "oteryn:quest-progress/m02-resolved-owner/stage".into(),
            minimum: 0,
            maximum: 1,
            set: 1,
            preserve: false,
            increment: false,
            at_least: false,
        }];
        let operation = world_callbacks::primary_transition(&source)?;
        assert!(
            validate(&operation, true).is_err(),
            "unknown bound operations remain closed by default"
        );
        validate_resolved_world(&catalogue, &operation, true, std::slice::from_ref(&source))?;
        assert!(
            validate_resolved_world(&catalogue, &operation, false, std::slice::from_ref(&source))
                .is_err()
        );
        let mut stripped = operation.clone();
        stripped.effects[0].effect = QuestEffectKind::Set(1);
        assert!(
            validate_resolved_world(&catalogue, &stripped, true, std::slice::from_ref(&source))
                .is_err()
        );
        stripped.key.push_str("-alias");
        assert!(
            validate_resolved_world(&catalogue, &stripped, true, std::slice::from_ref(&source))
                .is_err()
        );
        assert!(
            validate_resolved_world(
                &catalogue,
                &operation,
                true,
                &[source.clone(), source.clone()]
            )
            .is_err()
        );
        let published = registry()?;
        let protected = published
            .protected_tracks
            .iter()
            .next()
            .ok_or("published protected track")?;
        source.storage[0].track = protected.clone();
        assert!(
            validate_resolved_world(
                &catalogue,
                &world_callbacks::primary_transition(&source)?,
                true,
                &[source]
            )
            .is_err()
        );
        Ok(())
    }
}
