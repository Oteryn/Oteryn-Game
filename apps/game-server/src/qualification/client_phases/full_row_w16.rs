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
use oteryn_dev_client::{DevClientSession, EntityKind, SessionEvent};

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
    let fixture = character_prep::dedicated_character(clients, "NODE_BOOT_W16_ACCOUNT").await?;
    let fixture_clients = fixture.clients(clients);
    let clients = &fixture_clients;
    let mut client = connect_character(clients, 0xb6, 0).await?;

    // The first mechanic command is deliberately local and must produce the same real delta as
    // every other player walk.  It prevents a missing remote route from becoming a probe-only row.
    let before = client.world_spatial().actor_position;
    let start = tile(before)?;
    let expected = destination(world, start, StepDirection::East)
        .ok_or("step=initial-step observed=no-compiled-east-landing")?;
    let step = population_step(&mut client, world, StepDirection::East).await?;
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
            logout_w16(&mut client).await?;
            drop(client);
            client = approach(clients, world, 0xb6, key, target).await?;
            reached[index] += 1;
            talk(&mut client, key, target).await?;
        }
    }
    if !summer_placements_match(reached[0], reached[1]) {
        return Err(format!("step=summer-placements observed=reached-{reached:?} need=1-Mayor-and-2-Wayland-placements").into());
    }

    let saved = client.world_spatial().actor_position;
    client.take_events();
    use oteryn_protocol_oteryn::target_move as wire;
    // The normal typed client refuses floor 16 locally. Send the encoded
    // input once so this row tests the server's refusal and unchanged state.
    let payload = wire::encode_target_move_intent(wire::TargetMoveIntent::Walk(
        ActorPosition { floor: 16, ..saved },
    ))
    .map_err(|error| format!("step=invalid-floor-encode observed={error:?}"))?;
    let refused = client
        .session_mut()
        .gated_command(wire::CAPABILITY_TARGET_MOVE_V1, wire::COMMAND_TYPE_TARGET_MOVE_INTENT, &payload)
        .await?;
    if refused.status != CommandStatus::Rejected || refused.payload != [8, 3] {
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
    let mut fresh = connect_character(clients, 0xb7, 0).await?;
    if fresh.world_spatial().actor_position != saved {
        return Err(format!(
            "step=fresh-admission observed={:?}-expected={saved:?}",
            fresh.world_spatial().actor_position
        )
        .into());
    }
    for &(suffix, name) in SUMMER_ACTORS {
        let key = format!("oteryn:creature.{suffix}");
        if let Some((spawn, start)) = population_fixture(suffix) {
            if !world.enterable(tile(start)?) || !metadata.spawns.sources.iter().any(|source| {
                source.floor == spawn.floor && source.points.iter().any(|point| {
                    (point.x, point.y) == (spawn.x, spawn.y)
                        && metadata.spawns.creatures.get(point.creature as usize)
                            .is_some_and(|entry| entry.key == key)
                })
            }) {
                return Err(format!("W16 authored {name} fixture cells absent or blocked").into());
            }
            logout_w16(&mut fresh).await?;
            drop(fresh);
            fresh = stage_at(clients, 0xb8, suffix, start).await?;
        }
        observe_population(&mut fresh, world, &metadata.spawns, &key, name)
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

async fn population_step(
    client: &mut DevClientSession,
    world: &BundleWorld,
    direction: StepDirection,
) -> TestResult<oteryn_dev_client::StepOutcome> {
    let before = client.world_spatial().actor_position;
    let expected = destination(world, tile(before)?, direction).ok_or("W16 step has no compiled landing")?;
    let mut waited = false;
    loop {
        let outcome = client.step(direction).await?;
        if outcome.disposition == StepDisposition::Moved {
            return Ok(outcome);
        }
        evidence(&format!(
            "W16_STEP_REFUSED from={before:?} landing={expected:?} direction={direction:?} command={} sequence={} status={:?} disposition={:?} reason={:?}",
            outcome.command_id, outcome.result_server_sequence, outcome.status,
            outcome.disposition, client.session_mut().last_refusal_detail()
        ));
        if waited || outcome.disposition != StepDisposition::Blocked
            || client.world_spatial().actor_position != before
        {
            return Err(format!("W16 step refused: {outcome:?}").into());
        }
        let entities = client.world_entities().ok_or("W16 cannot identify a blocker without visibility")?;
        let occupant = entities.iter().find(|entity| tile(entity.position).ok() == Some(expected));
        let Some(occupant) = occupant else {
            return Err(format!("BLOCKER-SERVER W16 blocked compiled landing {expected:?} without visible occupant").into());
        };
        let blocker = occupant.entity;
        if occupant.kind != EntityKind::Creature {
            return Err(format!("W16 blocked by {:?} entity={blocker:?} tile={expected:?}", occupant.kind).into());
        }
        evidence(&format!("W16_CREATURE_WAIT entity={blocker:?} tile={expected:?} budget_ms=2000"));
        let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
        loop {
            client.service_liveness(Duration::from_millis(25)).await?;
            let entities = client.world_entities().ok_or("W16 creature visibility was lost")?;
            let current = entities.iter().find(|entity| entity.entity == blocker)
                .and_then(|entity| tile(entity.position).ok());
            if current.is_some_and(|at| at != expected)
                && !entities.iter().any(|entity| tile(entity.position).ok() == Some(expected))
            {
                evidence(&format!("W16_CREATURE_MOVED entity={blocker:?} from={expected:?} to={current:?}"));
                break;
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(format!("W16 creature {blocker:?} did not clear {expected:?}").into());
            }
        }
        waited = true;
    }
}

async fn follow_population_path(
    client: &mut DevClientSession,
    world: &BundleWorld,
    path: &[StepDirection],
) -> TestResult {
    for &direction in path {
        let from = tile(client.world_spatial().actor_position)?;
        let expected = destination(world, from, direction).ok_or("W16 compiled step has no landing")?;
        let outcome = population_step(client, world, direction).await?;
        if outcome.world_spatial_delta.is_none() || tile(client.world_spatial().actor_position)? != expected {
            return Err(format!("W16 wrong landing from={from:?} expected={expected:?}").into());
        }
        verdict::Harness::global().progress("walk");
        if outcome.step_wait_ms != 0 {
            client.service_liveness(Duration::from_millis(u64::from(outcome.step_wait_ms))).await?;
        }
        client.take_events();
    }
    Ok(())
}

async fn observe_population(
    client: &mut DevClientSession,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
    key: &str,
    name: &str,
) -> TestResult {
    let from = tile(client.world_spatial().actor_position)?;
    let mut approaches = std::collections::BTreeSet::new();
    for source in &spawns.sources {
        for point in &source.points {
            if !spawns.creatures.get(point.creature as usize).is_some_and(|entry| entry.key == key) {
                continue;
            }
            for dx in -1..=1 {
                for dy in -1..=1 {
                    if let (Some(x), Some(y)) = (point.x.checked_add_signed(dx), point.y.checked_add_signed(dy))
                        && world.base().tile(x, y, source.floor).is_some_and(|tile| !tile.protection_zone()) {
                            approaches.insert((x, y, source.floor));
                    }
                }
            }
        }
    }
    if approaches.is_empty() {
        return Err(format!("BLOCKED-CONTENT no compiled approach tiles for {key}").into());
    }
    let path = find_route(from, |at| approaches.contains(&(at.x, at.y, at.floor)),
        |at, direction| destination(world, at, direction))
        .map_err(|visited| format!("BLOCKED-CONTENT no compiled route for {key} from={from:?} visited={visited}"))?;
    evidence(&format!("W16_POPULATION_ROUTE source={key} from={from:?} steps={}", path.len()));
    follow_population_path(client, world, &path).await?;
    let here = client.world_spatial().actor_position;
    let actor = client.world_entities().and_then(|entities| entities.iter()
        .filter(|entry| entry.kind == EntityKind::Creature && entry.position.floor == here.floor
            && entry.display_name.is_some_and(|label| label.as_str().is_ok_and(|shown| shown.eq_ignore_ascii_case(name))))
        .min_by_key(|entry| entry.position.x.abs_diff(here.x).max(entry.position.y.abs_diff(here.y)))
        .map(|entry| entry.entity)).ok_or_else(|| format!("W16 no visible live {name}"))?;
    for _ in 0..32 {
        let target = client.world_entities().and_then(|entities| entities.iter()
            .find(|entry| entry.entity == actor).map(|entry| entry.position))
            .ok_or_else(|| format!("W16 creature {actor:?} vanished during approach"))?;
        let target = tile(target)?;
        let here = tile(client.world_spatial().actor_position)?;
        let path = route(world, here, |at| at.floor == target.floor
            && at.x.abs_diff(target.x).max(at.y.abs_diff(target.y)) <= 1)
            .ok_or_else(|| format!("W16 live creature {actor:?} unreachable from={here:?} target={target:?}"))?;
        let Some(direction) = path.first() else {
            evidence(&format!("W16_POPULATION_OBSERVED source={key} entity={actor:?} tile={target:?}"));
            return Ok(());
        };
        follow_population_path(client, world, std::slice::from_ref(direction)).await?;
    }
    Err(format!("W16 creature {actor:?} approach exceeded 32 steps").into())
}

fn population_fixture(suffix: &str) -> Option<(TilePos, ActorPosition)> {
    let (spawn_x, spawn_y, start_x, start_y, floor) = match suffix {
        "radiant_acolyte" => (33946, 32645, 33945, 32645, 5),
        "radiant_inquisitor" => (33945, 32646, 33944, 32646, 2),
        _ => return None,
    };
    Some((
        TilePos { x: spawn_x, y: spawn_y, floor: -floor },
        ActorPosition { x: start_x, y: start_y, floor: i16::from(floor) },
    ))
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

pub(super) async fn logout_w16(client: &mut DevClientSession) -> TestResult {
    let result = client.session_mut().logout().await?;
    if result.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("W16 logout refused: {result:?}").into());
    }
    Ok(())
}

pub(super) async fn approach(
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    tag: u8,
    key: &str,
    npc: ActorPosition,
) -> TestResult<DevClientSession> {
    let ((x, y), direction) = match (key, npc.x, npc.y, npc.floor) {
        ("oteryn:npc.mayor_pocaro", 34073, 32504, 6) => ((34075, 32502), StepDirection::West),
        ("oteryn:npc.wayland_smythers", 32961, 32076, 6) => ((32959, 32076), StepDirection::East),
        ("oteryn:npc.wayland_smythers", 33955, 32642, 6) => ((33955, 32644), StepDirection::North),
        _ => return Err(format!("W16 unexpected authored placement {key} at {npc:?}").into()),
    };
    let start = ActorPosition { x, y, floor: npc.floor };
    let expected = destination_with_teleports(world, tile(start)?, direction, false)
        .ok_or_else(|| format!("W16 no compiled landing from {start:?} toward {npc:?}"))?;
    let landing = ActorPosition {
        x: i32::from(expected.x),
        y: i32::from(expected.y),
        floor: -i16::from(expected.floor),
    };
    if landing == start || landing.floor != npc.floor
        || landing.x.abs_diff(npc.x).max(landing.y.abs_diff(npc.y)) > 2
    {
        return Err(format!("W16 invalid landing {landing:?} from {start:?} toward {npc:?}").into());
    }
    let mut client = stage_at(clients, tag, key, start).await?;
    let reply = client.session_mut().target_walk(landing).await?;
    require_target_walk(key, reply.status, &reply.payload)?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
    while client.world_spatial().actor_position != landing {
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("step=target-walk-{key} expected={landing:?} observed={:?}", client.world_spatial().actor_position).into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
    }
    evidence(&format!("W16_WALK npc={key} start={start:?} landing={landing:?}"));
    Ok(client)
}

async fn stage_at(
    clients: &SeamClients<'_>, tag: u8, key: &str, start: ActorPosition,
) -> TestResult<DevClientSession> {
    wait_character_released(clients, 0).await?;
    let staged_floor = tokio::time::timeout(Duration::from_secs(5), async {
        let mut connection = sqlx::PgConnection::connect(clients.url).await?;
        sqlx::query_scalar::<_, i16>(
            "UPDATE game_character_last_positions p
             SET tile_x=$4, tile_y=$5, tile_floor=CASE WHEN p.tile_floor < 0 THEN -$6::smallint ELSE $6::smallint END,
                 write_sequence=p.write_sequence+1
             WHERE p.character_id=encode($1,'hex')::uuid AND p.world_id=encode($2,'hex')::uuid
             AND EXISTS (SELECT 1 FROM game_character_roots r
                         WHERE r.character_id=p.character_id AND r.account_id=$3::uuid)
             AND NOT EXISTS (SELECT 1 FROM game_durability_reconnect_sessions s
                             WHERE s.character_id=p.character_id AND s.world_id=p.world_id
                             AND s.session_state<>3)
             RETURNING p.tile_floor",
        )
        .bind(clients.characters[0].as_slice())
        .bind(clients.world.as_slice())
        .bind(&clients.accounts[0])
        .bind(start.x)
        .bind(start.y)
        .bind(start.floor)
        .fetch_one(&mut connection)
        .await
    })
    .await??;
    evidence(&format!("FIXTURE q_w16 source={key} start={start:?} saved_floor={staged_floor}"));
    let client = connect_character(clients, tag, 0).await?;
    if client.world_spatial().actor_position != start {
        return Err(format!("W16 fixture admission expected={start:?} observed={:?}", client.world_spatial().actor_position).into());
    }
    Ok(client)
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
