//! Native palace entry and bounded owner-authored chamber progression.
use super::*;
use std::collections::BTreeMap;

#[test]
fn engenc_asura_entry_is_native_premium_once_and_does_not_complete() -> Result<(), String> {
    let bytes = std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../content/quests/authored-bindings/engenc-asura-palace.json"),
    )
    .map_err(|error| error.to_string())?;
    let bindings = Bindings::parse(&bytes)?;
    bindings.verify_embedded_recipe_evidence()?;
    let quest = &bindings.quests[0];
    assert_eq!(quest.quest, "oteryn:quest.authored.asura_palace_quest");
    let stage = &quest.stages[0];
    assert_eq!(stage.key, "s1");
    assert!(!stage.completes);
    let Trigger::StepIn { callback } = &stage.trigger else {
        return Err("native entry".into());
    };
    assert!(callback.requires_world_admission);
    assert!(callback.admission.as_ref().is_some_and(|a| a.premium));
    let native = callback.selector.native.as_ref().ok_or("native selector")?;
    assert_eq!(
        (native.position.x, native.position.y, native.position.floor),
        (32953, 32685, -7)
    );
    assert_eq!((native.ordinal, native.appearance), (0, 923));
    let base = QuestStateCatalogue::empty("asura-entry").ok_or("base")?;
    let catalogue = bindings.attach(&base)?;
    let transition = catalogue.transition(&callback.key).ok_or("transition")?;
    let changes = catalogue
        .evaluate(transition, &BTreeMap::new(), 0)
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(changes.len(), 2);
    assert_eq!((changes[0].before, changes[0].after), (0, 1));
    assert!(!catalogue.completes_after(transition, &changes));
    let state = changes
        .iter()
        .map(|change| (change.track.clone(), change.after))
        .collect();
    assert!(catalogue.evaluate(transition, &state, 0).is_err());
    let production = crate::quest::loader::load_embedded_quest_state("asura-entry")
        .map_err(|error| format!("{error:?}"))?;
    assert!(production.catalogue().transition(&callback.key).is_some());
    Ok(())
}

#[test]
fn engenc_asura_three_chambers_require_both_owned_defenders_before_each_door() -> Result<(), String>
{
    let bytes = std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../content/quests/authored-bindings/engenc-asura-palace.json"),
    )
    .map_err(|e| e.to_string())?;
    let bindings = Bindings::parse(&bytes)?;
    bindings.verify_embedded_recipe_evidence()?;
    let catalogue =
        bindings.attach(&QuestStateCatalogue::empty("asura-chambers").ok_or("base")?)?;
    let quest = &bindings.quests[0];
    let mut state = BTreeMap::from([(
        "oteryn:quest-progress/authored/asura_palace_quest/s1".into(),
        1,
    )]);
    for stage in ["s2", "s3", "s4"] {
        let entry = format!("oteryn:quest-callback/engenc/asura/{stage}/entry");
        let door = format!("oteryn:quest-callback/engenc/asura/{stage}/door");
        let entry = catalogue.transition(&entry).ok_or("entry")?;
        let door = catalogue.transition(&door).ok_or("door")?;
        let Trigger::Kill { counter } = &quest
            .stages
            .iter()
            .find(|s| s.key == stage)
            .ok_or("counter")?
            .trigger
        else {
            return Err("kill".into());
        };
        assert_eq!(counter.maximum, 2);
        let kill = catalogue
            .transition(&counter.key)
            .ok_or("kill transition")?;
        assert_eq!(counter.gate_value, 1);
        assert_eq!(state.get(&counter.gate).copied().unwrap_or(0), 0);
        let changes = catalogue
            .evaluate(entry, &state, 0)
            .map_err(|e| format!("{e:?}"))?;
        for change in changes {
            state.insert(change.track, change.after);
        }
        assert!(catalogue.evaluate(door, &state, 0).is_err());
        for expected in 1..=2 {
            let changes = catalogue
                .evaluate(kill, &state, 0)
                .map_err(|e| format!("{e:?}"))?;
            assert_eq!(changes.len(), 1, "preserve encounter epoch");
            assert_eq!(changes[0].after, expected);
            assert!(!catalogue.completes_after(kill, &changes));
            for change in changes {
                state.insert(change.track, change.after);
            }
            if expected == 1 {
                assert!(catalogue.evaluate(door, &state, 0).is_err());
            }
        }
        assert!(catalogue.evaluate(kill, &state, 0).is_err());
        let changes = catalogue
            .evaluate(door, &state, 0)
            .map_err(|e| format!("{e:?}"))?;
        assert!(!catalogue.completes_after(door, &changes));
        for change in changes {
            state.insert(change.track, change.after);
        }
        assert!(catalogue.evaluate(door, &state, 0).is_err());
    }
    Ok(())
}
