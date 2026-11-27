//! Extend admission with an immutable resolved World source, preserving published owners.
use super::*;

// Existing native readers can supply the entire embedded source, including
// both paid coffin routes. The unpublished subset keeps its document bound.
const MAX_RESOLVED_CALLBACKS: usize = world_callbacks::MAX_EMBEDDED_CALLBACKS;

pub fn validate_resolved_world(
    catalogue: &super::super::QuestStateCatalogue,
    transition: &QuestTransition,
    world_context: bool,
    callbacks: &[world_callbacks::Callback],
) -> Result<(), String> {
    let published = registry()?;
    // Published transitions also require a structurally valid supplied registry.
    // The catalogue composition and its embedded fallback are already validated.
    let composed = catalogue.authored_world_callbacks();
    if !composed.is_some_and(|registered| std::ptr::eq(registered, callbacks))
        && (callbacks.len() > MAX_RESOLVED_CALLBACKS
            || callbacks
                .iter()
                .map(|callback| &callback.key)
                .collect::<BTreeSet<_>>()
                .len()
                != callbacks.len()
            || callbacks
                .iter()
                .filter(|callback| !published.owners.contains_key(&callback.key))
                .count()
                > world_callbacks::MAX_CALLBACKS)
    {
        let registered = match composed {
            Some(registered) => registered,
            None => world_callbacks::embedded_callbacks()?,
        };
        if registered != callbacks {
            return Err("resolved World source bounds".into());
        }
    }
    if published.owners.contains_key(&transition.key) {
        return validate_for_catalogue(catalogue, transition, world_context);
    }
    let mut authored_tracks = BTreeSet::new();
    if let Some(bindings) = &catalogue.authored_bindings {
        for quest in &bindings.source.quests {
            authored_tracks.extend(quest.tracks.iter().map(|track| &track.key));
            for stage in &quest.stages {
                if stage
                    .transitions(&quest.quest)?
                    .iter()
                    .any(|op| op.key == transition.key)
                {
                    return validate_for_catalogue(catalogue, transition, world_context);
                }
            }
        }
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
    fn exact_journey_callbacks_share_published_tracks_under_world_admission() -> Result<(), String>
    {
        let source = super::super::super::loader::load_embedded_quest_state("content-1")
            .map_err(|error| format!("{error:?}"))?;
        let catalogue = source.catalogue();
        let callbacks = world_callbacks::load_for(catalogue)?;
        for key in [
            "oteryn:quest-callback/alawars-vault-cross-open-magic-wall",
            "oteryn:quest-callback/paradox-tower/native-skulls",
            "oteryn:quest-callback/koshei/tomb-entry",
        ] {
            let operation = catalogue.transition(key).ok_or(key)?;
            validate_resolved_world(catalogue, operation, true, &callbacks)?;
            assert!(validate_resolved_world(catalogue, operation, false, &callbacks).is_err());
            let mut alias = operation.clone();
            alias.key.push_str("-alias");
            assert!(validate_resolved_world(catalogue, &alias, true, &callbacks).is_err());
        }
        Ok(())
    }

    #[test]
    fn jack_retains_exact_npc_writes_to_the_zelos_passage_track() -> Result<(), String> {
        use crate::quest::{QuestComparison, grave_danger, loader};

        let source =
            loader::load_embedded_quest_state("content-1").map_err(|error| format!("{error:?}"))?;
        let catalogue = source.catalogue();
        let callbacks = world_callbacks::load_for(catalogue)?;
        for key in [grave_danger::START, grave_danger::ACCESS] {
            let transition = catalogue.transition(key).ok_or(key)?;
            validate_resolved_world(catalogue, transition, false, &callbacks)?;

            let mut renamed = transition.clone();
            renamed.key.push_str("-alias");
            assert!(validate_resolved_world(catalogue, &renamed, false, &callbacks).is_err());

            let mut modified = transition.clone();
            modified.effects[0].from = QuestComparison::Any;
            assert!(validate_resolved_world(catalogue, &modified, false, &callbacks).is_err());
        }
        Ok(())
    }

    #[test]
    fn published_registry_does_not_exhaust_the_resolved_source_bound() -> Result<(), String> {
        let source = super::super::super::loader::load_embedded_quest_state("content-1")
            .map_err(|error| format!("{error:?}"))?;
        let catalogue = source.catalogue();
        let base = world_callbacks::load()?;
        let bindings = catalogue.authored_bindings().ok_or("authored bindings")?;
        let callbacks = world_callbacks::load_for(catalogue)?;
        let door_routes: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../../content/quests/world-callbacks/cwdoor-world-routes.json"
        ))
        .map_err(|error| error.to_string())?;
        let door_routes = door_routes["callbacks"].as_array().ok_or("door routes")?;
        assert!(!door_routes.is_empty());
        for route in door_routes {
            assert!(base.iter().any(|callback| route["key"] == callback.key));
        }
        assert!(base.len() <= MAX_RESOLVED_CALLBACKS);
        assert!(bindings.world_callbacks().next().is_some());
        assert_eq!(
            callbacks.len(),
            base.len() + bindings.world_callbacks().count()
        );
        // The restored Twenty Miles routes remain part of the published set.
        assert_eq!(
            base.iter()
                .filter(
                    |callback| callback.quest == "oteryn:quest.twenty_miles_beneath_the_sea_quest"
                )
                .count(),
            44 + 30
        );
        let expected: BTreeSet<_> = base
            .iter()
            .chain(bindings.world_callbacks())
            .map(|callback| &callback.key)
            .collect();
        let keys: BTreeSet<_> = callbacks.iter().map(|callback| &callback.key).collect();
        assert_eq!(keys.len(), callbacks.len());
        assert_eq!(keys, expected);
        let transition = catalogue
            .transition("oteryn:quest-transition/kv/bear_chest3/chest")
            .ok_or("bear claim transition")?;
        validate_resolved_world(catalogue, transition, false, &callbacks)?;
        let mut substituted = callbacks.clone();
        substituted[0] = callbacks[1].clone();
        assert_eq!(
            validate_resolved_world(catalogue, transition, false, &substituted),
            Err("resolved World source bounds".into())
        );
        Ok(())
    }

    #[test]
    fn embedded_fallback_rejects_a_duplicate_callback() -> Result<(), String> {
        let source = super::super::super::loader::load_embedded_quest_state("content-1")
            .map_err(|error| format!("{error:?}"))?;
        let transition = source
            .catalogue()
            .transition("oteryn:quest-transition/kv/bear_chest3/chest")
            .ok_or("bear claim transition")?;
        let catalogue =
            super::super::super::QuestStateCatalogue::new("resolved-fixture", vec![], vec![])
                .map_err(|error| format!("{error:?}"))?;
        let mut callbacks = world_callbacks::load_for(&catalogue)?;
        validate_resolved_world(&catalogue, transition, false, &callbacks)?;

        callbacks[0] = callbacks[1].clone();
        assert_eq!(
            validate_resolved_world(&catalogue, transition, false, &callbacks),
            Err("resolved World source bounds".into())
        );
        Ok(())
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
        let mut oversized = world_callbacks::load()?;
        let existing: BTreeSet<_> = oversized.iter().map(|row| row.key.clone()).collect();
        let required = MAX_RESOLVED_CALLBACKS + 1 - oversized.len();
        let published = registry()?;
        oversized.extend(
            reward_callbacks::load()?
                .into_iter()
                .filter(|row| {
                    published.owners.contains_key(&row.key) && !existing.contains(&row.key)
                })
                .take(required)
                .map(|row| {
                    let mut extra = callback.clone();
                    extra.key = row.key;
                    extra
                }),
        );
        assert_eq!(oversized.len(), MAX_RESOLVED_CALLBACKS + 1);
        assert_eq!(
            validate_resolved_world(catalogue, transition, false, &oversized),
            Err("resolved World source bounds".into())
        );
        Ok(())
    }

    #[test]
    fn embedded_fallback_accepts_the_registry_and_keeps_the_resolved_bounds() -> Result<(), String>
    {
        let catalogue =
            super::super::super::QuestStateCatalogue::new("resolved-fixture", vec![], vec![])
                .map_err(|error| format!("{error:?}"))?;
        assert!(catalogue.authored_world_callbacks().is_none());
        let source = super::super::super::loader::load_embedded_quest_state("content-1")
            .map_err(|error| format!("{error:?}"))?;
        let transition = source
            .catalogue()
            .transition("oteryn:quest-transition/kv/bear_chest3/chest")
            .ok_or("bear claim transition")?;
        let callbacks = world_callbacks::load_for(&catalogue)?;
        assert_eq!(callbacks, world_callbacks::load()?);
        validate_resolved_world(&catalogue, transition, false, &callbacks)?;

        let mut substituted = callbacks.clone();
        substituted[0].quest.push_str("-substituted");
        // The embedded loader caps the total length, so exercise the duplicate-key guard.
        substituted[0].key = callbacks[1].key.clone();
        assert_eq!(
            validate_resolved_world(&catalogue, transition, false, &substituted),
            Err("resolved World source bounds".into())
        );
        let oversized = vec![callbacks[0].clone(); MAX_RESOLVED_CALLBACKS + 1];
        assert!(oversized.len() > MAX_RESOLVED_CALLBACKS);
        assert_eq!(
            validate_resolved_world(&catalogue, transition, false, &oversized),
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
