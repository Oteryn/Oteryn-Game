//! Admission projection of BED-REGEN durable Soul/food and online residuals.
use super::last_position::AdmissionPosition;
use crate::foundation::WorldId;

pub(super) fn admitted_rest(decided: &AdmissionPosition, world: WorldId) -> Option<[u32; 4]> {
    match decided {
        AdmissionPosition::Stored(row) if row.record.world_id == world => row.record.rest,
        AdmissionPosition::Arrival(_, _, rest) => *rest,
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::super::actor_spell::{
        ChannelSpellStates,
        tests::{FACTS, runtime_with_player},
    };
    use oteryn_simulation_determinism::SemanticTimeMicros;

    #[test]
    fn bed_regen_corrupt_restore_cannot_leave_a_retry_admissible() {
        let (runtime, actor, session) = runtime_with_player(91);
        let mut states = ChannelSpellStates::default();
        let now = SemanticTimeMicros::from_micros(0);
        assert!(
            states
                .initialize_admitted(
                    &runtime,
                    actor,
                    session,
                    FACTS,
                    (0, 0),
                    now,
                    None,
                    Some([5, 1_200_001, 0, 0])
                )
                .is_none()
        );
        assert!(states.get(&runtime, actor, session).is_none());
        assert!(
            states
                .initialize_admitted(
                    &runtime,
                    actor,
                    session,
                    FACTS,
                    (0, 0),
                    now,
                    None,
                    Some([5, 30_000, 1234, 567])
                )
                .is_some()
        );
        assert!(
            states
                .initialize_admitted(
                    &runtime,
                    actor,
                    session,
                    FACTS,
                    (0, 0),
                    now,
                    None,
                    Some([100, 0, 0, 0])
                )
                .is_some()
        );
        assert_eq!(
            states
                .get(&runtime, actor, session)
                .map(|s| s.bed_rest_snapshot(0)),
            Some([5, 30_000, 1234, 567])
        );
    }
}
