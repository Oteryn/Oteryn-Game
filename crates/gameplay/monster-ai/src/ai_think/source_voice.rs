//! Ordinary monster speech uses its admitted voice list and the active think cadence.
use super::*;

impl CreatureAiState {
    pub(super) fn select_voice(
        &mut self,
        occurrence: ThinkOccurrence,
        root: &GameplayDecisionRoot,
    ) -> Option<usize> {
        let voices = self.profile.behavior().voices.as_ref()?;
        if voices.interval_ms == 0 || voices.entries.is_empty() {
            return None;
        }

        // Secondary reference: Crystal 201189f8 monster.cpp:1424-1444 accrues active
        // think ticks and resets the clock at each roll, including a failed roll.
        self.voice_ticks_ms = self
            .voice_ticks_ms
            .saturating_add(CREATURE_THINK_INTERVAL_MILLIS);
        if self.voice_ticks_ms < voices.interval_ms {
            return None;
        }
        self.voice_ticks_ms = 0;
        let occurrence = decision_occurrence(occurrence);
        let chance = deterministic_decision_u64(root, occurrence, "AI_VOICE", 0).ok()?;
        if chance % PPM >= u64::from(voices.chance_ppm) {
            return None;
        }
        let line = deterministic_decision_u64(root, occurrence, "AI_VOICE", 1).ok()?;
        Some((line % voices.entries.len() as u64) as usize)
    }
}
