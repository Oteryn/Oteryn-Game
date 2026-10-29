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

    // Crystal 201189f: data/libs/systems/daily_reward.lua:1-6 unlocks HP/MP at
    // streaks 2/3, doubles them at 5/6, and unlocks soul at 7 (player.lua:789).
    // The production daily projection and resting consumer already exist on this base.
    #[test]
    fn daily_streak_projection_credits_resting_actor_pools_at_each_unlock() -> Result<(), String> {
        use super::super::resting_area::RestingAreaSessions;
        for (streak, expected) in [
            (1, (100, 10)),
            (2, (130, 10)),
            (3, (130, 50)),
            (4, (130, 50)),
            (5, (160, 50)),
            (6, (160, 90)),
            (7, (160, 90)),
        ] {
            let (runtime, actor, session) = runtime_with_player(92);
            let mut states = ChannelSpellStates::default();
            states
                .initialize_admitted(
                    &runtime,
                    actor,
                    session,
                    FACTS,
                    (0, 0),
                    SemanticTimeMicros::from_micros(0),
                    Some((100, 10)),
                    Some([5, 0, 0, 0]),
                )
                .ok_or("admitted actor")?;
            let projection = RestingAreaSessions::default();
            projection.install(session, actor, streak);
            let soul_before = states
                .get(&runtime, actor, session)
                .ok_or("actor")?
                .vitals()
                .soul;
            for _ in 0..120 {
                let (_, soul_due) = projection.cadence(session, actor, true, true, 2520, 120);
                states.apply_resting_regeneration(
                    &runtime,
                    actor,
                    session,
                    projection.streak(session, actor).ok_or("daily streak")?,
                    soul_due,
                );
            }
            let vitals = states
                .get(&runtime, actor, session)
                .ok_or("actor")?
                .vitals();
            assert_eq!((vitals.health, vitals.mana), expected, "streak {streak}");
            assert_eq!(vitals.soul, soul_before + u32::from(streak >= 7));
            // Both source stamina intervals: 3 minutes up to 2340, then 6 minutes.
            for (stamina, interval) in [(2340, 180), (2341, 360)] {
                projection.install(session, actor, streak);
                for _ in 1..interval {
                    assert!(
                        !projection
                            .cadence(session, actor, true, true, stamina, 120)
                            .0
                    );
                }
                assert_eq!(
                    projection
                        .cadence(session, actor, true, true, stamina, 120)
                        .0,
                    streak >= 4
                );
            }
        }
        Ok(())
    }

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
