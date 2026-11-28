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
//! 6. Follow the compiled Radiant spawn routes and require each live creature in the roster.
//!    Report the missing player-facing boss encounters after the population checks.


use super::*;
use oteryn_dev_client::{DevClientSession, SessionEvent};

const SUMMER_ACTORS: &[(&str, &str)] = &[
    ("radiant_acolyte", "Radiant Acolyte"),
    ("radiant_inquisitor", "Radiant Inquisitor"),
    ("radiant_paragon", "Radiant Paragon"),
    ("radiant_templar", "Radiant Templar"),
    ("radiant_warden", "Radiant Warden"),
    ("radiant_zealot", "Radiant Zealot"),
];

const NPCS: [&str; 2] = ["oteryn:npc.mayor_pocaro", "oteryn:npc.wayland_smythers"];

fn summer_placements_match(mayor: usize, wayland: usize) -> bool {
    mayor == 1 && wayland == 2
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = verdict::indexed_timeout(Duration::from_secs(180), verdict::observe(journey(clients))).await;
    let (status, detail) = match result {
        Ok(Ok(())) => (
            "PASS",
            "all-summer-map-dialogue-and-population-steps-validated".to_owned(),
        ),
        Ok(Err(error)) => ("FAIL", evidence_token(&error.to_string())),
        Err(_) => (
            "FAIL",
            "step=summer-route-observed=deadline-timeout".to_owned(),
        ),
    };
    if status == "FAIL" {
        evidence(&format!("W16_GAP {detail}"));
    }
    evidence(&format!(
        "SEAM_EVIDENCE e2e_all scenario=W16.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut client = connect_world(clients, 0xb6).await?;

    // The first mechanic command is deliberately local and must produce the same real delta as
    // every other player walk.  It prevents a missing remote route from becoming a probe-only row.
    let before = client.world_spatial().actor_position;
    let start = tile(before)?;
    let expected = destination(world, start, StepDirection::East)
        .ok_or("step=initial-step observed=no-compiled-east-landing")?;
    let step = client.step(StepDirection::East).await?;
    if step.disposition != StepDisposition::Moved || step.world_spatial_delta.is_none() {
        return Err(format!("step=initial-step observed={step:?}").into());
    }
    let after = tile(client.world_spatial().actor_position)?;
    if after != expected || after == start {
        return Err(format!("step=initial-step expected={expected:?} observed={after:?}").into());
    }

    let mut reached = [0usize; 2];
    for (index, key) in NPCS.into_iter().enumerate() {
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
            require_target_walk(key, reply.status, &reply.payload)?;
            wait_for_destination(&mut client, target, key).await?;
            reached[index] += 1;
            talk(&mut client, key, target).await?;
        }
    }
    if !summer_placements_match(reached[0], reached[1]) {
        return Err(format!("step=summer-placements observed=reached-{reached:?} need=1-Mayor-and-2-Wayland-placements").into());
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
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=logout observed=refused".into());
    }
    drop(client);
    let mut fresh = connect_world(clients, 0xb7).await?;
    if fresh.world_spatial().actor_position != saved {
        return Err(format!(
            "step=fresh-admission observed={:?}-expected={saved:?}",
            fresh.world_spatial().actor_position
        )
        .into());
    }
    for &(suffix, name) in SUMMER_ACTORS {
        let key = format!("oteryn:creature.{suffix}");
        super::full_row_w22a::travel_to_named_spawn(&mut fresh, world, &metadata.spawns, &key, name)
            .await
            .map_err(|error| format!("step=summer-population-route need={key} observed={error}"))?;
        if !fresh.world_entities().is_some_and(|entities| {
            entities.iter().any(|entry| {
                entry.kind == oteryn_dev_client::EntityKind::Creature
                    && entry.display_name.is_some_and(|label| {
                        label.as_str().is_ok_and(|shown| shown.eq_ignore_ascii_case(name))
                    })
            })
        }) {
            return Err(format!("step=summer-population-observation need={key} observed=no-matching-live-actor").into());
        }
    }
    if fresh.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=final-logout observed=refused".into());
    }
    Err("step=summer-boss-encounters observed=no-authored-player-entry need=Maior-Domus,Mimar-Haffar,Phosphorus,Rakesh-Moonfang,The-Moonsnow-Magnolia".into())
}

pub(super) fn require_target_walk(
    key: &str,
    status: CommandStatus,
    payload: &[u8],
) -> TestResult {
    // TARGET-MOVE-1 uses an Accepted envelope for both Queued and NoRoute.
    let disposition = match (status, payload) {
        (CommandStatus::Accepted, [8, 1]) => return Ok(()),
        (CommandStatus::Accepted, [8, 2]) => "no-route",
        (CommandStatus::Rejected, [8, 3]) => "refused",
        _ => "unexpected-reply",
    };
    Err(format!("step=target-walk-{key} observed={disposition} status={status:?} payload={payload:?}").into())
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
    fn target_walk_requires_queued_wire_disposition() {
        use super::{CommandStatus, require_target_walk};
        assert!(require_target_walk("npc", CommandStatus::Accepted, &[8, 1]).is_ok());
        for (status, payload, expected) in [
            (CommandStatus::Accepted, [8, 2], "no-route"),
            (CommandStatus::Rejected, [8, 3], "refused"),
            (CommandStatus::Rejected, [8, 1], "unexpected-reply"),
            (CommandStatus::Accepted, [8, 9], "unexpected-reply"),
        ] {
            assert!(require_target_walk("npc", status, &payload)
                .is_err_and(|error| error.to_string().contains(expected)));
        }
    }

    #[test]
    fn review_regression_population_plan_excludes_encounter_bosses() {
        assert_eq!(super::SUMMER_ACTORS.len(), 6);
        assert!(super::SUMMER_ACTORS.iter().all(|(key, _)| key.starts_with("radiant_")));
    }

    #[test]
    fn journey_regression_requires_each_summer_npcs_placements() {
        assert!(super::summer_placements_match(1, 2));
        assert!(!super::summer_placements_match(0, 3));
        assert!(!super::summer_placements_match(2, 1));
    }

    #[test]
    fn evidence_is_one_report_token() {
        let token = super::evidence_token("step=walk observed=Rejected [8, 1]");
        assert!(!token.is_empty());
        assert!(!token.chars().any(char::is_whitespace));
    }
}
