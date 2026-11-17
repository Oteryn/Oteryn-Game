//! M49 drives the real native actor registry with the existing checked rat fixture.
use super::*;

fn key(n: u8) -> String {
    format!("qn1.elemental:{n:032x}")
}

#[test]
fn qm49_elemental_boss_projection_retains_claim_at_native_actor_capacity() -> TestResult {
    let mut runtime = runtime(1)?;
    let pin = runtime.content_pin().clone();
    let generation = runtime.binding().scope_generation();
    let mut blockers = Vec::new();
    for x in 130..138 {
        blockers.push(checked(runtime.admit_encounter_actor(
            "creature:rat",
            20,
            position(x),
        ))?);
    }
    assert_eq!(
        checked(runtime.install_elemental_boss_spawn(
            &pin,
            generation,
            &key(1),
            "creature:rat",
            position(111),
            0,
        ))?,
        None
    );
    assert!(runtime.boss_spawns()[0].pending_claim);
    checked(runtime.remove_encounter_actor(blockers[0]))?;
    assert!(
        checked(runtime.install_elemental_boss_spawn(
            &pin,
            generation,
            &key(1),
            "creature:rat",
            position(111),
            0,
        ))?
        .is_some()
    );
    assert_eq!(checked(runtime.positioned_actor_census())?.len(), 8);
    assert!(
        blockers[1..]
            .iter()
            .all(|actor| runtime.contains_live_creature(*actor))
    );
    Ok(())
}

#[test]
fn qm49_elemental_boss_projection_deduplicates_queues_and_never_resurrects() -> TestResult {
    let mut runtime = runtime(1)?;
    let pin = runtime.content_pin().clone();
    let generation = runtime.binding().scope_generation();
    let first = checked(runtime.install_elemental_boss_spawn(
        &pin,
        generation,
        &key(1),
        "creature:rat",
        position(111),
        0,
    ))?
    .ok_or("first elemental actor")?;
    let step = Step {
        actor: first,
        expected: checked(runtime.read_actor_position(first))?,
        destination: position(112),
    };
    checked(runtime.commit_source_step(step))?;
    assert_eq!(
        checked(runtime.install_elemental_boss_spawn(
            &pin,
            generation,
            &key(1),
            "creature:rat",
            position(111),
            0,
        ))?,
        Some(first)
    );
    assert_eq!(
        checked(runtime.read_actor_position(first))?.position(),
        position(112)
    );
    assert_eq!(checked(runtime.positioned_actor_census())?.len(), 1);
    assert_eq!(
        checked(runtime.install_elemental_boss_spawn(
            &pin,
            generation,
            &key(2),
            "creature:rat",
            position(112),
            0,
        ))?,
        None,
        "occupied cells retain their already committed clock claim"
    );
    assert!(
        runtime
            .boss_spawns()
            .iter()
            .any(|spawn| spawn.key == key(2) && spawn.pending_claim)
    );
    checked(runtime.remove_encounter_actor(first))?;
    assert_eq!(
        checked(runtime.install_elemental_boss_spawn(
            &pin,
            generation,
            &key(1),
            "creature:rat",
            position(111),
            0,
        ))?,
        None,
        "a dead realization cannot respawn from the same clock cycle"
    );
    let second = checked(runtime.install_elemental_boss_spawn(
        &pin,
        generation,
        &key(2),
        "creature:rat",
        position(112),
        0,
    ))?
    .ok_or("unblocked elemental actor")?;
    let second_key = key(2);
    checked(runtime.retain_elemental_boss_spawns(&pin, generation, &[&second_key]))?;
    assert_eq!(runtime.boss_spawns().len(), 1);
    assert!(runtime.contains_live_creature(second));
    assert!(
        runtime
            .retain_elemental_boss_spawns(&pin, generation, &[])
            .is_err()
    );
    assert!(
        runtime.contains_live_creature(second),
        "absence never silently removes a live actor"
    );
    Ok(())
}

#[test]
fn qm49_elemental_boss_projection_fences_binding_and_caps_four_altars() -> TestResult {
    let mut runtime = runtime(1)?;
    let pin = runtime.content_pin().clone();
    let generation = runtime.binding().scope_generation();
    let stale = checked(ScopeOwnershipGeneration::new(2))?;
    let foreign = ChannelContentPin::test(checked(WorldId::decode(&id(9)))?);
    assert!(
        runtime
            .install_elemental_boss_spawn(&pin, stale, &key(1), "creature:rat", position(111), 0)
            .is_err()
    );
    assert!(
        runtime
            .install_elemental_boss_spawn(
                &foreign,
                generation,
                &key(1),
                "creature:rat",
                position(111),
                0
            )
            .is_err()
    );
    assert!(
        runtime
            .install_elemental_boss_spawn(
                &pin,
                generation,
                "qn1.elemental:bad",
                "creature:rat",
                position(111),
                0
            )
            .is_err()
    );
    assert!(runtime.boss_spawns().is_empty());
    for n in 1..=4 {
        assert!(
            checked(runtime.install_elemental_boss_spawn(
                &pin,
                generation,
                &key(n),
                "creature:rat",
                position(110 + i32::from(n)),
                0,
            ))?
            .is_some()
        );
    }
    assert!(
        runtime
            .install_elemental_boss_spawn(
                &pin,
                generation,
                &key(5),
                "creature:rat",
                position(115),
                0
            )
            .is_err()
    );
    assert!(
        runtime
            .install_elemental_boss_spawn(
                &pin,
                generation,
                &key(1),
                "creature:rat",
                position(119),
                0
            )
            .is_err()
    );
    assert!(
        runtime
            .install_elemental_boss_spawn(
                &pin,
                generation,
                &key(1),
                "creature:rat",
                position(111),
                1
            )
            .is_err()
    );
    assert_eq!(runtime.boss_spawns().len(), 4);
    assert_eq!(checked(runtime.positioned_actor_census())?.len(), 4);
    Ok(())
}
