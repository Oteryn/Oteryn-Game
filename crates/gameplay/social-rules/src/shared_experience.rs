//! Pure shared-experience activity times and captured-party heal credit.

use oteryn_simulation_determinism::SemanticTimeMicros;

/// PARTY-PVP0 §5.1 / PARTYPVP0-RL-06: two minutes, in the native semantic clock.
pub const SHARED_EXPERIENCE_ACTIVITY_MICROS: u64 = 120_000_000;

#[derive(Debug, Clone, Copy)]
pub struct SharedExperienceActivity {
    pub character: [u8; 16],
    pub lease_generation: u64,
    pub at: SemanticTimeMicros,
}

/// Retained committed event time observed under current native actor/session/lease guards.
/// Neither timestamp establishes current party membership or kill-settlement authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SharedExperienceActivityObservation {
    pub original_event_at: SemanticTimeMicros,
    pub observed_at: SemanticTimeMicros,
}

/// Retain the latest committed event under a character and lease generation.
pub fn record_shared_experience_activity(
    activity: &mut Option<SharedExperienceActivity>,
    next: SharedExperienceActivity,
) -> bool {
    if activity.is_some_and(|old| {
        old.character == next.character
            && old.lease_generation == next.lease_generation
            && old.at > next.at
    }) {
        return false;
    }
    *activity = Some(next);
    true
}

/// Whether a retained committed event is within the two-minute activity window.
pub fn shared_experience_active(activity: SharedExperienceActivityObservation) -> bool {
    activity
        .observed_at
        .elapsed_since(activity.original_event_at)
        .is_ok_and(|age| age <= SHARED_EXPERIENCE_ACTIVITY_MICROS)
}

/// Observe retained activity for the currently fenced character and generation.
pub fn committed_shared_experience_activity(
    activity: SharedExperienceActivity,
    character: &[u8; 16],
    lease_generation: u64,
    observed_at: SemanticTimeMicros,
) -> Option<SharedExperienceActivityObservation> {
    if character != &activity.character
        || lease_generation != activity.lease_generation
        || observed_at.elapsed_since(activity.at).is_err()
    {
        return None;
    }
    Some(SharedExperienceActivityObservation {
        original_event_at: activity.at,
        observed_at,
    })
}

/// Shared experience credits a heal only when it heals ANOTHER member of the caster's
/// captured party. A self-heal never counts, including an area heal such as Mass Healing
/// that reaches only the caster.
pub fn heal_credits_party_member<'a>(
    caster: &[u8; 16],
    members: impl IntoIterator<Item = &'a [u8; 16]>,
    target_character: &[u8; 16],
) -> bool {
    target_character != caster && members.into_iter().any(|member| member == target_character)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// LOOP-PROGRESS-1 (Codex P2): a Mass Healing that heals only the caster earns no
    /// activity; healing another member of the same party still does.
    #[test]
    fn heal_credits_only_another_party_member_never_the_caster() {
        let (caster, ally, stranger) = ([1; 16], [2; 16], [3; 16]);
        let members = [caster, ally];
        assert!(
            !heal_credits_party_member(&caster, &members, &caster),
            "a self-heal earns no shared-experience activity"
        );
        assert!(heal_credits_party_member(&caster, &members, &ally));
        assert!(!heal_credits_party_member(&caster, &members, &stranger));
    }
}
