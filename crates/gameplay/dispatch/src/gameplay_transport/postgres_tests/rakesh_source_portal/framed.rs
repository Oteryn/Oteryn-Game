//! Actual existing framed STEP and delivered runtime-switch/map data for native play.
use super::*;
use crate::gameplay_transport::{ComposedFreshAdmission, connection::AdmittedSession};
pub(super) async fn step(
    source: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult<Vec<serde_json::Value>> {
    let _ = super::super::pumpkin_skinning_use::take_claim_frames();
    let payload = oteryn_protocol_oteryn::world_spatial::encode_step_intent(
        oteryn_protocol_oteryn::world_spatial::StepDirection::East,
    );
    let (status, step_result) = super::super::pumpkin_skinning_use::exchange(
        source,
        admitted,
        oteryn_protocol_oteryn::world_spatial::COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        &payload,
        None,
    )
    .await?;
    assert_eq!(
        status,
        oteryn_protocol_oteryn::CommandStatus::Accepted,
        "actual framed source STEP"
    );
    assert_eq!(
        oteryn_protocol_oteryn::world_spatial::decode_step_result(&step_result).map_err(debug)?,
        oteryn_protocol_oteryn::world_spatial::StepDisposition::Moved,
        "actual framed command must move onto the captured source portal"
    );
    Ok(super::super::pumpkin_skinning_use::take_claim_frames())
}
pub(super) async fn export(
    source: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    frames: Vec<serde_json::Value>,
) -> TestResult {
    let owner = source
        .resolve_runtime_view(admitted)
        .await
        .map_err(|_| "resolve framed Source successor")?
        .ok_or("framed Source STEP must actually switch to the Instance runtime")?;
    let runtime = owner.runtime.lock().await;
    let at = runtime
        .read_actor_position(admitted.runtime_actor.ok_or("actual framed scoped actor")?)
        .map_err(debug)?
        .position();
    assert_eq!((at.x, at.y, at.floor), (33045, 32988, -8));
    let boss = runtime
        .positioned_actor_census()
        .map_err(debug)?
        .into_iter()
        .find_map(|(actor, _, session)| {
            session
                .is_none()
                .then(|| runtime.companion_snapshot(actor).ok())
                .flatten()
        })
        .ok_or("actual Source boss visible in destination")?;
    assert_eq!((boss.health, boss.maximum_health), (25500, 25500));
    assert_eq!(
        boss.state.policy.definition_key,
        "oteryn:creature.rakesh_moonfang"
    );
    assert!(
        frames
            .iter()
            .any(|frame| frame["kind"] == "snapshot" && frame["domain"] == 17),
        "the actual connection must receive map bootstrap data"
    );
    let barrier_index = frames
        .iter()
        .position(|frame| {
            frame["kind"] == "snapshot"
                && frame["domain"]
                    == oteryn_protocol_oteryn::activity_scope::STATE_DOMAIN_ACTIVITY_SCOPE
        })
        .ok_or("actual delivered activity scope barrier")?;
    let payload = frames[barrier_index]["payload_hex"]
        .as_str()
        .ok_or("actual scope bytes")?;
    let bytes = payload
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| {
            let text = std::str::from_utf8(pair)?;
            Ok::<u8, Box<dyn std::error::Error>>(u8::from_str_radix(text, 16)?)
        })
        .collect::<Result<Vec<_>, _>>()?;
    assert_eq!(payload.len() % 2, 0);
    let barrier =
        oteryn_protocol_oteryn::activity_scope::decode_activity_scope(&bytes).map_err(debug)?;
    assert_eq!(barrier.session, *admitted.game_session_id.as_bytes());
    assert!(matches!(
        barrier.scope,
        oteryn_protocol_oteryn::activity_scope::ActivityScope::Instance { .. }
    ));
    assert!(
        frames
            .iter()
            .skip(barrier_index + 1)
            .any(|frame| frame["kind"] == "snapshot" && frame["domain"] == 17),
        "destination map bootstrap must follow the actual scope barrier"
    );
    let capture = serde_json::json!({
        "schema":"srv7-rakesh-framed-source-entry-v1",
        "producer_test":"srv7_rakesh_real_framed_source_step_switches_instance",
        "selected_capabilities":admitted.continuity.selected_capabilities.as_slice(),
        "runtime_scope":"Instance",
        "observer_runtime":{"x":at.x,"y":at.y,"floor":at.floor},
        "source_portal":{"x":33039,"y":32964,"floor":8},
        "source_creature":{"key":boss.state.policy.definition_key,"health":boss.health,"maximum_health":boss.maximum_health},
        "wire_frames":frames,
    });
    drop(runtime);
    if let Some(path) = std::env::var_os("OTERYN_SRV7_RAKESH_CAPTURE") {
        std::fs::write(path, serde_json::to_vec_pretty(&capture)?)?;
    }
    eprintln!(
        "SRV7_RAKESH_FRAMED_SOURCE_EXECUTED actual_source_step=true actual_instance_switch=true source_health=25500 native_map_frames=true"
    );
    Ok(())
}
