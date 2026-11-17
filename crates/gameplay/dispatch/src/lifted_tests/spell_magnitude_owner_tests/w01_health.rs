use super::*;

#[test]
fn srx3_health_forgotten_actual_magnitude_doubles_healing_before_wheel_flat()
-> Result<(), Box<dyn std::error::Error>> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            let mut f =
                fixture_with_source_kind(0, true, true, Some("oteryn:creature.lady_tenebris"));
            f.state.conditions = Default::default();
            f.attributes.policy = MagnitudePolicy::BaselineTest;
            f.attributes.owner_revisions = Some((1, 1, Some(1)));
            f.attributes.wheel_flat_healing = Some(3);
            assert_eq!(
                f.runtime.native_health_event(f.bindings[1].actor)
                    .expect("current native policy")
                    .expect("actual attached source event")
                    .kind(),
                oteryn_gameplay_content::content::native_monster_callbacks::NativeHealthEventKind::HealthForgotten,
            );
            let mut native = plan();
            let NativeCombatPlan::Combat(combat) = &mut native else {
                panic!("actual Combat fixture")
            };
            combat.element = Element::Healing;
            combat.block_armor = false;
            combat.hits.truncate(1);
            combat.hits[0].magnitude = 100;
            combat.hits[0].bonus_percent = 0;
            combat.hits[0].side_percent = None;
            let hit = combat.hits[0].clone();
            let mut owner = qualify(&f, &native).expect("actual native magnitude owner");
            let current = f.runtime.observe_native_health_change(f.bindings[1].actor,&|_: crate::foundation::ExactActorRef, _: crate::foundation::MovementPositionSnapshot| None)
                .expect("actual source observation; real caster is on a different floor")
                .expect("actual attached HealthForgotten");
            owner.bind_native_health_observations(&f.runtime,&[current])
                .expect("same production target preprocessing");
            let magnitude = owner
                .finish(&native, &hit, &f.bindings[1], &mut |low, _| low)
                .expect("actual source ordered primary healing");
            assert_eq!(
                magnitude, 203,
                "HealthForgotten primary callback precedes Wheel flat healing"
            );
        })?
        .join()
        .map_err(|_| "native HealthForgotten magnitude worker panicked")?;
    Ok(())
}
