//! Exact owner-authored Asura callbacks; no donor implementation is claimed.
use super::Callback;

pub(super) fn admitted(callback: &Callback) -> bool {
    if callback.quest != "oteryn:quest.authored.asura_palace_quest"
        || !callback
            .key
            .starts_with("oteryn:quest-callback/engenc/asura/")
        || !callback
            .sources
            .iter()
            .any(|s| s == "authored per owner rule")
    {
        return false;
    }
    let Ok(document) = serde_json::from_str::<serde_json::Value>(include_str!(
        "../../../../../../content/quests/authored-bindings/engenc-asura-palace.json"
    )) else {
        return false;
    };
    document["quests"][0]["stages"]
        .as_array()
        .is_some_and(|stages| {
            stages.iter().any(|stage| {
                serde_json::from_value::<Callback>(stage["trigger"]["callback"].clone())
                    .is_ok_and(|bound| bound == *callback)
            })
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asura_chamber_loader_allows_fresh_use_with_saved_flags() -> Result<(), String> {
        use crate::quest::{authored_bindings::Bindings, loader, world_callbacks};
        use std::collections::BTreeMap;

        let bindings = Bindings::parse(include_bytes!(
            "../../../../../../content/quests/authored-bindings/engenc-asura-palace.json"
        ))?;
        let loaded = loader::load_embedded_quest_state_with_bindings("content-1", &bindings)
            .map_err(|error| format!("{error:?}"))?;
        let catalogue = loaded.catalogue();
        let callbacks = world_callbacks::load_for(catalogue)?;
        for (stage, previous) in [
            ("s2", "oteryn:quest-progress/authored/asura_palace_quest/s1"),
            ("s3", "oteryn:quest-progress/engenc/asura/s2/opened"),
            ("s4", "oteryn:quest-progress/engenc/asura/s3/opened"),
        ] {
            let entered = format!("oteryn:quest-progress/engenc/asura/{stage}/entered");
            let count = format!("oteryn:quest-progress/engenc/asura/{stage}/count");
            let opened = format!("oteryn:quest-progress/engenc/asura/{stage}/opened");
            let mut saved =
                BTreeMap::from([(previous.to_owned(), 1), (entered, 1), (count.clone(), 1)]);
            for part in ["entry", "door"] {
                if part == "door" {
                    saved.insert(count.clone(), 2);
                    saved.insert(opened.clone(), 1);
                }
                let key = format!("oteryn:quest-callback/engenc/asura/{stage}/{part}");
                let callback = callbacks
                    .iter()
                    .find(|callback| callback.key == key)
                    .ok_or(&key)?;
                assert!(
                    callback.activation.as_ref().is_none_or(|activation| {
                        saved.get(&activation.track).is_some_and(|value| {
                            (activation.minimum..=activation.maximum).contains(value)
                        })
                    }),
                    "{key} cannot be selected with saved progress"
                );
                let transition = catalogue.transition(&key).ok_or(&key)?;
                let result = catalogue.evaluate(transition, &saved, 0);
                assert!(result.is_ok(), "{key} refuses saved progress: {result:?}");
                if stage == "s4" && part == "entry" || stage == "s3" && part == "door" {
                    let mut orchid_seen = saved.clone();
                    orchid_seen.insert("oteryn:quest-progress/engenc/asura/s3/opened".into(), 2);
                    let changes = catalogue
                        .evaluate(transition, &orchid_seen, 0)
                        .map_err(|error| format!("{key} after Orchid: {error:?}"))?;
                    assert!(changes.iter().all(|change| change.before == change.after));
                }
            }
        }
        Ok(())
    }

    #[test]
    fn asura_loader_keeps_door_admission_personal() -> Result<(), String> {
        use crate::quest::{authored_bindings::Bindings, loader, world_callbacks};
        let bindings = Bindings::parse(include_bytes!(
            "../../../../../../content/quests/authored-bindings/engenc-asura-palace.json"
        ))?;
        let loaded = loader::load_embedded_quest_state_with_bindings("content-1", &bindings)
            .map_err(|error| format!("{error:?}"))?;
        let callbacks = world_callbacks::load_for(loaded.catalogue())?;
        for suffix in ["s2/door", "s3/door", "s4/door", "orchid/library-door"] {
            let key = format!("oteryn:quest-callback/engenc/asura/{suffix}");
            let door = callbacks.iter().find(|door| door.key == key).ok_or(&key)?;
            assert!(
                door.passages.is_empty(),
                "{key} opens shared STEP collision"
            );
            assert_eq!(door.effects.transform, None);
            let native = door.selector.native.as_ref().ok_or("native door")?.position;
            let target = door.teleport.ok_or("personal crossing")?;
            assert_eq!(target.floor, native.floor);
            assert_ne!(target, native);
            assert!(target.x.abs_diff(native.x) <= 1 && target.y.abs_diff(native.y) <= 1);
        }
        Ok(())
    }

    #[test]
    fn orchid_loader_allows_fresh_use_after_runtime_reset() -> Result<(), String> {
        use crate::quest::{authored_bindings::Bindings, loader};

        let bindings = Bindings::parse(include_bytes!(
            "../../../../../../content/quests/authored-bindings/engenc-asura-palace.json"
        ))?;
        let loaded = loader::load_embedded_quest_state_with_bindings("content-1", &bindings)
            .map_err(|error| format!("{error:?}"))?;
        let catalogue = loaded.catalogue();
        let tracks = std::collections::BTreeMap::from([
            ("oteryn:quest-progress/engenc/asura/s3/opened".into(), 2),
            ("oteryn:quest-progress/engenc/asura/s4/opened".into(), 1),
        ]);
        let entry = catalogue
            .transition("oteryn:quest-callback/engenc/asura/orchid/entry")
            .ok_or("Orchid entry transition")?;
        assert!(
            catalogue.evaluate(entry, &tracks, 0).is_ok(),
            "saved admission cannot restart the unfinished encounter"
        );
        Ok(())
    }

    #[test]
    fn orchid_loader_allows_library_door_to_reopen() -> Result<(), String> {
        use crate::quest::{authored_bindings::Bindings, loader};

        let bindings = Bindings::parse(include_bytes!(
            "../../../../../../content/quests/authored-bindings/engenc-asura-palace.json"
        ))?;
        let loaded = loader::load_embedded_quest_state_with_bindings("content-1", &bindings)
            .map_err(|error| format!("{error:?}"))?;
        let catalogue = loaded.catalogue();
        let tracks = std::collections::BTreeMap::from([
            ("oteryn:quest-progress/engenc/asura/orchid/count".into(), 1),
            (
                "oteryn:quest-progress/engenc/asura/orchid/library-open".into(),
                1,
            ),
        ]);
        let door = catalogue
            .transition("oteryn:quest-callback/engenc/asura/orchid/library-door")
            .ok_or("Orchid library door transition")?;
        assert!(
            catalogue.evaluate(door, &tracks, 0).is_ok(),
            "saved library flag prevents reopening the closed door"
        );
        Ok(())
    }

    #[test]
    fn orchid_native_loader_keeps_gate_reward_and_completion() -> Result<(), String> {
        use crate::quest::{authored_bindings::Bindings, loader, world_callbacks};

        let bindings = Bindings::parse(include_bytes!(
            "../../../../../../content/quests/authored-bindings/engenc-asura-palace.json"
        ))?;
        let loaded = loader::load_embedded_quest_state_with_bindings("content-1", &bindings)
            .map_err(|error| format!("{error:?}"))?;
        let catalogue = loaded.catalogue();
        let callbacks = world_callbacks::load_for(catalogue)?;
        let find = |part: &str| {
            let key = format!("oteryn:quest-callback/engenc/asura/orchid/{part}");
            callbacks
                .iter()
                .find(|callback| callback.key == key)
                .ok_or(key)
        };
        let entry = find("entry")?;
        assert_eq!(entry.trigger, world_callbacks::Trigger::Use);
        assert_eq!(
            entry
                .selector
                .native
                .as_ref()
                .ok_or("native door")?
                .position
                .x,
            32959
        );
        let encounter = entry.effects.encounter.as_ref().ok_or("Orchid encounter")?;
        assert_eq!(encounter.monsters.len(), 1);
        assert_eq!(
            encounter.monsters[0].creature,
            "oteryn:creature.the_flaming_orchid"
        );
        let death = catalogue
            .transition("oteryn:quest-transition/native-kills/engenc-asura-orchid")
            .ok_or("Orchid death")?;
        assert!(!death.completes);
        let ruby = find("ruby")?;
        assert!(
            catalogue
                .transition(&ruby.key)
                .ok_or("ruby claim")?
                .completes
        );
        assert_eq!(ruby.item_reward().as_ref().ok_or("ruby reward")?.count, 3);
        assert_eq!(
            find("sapphire")?
                .item_reward()
                .as_ref()
                .ok_or("sapphire reward")?
                .count,
            3
        );
        let door = find("library-door")?;
        assert_eq!(door.minimum_level, Some(250));
        assert!(door.teleport.is_some());
        Ok(())
    }

    #[test]
    fn engenc_authored_admission_is_exact_and_cannot_rebind_native_doors() -> Result<(), String> {
        let document: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../../content/quests/authored-bindings/engenc-asura-palace.json"
        ))
        .map_err(|e| e.to_string())?;
        let mut count = 0;
        for stage in document["quests"][0]["stages"].as_array().ok_or("stages")? {
            let Ok(mut callback) =
                serde_json::from_value::<Callback>(stage["trigger"]["callback"].clone())
            else {
                continue;
            };
            if !callback
                .key
                .starts_with("oteryn:quest-callback/engenc/asura/")
            {
                continue;
            }
            assert!(admitted(&callback));
            count += 1;
            if let Some(position) = callback.selector.position.as_mut() {
                position.x += 1;
            } else {
                callback
                    .selector
                    .native
                    .as_mut()
                    .ok_or("native position")?
                    .position
                    .x += 1;
            }
            assert!(!admitted(&callback));
        }
        assert_eq!(count, 10);
        Ok(())
    }
}
