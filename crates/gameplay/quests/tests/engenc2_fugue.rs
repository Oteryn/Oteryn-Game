use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffectKind, loader::load_embedded_quest_state, native_kills,
};
use std::collections::BTreeMap;

#[test]
fn fugue_death_requires_kesar_two_and_resets_the_source_spawn_flag() -> Result<(), String> {
    let counter = native_kills::counter("oteryn:quest-transition/native-kills/fugue")
        .ok_or("Fugue native death is not registered")?;
    let loaded = load_embedded_quest_state("engenc2-fugue").map_err(|e| format!("{e:?}"))?;
    let transition = loaded
        .catalogue()
        .transition(counter.transition)
        .ok_or("production Fugue transition")?;
    assert_eq!(counter.creature, "oteryn:creature.fugue");
    assert_eq!(counter.gate_bounds(), (2, 2));
    let mut tracks = BTreeMap::new();
    for value in [-1, 0, 1, 3, 4] {
        tracks.insert(counter.gate.into(), value);
        assert!(!native_kills::active(&tracks).contains(&counter.transition));
    }
    tracks.insert(counter.gate.into(), 2);
    assert!(native_kills::active(&tracks).contains(&counter.transition));
    assert_eq!(transition.effects.len(), 2);
    assert_eq!(transition.effects[0].from, QuestComparison::Eq(2));
    assert_eq!(transition.effects[0].effect, QuestEffectKind::Set(3));
    assert!(transition.effects[1].track.ends_with("/fugue_spawned"));
    assert_eq!(transition.effects[1].effect, QuestEffectKind::Set(0));
    Ok(())
}
