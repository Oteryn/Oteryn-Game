//! Capability 486 `ANALYSER_HIT_ATTRIBUTION_V1`: a received hit names its attacker.
use oteryn_client_runtime::analyser::{AnalyserFact, AttributedFact};

/// The facts without their attackers, as the analyser reducer consumes them.
#[must_use]
pub fn plain_facts(attributed: &[AttributedFact]) -> Vec<AnalyserFact> {
    attributed.iter().map(|entry| entry.fact.clone()).collect()
}

/// N006: the own character took a hit from an attacker while the player has an attack target,
/// so the client flashes a black frame around the own character (Tibia). A hit without an
/// attacker, or while no target is attacked, never flashes.
#[must_use]
pub fn own_hit_flash(attributed: &[AttributedFact], has_attack_target: bool) -> bool {
    has_attack_target
        && attributed.iter().any(|entry| {
            entry.attacker.is_some() && matches!(entry.fact, AnalyserFact::DamageInput { .. })
        })
}

/// How long the own-character frame blinks after a hit.
pub const HIT_FLASH_LIFETIME: std::time::Duration = std::time::Duration::from_millis(300);
const HIT_FLASH_PHASE_MS: u128 = 100;

/// Whether the hit frame is drawn `age` after the hit: black for 100 ms, off for 100 ms, black
/// for 100 ms, then gone.
#[must_use]
pub fn hit_flash_visible(age: std::time::Duration) -> bool {
    age < HIT_FLASH_LIFETIME && (age.as_millis() / HIT_FLASH_PHASE_MS).is_multiple_of(2)
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
