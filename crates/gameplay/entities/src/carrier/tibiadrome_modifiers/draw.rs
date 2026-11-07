//! Registered drome_modifier/drome_wave streams, with unbiased bounded draws.
use super::*;
/// Rotation-only seed: Worlds share the same two distinct modifiers.
pub fn drome_rotation_modifiers(ordinal: u32) -> [DromeModifier; 2] {
    let mut seed = [0; 32];
    seed[..4].copy_from_slice(&ordinal.to_be_bytes());
    let root = oteryn_simulation_determinism::GameplayDecisionRoot::from_bytes(seed);
    let occurrence = oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes([0; 16]);
    let draw =
        |index, count| bounded_draw(&root, occurrence, "drome_modifier", index, count).unwrap_or(0);
    let first = draw(0, 9);
    let second = draw(1, 8);
    [
        ALL[first],
        ALL[if second >= first { second + 1 } else { second }],
    ]
}
const ALL: [DromeModifier; 9] = [
    DromeModifier::Somersault,
    DromeModifier::GoingDownWithMe,
    DromeModifier::ExplodingCorpses,
    DromeModifier::ThatEscalatedQuickly,
    DromeModifier::TheFloorIsLava,
    DromeModifier::BeamMeUp,
    DromeModifier::TankedUp,
    DromeModifier::SownSorrow,
    DromeModifier::BadRoots,
];

pub(super) fn bounded_draw(
    root: &oteryn_simulation_determinism::GameplayDecisionRoot,
    occurrence: oteryn_simulation_determinism::DecisionOccurrenceId,
    purpose: &str,
    index: u64,
    count: usize,
) -> Option<usize> {
    let count = u64::try_from(count).ok().filter(|count| *count > 0)?;
    let ceiling = u64::MAX - u64::MAX % count;
    let mut draw = index.checked_mul(128)?;
    loop {
        let value = oteryn_simulation_determinism::deterministic_decision_u64(
            root, occurrence, purpose, draw,
        )
        .ok()?;
        if value < ceiling {
            return usize::try_from(value % count).ok();
        }
        draw = draw.checked_add(1)?;
    }
}
