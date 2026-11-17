//! File-authored counters use the same exact-operation boundary as retained counters.
use super::*;

#[test]
fn authored_native_counter_creatures_are_captured_only_for_loaded_catalogues() -> Result<(), String>
{
    let loaded = crate::quest::loader::load_embedded_quest_state("content-1")
        .map_err(|error| format!("{error:?}"))?;
    let empty =
        crate::quest::QuestStateCatalogue::empty("content-1").ok_or("empty catalogue rejected")?;
    for counter in qworld::counters()? {
        assert!(
            tracks_creature_for(loaded.catalogue(), counter.creature),
            "{}",
            counter.creature
        );
        assert!(!tracks_creature_for(&empty, counter.creature));
    }
    Ok(())
}

#[test]
fn authored_native_counter_rejects_changed_and_renamed_operations() -> Result<(), String> {
    let counters = qworld::counters()?;
    assert!(!counters.is_empty());
    for counter in counters {
        let operation = counter.transition();
        crate::quest::admission_binding::validate(&operation, false)?;
        let mut changed = operation.clone();
        changed.effects[1].effect = crate::quest::QuestEffectKind::Add(2);
        assert!(
            crate::quest::admission_binding::validate(&changed, false).is_err(),
            "one death cannot become two credits for {}",
            counter.transition
        );
        let mut renamed = operation;
        renamed.key = "oteryn:quest-transition/forged-native-counter".into();
        assert!(crate::quest::admission_binding::validate(&renamed, false).is_err());
    }
    Ok(())
}
