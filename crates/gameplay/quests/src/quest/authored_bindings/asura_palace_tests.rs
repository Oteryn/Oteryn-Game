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
fn engenc_asura_chambers_and_orchid_require_owned_defenders_before_each_door() -> Result<(), String>
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
    let orchid = catalogue
        .transition("oteryn:quest-callback/engenc/asura/s5/entry")
        .ok_or("orchid entry")?;
    assert!(catalogue.evaluate(orchid, &state, 0).is_err());
    let claim = |name: &str, order: &str| {
        catalogue
            .transition(&format!(
                "oteryn:quest-callback/engenc/asura/s6/{name}/{order}"
            ))
            .ok_or("treasure")
    };
    assert!(
        catalogue
            .evaluate(claim("sapphires", "first")?, &state, 0)
            .is_err()
    );
    // The Flaming Orchid room (s5) holds one boss; the chambers hold two defenders each.
    for (stage, kills) in [("s2", 2), ("s3", 2), ("s4", 2), ("s5", 1)] {
        if stage == "s5" {
            assert!(catalogue.evaluate(orchid, &state, 0).is_ok());
        } else {
            assert!(catalogue.evaluate(orchid, &state, 0).is_err());
        }
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
        assert_eq!(counter.maximum, kills);
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
        for expected in 1..=kills {
            let changes = catalogue
                .evaluate(kill, &state, 0)
                .map_err(|e| format!("{e:?}"))?;
            assert_eq!(changes.len(), 1, "preserve encounter epoch");
            assert_eq!(changes[0].after, expected);
            assert!(!catalogue.completes_after(kill, &changes));
            for change in changes {
                state.insert(change.track, change.after);
            }
            if expected < kills {
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
    // Crystal 201189f8 the_lament_asuras/actions_doors.lua:2,12 gates only the Orchid door.
    for stage in &quest.stages {
        if let Trigger::MapUse { callback } = &stage.trigger {
            let level = (stage.key == "s5-door").then_some(250);
            assert_eq!(callback.minimum_level, level, "{}", stage.key);
        }
    }
    // Each top-floor container pays once; whichever is claimed second completes the quest.
    for (first, second) in [("sapphires", "rubies"), ("rubies", "sapphires")] {
        let mut state = state.clone();
        for (name, order, completes) in [(first, "first", false), (second, "last", true)] {
            // Before any claim the "last" variant is refused; after one, the "first" one is.
            let wrong = if completes { "first" } else { "last" };
            assert!(catalogue.evaluate(claim(name, wrong)?, &state, 0).is_err());
            let transition = claim(name, order)?;
            let changes = catalogue
                .evaluate(transition, &state, 0)
                .map_err(|e| format!("{e:?}"))?;
            assert_eq!(catalogue.completes_after(transition, &changes), completes);
            for change in changes {
                state.insert(change.track, change.after);
            }
            assert!(catalogue.evaluate(transition, &state, 0).is_err());
            assert!(
                catalogue
                    .evaluate(claim(name, "first")?, &state, 0)
                    .is_err()
            );
            assert!(catalogue.evaluate(claim(name, "last")?, &state, 0).is_err());
        }
    }
    Ok(())
}
