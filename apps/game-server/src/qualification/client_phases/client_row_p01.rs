//! P01 client replay capture: real CHARM_PROGRESSION replies for the initial QUERY, a refused
//! ASSIGN of a locked charm, and the QUERY after a credited hunt kill. The consumer is `apply` in
//! `apps/client/src/features/0067-bestiary-charm-progression/protocol.rs`.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::charm::CharmAssignDisposition;
use oteryn_protocol_oteryn::charm_progression::{
    self as wire, CharmProgressionAction, CharmProgressionOutcome, CharmProgressionReply,
};
use std::fmt::Write as _;
use std::num::NonZeroU32;

// The authored Rat approach is 208 wire steps. The client packet now runs a
// first credited hunt to establish its baseline, then the captured hunt.
const DEADLINE: Duration = Duration::from_secs(600);

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    use std::io::Write as _;

    let root = required("E2E_ALL_CAPTURE_DIR")?;
    let mut file = std::fs::File::create_new(Path::new(&root).join(name))?;
    file.write_all(bytes)?;
    Ok(())
}

async fn send(
    client: &mut DevClientSession,
    action: CharmProgressionAction,
    capture_name: &str,
    frames: &mut String,
) -> TestResult<CharmProgressionReply> {
    let payload = wire::encode_charm_progression_request(&action)
        .map_err(|error| format!("encode {action:?}: {error:?}"))?;
    let command_id = client.session_mut().next_command_id();
    let reply = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_BESTIARY_CHARM_PROGRESSION_V1,
            wire::COMMAND_TYPE_CHARM_PROGRESSION,
            &payload,
        )
        .await?;
    require_mechanic_acceptance(reply.status, &format!("CHARM_PROGRESSION {action:?}"))?;
    if reply.payload.is_empty() {
        return Err(format!("{action:?} returned an empty reply").into());
    }
    if reply.command_id != command_id {
        return Err(format!("{action:?} returned a different command ID").into());
    }
    let session = uuid_text(client.join_snapshot().game_session_id.as_bytes());
    let envelope = serde_json::to_vec(&serde_json::json!({
        "command_id": reply.command_id,
        "game_session_id": session,
        "payload": reply.payload,
        "server_sequence": reply.server_sequence,
        "status": reply.status as u32,
    }))?;
    capture(capture_name, &envelope)?;
    writeln!(
        frames,
        "{capture_name}={}:{}",
        command_id, reply.server_sequence
    )?;
    wire::decode_charm_progression_reply(&reply.payload)
        .map_err(|error| format!("decode {action:?}: {error:?}").into())
}

fn kills(reply: &CharmProgressionReply, race: NonZeroU32) -> u32 {
    reply
        .bestiary
        .iter()
        .find(|entry| entry.race == race)
        .map_or(0, |entry| entry.kill_count)
}

async fn walk_rat_route(
    client: &mut DevClientSession,
    world: &BundleWorld,
    path: &[StepDirection],
) -> TestResult {
    for &direction in path {
        let before = tile(client.world_spatial().actor_position)?;
        let expected =
            destination(world, before, direction).ok_or("Rat route step has no landing")?;
        let step = client.step(direction).await?;
        if step.status != CommandStatus::Accepted
            || step.disposition != StepDisposition::Moved
            || tile(client.world_spatial().actor_position)? != expected
        {
            return Err(format!(
                "Rat route step {direction:?} from {before:?} expected {expected:?}: {step:?}"
            )
            .into());
        }
        verdict::Harness::global().progress("rat-walk");
        client.take_events();
        tokio::time::sleep(Duration::from_millis(u64::from(step.step_wait_ms))).await;
    }
    Ok(())
}

fn same_floor_within(at: TilePos, point: TilePos, distance: u16) -> bool {
    at.floor == point.floor && at.x.abs_diff(point.x).max(at.y.abs_diff(point.y)) <= distance
}

async fn approach_rat(
    client: &mut DevClientSession,
    world: &BundleWorld,
    approach: TilePos,
    spawn: TilePos,
    can_land: impl Fn(TilePos) -> bool,
) -> TestResult<oteryn_dev_client::WorldSpatialEntity> {
    use oteryn_dev_client::EntityKind;

    let from = tile(client.world_spatial().actor_position)?;
    let path = route_avoiding(world, from, |at| at == approach, &can_land)
        .ok_or("authored Rat approach is unreachable")?;
    walk_rat_route(client, world, &path).await?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
    let target = loop {
        if let Some(target) = client
            .world_entities()
            .and_then(|entities| {
                entities.iter().find(|entity| {
                    entity.kind == EntityKind::Creature
                        && entity
                            .display_name
                            .as_ref()
                            .and_then(|name| name.as_str().ok())
                            == Some("Rat")
                        && tile(entity.position).is_ok_and(|at| same_floor_within(at, spawn, 7))
                })
            })
            .copied()
        {
            break target;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("no wire-visible Rat near authored spawn {spawn:?}").into());
        }
        client.service_liveness(Duration::from_millis(100)).await?;
        client.take_events();
    };
    let target_position = tile(target.position)?;
    let path = route_avoiding(
        world,
        approach,
        |at| at != target_position && same_floor_within(at, target_position, 1),
        |at| can_land(at) && at != target_position,
    )
    .ok_or("selected Rat is unreachable")?;
    walk_rat_route(client, world, &path).await?;
    let current = client
        .world_entities()
        .and_then(|entities| entities.get(&target.entity))
        .copied()
        .ok_or("selected Rat left visibility before the first attack")?;
    if !same_floor_within(
        tile(client.world_spatial().actor_position)?,
        tile(current.position)?,
        1,
    ) {
        return Err("selected Rat moved out of melee reach before the first attack".into());
    }
    Ok(current)
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut client = full_row_p01::prepare_hunter(clients, "P01.client", 0x21).await?;
    let rat_index = full_row_p05::canonical_races()?
        .iter()
        .position(|(definition, _)| definition.key() == "oteryn:creature.rat")
        .ok_or("Rat is absent from the canonical Bestiary")?;
    let race = NonZeroU32::new(u32::try_from(rat_index)? + 1).ok_or("Rat race index")?;
    let corpse_stage = full_row_p01::rat_corpse_stage()?;
    let mut frames = String::new();
    let initial = send(
        &mut client,
        CharmProgressionAction::Query,
        "p01-initial.json",
        &mut frames,
    )
    .await?;
    if initial.outcome != CharmProgressionOutcome::Query || kills(&initial, race) != 0 {
        return Err("P01 client requires a zero-kill Rat baseline".into());
    }
    if !client
        .selected_capabilities()
        .contains(&oteryn_protocol_oteryn::entity_names::CAPABILITY_WORLD_SPATIAL_ENTITIES_V2)
    {
        return Err("P01 Rat selection requires wire entity names".into());
    }
    // The static map admits creature spawn tiles, but a live creature can
    // occupy its authored tile and refuse a STEP into it.
    let spawn_tiles: std::collections::HashSet<_> = metadata
        .spawns
        .sources
        .iter()
        .flat_map(|source| {
            source
                .points
                .iter()
                .map(move |point| (point.x, point.y, source.floor))
        })
        .collect();
    let can_land = |at: TilePos| !spawn_tiles.contains(&(at.x, at.y, at.floor));

    let rat_points: Vec<_> = metadata
        .spawns
        .sources
        .iter()
        .flat_map(|source| {
            source.points.iter().filter_map(|point| {
                metadata
                    .spawns
                    .creatures
                    .get(point.creature as usize)
                    .is_some_and(|entry| entry.key == "oteryn:creature.rat")
                    .then_some((
                        TilePos {
                            x: point.x,
                            y: point.y,
                            floor: source.floor,
                        },
                        point.respawn_ms,
                    ))
            })
        })
        .collect();
    let from = tile(client.world_spatial().actor_position)?;
    let path = route_avoiding(
        world,
        from,
        |at| {
            world
                .base()
                .tile(at.x, at.y, at.floor)
                .is_some_and(|tile| !tile.protection_zone())
                && rat_points
                    .iter()
                    .any(|(point, _)| at != *point && same_floor_within(at, *point, 1))
        },
        &can_land,
    )
    .ok_or("no reachable authored Rat spawn")?;
    let approach = path
        .iter()
        .try_fold(from, |at, &direction| destination(world, at, direction))
        .ok_or("Rat approach route has no landing")?;
    let &(spawn, respawn_ms) = rat_points
        .iter()
        .find(|(point, _)| approach != *point && same_floor_within(approach, *point, 1))
        .ok_or("Rat approach has no authored spawn")?;
    let target = approach_rat(&mut client, world, approach, spawn, &can_land).await?;
    verdict::Harness::global().bounded_wait(
        tokio::time::Instant::now() + Duration::from_secs(125),
        "baseline-rat-combat",
    );
    client_hunt::hunt_named_target(&mut client, target, corpse_stage).await?;
    let before = send(
        &mut client,
        CharmProgressionAction::Query,
        "p01-before.json",
        &mut frames,
    )
    .await?;
    if before.outcome != CharmProgressionOutcome::Query || before.charms.charms.is_empty() {
        return Err(format!("QUERY outcome {:?}", before.outcome).into());
    }
    if kills(&before, race) != 1 {
        return Err("first Rat hunt did not establish exactly one Bestiary kill".into());
    }
    let (index, locked) = before
        .charms
        .charms
        .iter()
        .enumerate()
        .find(|(_, charm)| charm.unlocked_stage == 0 && charm.next_stage_cost > 0)
        .ok_or("no locked charm to refuse")?;
    let charm = NonZeroU32::new(u32::try_from(index)? + 1).ok_or("charm index")?;
    let refused = send(
        &mut client,
        CharmProgressionAction::Assign { charm, race },
        "p01-refused.json",
        &mut frames,
    )
    .await?;
    if refused.outcome != CharmProgressionOutcome::Assign(CharmAssignDisposition::CharmLocked)
        || refused.charms != before.charms
        || refused.bestiary != before.bestiary
    {
        return Err(format!("locked ASSIGN reply {refused:?}").into());
    }
    // Leave this same spawn's view for its authored respawn interval before
    // the second hunt. A player in view would defer the spawn.
    let from = tile(client.world_spatial().actor_position)?;
    let retreat = route_avoiding(
        world,
        from,
        |at| {
            at.floor == spawn.floor
                && (!(-8..=9).contains(&(i32::from(spawn.x) - i32::from(at.x)))
                    || !(-6..=7).contains(&(i32::from(spawn.y) - i32::from(at.y))))
        },
        &can_land,
    )
    .ok_or("authored Rat spawn has no offscreen waiting tile")?;
    walk_rat_route(&mut client, world, &retreat).await?;
    let respawn = tokio::time::Instant::now() + Duration::from_millis(u64::from(respawn_ms));
    verdict::Harness::global().bounded_wait(respawn, "rat-respawn-outside-view");
    while tokio::time::Instant::now() < respawn {
        client.service_liveness(Duration::from_millis(100)).await?;
        client.take_events();
    }
    let target = approach_rat(&mut client, world, approach, spawn, &can_land).await?;
    verdict::Harness::global().bounded_wait(
        tokio::time::Instant::now() + Duration::from_secs(125),
        "captured-rat-combat",
    );
    client_hunt::hunt_named_target(&mut client, target, corpse_stage).await?;
    let after = send(
        &mut client,
        CharmProgressionAction::Query,
        "p01-after.json",
        &mut frames,
    )
    .await?;
    if after.outcome != CharmProgressionOutcome::Query {
        return Err(format!("post-hunt QUERY outcome {:?}", after.outcome).into());
    }
    let count_before = kills(&before, race);
    let count_after = kills(&after, race);
    let mut expected_bestiary = before.bestiary.clone();
    let rat = expected_bestiary
        .iter_mut()
        .find(|entry| entry.race == race)
        .ok_or("Rat absent from baseline Bestiary")?;
    rat.kill_count += 1;
    if after.bestiary != expected_bestiary {
        return Err(format!("second Rat hunt changed unexpected Bestiary state: {after:?}").into());
    }
    let old_session = client.join_snapshot().game_session_id;
    full_row_p01::logout_when_ready(&mut client).await?;
    drop(client);
    let mut fresh = connect_world(clients, 0x22).await?;
    if fresh.join_snapshot().game_session_id == old_session {
        return Err("P01.client relog reused the old session".into());
    }
    let durable = send(
        &mut fresh,
        CharmProgressionAction::Query,
        "p01-durable.json",
        &mut frames,
    )
    .await?;
    if durable != after {
        return Err(format!("P01.client relog lost the changed views: {durable:?}").into());
    }
    full_row_p01::logout_when_ready(&mut fresh).await?;
    let session = uuid_text(old_session.as_bytes());
    let durable_session = uuid_text(fresh.join_snapshot().game_session_id.as_bytes());
    let expected = format!(
        "session={session}\ndurable_session={durable_session}\nrace={race}\nkills_before={count_before}\nkills_after={count_after}\npoints_before={}\npoints_after={}\nlocked_charm={charm}\nlocked_stage={}\nrefusal=CharmLocked\n",
        before.charms.charm_points_available,
        after.charms.charm_points_available,
        locked.unlocked_stage
    );
    frames.push_str(&expected);
    capture("p01-expected.txt", frames.as_bytes())?;
    Ok(expected.replace('\n', ";"))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(DEADLINE, verdict::observe(journey(clients))).await {
        Ok(Ok(expected)) => evidence(&format!(
            "e2e_capture scenario=P01.client status=CAPTURED evidence={expected}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=P01.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=P01.client status=FAIL evidence=timeout"),
    }
}
