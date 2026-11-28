//! Node control-socket qualification for the ordinary Forge population cycle.
use super::{ChannelRuntimeV1, DurabilityRoot, Mutex};

pub(super) const PREFIX: &[u8] = b"forge-population-draw ";

pub(crate) async fn draw(
    root: &DurabilityRoot,
    fixture_enabled: bool,
    runtime: Option<&Mutex<ChannelRuntimeV1>>,
    request: &[u8],
) -> String {
    if !enabled(fixture_enabled) {
        return "rejected reason=not_qualification".into();
    }
    #[cfg(feature = "qualification-spawn")]
    {
        match qualified_draw(root, runtime, request).await {
            Ok(census) => census.to_string(),
            Err(reason) => format!("rejected reason={reason}"),
        }
    }
    #[cfg(not(feature = "qualification-spawn"))]
    {
        let _ = (root, runtime, request);
        "rejected reason=not_qualification".into()
    }
}

fn enabled(fixture_enabled: bool) -> bool {
    cfg!(feature = "qualification-spawn") && fixture_enabled
}

#[cfg(feature = "qualification-spawn")]
async fn qualified_draw(
    root: &DurabilityRoot,
    runtime: Option<&Mutex<ChannelRuntimeV1>>,
    request: &[u8],
) -> Result<serde_json::Value, &'static str> {
    use crate::durability::{DurabilityError, fresh_admission::FreshAdmissionStore};
    use crate::foundation::{
        ExactActorRef, GameSessionId, GameSessionState, MovementLocalPosition,
    };
    use oteryn_gameplay_domain::domain::forge_creature::{
        FIENDISH_RESPAWN_MICROS, FIENDISH_SIMULTANEOUS_MAX, INFLUENCED_SIMULTANEOUS_MAX,
        influenced_available_slots,
    };
    use std::time::Duration;
    use tokio::time::{Instant, timeout_at};

    let fields = request.strip_prefix(PREFIX).ok_or("malformed_request")?;
    let (session, seed) = std::str::from_utf8(fields)
        .ok()
        .and_then(|line| line.strip_suffix('\n'))
        .and_then(|line| line.split_once(' '))
        .ok_or("malformed_request")?;
    let session = super::parse_operation_id(session.as_bytes())
        .and_then(|id| GameSessionId::decode(&id).ok())
        .ok_or("malformed_request")?;
    if seed.len() > 20 || !seed.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err("malformed_request");
    }
    let seed = seed.parse::<u64>().map_err(|_| "malformed_request")?;
    let runtime = runtime.ok_or("native_unavailable")?;
    // Reserve time for a normal 10 s population interval and leave the socket
    // watchdog time to write a refusal even if no owner cycle runs.
    let deadline = Instant::now() + Duration::from_secs(15);
    let census = timeout_at(deadline, async {
        let store = FreshAdmissionStore::from_root(root.clone());
        let current = crate::qualification_spell_character::retry_holder(
            Some(root),
            deadline,
            |error| matches!(error, DurabilityError::RootUnavailable),
            async || store.current_session(session).await,
        )
        .await
        .map_err(|error| {
            if Instant::now() >= deadline {
                "deadline"
            } else if matches!(error, DurabilityError::Unavailable) {
                "lease_required"
            } else {
                "lease_unavailable"
            }
        })?;
        if current.session_state() != GameSessionState::Active
            || current.current_transport().is_none()
        {
            return Err("lease_required");
        }
        let receive = runtime
            .lock()
            .await
            .arm_qualification_forge_draw(
                session,
                current.current_character_lease(),
                current.current_runtime_scope(),
                current.current_scope_generation(),
                seed,
                deadline,
            )
            .map_err(draw_refusal)?;
        receive
            .await
            .map_err(|_| "cycle_failed")?
            .map_err(draw_refusal)
    })
    .await
    .map_err(|_| "deadline")??;
    let id = |actor: ExactActorRef| super::uuid_text(&actor.placement_identity());
    let position =
        |MovementLocalPosition { x, y, floor }| serde_json::json!({"x": x, "y": y, "floor": floor});
    let selected: Vec<_> = census
        .selected
        .into_iter()
        .map(|selection| {
            serde_json::json!({
                "id": id(selection.actor),
                "generation": selection.actor.actor_local_generation(),
                "stacks": selection.kind.reward_stacks(),
                "creature_key": selection.creature_key,
                "position": position(selection.position),
            })
        })
        .collect();
    let retained: Vec<_> = census
        .retained
        .into_iter()
        .map(|(actor, kind)| serde_json::json!({
            "id": id(actor), "generation": actor.actor_local_generation(), "stacks": kind.reward_stacks()
        }))
        .collect();
    let eligible: Vec<_> = census
        .eligible
        .into_iter()
        .map(|(identity, actor)| serde_json::json!({
            "id": id(actor), "generation": actor.actor_local_generation(), "spawn_identity": identity
        }))
        .collect();
    let candidates: Vec<_> = census
        .candidates
        .into_iter()
        .map(|candidate| {
            serde_json::json!({
                "id": id(candidate.actor),
                "generation": candidate.actor.actor_local_generation(),
                "spawn_identity": candidate.spawn_identity,
                "stacks": candidate.kind.map(|kind| kind.reward_stacks()),
                "owned": candidate.owned,
                "forge_difficulty": candidate.forge_difficulty,
                "creature_key": candidate.creature_key,
                "position": position(candidate.position),
            })
        })
        .collect();
    let fiendish_slots: Vec<_> = census
        .fiendish_slots
        .into_iter()
        .map(|(actor, due)| serde_json::json!({
            "id": id(actor), "generation": actor.actor_local_generation(), "replacement_due_micros": due
        }))
        .collect();
    let excluded_owned_summons: Vec<_> = census
        .excluded_owned_summons
        .into_iter()
        .map(|actor| serde_json::json!({"id": id(actor), "reason": "owned_summon"}))
        .collect();
    Ok(serde_json::json!({
        "now_micros": census.now_micros,
        "due_micros": census.due_micros,
        "seed": census.seed,
        "artifact_digest": crate::node::operator_files::hex(&census.artifact_digest),
        "population_tick_micros": census.population_tick_micros,
        "fiendish_respawn_micros": FIENDISH_RESPAWN_MICROS,
        "fiendish_cap": FIENDISH_SIMULTANEOUS_MAX,
        "influenced_cap": INFLUENCED_SIMULTANEOUS_MAX,
        "influenced_per_tick_cap": influenced_available_slots(0),
        "influenced_count": census.influenced_count,
        "influenced_available_slots": census.influenced_available_slots,
        "fiendish_slots": fiendish_slots,
        "candidates": candidates,
        "selected": selected,
        "retained": retained,
        "fiendish_vacancy": census.fiendish_vacancy,
        "fiendish_available_slots": census.fiendish_available_slots,
        "eligible": eligible,
        "excluded_owned_summons": excluded_owned_summons,
    }))
}

#[cfg(feature = "qualification-spawn")]
fn draw_refusal(
    refusal: oteryn_gameplay_entities::carrier::ForgePopulationDrawRefusal,
) -> &'static str {
    use oteryn_gameplay_entities::carrier::ForgePopulationDrawRefusal::*;
    match refusal {
        LeaseRequired => "lease_required",
        Deadline => "deadline",
        Busy => "draw_busy",
    }
}

#[cfg(test)]
#[test]
fn forge_population_draw_requires_feature_and_fixture_gate() {
    assert!(!enabled(false));
    assert_eq!(enabled(true), cfg!(feature = "qualification-spawn"));
}
