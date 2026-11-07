//! Vocationless admission uses the same accepted maxima and owned build as every class.
use super::*;

pub(super) fn load(
    build: &DurableBuildState,
    progression: Option<&CharacterProgressionState>,
    character_revision: CharacterRevision,
) -> CastFactsLoad {
    let level = progression.map_or(1, |progress| progress.level);
    match facts(build, level) {
        Some(facts) => CastFactsLoad::NoVocation {
            facts,
            character_revision,
        },
        None => CastFactsLoad::Unavailable,
    }
}

pub(super) fn facts(build: &DurableBuildState, level: u32) -> Option<CharacterCastFacts> {
    let (max_health, max_mana, max_soul) = maxima(NO_VOCATION, level)?;
    Some(CharacterCastFacts {
        vocation: Vocation::None,
        level,
        magic_level: u32::from(build.magic().0),
        max_health,
        max_mana,
        max_soul,
    })
}
