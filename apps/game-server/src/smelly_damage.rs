//! Narrow actual HP consumer for source range on unambiguous native top-player tiles.
//! Not a whole Encounter or elemental/presentation owner. Caller qualifies Earth mitigation
//! and wall legality through their actual native owners before enabling this partial route.
use crate::foundation::{
    CarrierError, ChannelRuntimeV1, ExactActorRef, RuntimeScopeRefV1, RuntimeWorkStamp,
    ScopeRuntimeFence,
};
use crate::player_lethal::{PlayerDamageReceipt, PlayerLethalVitals};
use crate::smelly_cheese::{SmellyDuePulse, SmellyPulse};
#[derive(Debug)]
pub(crate) enum SmellyDamageError {
    Owner(CarrierError),
    InvalidRangeDraw,
    WrongPulse,
    StaleOwner,
}
/// Called once for a drained500ms pulse under the same Channel-owner work item.
/// `draw` is the actual server RNG; it draws independently for every qualifying player.
/// `native_legal` must be the existing map/target owner (not authoring metadata).
// Keep commit_smelly_unambiguous_damage ABI explicit: current runtime owner, independent fence/stamp, exact actor/session/source and occurrence/policy facts are separate admission inputs.
#[allow(clippy::too_many_arguments)]
pub(crate) fn commit_smelly_unambiguous_damage(
    runtime: &mut ChannelRuntimeV1,
    fence: &ScopeRuntimeFence,
    stamp: RuntimeWorkStamp,
    vitals: &mut impl PlayerLethalVitals,
    pulse: SmellyDuePulse,
    mut draw: impl FnMut() -> u32,
    mut native_legal: impl FnMut(ExactActorRef, ExactActorRef) -> bool,
    now: crate::foundation::owner_timer::SemanticTimeMicros,
) -> Result<Vec<PlayerDamageReceipt>, SmellyDamageError> {
    let binding = runtime.binding();
    if !fence.is_current_for_scope(
        RuntimeScopeRefV1::channel(binding.world_id(), binding.channel_id()),
        binding.scope_generation(),
    ) || !fence.accepts_stamp(stamp)
    {
        return Err(SmellyDamageError::StaleOwner);
    }
    if pulse.kind() != SmellyPulse::Damage500 {
        return Err(SmellyDamageError::WrongPulse);
    }
    let caster = pulse.caster();
    let cast_sequence = pulse.sequence();
    let targets = runtime
        .smelly_unambiguous_targets(caster)
        .map_err(SmellyDamageError::Owner)?;
    // Freeze all independent draws/legality before any HP mutation. An invalid draw applies
    // no partial cast. Actual health replay is owned by PlayerLethalVitals, not this helper.
    let mut accepted = Vec::new();
    for (target, session) in targets {
        if !native_legal(caster, target) {
            continue;
        }
        let magnitude = draw();
        if !(400..=800).contains(&magnitude) {
            return Err(SmellyDamageError::InvalidRangeDraw);
        }
        accepted.push((target, session, magnitude));
    }
    let mut receipts = Vec::new();
    for (target, session, magnitude) in accepted {
        let occurrence = format!(
            "smelly:{}:{}:{}",
            hex(caster.placement_identity()),
            cast_sequence,
            hex(target.placement_identity())
        );
        if let Some(receipt) =
            vitals.apply_attack_damage(runtime, target, session, magnitude, &occurrence, now)
        {
            receipts.push(receipt);
        }
    }
    Ok(receipts)
}
fn hex(bytes: [u8; 16]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::foundation::owner_timer::{OwnerClock, SemanticTimeMicros, VirtualOwnerClock};
    use crate::foundation::{MovementLocalPosition, RuntimeScopeRefV1};
    use crate::gameplay_transport::actor_spell::{
        ChannelSpellStates,
        tests::{FACTS, runtime_with_player},
    };
    use crate::smelly_cheese::{SmellyCastOccurrence, SmellyCheeseTimers, SmellyPulse};
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
