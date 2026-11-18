//! Physical creature movement drives StepIn and credits both room players.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, Dependencies, TestResult};
use oteryn_gameplay_quests::quest::{
    world_callbacks::{Callback, Position},
    zamulosh,
};

pub(super) const NAME: &str = "bosscred-zamulosh-lovely-step";
pub(super) const PLATE: Position = Position {
    x: 33626,
    y: 32666,
    floor: -12,
};
pub(super) const START: Position = Position { x: 33624, ..PLATE };
const CREATURE_START: Position = Position { x: 33625, ..PLATE };
const WITNESS: Position = Position { x: 33625, y: 32667, ..PLATE };
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
    callback.selector.action = None;
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

pub(super) const HABITAT_NAME: &str = "bosscred-zamulosh-habitat-grass";
pub(super) const GRASS_LEVER: Position = Position { x: 33631, y: 32665, floor: -12 };
pub(super) const HABITAT_START: Position = Position { y: 32666, ..GRASS_LEVER };
const HABITAT_COUNT: &str = "oteryn:quest-progress/ferumbras-ascension/habitats";

#[test]
fn bosscred_zamulosh_habitat_use_grants_credit_and_delays_the_terrain_swap() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run(HABITAT_NAME, || {
        super::quest_world_callbacks::scenario(HABITAT_NAME, GRASS_LEVER, false)
    })
}

pub(super) fn habitat_world(
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    use super::l6_world_pickups::{NativeCrop, native_scene};
    native_scene(native, vec![NativeCrop {
        bytes: include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content/world/placements/region-z12-x131-y127.b3")),
        region: (12, 131, 127),
        bounds: (33611, 32658, 33648, 32673),
        start: (HABITAT_START.x, HABITAT_START.y),
        expected: Vec::new(),
    }], &[])
}

pub(super) async fn habitat(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    first: &mut AdmittedSession,
    deps: &Dependencies,
    harness: &Harness,
) -> TestResult {
    use super::quest_world_callbacks::local;
    use oteryn_protocol_oteryn::world_object::UseDisposition;
    let (_, mut second) = super::grizzly_boss::vacancy::second_player(
        owner, deps, harness, local(Position { x: 33630, ..HABITAT_START }),
    ).await?;
    let source = crate::content::LogicalCell { x: 33616, y: 32664, z: 12 };
    let destination = crate::content::LogicalCell { x: 33639, ..source };
    let map = owner.bundle_map().ok_or("habitat map")?;
    let expected = map.spell_tiles.lookup(source).map_err(debug)?.clone().into_owned();
    let original = map.spell_tiles.lookup(destination).map_err(debug)?.into_owned();
    assert_ne!(original.top_source_ids(), expected.top_source_ids());
    let use_lever = super::questpc7_ferumbras::use_at;
    assert_eq!(use_lever(owner, first, GRASS_LEVER, 9125).await?, UseDisposition::Committed);
    let map = owner.bundle_map().ok_or("habitat map")?;
    assert_eq!(map.spell_tiles.lookup(destination).map_err(debug)?.as_ref(), &original);
    assert_eq!(super::questpc7_ferumbras::value(owner, first, HABITAT_COUNT).await?, 1);
    assert_eq!(use_lever(owner, &mut second, GRASS_LEVER, 9126).await?, UseDisposition::Committed);
    assert_eq!(use_lever(owner, &mut second, GRASS_LEVER, 9125).await?, UseDisposition::Committed);
    assert_eq!(super::questpc7_ferumbras::value(owner, &second, HABITAT_COUNT).await?, 1);
    owner.clock_origin = owner.clock_origin.checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock origin")?;
    owner.drain_zamulosh_terrain().await;
    let map = owner.bundle_map().ok_or("habitat map")?;
    let transformed = map.spell_tiles.lookup(destination).map_err(debug)?;
    assert_eq!(transformed.ground_source_id(), expected.ground_source_id());
    assert_eq!(transformed.top_source_ids(), expected.top_source_ids());
    Ok(())
}
