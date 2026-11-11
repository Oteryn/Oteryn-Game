//! W16 player scenario, derived from the matrix row and its Summer population owner.
//!
//! 1. Log in a provisioned node-boot character and issue an ordinary STEP; require an accepted
//!    movement disposition and the corresponding spatial delta before testing remote content.
//! 2. Issue native TARGET_WALK commands, in turn, for Mayor Pocaro and both Wayland Smythers
//!    placements from the compiled production bundle.  Require each accepted reply and every
//!    spatial delta until the character is within speech range of the authored placement.
//! 3. At each reached Summer NPC, send NPC_TALK `hi`; require `Ok` plus a conversation delta
//!    (Mayor/Wayland identity text).  Send `danger`, `name`, and `bye` to Wayland and require the
//!    catalogue replies; this is the positive NPC reachability path named by the owner test.
//! 4. Send TARGET_WALK to an invalid floor as the refusal path; require a rejected reply and no
//!    position delta.  This proves an invalid destination is not mistaken for map reachability.
//! 5. Log out, obtain a fresh admission, and require a new game session at the last reached cell;
//!    NPC dialogue is static population, so no inventory, quest, or economy mutation is expected.
//! 6. Only after the real STEP/TARGET_WALK attempts, report the first observed refusal, missing
//!    delta, or bounded timeout.  The row cannot pass unless all authored Summer NPC destinations
//!    and their replies were reached through player movement; the owner's boss profiles have no
//!    authored player-reachable spawn/encounter command in the compiled world and therefore do
//!    not get source-only or synthetic PASS evidence here.

use super::*;
use super::{
    row_connect_world as connect_world,
    row_finish as row_logout,
};
use oteryn_dev_client::{DevClientSession, SessionEvent};

const NPCS: [&str; 2] = ["oteryn:npc.mayor_pocaro", "oteryn:npc.wayland_smythers"];

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = row_complete(journey(clients)).await;
    let (status, detail) = match result {
        Ok(Ok(())) => (
            "PASS",
            "all-summer-map-and-dialogue-steps-validated".to_owned(),
        ),
        Ok(Err(error)) => ("FAIL", evidence_token(&error.to_string())),
        Err(_) => (
            "FAIL",
            "step=summer-route-observed=deadline-timeout".to_owned(),
        ),
    };
    evidence(&format!(
        "SEAM_EVIDENCE e2e_all scenario=W16.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    row_complete(async {
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(180);
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let mut client = connect_world(clients, 0xb6).await?;
    let mut row_needs_logout_0 = true;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
        // The first mechanic command is deliberately local and must produce the same real delta as
        // every other player walk.  It prevents a missing remote route from becoming a probe-only row.
        let before = client.world_spatial().actor_position;
        let step = client.step_retrying(StepDirection::East).await?;
        if step.disposition != StepDisposition::Moved || step.world_spatial_delta.is_none() {
            return Err(format!("step=initial-step observed={step:?}").into());
        }
        if client.world_spatial().actor_position == before {
            return Err("step=initial-step observed=accepted-without-position-delta".into());
        }

        let mut reached = 0usize;
        for key in NPCS {
            let npc = metadata
                .npcs
                .npcs
                .iter()
                .find(|npc| npc.key == key)
                .ok_or_else(|| format!("step=compiled-placement observed=missing-{key}"))?;
            for placement in &npc.placements {
                client.take_events();
                let target = ActorPosition {
                    x: i32::from(placement.x),
                    y: i32::from(placement.y),
                    floor: -i16::from(placement.floor),
                };
                let reply = client.session_mut().target_walk(target).await?;
                if reply.status != CommandStatus::Accepted {
                    return Err(format!(
                        "step=target-walk-{key} observed=status-{:?}-payload-{:?}",
                        reply.status, reply.payload
                    )
                    .into());
                }
                wait_for_destination(&mut client, target, key).await?;
                reached += 1;
                talk(&mut client, key, target).await?;
            }
        }
        if reached != 3 {
            return Err(
                format!("step=summer-placements observed=reached-{reached}-expected-3").into(),
            );
        }

        let saved = client.world_spatial().actor_position;
        client.take_events();
        let refused = client
            .session_mut()
            .target_walk(ActorPosition { floor: 16, ..saved })
            .await?;
        if refused.status != CommandStatus::Rejected {
            return Err(format!("step=invalid-floor-refusal observed={refused:?}").into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        if client.world_spatial().actor_position != saved {
            return Err("step=invalid-floor-refusal observed=position-changed".into());
        }
        if client.row_logout().await?.result
            != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
        {
            return Err("step=logout observed=refused".into());
        }
        row_needs_logout_0 = false;
        if row_needs_logout_0 {
            tokio::time::timeout(Duration::from_secs(10), row_logout(&mut client)).await??;
            row_needs_logout_0 = false;
        }
        let mut fresh = connect_world(clients, 0xb7).await?;
        let row_needs_logout_1 = true;
        let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
            if fresh.world_spatial().actor_position != saved {
                return Err(format!(
                    "step=fresh-admission observed={:?}-expected={saved:?}",
                    fresh.world_spatial().actor_position
                )
                .into());
            }
            Ok(())
        })
        .await
        .unwrap_or_else(|error| Err(error.into()));
        if row_needs_logout_1 {
            tokio::time::timeout(Duration::from_secs(10), row_logout(&mut fresh)).await??;
        }
        row_result
    })
    .await
    .unwrap_or_else(|error| Err(error.into()));
    if row_needs_logout_0 {
        tokio::time::timeout(Duration::from_secs(10), row_logout(&mut client)).await??;
    }
    row_result

    }).await?
}

async fn wait_for_destination(
    client: &mut DevClientSession,
    target: ActorPosition,
    key: &str,
) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
    loop {
        let at = client.world_spatial().actor_position;
        if at.floor == target.floor && at.x.abs_diff(target.x).max(at.y.abs_diff(target.y)) <= 3 {
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("step=target-walk-{key} observed=timeout-at-{at:?}").into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
    }
}

async fn talk(client: &mut DevClientSession, key: &str, target: ActorPosition) -> TestResult {
    use oteryn_dev_client::client_runtime::npc_shop as runtime;
    use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
    let actor = client
        .world_entities()
        .and_then(|entities| {
            entities.iter().find(|entry| {
                entry.kind == oteryn_dev_client::EntityKind::Npc && entry.position == target
            })
        })
        .map(|entry| entry.entity)
        .ok_or_else(|| format!("step=3-npc-talk-{key} observed=no-visible-npc"))?;
    let exchanges = if key.ends_with("wayland_smythers") {
        &[
            ("hi", "Wayland Smythers, in service of science"),
            ("danger", "humble messenger"),
            ("name", "Wayland Smythers"),
            ("bye", "do be careful"),
        ][..]
    } else {
        &[
            ("hi", "mayor of what remains"),
            ("bye", "Keep your head down out there"),
        ][..]
    };
    for (text, expected) in exchanges {
        client.take_events();
        let disposition = runtime::talk(
            client.session_mut(),
            &NpcTalkIntent {
                npc_actor: actor,
                text: (*text).into(),
            },
        )
        .await?;
        if disposition != NpcIntentDisposition::Ok {
            return Err(format!("step=3-npc-talk-{key}-{text} observed={disposition:?}").into());
        }

        let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
        let mut lines = Vec::new();
        while lines.is_empty() && tokio::time::Instant::now() < deadline {
            client.service_liveness(Duration::from_millis(25)).await?;
            for event in client.take_events() {
                if let SessionEvent::NpcConversation(delta) = event {
                    lines.extend(delta.value.lines);
                }
            }
        }
        if !lines.iter().any(|line| line.contains(expected)) {
            return Err(format!(
                "step=3-npc-talk-{key}-{text} observed=replies-{lines:?}-expected-text-{expected:?}"
            )
            .into());
        }
    }
    Ok(())
}

fn evidence_token(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '=' | ':' | '.')
            {
                character
            } else {
                '-'
            }
        })
        .take(400)
        .collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn evidence_is_one_report_token() {
        let token = super::evidence_token("step=walk observed=Rejected [8, 1]");
        assert!(!token.is_empty());
        assert!(!token.chars().any(char::is_whitespace));
    }
}
