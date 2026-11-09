//! Initial-value contract for explicitly bound imported EXPLORE stage counters.

use super::{ActorKind, Callback, Storage, Trigger};

/// Chosen exploration is a separate bounded profile, never a fabricated donor citation.
pub(super) fn admitted(callback: &Callback) -> bool {
    let Some(tail) = callback.quest.strip_prefix("oteryn:quest.") else {
        return false;
    };
    let prefix = tail.strip_prefix("authored.").map_or_else(
        || format!("oteryn:quest-progress/chosen-source/{tail}/"),
        |tail| format!("oteryn:quest-progress/authored/{tail}/"),
    );
    let Some(native) = &callback.selector.native else {
        return false;
    };
    callback.key.starts_with("oteryn:quest-callback/m01b/")
        && callback.trigger == Trigger::StepIn
        && callback.actor_kind == ActorKind::Player
        && callback.requires_world_admission
        && callback
            .admission
            .as_ref()
            .is_none_or(|a| a.premium && !a.regular_blessings && !a.promotion)
        && callback.selector.position == Some(native.position)
        && callback.selector.appearance == Some(native.appearance)
        && native.ordinal == 0
        && callback.selector.unique.is_none()
        && callback.selector.action.is_none()
        && callback.lever.is_none()
        && !callback.reversible
        && !callback.opening_only
        && callback.passages.is_empty()
        && callback.teleport.is_none()
        && callback.facing.is_none()
        && !callback.return_on_failure
        && callback.effects.is_empty()
        && callback.offerings.is_empty()
        && callback.item_on_target.is_none()
        && callback.experience.is_none()
        && callback.monk_reward.is_none()
        && callback.native_rule.is_none()
        && callback.storage.iter().all(|s| {
            s.track.starts_with(&prefix) && s.minimum >= 0 && s.maximum >= s.minimum && !s.at_least
        })
        && callback.storage.iter().filter(|s| !s.preserve).count() == 2
        && callback
            .storage
            .iter()
            .filter(|s| !s.preserve && s.increment && s.set == 1)
            .count()
            == 1
        && callback
            .storage
            .iter()
            .filter(|s| {
                !s.preserve
                    && !s.increment
                    && s.track.contains("/visited/")
                    && s.minimum == 0
                    && s.maximum == 0
                    && s.set == 1
            })
            .count()
            == 1
        && callback.sources.iter().any(|s| {
            matches!(
                s.as_str(),
                "OTERYN_CHOSEN_EXPLORE_ENTRY:canonical-area-point;first-cycle-only"
                    | "OTERYN_CHOSEN_EXPLORE_ENTRY:source-map-approach;first-cycle-only"
                    | "OTERYN_CHOSEN_EXPLORE_ENTRY:native-layer-approach;first-cycle-only"
            )
        })
}

pub(super) fn initial(callback: &Callback, storage: &Storage) -> i64 {
    if let Some(initial) = super::character_requirements::entry_initial(callback, storage) {
        return initial;
    }
    let chosen_entry = callback
        .quest
        .strip_prefix("oteryn:quest.")
        .is_some_and(|quest| {
            storage.track == format!("oteryn:quest-progress/chosen-source/{quest}/s1")
                && callback.selector.native.is_some()
                && callback.requires_world_admission
                && callback.actor_kind == ActorKind::Player
                && !callback.reversible
                && storage.minimum <= 0
                && storage.maximum >= 0
                && storage.at_least
                && storage.set == 1
                && !storage.preserve
                && !storage.increment
                && callback.sources.iter().any(|source| {
                    source == "OTERYN_CHOSEN_ADMISSION:recipe.requirements;initial-entry-only"
                })
        });
    if (chosen_entry && callback.key != "oteryn:quest-callback/qm03/pits")
        || (admitted(callback) && !storage.track.is_empty())
    {
        0
    } else {
        -1
    }
}

#[cfg(test)]
mod tests {
    use super::super::*;

    #[test]
    fn imported_explore_cobra_preserves_predecessor_bounds_and_binds_the_guards()
    -> Result<(), String> {
        let state = crate::quest::loader::load_embedded_quest_state("content-1")
            .map_err(|error| format!("{error:?}"))?;
        let callback = load()?
            .into_iter()
            .find(|c| c.key == "oteryn:quest-callback/m01b/grave-danger/s4/entry")
            .ok_or("bound Cobra investigation")?;
        assert!(super::admitted(&callback));
        for (stage, maximum) in [("s1", 12), ("s2", 5), ("s3", 1), ("s4", 1)] {
            let track = state
                .catalogue()
                .track(&format!(
                    "oteryn:quest-progress/chosen-source/grave_danger_quest/{stage}"
                ))
                .ok_or("Cobra predecessor track")?;
            assert_eq!((track.initial, track.min, track.max), (0, 0, maximum));
        }
        assert_eq!(callback.storage.iter().filter(|s| s.preserve).count(), 3);
        let bound = gated_binding(&callback)?;
        let mut stripped = callback;
        let guard = stripped
            .storage
            .iter_mut()
            .find(|s| s.track.ends_with("/s3"))
            .ok_or("King Zelos guard")?;
        guard.minimum = 0;
        guard.maximum = 0;
        guard.set = 0;
        assert_ne!(
            gated_binding(&stripped)?,
            bound,
            "boss guards belong to native authority"
        );
        Ok(())
    }

    #[test]
    fn imported_explore_fury_layer_approach_is_admitted_and_city_bound() -> Result<(), String> {
        crate::quest::loader::load_embedded_quest_state("content-1")
            .map_err(|error| format!("{error:?}"))?;
        let callback = load()?
            .into_iter()
            .find(|c| c.key == "oteryn:quest-callback/m01b/fury-gates-mini-world-change/s1/entry")
            .ok_or("bound Fury survey")?;
        assert!(super::admitted(&callback));
        assert_eq!(callback.world_change.as_deref(), Some("fury_gates/thais"));
        let binding = gated_binding(&callback)?;
        assert!(binding.is_some());
        for replacement in [None, Some("fury_gates"), Some("fury_gates/venore")] {
            let mut altered = callback.clone();
            altered.world_change = replacement.map(str::to_owned);
            assert_ne!(gated_binding(&altered)?, binding);
        }
        Ok(())
    }

    #[test]
    fn imported_explore_world_requirement_is_part_of_the_native_binding() -> Result<(), String> {
        let callback = load()?
            .into_iter()
            .find(|c| c.key == "oteryn:quest-callback/m01b/grimvale-mini-world-change/s1/entry")
            .ok_or("bound current-moon entry")?;
        assert!(super::admitted(&callback));
        assert_eq!(callback.world_change.as_deref(), Some("grimvale"));
        let required = gated_binding(&callback)?;
        assert!(required.is_some(), "native World-bound occurrence");
        let mut stripped = callback;
        stripped.world_change = None;
        assert_ne!(
            gated_binding(&stripped)?,
            required,
            "the admitted native occurrence cannot lose its current-World predicate"
        );
        stripped.world_change = Some("fury_gates".into());
        assert_ne!(
            gated_binding(&stripped)?,
            required,
            "a different World event cannot substitute for the admitted moon predicate"
        );
        Ok(())
    }

    #[test]
    fn alawar_source_approach_is_bound_without_changing_its_teleport() -> Result<(), String> {
        let callbacks = load()?;
        let approach = callbacks
            .iter()
            .find(|c| c.key == "oteryn:quest-callback/m01b/alawar-s-vault/s1/entry")
            .ok_or("bound Alawar approach")?;
        assert!(super::admitted(approach));
        assert!(approach.teleport.is_none());
        let original = callbacks
            .iter()
            .find(|c| c.key == "oteryn:quest-callback/alawars-vault-entrance")
            .ok_or("original Alawar portal")?;
        assert_eq!(
            original.teleport,
            Some(Position {
                x: 32107,
                y: 31567,
                floor: -9
            })
        );
        assert!(original.storage.is_empty());
        Ok(())
    }

    #[test]
    fn asura_explore_binding_cannot_strip_current_premium() -> Result<(), String> {
        let bindings = crate::quest::authored_bindings::Bindings::parse(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/quests/authored-bindings/engenc-asura-palace.json"
        )))?;
        let callback = bindings
            .world_callbacks()
            .find(|c| c.key == "oteryn:quest-callback/m01b/asura-palace/s1/entry")
            .cloned()
            .ok_or("bound Asura entry")?;
        assert!(callback.admission.as_ref().is_some_and(|a| a.premium));
        assert!(super::admitted(&callback));
        let mut unsupported = callback.clone();
        unsupported
            .admission
            .as_mut()
            .ok_or("Premium policy")?
            .promotion = true;
        assert!(
            !super::admitted(&unsupported),
            "this bounded explore profile cannot silently admit an unsupported predicate"
        );
        let required = gated_binding(&callback)?.ok_or("bound Premium occurrence")?;
        let mut stripped = callback;
        stripped.admission = None;
        assert_ne!(
            gated_binding(&stripped)?,
            Some(required),
            "a native occurrence admitted for a different access policy cannot be substituted"
        );
        Ok(())
    }

    #[test]
    fn geomancer_explore_entry_has_its_own_zero_initial_counter() -> Result<(), String> {
        let callbacks = load()?;
        let callback = callbacks
            .iter()
            .find(|c| c.key == "oteryn:quest-callback/m01b/geomancer/s1/entry")
            .ok_or("bound Geomancer entry")?;
        assert!(super::admitted(callback));
        let catalogue = extend_callbacks(
            QuestStateCatalogue::new("content-1", vec![], vec![]).map_err(|e| format!("{e:?}"))?,
            std::slice::from_ref(callback),
        )?;
        let track = catalogue
            .track("oteryn:quest-progress/chosen-source/geomancer_quest/s1")
            .ok_or("Geomancer explore counter")?;
        assert_eq!((track.initial, track.min, track.max), (0, 0, 1));
        assert!(
            catalogue
                .track("oteryn:quest-progress/chosen-source/longsword_quest/s1")
                .is_none()
        );
        Ok(())
    }

    #[test]
    fn qm55_held_chakoya_and_admitted_premium_entry_credit_once() -> Result<(), String> {
        let callbacks = load()?;
        assert!(
            !callbacks
                .iter()
                .any(|c| c.quest == "oteryn:quest.authored.chakoya_iceberg_mini_world_change"),
            "M31: the HELD World owner cannot supply an EXPLORE callback"
        );
        let bindings = crate::quest::authored_bindings::Bindings::parse(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/quests/authored-bindings/engenc-asura-palace.json"
        )))?;
        let callback = bindings
            .world_callbacks()
            .find(|c| c.quest == "oteryn:quest.authored.asura_palace_quest")
            .ok_or("bound Asura Palace entry")?;
        assert!(!callbacks.iter().any(|other| other.key == callback.key));
        assert!(super::admitted(callback));
        assert!(callback.admission.as_ref().is_some_and(|gate| gate.premium));
        let catalogue = extend_callbacks(
            QuestStateCatalogue::new("content-1", vec![], vec![]).map_err(|e| format!("{e:?}"))?,
            std::slice::from_ref(callback),
        )?;
        let transition = catalogue
            .transition(&callback.key)
            .ok_or("entry transition")?;
        assert!(!transition.completes);
        assert!(transition.experience.is_none());
        let track = "oteryn:quest-progress/authored/asura_palace_quest/s1";
        assert_eq!(catalogue.track(track).ok_or("canonical stage")?.initial, 0);
        let changes = catalogue
            .evaluate(transition, &std::collections::BTreeMap::new(), 0)
            .map_err(|e| format!("first entry: {e:?}"))?;
        assert_eq!(changes.len(), 2);
        assert!(
            changes
                .iter()
                .all(|change| change.before == 0 && change.after == 1)
        );
        assert!(changes.iter().any(|change| change.track == track));
        let stored = changes
            .into_iter()
            .map(|change| (change.track, change.after))
            .collect();
        assert_eq!(
            catalogue.evaluate(transition, &stored, 0),
            Err(crate::quest::QuestRefusal::StageMismatch)
        );
        Ok(())
    }

    #[test]
    fn imported_explore_counter_and_visit_start_at_zero() -> Result<(), String> {
        let callbacks = load()?;
        let callback = callbacks
            .iter()
            .find(|c| c.key == "oteryn:quest-callback/m01b/longsword/s1/entry")
            .ok_or("bound Longsword entry")?;
        let catalogue = extend_callbacks(
            QuestStateCatalogue::new("content-1", vec![], vec![]).map_err(|e| format!("{e:?}"))?,
            std::slice::from_ref(callback),
        )?;
        for storage in &callback.storage {
            let track = catalogue.track(&storage.track).ok_or("explore track")?;
            assert_eq!(
                (track.initial, track.min, track.max),
                (0, 0, 1),
                "imported stage counters must not inherit the source absent=-1 sentinel"
            );
        }
        Ok(())
    }

    #[test]
    fn chosen_explore_profile_cannot_admit_use_or_foreign_tracks() -> Result<(), String> {
        let callbacks = load()?;
        let original = callbacks
            .iter()
            .find(|c| c.key == "oteryn:quest-callback/m01b/longsword/s1/entry")
            .ok_or("bound Longsword entry")?;
        assert!(super::admitted(original));
        let mut callback = original.clone();
        callback.trigger = Trigger::Use;
        assert!(!super::admitted(&callback));
        callback = original.clone();
        callback.storage[0].track = "oteryn:quest-progress/foreign/s1".into();
        assert!(!super::admitted(&callback));
        callback = original.clone();
        callback.sources.clear();
        assert!(!super::admitted(&callback));
        callback = original.clone();
        callback.experience = Some(1);
        callback
            .sources
            .push("crystal:201189f871d23597b82caf88fd611454a2ddefb8:fixture".into());
        assert!(
            parse(&serde_json::json!({"schema": 1, "callbacks": [callback]}).to_string()).is_err()
        );
        let legacy = callbacks
            .iter()
            .find(|c| c.key == "oteryn:quest-callback/fish-napping-key")
            .ok_or("legacy fish callback")?;
        assert!(!super::admitted(legacy));
        assert_eq!(
            super::initial(legacy, legacy.storage.first().ok_or("legacy storage")?),
            -1
        );
        Ok(())
    }
}
