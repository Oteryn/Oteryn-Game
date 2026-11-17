//! W19b: Crystal dryad_garden.lua:7-24 teleports before emitting watersplash.
use super::TestResult;
use super::fixture::debug;
use super::quest_world_callbacks::{local, scenario, wire};
use crate::foundation::ExactActorRef;
use crate::gameplay_transport::{
    ComposedFreshAdmission,
    connection::{AdmittedSession, FreshAdmissionAuthority},
};
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
use oteryn_protocol_oteryn::presentation::PresentationEventKind;
use oteryn_protocol_oteryn::world_object::{
    UseDisposition, WorldObjectTarget, decode_use_result, encode_use_intent,
};

#[test]
fn srvwld_dryad_garden_enter_teleports_and_publishes_destination_splash() -> TestResult {
    garden(
        "dryad-garden-enter",
        Position {
            x: 33264,
            y: 32012,
            floor: -7,
        },
    )
}

#[test]
fn srvwld_dryad_garden_leave_teleports_and_publishes_destination_splash() -> TestResult {
    garden(
        "dryad-garden-leave",
        Position {
            x: 33202,
            y: 32012,
            floor: -11,
        },
    )
}

fn garden(name: &'static str, trigger: Position) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run(name, move || scenario(name, trigger, false))?;
    eprintln!(
        "SRVWLD_DRYAD_POSTGRES_CASE_COMPLETED {name} elapsed_seconds={:.3}",
        started.elapsed().as_secs_f64()
    );
    Ok(())
}

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    actor: ExactActorRef,
    callback: &Callback,
    trigger: Position,
) -> TestResult {
    let key = oteryn_world_bundle::bundle::placement_key(trigger.floor, trigger.x, trigger.y, 1)
        .ok_or("key")?;
    let mut placement = owner
        .bundle_map()
        .ok_or("map")?
        .overlay
        .base()
        .digest()
        .to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let target = WorldObjectTarget {
        placement,
        expected_revision: 0,
    };
    let (cursor, _) = owner
        .observe_sounds(actor, admitted.game_session_id, u64::MAX)
        .await
        .ok_or("presentation cursor")?;
    let reply = wire(
        owner,
        admitted,
        1,
        2,
        encode_use_intent(&target).map_err(debug)?,
    )
    .await?;
    assert_eq!(
        decode_use_result(&reply).map_err(debug)?,
        UseDisposition::Committed
    );
    let destination = local(callback.teleport.ok_or("destination")?);
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?
            .position(),
        destination
    );
    let (_, events) = owner
        .observe_sounds(actor, admitted.game_session_id, cursor)
        .await
        .ok_or("presentation")?;
    let splashes: Vec<_> = events
        .iter()
        .filter_map(|event| match event.kind {
            PresentationEventKind::MagicEffect {
                effect_id: 54,
                position,
                ..
            } => Some(position),
            _ => None,
        })
        .collect();
    assert_eq!(
        splashes,
        vec![oteryn_protocol_oteryn::world_spatial::ActorPosition {
            x: destination.x,
            y: destination.y,
            floor: destination.floor,
        }]
    );
    Ok(())
}
