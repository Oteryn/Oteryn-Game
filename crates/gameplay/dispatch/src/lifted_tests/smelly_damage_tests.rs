#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use crate::foundation::owner_timer::{OwnerClock, SemanticTimeMicros, VirtualOwnerClock};
    use crate::foundation::{MovementLocalPosition, RuntimeScopeRefV1};
    use crate::gameplay_transport::actor_spell::{
        ChannelSpellStates,
        tests::{FACTS, runtime_with_player},
    };
    use crate::player_lethal::PlayerLethalVitals;
    use crate::smelly_cheese::{SmellyCastOccurrence, SmellyCheeseTimers, SmellyPulse};
    use crate::smelly_damage::*;

    #[test]
    fn actual_hp_nonlethal_a_b_retry_a_and_presentation_never_redamage() {
        let (mut runtime, player, session) = runtime_with_player(0x59);
        runtime
            .initialize_movement_test_position(
                player,
                MovementLocalPosition {
                    x: 101,
                    y: 100,
                    floor: 7,
                },
            )
            .expect("smelly_damage.rs:tests:103: qualified fixture operation must succeed");
        let caster = runtime
            .admit_monster_lab_creature(
                MovementLocalPosition {
                    x: 100,
                    y: 100,
                    floor: 7,
                },
                "oteryn:creature.smelly_cheese",
                5000,
            )
            .expect("smelly_damage.rs:tests:114: qualified fixture operation must succeed");
        let mut states = ChannelSpellStates::default();
        let facts = crate::spell::cast::CharacterCastFacts {
            max_health: 2000,
            ..FACTS
        };
        states
            .initialize(
                &runtime,
                player,
                session,
                facts,
                (0, 0),
                oteryn_simulation_determinism::SemanticTimeMicros::from_micros(0),
            )
            .expect("smelly_damage.rs:tests:129: qualified fixture operation must succeed");
        let scope = RuntimeScopeRefV1::channel(
            runtime.binding().world_id(),
            runtime.binding().channel_id(),
        );
        let generation = runtime.binding().scope_generation();
        let (fence, stamp) = crate::foundation::crystal_timer_fixture(scope, generation)
            .expect("smelly_damage.rs:tests:135: qualified fixture operation must succeed");
        let clock = VirtualOwnerClock::new(SemanticTimeMicros::from_micros(0));
        let mut timers = SmellyCheeseTimers::new(scope, generation)
            .expect("smelly_damage.rs:tests:137: qualified fixture operation must succeed");
        let a = SmellyCastOccurrence {
            caster,
            sequence: 1,
        };
        assert!(
            timers
                .schedule_once(&runtime, &fence, stamp, a, clock.now())
                .expect("smelly_damage.rs:tests:145: qualified fixture operation must succeed")
        );
        clock.advance(499_999);
        assert!(
            timers
                .drain(&clock, &fence, |a| runtime.contains_live_creature(a))
                .is_empty()
        );
        clock.advance(1);
        let mut due = timers.drain(&clock, &fence, |a| runtime.contains_live_creature(a));
        assert_eq!(due.len(), 1);
        let a_receipts = commit_smelly_unambiguous_damage(
            &mut runtime,
            &fence,
            stamp,
            &mut states,
            due.remove(0),
            || 400,
            |_, _| true,
            clock.now(),
        )
        .expect("smelly_damage.rs:tests:166: qualified fixture operation must succeed");
        assert_eq!(a_receipts.len(), 1);
        assert_eq!(a_receipts[0].health_after, 1600);
        let b = states
            .apply_attack_damage(
                &mut runtime,
                player,
                session,
                20,
                "other-attack:B",
                clock.now(),
            )
            .expect("smelly_damage.rs:tests:178: qualified fixture operation must succeed");
        assert_eq!(b.health_after, 1580);
        assert!(
            !timers
                .schedule_once(&runtime, &fence, stamp, a, clock.now())
                .expect("smelly_damage.rs:tests:183: qualified fixture operation must succeed")
        );
        assert!(
            timers
                .drain(&clock, &fence, |a| runtime.contains_live_creature(a))
                .is_empty()
        );
        clock.advance(199_999);
        assert!(
            timers
                .drain(&clock, &fence, |a| runtime.contains_live_creature(a))
                .is_empty()
        );
        clock.advance(1);
        let mut visual = timers.drain(&clock, &fence, |a| runtime.contains_live_creature(a));
        assert_eq!(visual.len(), 1);
        assert_eq!(visual[0].kind(), SmellyPulse::Presentation700);
        assert!(matches!(
            commit_smelly_unambiguous_damage(
                &mut runtime,
                &fence,
                stamp,
                &mut states,
                visual.remove(0),
                || 400,
                |_, _| true,
                clock.now()
            ),
            Err(SmellyDamageError::WrongPulse)
        ));
        let after = states
            .apply_attack_damage(
                &mut runtime,
                player,
                session,
                1,
                "read-through-native-hit:C",
                clock.now(),
            )
            .expect("smelly_damage.rs:tests:222: qualified fixture operation must succeed");
        assert_eq!(after.health_after, 1579);
    }
    #[test]
    fn retained_due_pulse_cannot_write_hp_after_independent_scope_grant_changes() {
        let (mut runtime, player, session) = runtime_with_player(0x5a);
        runtime
            .initialize_movement_test_position(
                player,
                MovementLocalPosition {
                    x: 101,
                    y: 100,
                    floor: 7,
                },
            )
            .expect("smelly_damage.rs:tests:237: qualified fixture operation must succeed");
        let caster = runtime
            .admit_monster_lab_creature(
                MovementLocalPosition {
                    x: 100,
                    y: 100,
                    floor: 7,
                },
                "oteryn:creature.smelly_cheese",
                5000,
            )
            .expect("smelly_damage.rs:tests:248: qualified fixture operation must succeed");
        let mut states = ChannelSpellStates::default();
        states
            .initialize(
                &runtime,
                player,
                session,
                FACTS,
                (0, 0),
                oteryn_simulation_determinism::SemanticTimeMicros::from_micros(0),
            )
            .expect("smelly_damage.rs:tests:259: qualified fixture operation must succeed");
        let binding = runtime.binding();
        let scope = RuntimeScopeRefV1::channel(binding.world_id(), binding.channel_id());
        let generation = binding.scope_generation();
        let (mut fence, stamp) = crate::foundation::crystal_timer_fixture(scope, generation)
            .expect("smelly_damage.rs:tests:264: qualified fixture operation must succeed");
        let clock = VirtualOwnerClock::new(SemanticTimeMicros::from_micros(0));
        let mut timers = SmellyCheeseTimers::new(scope, generation)
            .expect("smelly_damage.rs:tests:266: qualified fixture operation must succeed");
        timers
            .schedule_once(
                &runtime,
                &fence,
                stamp,
                SmellyCastOccurrence {
                    caster,
                    sequence: 1,
                },
                clock.now(),
            )
            .expect("smelly_damage.rs:tests:278: qualified fixture operation must succeed");
        clock.advance(500_000);
        let mut due = timers.drain(&clock, &fence, |a| runtime.contains_live_creature(a));
        assert_eq!(due.len(), 1);
        let before = crate::gameplay_transport::actor_spell::observe_vitals(
            &runtime, &states, player, session,
        )
        .expect("smelly_damage.rs:tests:285: qualified fixture operation must succeed");
        // Change only independent current authority after preparing the sealed pulse.
        fence
            .apply_external_grant(
                crate::foundation::ScopeOwnershipGeneration::new(2)
                    .expect("smelly_damage.rs:tests:288: qualified fixture operation must succeed"),
            )
            .expect("smelly_damage.rs:tests:289: qualified fixture operation must succeed");
        assert!(matches!(
            commit_smelly_unambiguous_damage(
                &mut runtime,
                &fence,
                stamp,
                &mut states,
                due.remove(0),
                || 400,
                |_, _| true,
                clock.now()
            ),
            Err(SmellyDamageError::StaleOwner)
        ));
        assert_eq!(
            crate::gameplay_transport::actor_spell::observe_vitals(
                &runtime, &states, player, session
            ),
            Some(before)
        );
    }
}
