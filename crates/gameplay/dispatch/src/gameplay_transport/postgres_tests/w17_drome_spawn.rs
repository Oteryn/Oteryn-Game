//! W17 exercises the preparing-to-fighting transition without native profiles.
use super::{TestResult, reaudit_arena_lifecycle_fixtures as arena};

#[test]
fn w17_drome_postgres_wave_refuses_missing_native_profiles() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| {
            arena::physical_context(0, true, false, arena::Physical::MissingProfiles)
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "W17 missing-profile dispatch scenario panicked")??;
    eprintln!(
        "W17_DROME POSTGRES_CASE_COMPLETED wave_refuses_missing_native_profiles duration={:?}",
        started.elapsed()
    );
    Ok(())
}

#[test]
fn w17_drome_draw_replays_and_uses_the_five_verified_outfits() -> TestResult {
    use oteryn_gameplay_entities::runtime_actor_carrier::tibiadrome_modifiers::{
        DromeCreatureRole as Role, drome_wave_opponent,
    };

    assert_eq!(
        Role::POOL.map(|role| (role.definition_key(), role.outfit_look_type())),
        [
            ("oteryn:creature.domestikion", 1426),
            ("oteryn:creature.hoodinion", 1424),
            ("oteryn:creature.mearidion", 1425),
            ("oteryn:creature.scissorion", 1423),
            ("oteryn:creature.murmillion", 1422),
        ]
    );
    let draw = |run, wave| {
        (0..17)
            .map(|slot| drome_wave_opponent(run, wave, slot))
            .collect::<Option<Vec<_>>>()
            .ok_or("each of the seventeen slots has a deterministic opponent")
    };
    let wave = draw([17; 16], 0)?;
    assert_eq!(wave, draw([17; 16], 0)?);
    assert_ne!(wave, draw([17; 16], 1)?);
    assert_ne!(wave, draw([18; 16], 0)?);
    let waves = (0..16)
        .map(|wave| draw([17; 16], wave))
        .collect::<Result<Vec<_>, _>>()?;
    for role in Role::POOL {
        assert!(waves.iter().any(|wave| wave.contains(&role)));
    }
    assert!(
        waves.iter().any(|wave| {
            let first_four = &wave[..4];
            first_four
                .iter()
                .enumerate()
                .any(|(slot, role)| first_four[..slot].contains(role))
        }),
        "even a solo wave can draw the same species twice"
    );
    Ok(())
}
