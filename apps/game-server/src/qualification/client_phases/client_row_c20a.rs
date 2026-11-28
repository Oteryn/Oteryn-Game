//! C20a client replay capture: query GeneralStats before and after a real lethal PvE encounter.

use super::*;
use oteryn_dev_client::client_runtime::raw_feature::GatedCommandResult;
use oteryn_dev_client::client_ui::cyclopedia_character::{
    self as panel, CyclopediaCharacterPanel, Intent, TabExtensions,
};
use oteryn_dev_client::{DevClientSession, SessionEvent};

const CREATURE: &str = "oteryn:creature.dragon";
const BEFORE: (u64, u32) = (9300, 10);
const AFTER: (u64, u32) = (8370, 9);
use oteryn_protocol_oteryn::{CommandStatus, cyclopedia_character as wire};

pub(super) async fn run_packet(
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
) {
    let result = verdict::indexed_timeout(
        Duration::from_secs(240),
        capture_journey(clients, world, spawns),
    )
    .await
    .map_err(|_| "C20a client capture timed out".into())
    .and_then(|result| result);
    match result {
        Ok(()) => evidence(
            "SEAM_EVIDENCE e2e_all scenario=C20a.client status=PASS evidence=exact-death-loss-relog-and-live-general-stats-replay",
        ),
        Err(error) => evidence(&format!(
            "SEAM_EVIDENCE e2e_all scenario=C20a.client status=FAIL evidence={error}"
        )),
    }
}

async fn stats(client: &mut DevClientSession) -> TestResult<GatedCommandResult> {
    let mut view = CyclopediaCharacterPanel {
        selected: true,
        limit: 1,
        tab: wire::CharacterTab::GeneralStats,
        ..Default::default()
    };
    let request = panel::request(&mut view, Intent::Tab).ok_or("C20a stats request unavailable")?;
    let reply = client
        .session_mut()
        .gated_command(request.capability, request.kind, &request.payload)
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("C20a GeneralStats query refused: {:?}", reply.status).into());
    }
    Ok(reply)
}

fn progress(reply: &GatedCommandResult) -> TestResult<(u64, u32)> {
    let payload = &reply.payload;
    if payload.first() != Some(&(wire::CharacterTab::GeneralStats as u8)) {
        return Err("C20a malformed GeneralStats reply".into());
    }
    Ok((
        u64::from_le_bytes(payload.get(1..9).ok_or("C20a XP missing")?.try_into()?),
        u32::from_le_bytes(payload.get(9..13).ok_or("C20a level missing")?.try_into()?),
    ))
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    let reply = client.session_mut().logout().await?;
    if reply.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("C20a logout refused: {reply:?}").into());
    }
    Ok(())
}

async fn capture_journey(
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
) -> TestResult {
    let directory =
        std::env::var_os("E2E_ALL_CAPTURE_DIR").ok_or("C20a capture directory missing")?;
    let directory = Path::new(&directory);
    character_prep::declare_seeded(
        "C20a.client",
        "sorcerer-level-10",
        "earnable XP exceeds the row budget",
    );
    let mut client = connect_world(clients, 0xca).await?;
    let setup_session = client.join_snapshot().game_session_id;
    request_fixture_control_at(
        "NODE_BOOT_CONTROL_SOCKET",
        uuid_text(setup_session.as_bytes()),
        "--profile",
        "c20a".into(),
    )
    .await?;
    logout(&mut client).await?;
    drop(client);
    let mut client = connect_world(clients, 0xcb).await?;
    let session = client.join_snapshot().game_session_id;
    if setup_session == session {
        return Err("C20a preparation reused the original session".into());
    }
    let before = stats(&mut client).await?;
    if progress(&before)? != BEFORE {
        return Err(format!("C20a wrong pre-death progress: {:?}", progress(&before)?).into());
    }
    let vocation_selected = general_stats::selected(&client);
    let home = world
        .base()
        .subareas
        .default_town
        .and_then(|id| world.base().subareas.town(id))
        .and_then(|town| town.temple)
        .ok_or("C20a compiled temple missing")?;
    let basis = spatial_basis::current(&mut client)?;
    let temple = ActorPosition {
        x: i32::from(home.x),
        y: i32::from(home.y),
        floor: basis.floor(home.floor),
    };
    let baseline = *client
        .actor_vitals()
        .ok_or("C20a pre-encounter vitals missing")?;
    if baseline.health == 0 || baseline.health != baseline.max_health {
        return Err(format!("C20a fixture did not enter at full health: {baseline:?}").into());
    }
    client.take_events();
    let session_label = uuid_text(session.as_bytes());
    if !spawns.creatures.iter().any(|entry| entry.key == CREATURE) {
        return Err("C20a native Dragon definition missing from the world bundle".into());
    }
    let open = |at: TilePos| {
        world
            .base()
            .tile(at.x, at.y, at.floor)
            .is_some_and(|tile| !tile.protection_zone())
    };
    let from = tile(client.world_spatial().actor_position)?;
    let path = route(world, from, |at| {
        at.floor == from.floor
            && open(at)
            && [
                StepDirection::North,
                StepDirection::East,
                StepDirection::South,
                StepDirection::West,
            ]
            .into_iter()
            .filter(|direction| destination(world, at, *direction).is_some_and(open))
            .count()
                >= 3
    })
    .ok_or("C20a no reachable open encounter staging ground")?;
    let expected = path
        .iter()
        .try_fold(from, |at, direction| destination(world, at, *direction))
        .ok_or("C20a route does not reach its encounter site")?;
    let mut at = from;
    for direction in path {
        let landing = destination(world, at, direction).ok_or("C20a route step has no landing")?;
        let step = client.step(direction).await?;
        let observed = tile(client.world_spatial().actor_position)?;
        if step.disposition != StepDisposition::Moved || observed != landing {
            let occupant = client.world_entities().and_then(|entities| {
                entities
                    .iter()
                    .find(|entry| tile(entry.position).ok() == Some(landing))
                    .map(|entry| entry.entity)
            });
            return Err(format!(
                "C20a encounter step failed: from={at:?} expected={landing:?} actual={observed:?} occupant={occupant:?} reply={step:?}"
            ).into());
        }
        verdict::Harness::global().progress("walk");
        at = observed;
        tokio::time::sleep(Duration::from_millis(u64::from(step.step_wait_ms))).await;
    }
    let encounter = client.world_spatial().actor_position;
    if tile(encounter)? != expected || encounter == temple {
        return Err(format!(
            "C20a wrong encounter position: {encounter:?}, expected={expected:?}, temple={temple:?}"
        )
        .into());
    }
    if !client
        .actor_vitals()
        .is_some_and(|vitals| vitals.health == baseline.health)
    {
        return Err("C20a incoming damage before the controlled encounter".into());
    }
    client.take_events();
    if !client
        .selected_capabilities()
        .contains(&oteryn_protocol_oteryn::analyser::CAPABILITY_ANALYSER_HIT_ATTRIBUTION_V1)
    {
        return Err("C20a live attacker attribution capability missing".into());
    }
    let answer = request_spawn(session_label.clone(), CREATURE.into()).await?;
    let actor = parse_spawned_entity(&answer)?;
    evidence(&format!(
        "C20A_ENCOUNTER session={session_label} input=fixture-native-dragon spawn={answer:?} player={encounter:?} temple={temple:?} baseline_health={}",
        baseline.health
    ));
    let deadline = tokio::time::Instant::now() + Duration::from_secs(180);
    verdict::Harness::global().bounded_wait(deadline, "bounded-observation");
    let mut spawn_visible = false;
    let mut attacker_sequence = None;
    let mut damage_sequence = None;
    let mut min_health = baseline.health;
    loop {
        if !spawn_visible
            && client.world_entities().is_some_and(|entities| {
                entities.iter().any(|entry| {
                    entry.entity == actor
                        && entry.kind == oteryn_dev_client::EntityKind::Creature
                        && entry.position.floor == encounter.floor
                        && entry.display_name.is_some_and(|name| {
                            name.as_str()
                                .is_ok_and(|name| name.eq_ignore_ascii_case("Dragon"))
                        })
                })
            })
        {
            spawn_visible = true;
            evidence(&format!(
                "C20A_SPAWN_VISIBLE session={session_label} actor={actor:?} server_sequence={}",
                client.last_server_sequence()
            ));
        }
        for event in client.take_events() {
            if let SessionEvent::AnalyserAttributedFacts(ref delta) = event {
                for fact in &delta.value {
                    if let Some(attacker) = fact.attacker {
                        if attacker != actor {
                            return Err(format!(
                                "C20a unexpected attacker: expected={actor:?} observed={attacker:?} server_sequence={}",
                                delta.server_sequence
                            ).into());
                        }
                        if attacker_sequence.is_none() {
                            attacker_sequence = Some(delta.server_sequence);
                            evidence(&format!(
                                "C20A_ATTACKER session={session_label} actor={actor:?} server_sequence={}",
                                delta.server_sequence
                            ));
                        }
                    }
                }
            }
            if let SessionEvent::ActorVitals(delta) = event
                && delta.value.health < min_health
            {
                min_health = delta.value.health;
                damage_sequence.get_or_insert(delta.server_sequence);
                evidence(&format!(
                    "C20A_DAMAGE session={session_label} server_sequence={} health={min_health}",
                    delta.server_sequence
                ));
            }
        }
        let position = client.world_spatial().actor_position;
        if spawn_visible
            && attacker_sequence.is_some()
            && damage_sequence.is_some()
            && position == temple
            && client
                .actor_vitals()
                .is_some_and(|vitals| vitals.health > 0 && vitals.health == vitals.max_health)
        {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "C20a missing temple respawn: session={session_label} spawn_visible={spawn_visible} attacker_sequence={attacker_sequence:?} first_damage_sequence={damage_sequence:?} min_health={min_health} position={position:?} expected={temple:?}"
            ).into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
    let after = stats(&mut client).await?;
    if progress(&after)? != AFTER {
        return Err(format!(
            "C20a wrong death progress: expected={AFTER:?} observed={:?}",
            progress(&after)?
        )
        .into());
    }
    let combat_deadline = tokio::time::Instant::now() + Duration::from_secs(60);
    verdict::Harness::global().bounded_wait(combat_deadline, "c20a-combat-lock");
    while client
        .session_mut()
        .combat_state()
        .ok_or("C20a combat state missing before logout")?
        .in_fight
    {
        if tokio::time::Instant::now() >= combat_deadline {
            return Err("C20a combat lock did not clear at the respawn tile".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    }
    logout(&mut client).await?;
    drop(client);
    let mut fresh = connect_world(clients, 0xcc).await?;
    let persisted = progress(&stats(&mut fresh).await?)?;
    if fresh.join_snapshot().game_session_id == session || persisted != AFTER {
        return Err(
            format!("C20a death outcome did not survive fresh admission: {persisted:?}").into(),
        );
    }
    logout(&mut fresh).await?;
    let session = uuid_text(session.as_bytes());
    replay(directory, &session, vocation_selected, [&before, &after])?;
    std::fs::write(
        directory.join("c20a-expected.txt"),
        format!("{} {}\n{} {}\n", BEFORE.0, BEFORE.1, AFTER.0, AFTER.1),
    )?;
    Ok(())
}

fn replay(
    directory: &Path,
    session: &str,
    vocation: bool,
    replies: [&GatedCommandResult; 2],
) -> TestResult {
    if replies[0].server_sequence >= replies[1].server_sequence
        || replies[0].command_id == replies[1].command_id
    {
        return Err("C20a captures did not have distinct ordered live replies".into());
    }
    let path = directory.join(format!("c20a-{session}.json"));
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)?;
    let frames = replies.map(|reply| (reply.command_id, reply.server_sequence, &reply.payload));
    serde_json::to_writer(file, &(session, vocation, frames))?;
    type CapturedReply = (u64, u64, Vec<u8>);
    let (captured_session, captured_vocation, captured): (String, bool, [CapturedReply; 2]) =
        serde_json::from_reader(std::fs::File::open(&path)?)?;
    if captured_session != session || captured_vocation != vocation {
        return Err("C20a capture identity mismatch".into());
    }
    let mut view = CyclopediaCharacterPanel {
        selected: true,
        limit: 1,
        tab: wire::CharacterTab::GeneralStats,
        extensions: TabExtensions {
            vocation,
            death_details: false,
        },
        ..Default::default()
    };
    for ((((command_id, server_sequence, payload), live), (xp, level)), name) in captured
        .into_iter()
        .zip(replies)
        .zip([BEFORE, AFTER])
        .zip(["c20a-before.bin", "c20a-after.bin"])
    {
        if (command_id, server_sequence, &payload)
            != (live.command_id, live.server_sequence, &live.payload)
        {
            return Err("C20a capture differs from the live reply".into());
        }
        panel::request(&mut view, Intent::Tab).ok_or("C20a replay request unavailable")?;
        let reply = GatedCommandResult {
            command_id,
            server_sequence,
            status: live.status,
            payload,
        };
        if !panel::apply_result(&mut view, &reply) {
            return Err("C20a replay rejected a live reply".into());
        }
        let result = view
            .result
            .as_deref()
            .ok_or("C20a replay produced no view")?;
        if !result
            .lines()
            .any(|line| line == format!("Experience: {xp}"))
            || !result.lines().any(|line| line == format!("Level: {level}"))
        {
            return Err(format!("C20a replay rendered the wrong stats: {result:?}").into());
        }
        std::fs::write(directory.join(name), &reply.payload)?;
        std::fs::write(
            directory.join(format!("{name}.selection")),
            format!(
                "vocation_selected={}\nvocation_id={}\n",
                u8::from(vocation),
                view.vocation_id
                    .map_or_else(|| "none".to_owned(), |id| id.to_string())
            ),
        )?;
    }
    Ok(())
}
