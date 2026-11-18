//! Physical creature movement drives StepIn and credits both room players.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, Dependencies, TestResult};
use oteryn_gameplay_quests::quest::{
    world_callbacks::{Callback, Position},
    zamulosh,
};

pub(super) const NAME: &str = "bosscred-zamulosh-lovely-step";
pub(super) const PLATE: Position = Position {
    x: 33618,
    y: 32666,
    floor: -12,
};
pub(super) const START: Position = Position { x: 33616, ..PLATE };
const CREATURE_START: Position = Position { x: 33617, ..PLATE };
const WITNESS: Position = Position { y: 32667, ..PLATE };
const CREATURE: &str = "oteryn:creature.lovely_frazzlemaw";

#[test]
fn bosscred_zamulosh_matching_creature_step_credits_two_spectators() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run(NAME, || {
        super::quest_world_callbacks::scenario(NAME, PLATE, false)
    })
}

pub(super) fn fixture_callback(mut callback: Callback) -> Callback {
    callback.selector.position = Some(PLATE);
    callback.selector.unique = None;
    callback.selector.action = Some(34315);
    callback.selector.appearance = Some(417);
    callback.teleport = None;
    callback
}

pub(super) fn world(callback: &Callback) -> TestResult<crate::map::boot::BundleWorld> {
    super::quest_world_callbacks::world_with_cells(
        callback,
        PLATE,
        &[START, CREATURE_START, WITNESS],
    )
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    first: &mut AdmittedSession,
    deps: &Dependencies,
    harness: &Harness,
) -> TestResult {
    use super::quest_world_callbacks::local;
    super::questpc7_native_deaths::prepare_native_hunter(owner, first).await?;
    let (_, second) =
        super::grizzly_boss::vacancy::second_player(owner, deps, harness, local(WITNESS)).await?;
    let target = {
        let mut runtime = owner.runtime.lock().await;
        let hp = runtime
            .companion_policy(CREATURE)
            .map_err(debug)?
            .maximum_health;
        let target = runtime
            .admit_encounter_actor(CREATURE, hp, local(CREATURE_START))
            .map_err(debug)?;
        runtime
            .install_creature_policy(target, CREATURE)
            .map_err(debug)?;
        let map = owner.bundle_map().ok_or("habitat map")?;
        let blockers = crate::map::floor::BundleDoors(&map.facts);
        let moved = crate::movement::relocate::relocate_creature(
            &mut runtime,
            target,
            owner.movement_cells(),
            &blockers,
            local(PLATE),
        )
        .map_err(debug)?;
        assert_eq!(moved.position(), local(PLATE));
        assert!(
            map.facts
                .record_monster_step(crate::map::facts::MonsterStep {
                    actor: target,
                    from: local(CREATURE_START),
                    to: moved.position(),
                })
        );
        let states = owner.states.spell_states.lock().await;
        crate::gameplay_transport::paced_movement::monster_trap::apply_monster_trap_steps(
            &mut runtime,
            &states,
            map,
            owner.owner_now().get(),
        );
        target
    };
    owner.settle_zamulosh_steps().await;
    assert!(!owner.runtime.lock().await.contains_live_creature(target));
    for player in [&*first, &second] {
        assert_eq!(
            super::questpc7_ferumbras::value(owner, player, zamulosh::TRACK).await?,
            1
        );
    }
    Ok(())
}
