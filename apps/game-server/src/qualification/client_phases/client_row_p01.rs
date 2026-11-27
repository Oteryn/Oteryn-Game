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

const DEADLINE: Duration = Duration::from_secs(600);

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

fn kills(reply: &CharmProgressionReply, race: NonZeroU32) -> u32 {
    reply
        .bestiary
        .iter()
        .find(|entry| entry.race == race)
        .map_or(0, |entry| entry.kill_count)
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

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
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
    full_row_p01::prepare_rat_ground(&mut client, world).await?;
    let target = full_row_p01::place_rat(&mut client).await?;
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
    let target = full_row_p01::place_rat(&mut client).await?;
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
    full_row_p01::close_relog_fixture(&mut fresh).await?;
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
