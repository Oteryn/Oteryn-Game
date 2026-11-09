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
        && callback
            .sources
            .iter()
            .any(|s| s == "OTERYN_CHOSEN_EXPLORE_ENTRY:canonical-area-point;first-cycle-only")
}

pub(super) fn initial(callback: &Callback, storage: &Storage) -> i64 {
    if admitted(callback) && !storage.track.is_empty() {
        0
    } else {
        -1
    }
}

#[cfg(test)]
mod tests {
    use super::super::*;

    #[test]
    fn qm55_chakoya_premium_entry_credits_the_canonical_stage_once() -> Result<(), String> {
        let callbacks = load()?;
        let callback = callbacks
            .iter()
            .find(|c| c.quest == "oteryn:quest.authored.chakoya_iceberg_mini_world_change")
            .ok_or("bound Chakoya entry")?;
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
        let track = "oteryn:quest-progress/authored/chakoya_iceberg_mini_world_change/s1";
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
