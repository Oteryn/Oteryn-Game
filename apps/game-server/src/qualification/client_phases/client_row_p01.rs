//! P01 client replay capture: real CHARM_PROGRESSION replies for the initial QUERY, a refused
//! ASSIGN of a locked charm, and the QUERY after a credited hunt kill. The consumer is `apply` in
//! `apps/client/src/features/0067-bestiary-charm-progression/protocol.rs`.
use super::*;
use oteryn_dev_client::{DevClientSession, EntityKind, SessionEvent, WorldSpatialEntity};
use oteryn_protocol_oteryn::charm::{CharmAssignDisposition, CharmKind};
use oteryn_protocol_oteryn::charm_progression::{
    self as wire, CharmProgressionAction, CharmProgressionOutcome, CharmProgressionReply,
};
use std::fmt::Write as _;
use std::collections::BTreeSet;
use std::num::NonZeroU32;

const DEADLINE: Duration = Duration::from_secs(1_800);
const RAT: &str = "oteryn:creature.rat";
const TROLL: &str = "oteryn:creature.troll";
const TROLL_SPAWN: TilePos = TilePos {
    x: 32470,
    y: 32249,
    floor: -7,
};
const PROTECTED_LOGOUT: TilePos = TilePos {
    x: 32484,
    y: 32286,
    floor: -7,
};
const RAT_SPAWNS: [TilePos; 2] = [
    TilePos {
        x: 32491,
        y: 32169,
        floor: -7,
    },
    TilePos {
        x: 32503,
        y: 32174,
        floor: -7,
    },
];
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

pub(super) fn route_occupants(
    client: &DevClientSession,
) -> impl Iterator<Item = (TilePos, WorldSpatialEntity)> + '_ {
    client
        .world_entities()
        .into_iter()
        .flat_map(|entities| entities.others())
        .filter(|entity| {
            matches!(
                entity.kind,
                EntityKind::Player | EntityKind::Creature | EntityKind::Npc
            )
        })
        .filter_map(|entity| tile(entity.position).ok().map(|at| (at, *entity)))
}

pub(super) fn occupied_rat_route_tiles(client: &DevClientSession) -> BTreeSet<TilePos> {
    route_occupants(client).map(|(at, _)| at).collect()
}

pub(super) const VISIBILITY_TICK: Duration = Duration::from_millis(250);

pub(super) async fn walk_rat_route(
    client: &mut DevClientSession,
    world: &BundleWorld,
    path: &[StepDirection],
    reserved: &BTreeSet<TilePos>,
    mut observe: impl FnMut(&SessionEvent),
) -> TestResult {
    let mut remaining = VecDeque::from(path.to_vec());
    let center = tile(client.world_spatial().actor_position)?;
    let mut repairs = 0;
    while let Some(direction) = remaining.pop_front() {
        let before = tile(client.world_spatial().actor_position)?;
        let landing =
            destination(world, before, direction).ok_or("P01 route step has no landing")?;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
        loop {
            let occupants: Vec<_> = route_occupants(client)
                .filter(|(at, _)| *at == landing)
                .map(|(_, entity)| (entity.entity, entity.display_name))
                .collect();
            if !occupants.is_empty() {
                let occupied = occupied_rat_route_tiles(client);
                if let Some(path) = route_around_occupant(
                    before,
                    center,
                    landing,
                    &remaining,
                    |at| !occupied.contains(&at) && !reserved.contains(&at),
                    |at, step| destination(world, at, step),
                ) {
                    repairs += 1;
                    if repairs > 8 {
                        return Err("P01 exhausted eight occupied-tile route repairs".into());
                    }
                    evidence(&format!(
                        "P01_ROUTE detour from={before:?} landing={landing:?} occupants={occupants:?} steps={}",
                        path.len()
                    ));
                    remaining = path.into();
                    break;
                }
                if tokio::time::Instant::now() >= deadline {
                    return Err(format!(
                        "P01 landing {landing:?} remained occupied by {occupants:?}"
                    )
                    .into());
                }
                evidence(&format!(
                    "P01_ROUTE wait landing={landing:?} occupants={occupants:?}"
                ));
                client.service_liveness(VISIBILITY_TICK).await?;
                client.take_events().iter().for_each(&mut observe);
                continue;
            }

            let step = client.step(direction).await?;
            let observed = tile(client.world_spatial().actor_position)?;
            if step.status != CommandStatus::Accepted {
                return Err(format!("P01 STEP {before:?} to {landing:?}: {step:?}").into());
            }
            match step.disposition {
                StepDisposition::Moved if observed == landing => {
                    verdict::Harness::global().progress("p01-walk");
                    client.take_events().iter().for_each(&mut observe);
                    client
                        .service_liveness(Duration::from_millis(u64::from(step.step_wait_ms)))
                        .await?;
                    break;
                }
                StepDisposition::Blocked if observed == before => {
                    client.service_liveness(VISIBILITY_TICK).await?;
                    let occupants: Vec<_> = route_occupants(client)
                        .filter(|(at, _)| *at == landing)
                        .map(|(_, entity)| (entity.entity, entity.display_name))
                        .collect();
                    let map = client.session_mut().world_map_snapshot()
                        .and_then(|map| map.tiles.into_iter().find(|entry| {
                            tile(entry.position).is_ok_and(|at| at == landing)
                        }));
                    evidence(&format!(
                        "P01_ROUTE blocked from={before:?} landing={landing:?} command_id={} sequence={} occupants={occupants:?} map={map:?}",
                        step.command_id, step.result_server_sequence
                    ));
                    if tile(client.world_spatial().actor_position)? != before {
                        return Err("P01 position changed after a blocked STEP".into());
                    }
                    return Err(format!("P01 first STEP blocked command_id={} sequence={}", step.command_id, step.result_server_sequence).into());
                }
                _ => return Err(format!(
                    "P01 STEP from={before:?} expected={landing:?} observed={observed:?}: {step:?}"
                ).into()),
            }
        }
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
        full_row_p01::HuntEvidence::Principal(clients),
    )
    .await?;
    if kill.race != race || kill.raw_xp != 5 || kill.gained_xp != 5 || kill.fire_damage {
        return Err(format!("P01.client Rat XP or credit mismatch: {kill:?}").into());
    }
    Ok(())
}

async fn logout_from_protected_ground(
    client: &mut DevClientSession,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
) -> TestResult {
    let mut reserved = BTreeSet::new();
    // Occupants outside the current view can still be on their authored spawn cells.
    reserved.extend(spawns.sources.iter().flat_map(|source| {
        source.points.iter().map(|point| TilePos {
            x: point.x,
            y: point.y,
            floor: source.floor,
        })
    }));
    let protected = |at: TilePos| {
        at == PROTECTED_LOGOUT
            && world
                .base()
                .tile(at.x, at.y, at.floor)
                .is_some_and(|tile| tile.protection_zone())
    };
    let from = tile(client.world_spatial().actor_position)?;
    let occupied = occupied_rat_route_tiles(client);
    let path = route_avoiding(world, from, protected, |at| !occupied.contains(&at) && !reserved.contains(&at))
        .ok_or_else(|| format!("no walkable route from {from:?} to {PROTECTED_LOGOUT:?}"))?;
    walk_rat_route(client, world, &path, &reserved, |_| {}).await?;
    let reached = tile(client.world_spatial().actor_position)?;
    if !protected(reached) {
        return Err(format!("client logout ground is not protected: {reached:?}").into());
    }
    full_row_p01::logout_when_ready(client).await?;
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
    full_row_p01::prepare_unarmed(&mut client).await?;
    let races = full_row_p05::canonical_races()?;
    let race_id = |key| -> TestResult<NonZeroU32> {
        let index = races
            .iter()
            .position(|(definition, _)| definition.key() == key)
            .ok_or("creature is absent from the canonical Bestiary")?;
        Ok(NonZeroU32::new(u32::try_from(index)? + 1).ok_or("zero race index")?)
    };
    let race = race_id(RAT)?;
    let refusal_race = race_id(TROLL)?;
    let corpse_stage = full_row_p01::rat_corpse_stage()?;
    let mut frames = String::new();
    let before_mastery = send(
        &mut client,
        CharmProgressionAction::Query,
        "p01-pre-mastery.json",
        &mut frames,
    )
    .await?;
    if before_mastery.outcome != CharmProgressionOutcome::Query
        || before_mastery.charms.charm_points_available != 0
        || !matches!(before_mastery.bestiary.as_slice(), [entry]
            if entry.race == refusal_race && entry.kill_count == 249
                && entry.kill_thresholds == RAT_THRESHOLDS && !entry.is_complete())
    {
        return Err("P01.client requires one incomplete Troll and zero Charm Points".into());
    }
    let (_, troll) = full_row_p01::authored_target(
        &mut client,
        world,
        &metadata.spawns,
        TROLL,
        "Troll",
        Some(TROLL_SPAWN),
    )
    .await?;
    let troll_corpse = provisioning::item_reference(&provisioning::item_index()?, 5960)?.get();
    let troll_kill = full_row_p01::hunt_exact_creature(
        &mut client,
        troll,
        TROLL,
        refusal_race,
        troll_corpse,
        None,
        full_row_p01::HuntEvidence::Principal(clients),
    )
    .await?;
    if troll_kill.raw_xp != 20 || troll_kill.gained_xp != 20 || troll_kill.fire_damage {
        return Err(format!("unassigned Troll control mismatch: {troll_kill:?}").into());
    }
    let initial = send(
        &mut client,
        CharmProgressionAction::Query,
        "p01-initial.json",
        &mut frames,
    )
    .await?;
    let mut expected_mastery = before_mastery;
    expected_mastery.bestiary[0].kill_count = 250;
    expected_mastery.charms.charm_points_available = 5;
    if initial != expected_mastery {
        return Err(format!("live Troll mastery or +5 Charm Points missing: {initial:?}").into());
    }
    evidence("P01_CHECK client-earned-troll-mastery PASS kills=249->250 charm_points=0->5");
    if !client
        .selected_capabilities()
        .contains(&oteryn_protocol_oteryn::entity_names::CAPABILITY_WORLD_SPATIAL_ENTITIES_V2)
    {
        return Err("P01 Rat selection requires wire entity names".into());
    }
    let (_, target) = full_row_p01::authored_target(
        &mut client,
        world,
        &metadata.spawns,
        RAT,
        "Rat",
        Some(RAT_SPAWNS[0]),
    )
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
    let catalogue = crate::content::charm_source::CanonicalCharmCatalogue::embedded()?;
    let enflame_index = catalogue
        .catalogue()
        .definitions()
        .position(|entry| entry.key.as_str() == "oteryn:charm.enflame")
        .ok_or("canonical Enflame is missing")?;
    let charm = NonZeroU32::new(u32::try_from(enflame_index)? + 1).ok_or("zero charm index")?;
    let locked = before
        .charms
        .charms
        .iter()
        .find(|entry| entry.charm == charm)
        .ok_or("Enflame is missing")?;
    if locked.kind != CharmKind::Major
        || locked.unlocked_stage != 0
        || locked.assigned_race.is_some()
        || !locked.effect_active
        || locked.next_stage_cost != 400
    {
        return Err(format!("incorrect Enflame refusal baseline: {locked:?}").into());
    }
    let coins = client_row_e02::coin_refs()?;
    let carried = client_row_e02::backpack_coins(&client, coins)?;
    let bank = full_row_e06::bank_balance(clients, 1).await?;
    if bank == 0 && carried == [0; 3] {
        return Err("P01.client needs positive gold to detect a refused assignment debit".into());
    }
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
    let refusal_readback = send(
        &mut client,
        CharmProgressionAction::Query,
        "p01-refusal-readback.json",
        &mut frames,
    )
    .await?;
    if refusal_readback != expected_before
        || client_row_e02::backpack_coins(&client, coins)? != carried
        || full_row_e06::bank_balance(clients, 1).await? != bank
    {
        return Err("post-ASSIGN Query, carried coins, or bank balance changed".into());
    }
    evidence("P01_CHECK client-locked-assign PASS completed-race independent-query no-gold-debit");
    client.service_liveness(VISIBILITY_TICK).await?;
    let (_, target) = full_row_p01::authored_target(
        &mut client,
        world,
        &metadata.spawns,
        RAT,
        "Rat",
        Some(RAT_SPAWNS[1]),
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
    let relog_carried = client_row_e02::backpack_coins(&client, coins)?;
    if full_row_e06::bank_balance(clients, 1).await? != bank {
        return Err("P01.client bank changed before logout after the refusal".into());
    }
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
    if client_row_e02::backpack_coins(&fresh, coins)? != relog_carried
        || full_row_e06::bank_balance(clients, 1).await? != bank
    {
        return Err("P01.client refused assignment changed the wallet across relog".into());
    }
    evidence("P01_CHECK client-refusal-durable-wallet PASS carried-and-bank-unchanged");
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
