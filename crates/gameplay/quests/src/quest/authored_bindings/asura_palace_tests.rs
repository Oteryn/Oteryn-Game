//! Only the source-resolved entry is admitted; chamber identities must be explicit.
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
