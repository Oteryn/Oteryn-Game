//! Reuse the facts loader's existing root/build/progression fixtures.
use super::*;

#[test]
fn vocationless_vitals_use_owned_magic_and_level_or_the_level_one_seed() {
    let root = root();
    for (xp, level, health, mana) in [(None, 1, 150, 55), (Some(progression(&root)), 20, 245, 150)]
    {
        let build = build("none");
        let loaded = facts_from_owned_state(root.character_id, &root, &root, &build, xp.as_ref());
        assert!(
            matches!(loaded, CastFactsLoad::NoVocation { facts, character_revision }
            if facts.vocation == Vocation::None && facts.level == level
                && facts.magic_level == 42 && facts.max_health == health
                && facts.max_mana == mana && facts.max_soul == 100
                && character_revision == root.revision)
        );
    }
}
