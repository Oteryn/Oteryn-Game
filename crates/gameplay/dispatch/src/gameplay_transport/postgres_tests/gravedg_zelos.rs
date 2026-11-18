//! Native USE and death/clock dispatch for the Grave Danger ritual.
use super::{AdmittedSession, ComposedFreshAdmission, TestResult, fixture::debug};
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, EncounterPhase, Position};

const KNIGHTS: [&str; 4] = [
    "oteryn:creature.magnor_mournbringer",
    "oteryn:creature.nargol_the_impaler",
    "oteryn:creature.rewar_the_bloody",
    "oteryn:creature.the_red_knight",
];

#[test]
fn gravedg_zelos_lever_and_nargol_regeneration_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::engenc2_cults::phase_scenario(false, true)
}

pub(super) fn configure(callback: &mut Callback) -> TestResult {
    callback.key = "oteryn:quest-callback/grave-danger/zelos-ritual".into();
    callback.storage.clear();
    let encounter = callback.effects.encounter.as_mut().ok_or("encounter")?;
    encounter.phase = Some(EncounterPhase::GraveDangerZelos {
        king: Position { x: 115, y: 113, floor: -7 },
        exit: Position { x: 103, y: 100, floor: -7 },
    });
    encounter.minimum_level = 1;
    encounter.monsters.truncate(4);
    for (monster, name) in encounter.monsters.iter_mut().zip(KNIGHTS) {
        monster.creature = name.into();
    }
    Ok(())
}

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult {
    let find = |runtime: &crate::foundation::ChannelRuntimeV1, name: &str| {
        runtime
            .current_live_creature_census()
            .map(|actors| actors.into_iter().find(|actor| {
                runtime.matches_live_creature_identity(*actor, name.as_bytes())
            }))
            .map_err(debug)
    };
    let nargol = {
        let runtime = owner.runtime.lock().await;
        find(&runtime, KNIGHTS[1])?.ok_or("Nargol spawned by native USE")?
    };
    super::engenc2_cults::kill(owner, admitted, nargol, KNIGHTS[1], 101).await?;
    let mut runtime = owner.runtime.lock().await;
    let mut states = owner.states.spell_states.lock().await;
    let now = owner.owner_now().get();
    crate::gameplay_transport::encounter_phases::tick(&mut runtime, &mut states, now);
    let mass = "oteryn:creature.regenerating_mass";
    assert!(find(&runtime, mass)?.is_some(), "mass replaces Nargol");
    crate::gameplay_transport::encounter_phases::tick(&mut runtime, &mut states, now + 29_999_999);
    assert!(find(&runtime, mass)?.is_some(), "mass survives until the deadline");
    crate::gameplay_transport::encounter_phases::tick(&mut runtime, &mut states, now + 30_000_000);
    assert!(find(&runtime, mass)?.is_none(), "mass expires at 30 seconds");
    assert_ne!(
        find(&runtime, KNIGHTS[1])?.ok_or("regenerated Nargol")?,
        nargol
    );
    assert!(find(&runtime, "oteryn:creature.king_zelos")?.is_none());

    crate::gameplay_transport::encounter_phases::tick(
        &mut runtime, &mut states, now + 24 * 60 * 1_000_000,
    );
    for knight in KNIGHTS {
        assert!(find(&runtime, knight)?.is_none(), "timeout cleans up {knight}");
    }
    let player = admitted.runtime_actor.ok_or("admitted player")?;
    let exit = runtime.read_actor_position(player).map_err(debug)?.position();
    assert_eq!((exit.x, exit.y, exit.floor), (103, 100, 7));
    Ok(())
}
