//! A creature push drives the mounted StepIn callback and credits both room players.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, Dependencies, TestResult};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::{
    world_callbacks::{Callback, Position},
    zamulosh,
};
use oteryn_protocol_oteryn::{
    creature_push::{CreaturePushIntent, encode_creature_push_intent},
    world_spatial::ActorPosition,
    world_spatial_entities::EntityRef,
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
    use super::quest_world_callbacks::{local, wire};
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
        let generation = runtime
            .visible_entities()
            .creatures
            .into_iter()
            .find(|row| row.actor == target)
            .ok_or("visible lovely creature")?
            .generation;
        (target, generation)
    };
    first.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 13, 82])
            .ok_or("creature push capability")?;
    let destination = local(PLATE);
    let payload = encode_creature_push_intent(&CreaturePushIntent {
        target: EntityRef {
            identity: target.0.placement_identity(),
            generation: target.1,
        },
        destination: ActorPosition {
            x: destination.x,
            y: destination.y,
            floor: destination.floor,
        },
    })
    .map_err(debug)?;
    let command = first.continuity.next_command_id;
    assert_eq!(wire(owner, first, command, 98, payload).await?, [8, 1]);
    assert!(!owner.runtime.lock().await.contains_live_creature(target.0));
    for player in [&*first, &second] {
        assert_eq!(
            super::questpc7_ferumbras::value(owner, player, zamulosh::TRACK).await?,
            1
        );
    }
    Ok(())
}
