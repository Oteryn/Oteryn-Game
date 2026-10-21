#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use crate::foundation::ChannelRuntimeV1;
    use crate::foundation::ExactActorRef;
    use crate::foundation::RuntimeScopeRefV1;
    use crate::foundation::RuntimeWorkStamp;
    use crate::foundation::ScopeOwnershipGeneration;
    use crate::foundation::ScopeRuntimeFence;
    use crate::foundation::owner_timer::OwnerTimerError;
    use crate::foundation::owner_timer::SemanticTimeMicros;
    use crate::foundation::{crystal_timer_fixture, owner_timer::VirtualOwnerClock};
    use crate::gameplay_transport::actor_spell::ChannelSpellStates;
    use crate::monster_owner_cycle::*;
    fn owner() -> (
        ChannelRuntimeV1,
        ChannelSpellStates,
        ExactActorRef,
        ScopeRuntimeFence,
        RuntimeWorkStamp,
    ) {
        let (mut runtime, _, _) =
            crate::gameplay_transport::actor_spell::tests::runtime_with_player(0x51);
        let creature = runtime
            .admit_test_creature(crate::foundation::MovementLocalPosition {
                x: 10,
                y: 10,
                floor: 7,
            })
            .expect("creature");
        let b = runtime.binding();
        let (fence, stamp) = crystal_timer_fixture(
            RuntimeScopeRefV1::channel(b.world_id(), b.channel_id()),
            b.scope_generation(),
        )
        .expect("test-only Foundation grant");
        (
            runtime,
            ChannelSpellStates::default(),
            creature,
            fence,
            stamp,
        )
    }
    #[test]
    fn cycle_due_is_once_typed_and_rearms_without_overdue_catchup() {
        let (mut runtime, mut states, creature, mut fence, _stamp) = owner();
        let mut lane = MonsterThinkLane::new(&runtime, &fence).expect("lane");
        let clock = VirtualOwnerClock::new(SemanticTimeMicros::from_micros(0));
        let first = lane
            .schedule(
                &runtime,
                &mut fence,
                creature,
                SemanticTimeMicros::from_micros(10),
            )
            .expect("schedule");
        assert_eq!(first.sequence, 0);
        clock.advance(5_000_000);
        let got = lane
            .run_due(
                &mut runtime,
                &mut states,
                &mut fence,
                &clock,
                |r, _, f, s, o, d| {
                    assert!(r.contains_live_creature(o.actor));
                    assert!(f.accepts_stamp(s));
                    (o.sequence, d.get())
                },
            )
            .expect("dispatch");
        assert_eq!(got, vec![(0, 5_000_000)]);
        assert!(
            lane.run_due(
                &mut runtime,
                &mut states,
                &mut fence,
                &clock,
                |_, _, _, _, _, _| ()
            )
            .expect("retry")
            .is_empty()
        );
        clock.advance(1_000_000);
        assert_eq!(
            lane.run_due(
                &mut runtime,
                &mut states,
                &mut fence,
                &clock,
                |_, _, _, _, o, _| o.sequence
            )
            .expect("next"),
            vec![1]
        );
    }
    #[test]
    fn cycle_refuses_current_fence_of_another_scope_and_changed_generation() {
        let (mut runtime, mut states, creature, mut fence, _stamp) = owner();
        let mut lane = MonsterThinkLane::new(&runtime, &fence).expect("lane");
        lane.schedule(
            &runtime,
            &mut fence,
            creature,
            SemanticTimeMicros::from_micros(0),
        )
        .expect("schedule");
        let b = runtime.binding();
        let other_channel = crate::foundation::ChannelId::decode(&[
            1, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, 90,
        ])
        .expect("channel");
        let (mut foreign, _) = crystal_timer_fixture(
            RuntimeScopeRefV1::channel(b.world_id(), other_channel),
            b.scope_generation(),
        )
        .expect("other scope grant");
        let clock = VirtualOwnerClock::new(SemanticTimeMicros::from_micros(0));
        assert_eq!(
            lane.run_due(
                &mut runtime,
                &mut states,
                &mut foreign,
                &clock,
                |_, _, _, _, _, _| ()
            ),
            Err(CycleError::StaleOwner)
        );
        fence
            .apply_external_grant(ScopeOwnershipGeneration::new(2).expect("new generation"))
            .expect("advance real test fence");
        assert_eq!(
            lane.run_due(
                &mut runtime,
                &mut states,
                &mut fence,
                &clock,
                |_, _, _, _, _, _| ()
            ),
            Err(CycleError::StaleOwner)
        );
        assert!(runtime.contains_live_creature(creature));
    }
    #[test]
    fn cycle_dead_target_never_reaches_consumer_and_tracker_is_pruned() {
        let (mut runtime, mut states, creature, mut fence, _stamp) = owner();
        let mut lane = MonsterThinkLane::new(&runtime, &fence).expect("lane");
        lane.schedule(
            &runtime,
            &mut fence,
            creature,
            SemanticTimeMicros::from_micros(0),
        )
        .expect("schedule");
        runtime.remove_test_actor(creature).expect("removed target");
        let clock = VirtualOwnerClock::new(SemanticTimeMicros::from_micros(0));
        let mut stale_consumer_called = false;
        assert!(
            lane.run_due(
                &mut runtime,
                &mut states,
                &mut fence,
                &clock,
                |_, _, _, _, _, _| {
                    stale_consumer_called = true;
                }
            )
            .expect("drain")
            .is_empty()
        );
        assert!(
            !stale_consumer_called,
            "stale actor must never reach callback"
        );
        assert!(lane.actors.is_empty());
        assert!(lane.tracker.is_empty());
    }

    #[test]
    fn cycle_duplicate_and_stale_target_refusals_mint_no_work_ordinal() {
        let (mut runtime, _, creature, mut fence, _stamp) = owner();
        let mut lane = MonsterThinkLane::new(&runtime, &fence).expect("lane");
        lane.schedule(
            &runtime,
            &mut fence,
            creature,
            SemanticTimeMicros::from_micros(0),
        )
        .expect("schedule");
        assert_eq!(
            lane.schedule(
                &runtime,
                &mut fence,
                creature,
                SemanticTimeMicros::from_micros(0)
            ),
            Err(CycleError::Timer(OwnerTimerError::PendingLimitReached))
        );
        let generation = runtime.binding().scope_generation();
        assert_eq!(
            fence
                .accept_input(generation)
                .expect("probe after refusal")
                .get(),
            3
        );
        runtime.remove_test_actor(creature).expect("stale target");
        assert_eq!(
            lane.schedule(
                &runtime,
                &mut fence,
                creature,
                SemanticTimeMicros::from_micros(0)
            ),
            Err(CycleError::StaleActor)
        );
        assert_eq!(
            fence
                .accept_input(generation)
                .expect("probe after stale target")
                .get(),
            4
        );
    }
}
