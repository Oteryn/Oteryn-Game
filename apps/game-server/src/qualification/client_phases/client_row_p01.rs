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

const DEADLINE: Duration = Duration::from_secs(1_800);
const RAT: &str = "oteryn:creature.rat";
const RAT_THRESHOLDS: [u32; 3] = [10, 100, 250];

pub(super) fn capture(name: &str, bytes: &[u8]) -> TestResult {
    use std::io::Write as _;

    let root = required("E2E_ALL_CAPTURE_DIR")?;
    let mut file = std::fs::File::create_new(Path::new(&root).join(name))?;
    file.write_all(bytes)?;
    Ok(())
}

pub(super) async fn send(
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

pub(super) fn occupied_rat_route_tiles(
    client: &DevClientSession,
) -> std::collections::BTreeSet<TilePos> {
    use oteryn_dev_client::EntityKind;
    client
        .world_entities()
        .into_iter()
        .flat_map(|entities| entities.iter())
        .filter(|entity| {
            matches!(
                entity.kind,
                EntityKind::Player | EntityKind::Creature | EntityKind::Npc
            )
        })
        .filter_map(|entity| tile(entity.position).ok())
        .collect()
}

pub(super) async fn walk_rat_route(
    client: &mut DevClientSession,
    world: &BundleWorld,
    path: &[StepDirection],
) -> TestResult {
    for &direction in path {
        let before = tile(client.world_spatial().actor_position)?;
        let expected =
            destination(world, before, direction).ok_or("Rat route step has no landing")?;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
        while client.world_entities().is_some_and(|entities| {
            entities.iter().any(|entity| {
                matches!(
                    entity.kind,
                    oteryn_dev_client::EntityKind::Player
                        | oteryn_dev_client::EntityKind::Creature
                        | oteryn_dev_client::EntityKind::Npc
                ) && tile(entity.position).is_ok_and(|at| at == expected)
            })
        }) {
            if tokio::time::Instant::now() >= deadline {
                let occupants: Vec<_> = client
                    .world_entities()
                    .into_iter()
                    .flat_map(|entities| entities.iter())
                    .filter(|entity| tile(entity.position).is_ok_and(|at| at == expected))
                    .collect();
                return Err(format!(
                    "Rat route landing {expected:?} remained occupied by {occupants:?}"
                )
                .into());
            }
            client.service_liveness(Duration::from_millis(50)).await?;
            client.take_events();
        }
        let step = client.step(direction).await?;
        if step.status != CommandStatus::Accepted
            || step.disposition != StepDisposition::Moved
            || tile(client.world_spatial().actor_position)? != expected
        {
            let occupants: Vec<_> = client
                .world_entities()
                .into_iter()
                .flat_map(|entities| entities.iter())
                .filter(|entity| tile(entity.position).is_ok_and(|at| at == expected))
                .collect();
            return Err(format!(
                "Rat route step {direction:?} from {before:?} expected {expected:?}: {step:?}; occupants={occupants:?}"
            )
            .into());
        }
        verdict::Harness::global().progress("rat-walk");
        client.take_events();
        client
            .service_liveness(Duration::from_millis(u64::from(step.step_wait_ms)))
            .await?;
    }
    Ok(())
}

async fn verify_rat_kill(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
    target: oteryn_dev_client::WorldSpatialEntity,
    race: NonZeroU32,
    corpse: client_hunt::CorpseStage,
) -> TestResult {
    let kill = full_row_p01::hunt_exact_creature(
        client,
        target,
        RAT,
        race,
        corpse.fresh_ref,
        Some(corpse),
        Some(clients),
    )
    .await?;
    if kill.race != race || kill.raw_xp != 5 {
        return Err(format!("P01.client Rat XP or credit mismatch: {kill:?}").into());
    }
    Ok(())
}

async fn logout_from_protected_ground(
    client: &mut DevClientSession,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
) -> TestResult {
    let mut occupied = occupied_rat_route_tiles(client);
    occupied.insert(TilePos {
        x: 32444,
        y: 32178,
        floor: -7,
    });
    // Occupants outside the current view can still be on their authored spawn cells.
    occupied.extend(spawns.sources.iter().flat_map(|source| {
        source.points.iter().map(|point| TilePos {
            x: point.x,
            y: point.y,
            floor: source.floor,
        })
    }));
    let protected = |at: TilePos| {
        world
            .base()
            .tile(at.x, at.y, at.floor)
            .is_some_and(|tile| tile.protection_zone())
    };
    let mut path = std::collections::VecDeque::new();
    let mut detours = 0;
    while !protected(tile(client.world_spatial().actor_position)?) {
        let current = tile(client.world_spatial().actor_position)?;
        if path.is_empty() {
            path = route_avoiding(world, current, protected, |at| !occupied.contains(&at))
                .ok_or_else(|| format!("no walkable route from {current:?} to protected ground"))?
                .into();
        }
        let direction = path.pop_front().ok_or("protected route has no next step")?;
        let landing =
            destination(world, current, direction).ok_or("protected route step has no landing")?;
        let visible = occupied_rat_route_tiles(client);
        if visible.contains(&landing) {
            detours += 1;
            evidence(&format!(
                "P01_CHECK client-logout-route occupied={landing:?} from={current:?} detour={detours}"
            ));
            if detours > 8 {
                return Err("protected logout route exceeded eight occupancy detours".into());
            }
            occupied.extend(visible);
            path.clear();
            continue;
        }
        walk_rat_route(client, world, &[direction]).await?;
    }
    let reached = tile(client.world_spatial().actor_position)?;
    if !protected(reached) {
        return Err(format!("client logout ground is not protected: {reached:?}").into());
    }
    let (outcome, detail, _, _) = logout_once(client).await?;
    if outcome != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("P01.client first logout refused at {reached:?}: {detail}").into());
    }
    evidence(&format!(
        "P01_CHECK client-protected-logout PASS position={reached:?} first-attempt"
    ));
    Ok(())
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
    let races = full_row_p05::canonical_races()?;
    let race_id = |key| -> TestResult<NonZeroU32> {
        let index = races
            .iter()
            .position(|(definition, _)| definition.key() == key)
            .ok_or("creature is absent from the canonical Bestiary")?;
        Ok(NonZeroU32::new(u32::try_from(index)? + 1).ok_or("zero race index")?)
    };
    let race = race_id(RAT)?;
    let refusal_race = race_id("oteryn:creature.troll")?;
    let corpse_stage = full_row_p01::rat_corpse_stage()?;
    let mut frames = String::new();
    let initial = send(
        &mut client,
        CharmProgressionAction::Query,
        "p01-initial.json",
        &mut frames,
    )
    .await?;
    if initial.outcome != CharmProgressionOutcome::Query
        || !matches!(initial.bestiary.as_slice(), [entry]
            if entry.race == refusal_race && entry.is_complete())
    {
        return Err("P01.client requires only a completed Troll and no Rat entry".into());
    }
    if !client
        .selected_capabilities()
        .contains(&oteryn_protocol_oteryn::entity_names::CAPABILITY_WORLD_SPATIAL_ENTITIES_V2)
    {
        return Err("P01 Rat selection requires wire entity names".into());
    }
    let (first_spawn, target) =
        full_row_p01::authored_target(&mut client, world, &metadata.spawns, RAT, "Rat", None, true)
            .await?;
    verify_rat_kill(&mut client, clients, target, race, corpse_stage).await?;
    let before = send(
        &mut client,
        CharmProgressionAction::Query,
        "p01-before.json",
        &mut frames,
    )
    .await?;
    let mut expected_before = initial;
    expected_before
        .bestiary
        .push(oteryn_protocol_oteryn::bestiary::BestiaryRaceProgress {
            race,
            kill_count: 1,
            kill_thresholds: RAT_THRESHOLDS,
        });
    expected_before.bestiary.sort_by_key(|entry| entry.race);
    if before != expected_before {
        return Err(format!(
            "first Rat kill changed unexpected Bestiary or Charm state: {before:?}"
        )
        .into());
    }
    evidence("P01_CHECK client-first-rat-delta PASS absent-to-one exact-views");
    let locked = before
        .charms
        .charms
        .iter()
        .find(|charm| charm.unlocked_stage == 0 && charm.next_stage_cost > 0)
        .ok_or("no locked charm to refuse")?;
    let charm = locked.charm;
    let refused = send(
        &mut client,
        CharmProgressionAction::Assign {
            charm,
            race: refusal_race,
        },
        "p01-refused.json",
        &mut frames,
    )
    .await?;
    if refused.outcome != CharmProgressionOutcome::Assign(CharmAssignDisposition::CharmLocked)
        || refused.fee_gold != 0
        || refused.charms != before.charms
        || refused.bestiary != before.bestiary
    {
        return Err(format!("locked ASSIGN reply {refused:?}").into());
    }
    evidence("P01_CHECK client-locked-assign PASS completed-race zero-fee unchanged");
    let (_, target) = full_row_p01::authored_target(
        &mut client,
        world,
        &metadata.spawns,
        RAT,
        "Rat",
        Some(first_spawn),
        true,
    )
    .await?;
    verify_rat_kill(&mut client, clients, target, race, corpse_stage).await?;
    let after = send(
        &mut client,
        CharmProgressionAction::Query,
        "p01-after.json",
        &mut frames,
    )
    .await?;
    let mut expected_after = expected_before;
    expected_after
        .bestiary
        .iter_mut()
        .find(|entry| entry.race == race)
        .ok_or("Rat absent from expected Bestiary")?
        .kill_count = 2;
    if after != expected_after {
        return Err(format!(
            "second Rat kill changed unexpected Bestiary or Charm state: {after:?}"
        )
        .into());
    }
    evidence("P01_CHECK client-second-rat-delta PASS one-to-two exact-views");
    let old_session = client.join_snapshot().game_session_id;
    logout_from_protected_ground(&mut client, world, &metadata.spawns).await?;
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
    if durable != expected_after {
        return Err(format!("P01.client relog lost the changed views: {durable:?}").into());
    }
    evidence("P01_CHECK client-relog PASS exact-durable-views");
    full_row_p01::close_relog_fixture(&mut fresh).await?;
    let session = uuid_text(old_session.as_bytes());
    let durable_session = uuid_text(fresh.join_snapshot().game_session_id.as_bytes());
    let expected = format!(
        "session={session}\ndurable_session={durable_session}\nrace={race}\nrefusal_race={refusal_race}\nkills_before=1\nkills_after=2\npoints_before={}\npoints_after={}\nlocked_charm={charm}\nlocked_stage={}\nrefusal=CharmLocked\n",
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
