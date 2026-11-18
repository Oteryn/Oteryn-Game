//! Q01: quest reward chests reached by real travel (FLOOR-E2E).
//!
//! Each row is one authored reward chest. The character starts wherever the durable world left
//! it, plans a route with `travel::Planner` (steps, doors, stairs, holes, ramps, ladders and a
//! fixture-carried rope, shovel or pick), prints it, walks it with STEP and USE commands only, USEs the chest and must
//! then carry the exact reward item. A refused door (the level-35 door before Noble Armor for a
//! lower character), a refused floor change or a missing route is the row's FAIL with that
//! exact refusal. A refused door is first banned and the route replanned; the row fails only
//! when no other way remains.
//!
//! Setup, before any travel: a fresh level-1 character walks to the level door before the Noble
//! Armor chests and must be refused by its USE; then the node-boot fixture (`provision.py
//! qualify <session> knight`, level 100) gives the character a qualifying build, and the row
//! logs out, reconnects and reads the level back. Where the route needs a ship, the character
//! walks to the captain and buys the passage through the NPC dialogue (`travel.rs`); the fixture
//! mints no gold, so a captain's fare is a FAIL naming the exact need.
//!
//! Before connecting, each row also plans from the start town and prints that route (or why none
//! was found), so the evidence shows the whole multi-floor way from Thais to the chest. Rows run
//! nearest-first: Noble Armor, then Dark Armor, then the Double Hero chests.
use super::*;
use oteryn_protocol_oteryn::world_object::UseDisposition;
use std::sync::OnceLock;

pub(super) struct Chest {
    pub(super) row: &'static str,
    quest: &'static str,
    at: TilePos,
    unique: u16,
    item: u32,
    /// The seam character that claims it; two chests of one quest use different characters.
    character: usize,
    tag: u8,
}

/// The level every Q01 character is raised to, and the lowest level any door on the way needs.
const DOOR_LEVEL: u32 = 35;

const fn chest(
    row: &'static str,
    quest: &'static str,
    (x, y, z): (u16, u16, i8),
    unique: u16,
    item: u32,
    character: usize,
    tag: u8,
) -> Chest {
    Chest {
        row,
        quest,
        at: TilePos { x, y, floor: -z },
        unique,
        item,
        character,
        tag,
    }
}

#[rustfmt::skip]
pub(super) const CHESTS: [Chest; 16] = [
    chest("Q01a", "noble_armor", (32451, 32048, 8), 6132, 3380, 1, 0xf1),
    chest("Q01b", "noble_armor", (32455, 32048, 8), 6133, 3385, 0, 0xf2),
    chest("Q01c", "dark_armor", (33178, 31870, 12), 6028, 3383, 1, 0xf3),
    chest("Q01d", "double_hero", (33109, 31679, 13), 6032, 3039, 1, 0xf4),
    chest("Q01e", "double_hero", (33110, 31679, 13), 6033, 3093, 0, 0xf5),
    chest("Q03a", "battle_axe", (32305, 32254, 9), 6180, 3266, 0, 0xeb),
    chest("Q03b", "deeper_fibula", (32256, 32500, 10), 6012, 3318, 1, 0xec),
    chest("Q03c", "devil_helmet", (32459, 32144, 15), 6139, 3029, 0, 0xed),
    chest("Q03d", "devil_helmet", (32465, 32148, 15), 6140, 3356, 1, 0xee),
    chest("Q03e", "devil_helmet", (32466, 32148, 15), 6141, 3269, 0, 0xef),
    chest("Q03f", "ghoul_room", (32500, 32175, 14), 6136, 3083, 1, 0x13),
    chest("Q03g", "ghoul_room", (32500, 32177, 14), 6137, 3093, 0, 0x14),
    chest("Q03h", "minotaur_leather", (32302, 32415, 7), 6149, 5878, 1, 0x19),
    chest("Q03i", "naginata", (32346, 32063, 12), 6135, 3314, 0, 0x1a),
    chest("Q03j", "silver_amulet", (32506, 32269, 9), 6154, 3054, 1, 0x1b),
    chest("Q03k", "six_rubies", (32371, 32262, 12), 6156, 3030, 0, 0x1c),
];

pub(super) fn row_status(result: &TestResult) -> &'static str {
    match result {
        Ok(()) => "PASS",
        Err(_) => "FAIL",
    }
}

static PLANNER: OnceLock<travel::Planner<'static>> = OnceLock::new();

pub(super) fn planner(
    world: &'static BundleWorld,
    bytes: &[u8],
) -> TestResult<&'static travel::Planner<'static>> {
    if PLANNER.get().is_none() {
        let loaded = travel::Planner::load(world, bytes)?;
        let _ = PLANNER.set(loaded);
    }
    PLANNER
        .get()
        .ok_or_else(|| "travel planner not initialised".into())
}

pub(super) fn carried(
    client: &oteryn_dev_client::DevClientSession,
    reference: std::num::NonZeroU32,
) -> usize {
    client.inventory().map_or(0, |inventory| {
        inventory
            .entries
            .iter()
            .filter(|entry| entry.item_definition_ref == reference)
            .count()
    })
}

async fn journey(clients: &SeamClients<'_>, chest: &Chest) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes.clone(),
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let planner = planner(world, &bytes)?;
    let label = format!("{}:{}", chest.row, chest.quest);
    let &(authored, ordinal) = planner
        .uniques
        .get(&chest.unique)
        .ok_or_else(|| format!("{label} unique {} is not in the world bundle", chest.unique))?;
    if authored != chest.at {
        return Err(format!(
            "{label} unique {} is at {authored:?}, not the chest tile {:?}",
            chest.unique, chest.at
        )
        .into());
    }
    let reference = provisioning::item_reference(&provisioning::item_index()?, chest.item)?;
    from_town(planner, chest, &label)?;
    let mut client = connect_character(clients, chest.tag, chest.character).await?;
    let result = match setup(clients, planner, chest, &label, &mut client).await {
        Ok(()) => claim(&mut client, planner, chest, &label, ordinal, reference).await,
        Err(error) => Err(error),
    };
    let logout = final_logout(&mut client).await;
    result.and(logout)
}

/// The character's level, read from the GeneralStats tab.
pub(super) async fn level(client: &mut oteryn_dev_client::DevClientSession) -> TestResult<u32> {
    use oteryn_protocol_oteryn::cyclopedia_character as wire;
    let reply = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_CYCLOPEDIA_CHARACTER_V1,
            wire::COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB,
            &[wire::CharacterTab::GeneralStats as u8, 0, 0, 1],
        )
        .await?;
    if reply.status != CommandStatus::Accepted
        || reply.payload.first() != Some(&(wire::CharacterTab::GeneralStats as u8))
    {
        return Err(format!("GeneralStats query refused: {:?}", reply.status).into());
    }
    let bytes = reply
        .payload
        .get(9..13)
        .ok_or("GeneralStats reply too short for a level")?;
    Ok(u32::from_le_bytes(bytes.try_into()?))
}

/// A below-level character USEs the level door before the chest and must be refused. A walk
/// that cannot reach the door is recorded, not hidden; an opened door is the row's FAIL.
async fn below_level_check(
    client: &mut oteryn_dev_client::DevClientSession,
    planner: &travel::Planner<'_>,
    chest: &Chest,
    label: &str,
    level: u32,
) -> TestResult<TilePos> {
    let action = u16::try_from(1000 + DOOR_LEVEL)?;
    let door = *planner
        .level_doors(chest.at, 12, action)
        .first()
        .ok_or_else(|| {
            format!("{label} no level-{DOOR_LEVEL} door within 12 tiles of the chest")
        })?;
    let mut passages = travel::Passages::default();
    travel::travel_to(
        client,
        planner,
        &format!("{label}:below-level"),
        door,
        &travel::beside(door),
        &mut passages,
    )
    .await?;
    let reply = travel::use_map_object(client, &planner.door_placement(door)?).await?;
    evidence(&format!(
        "Q01_CHECK {label} below_level level={level} door={},{},{} action={action} use={reply:?}",
        door.x, door.y, -door.floor
    ));
    validate_level_refusal(label, level, reply)?;
    Ok(door)
}

fn validate_level_refusal(label: &str, level: u32, reply: UseDisposition) -> TestResult {
    if level >= DOOR_LEVEL || reply != UseDisposition::Locked {
        return Err(format!(
            "{label} level {level} door-{DOOR_LEVEL} probe returned {reply:?}; expected a below-level Locked reply"
        ).into());
    }
    Ok(())
}

#[test]
fn level_door_requires_locked_reply_from_below_level_character() {
    assert!(validate_level_refusal("test", 1, UseDisposition::Locked).is_ok());
    assert!(validate_level_refusal("test", 1, UseDisposition::TooFar).is_err());
    assert!(validate_level_refusal("test", DOOR_LEVEL, UseDisposition::Locked).is_err());
}

/// Verify the below-level door refusal on a low-level participant and the
/// positive control on the character that meets the authored threshold.
async fn setup(
    clients: &SeamClients<'_>,
    planner: &travel::Planner<'_>,
    chest: &Chest,
    label: &str,
    client: &mut oteryn_dev_client::DevClientSession,
) -> TestResult {
    let before = level(client).await?;
    let probed_door = if chest.quest == "noble_armor" {
        if before < DOOR_LEVEL {
            below_level_check(client, planner, chest, label, before).await?;
            return Err(Box::new(verdict::MissingServerRoute(concat!("source:", file!(), ":", line!()))));
        }
        let mut peer = full_e2ef5::with_observer_liveness(client, connect_character(clients, chest.tag, 1 - chest.character)).await?;
        let probe = full_e2ef5::with_observer_liveness(client, async {
            let peer_level = level(&mut peer).await?;
            below_level_check(&mut peer, planner, chest, label, peer_level).await
        }).await;
        let release = full_e2ef5::with_observer_liveness(client, full_e2ef5::logout(&mut peer)).await;
        let door = probe?;
        release?;
        Some(door)
    } else {
        None
    };
    if before < DOOR_LEVEL {
        return Err(Box::new(verdict::MissingServerRoute(concat!("source:", file!(), ":", line!()))));
    }
    if let Some(door) = probed_door {
        let mut passages = travel::Passages::default();
        travel::travel_to(
            client,
            planner,
            &format!("{label}:at-level"),
            door,
            &travel::beside(door),
            &mut passages,
        )
        .await?;
        let placement = planner.door_placement(door)?;
        let ordinal = *placement.last().ok_or("empty level-door placement")?;
        let mut revision = client.world_object_overlay().iter()
            .find(|entry| entry.placement == placement).map_or(0, |entry| entry.revision);
        if let Some(map) = client.session_mut().world_map_snapshot() {
            revision = door_revision(&map.tiles, door, ordinal).unwrap_or(revision);
        }
        for event in client.take_events() {
            if let oteryn_dev_client::SessionEvent::WorldMap(delta) = event {
                revision = door_revision(&delta.value.tiles, door, ordinal).unwrap_or(revision);
            }
        }
        let reply = use_door_with_cooldown(client, &placement, revision).await?;
        if reply != UseDisposition::Committed {
            return Err(format!(
                "{label} level {before} same-door positive control returned {reply:?}"
            )
            .into());
        }
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        let new_revision = loop {
            let updated = client.take_events().into_iter().filter_map(|event| {
                if let oteryn_dev_client::SessionEvent::WorldMap(delta) = event {
                    door_revision(&delta.value.tiles, door, ordinal)
                } else { None }
            }).max().or_else(|| client.world_object_overlay().iter()
                .find(|entry| entry.placement == placement).map(|entry| entry.revision));
            if let Some(updated) = revised_door(revision, updated) { break updated; }
            if tokio::time::Instant::now() >= deadline {
                return Err(format!("{label} opened level door without publishing its new revision").into());
            }
            client.service_liveness(Duration::from_millis(50)).await?;
        };
        let restored = use_door_with_cooldown(client, &placement, new_revision).await?;
        if restored != UseDisposition::Committed {
            return Err(format!(
                "{label} same-door positive control could not restore door: {restored:?}"
            )
            .into());
        }
    }
    Ok(())
}

/// Evidence only: the route from town with the fixture starter rope.
/// Tool legs name their inventory requirement.
fn from_town(planner: &travel::Planner<'_>, chest: &Chest, label: &str) -> TestResult {
    verdict::Harness::global().progress("walk");
    let goal = planner.reward_goal(chest.at);
    let planned = planner.plan(TOWN, chest.at, &goal, true, &travel::Passages::default());
    verdict::Harness::global().progress("walk");
    match q03a_route(chest.row, planned)? {
        Ok(route) => {
            evidence(&format!(
                "TRAVEL_ROUTE {label} from-town {TOWN:?} actions={}",
                route.len()
            ));
            for line in planner.describe(TOWN, &route) {
                evidence(&format!("TRAVEL_ROUTE {label} from-town {line}"));
            }
        }
        Err(reason) => {
            evidence(&format!(
                "TRAVEL_ROUTE {label} from-town {TOWN:?} unreached: {reason}"
            ));
            match chest.row {
                "Q01a" | "Q01b" => {
                    let entries = [
                        (32558, 31960, 7),
                        (32546, 31969, 8),
                        (32411, 32008, 7),
                        (32441, 32016, 8),
                        (32453, 32039, 9),
                        (32453, 32040, 8),
                        (32448, 32042, 8),
                    ]
                    .map(|(x, y, z)| planner.tile_evidence(TilePos { x, y, floor: -z }));
                    for entry in &entries {
                        evidence(&format!("TRAVEL_MAP_GAP {label} {entry}"));
                    }
                    return Err(format!("MISSING served bundle has no supported route into noble_armor; confirmed full-bundle entrance waypoints: [{}]; {reason}", entries.join("; ")).into());
                },
                "Q01c" => {
                    let entries = [8, 11, 12].map(|z| planner.tile_evidence(TilePos { x:33178, y:31870, floor:-z }));
                    return Err(format!("MISSING served bundle has no supported route into dark_armor; target column: [{}]; {reason}", entries.join("; ")).into());
                },
                _ => {}
            }
        },
    }
    for line in planner.offers_near(chest.at, 200) {
        evidence(&format!("TRAVEL_OFFERS {label} {line}"));
    }
    Ok(())
}

/// Walk to the chest, USE it and require the reward item in the backpack.
async fn claim(
    client: &mut oteryn_dev_client::DevClientSession,
    planner: &travel::Planner<'_>,
    chest: &Chest,
    label: &str,
    ordinal: u8,
    reference: std::num::NonZeroU32,
) -> TestResult {
    let mut passages = travel::Passages::default();
    travel::travel_to(
        client,
        planner,
        label,
        chest.at,
        &planner.reward_goal(chest.at),
        &mut passages,
    )
    .await?;
    let before = carried(client, reference);
    let placement = planner.placement(chest.at, ordinal)?;
    let reply = travel::use_map_object(client, &placement).await?;
    evidence(&format!(
        "Q01_CHECK {label} chest={:?} use={reply:?} carried_before={before}",
        chest.at
    ));
    if reply != UseDisposition::Committed {
        return Err(format!("{label} chest USE refused: {reply:?}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while carried(client, reference) <= before {
        verdict::Harness::global().progress("quest reward inventory");
        if tokio::time::Instant::now() >= deadline {
            return Err(format!(
                "{label} chest USE committed but item {} never reached the backpack",
                chest.item
            )
            .into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
        client.take_events();
    }
    evidence(&format!(
        "Q01_CHECK {label} item={} PASS carried={}",
        chest.item,
        carried(client, reference)
    ));
    Ok(())
}

pub(super) async fn run_packet(clients: &SeamClients<'_>, chest: &Chest) {
    let started = std::time::Instant::now();
    let result: TestResult = verdict::observe(journey(clients, chest)).await;
    if let Err(error) = &result {
        evidence(&format!("Q01_GAP {} {error}", chest.row));
    }
    evidence(&format!(
        "e2e_all scenario={}.full status={} seconds={:.3}",
        chest.row,
        row_status(&result),
        started.elapsed().as_secs_f64()
    ));
}

pub(super) fn q03a_route<T>(row: &str, plan: Result<T, String>) -> TestResult<Result<T, String>> {
    match plan {
        Err(reason) if row == "Q03a" && reason.starts_with("every reachable tile of the search box exhausted") => {
            Err(format!("{row} from-town route: {reason}").into())
        }
        Err(reason) if !reason.starts_with("every reachable tile of the search box exhausted") => {
            Err(format!("{row} from-town route: {reason}").into())
        }
        outcome => Ok(outcome),
    }
}

#[test]
fn q03a_admission_depends_on_current_route() {
    let reachable = q03a_route("Q03a", Ok(()));
    assert!(matches!(reachable, Ok(Ok(()))));
    let unreachable = q03a_route::<()>("Q03a", Err("every reachable tile of the search box exhausted (10 settled)".into())).map(|_| ());
    assert_eq!(row_status(&unreachable), "FAIL");
    assert!(unreachable.is_err_and(|error| error.to_string().contains("10 settled")));
}

#[test]
fn round2_planner_limit_is_not_missing_content() {
    let limited = q03a_route::<()>("Q03a", Err("search limit of 20 tiles hit after 20 settled".into()))
        .and_then(|planned| Ok(planned?));
    assert_eq!(row_status(&limited), "FAIL");
}

fn revised_door(previous: u64, observed: Option<u64>) -> Option<u64> {
    observed.filter(|revision| *revision > previous)
}

#[test]
fn round2_door_restore_waits_for_the_new_revision() {
    assert_eq!(revised_door(1, None), None);
    assert_eq!(revised_door(1, Some(1)), None);
    assert_eq!(revised_door(1, Some(2)), Some(2));
}

fn door_revision(tiles: &[oteryn_protocol_oteryn::world_map::MapTile], door: TilePos, ordinal: u8) -> Option<u64> {
    use oteryn_protocol_oteryn::world_map::MapOrigin;
    let tile = tiles.iter().find(|tile| (tile.position.x, tile.position.y, tile.position.floor)
        == (i32::from(door.x), i32::from(door.y), i16::from(door.floor)))?;
    tile.items.iter().find_map(|item| match item.origin {
        MapOrigin::BaseOrdinal { ordinal: actual, object_revision } if actual == ordinal => Some(object_revision),
        _ => None,
    })
}

#[cfg(test)]
struct FixtureProcess(std::process::Child);

#[cfg(test)]
impl Drop for FixtureProcess {
    fn drop(&mut self) {
        drop(self.0.stdin.take());
        if !matches!(self.0.try_wait(), Ok(Some(_))) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}

#[test]
fn round2_cancelled_provisioning_reaps_the_child() -> TestResult {
    let process = FixtureProcess(std::process::Command::new("sleep").arg("30").spawn()?);
    let pid = process.0.id().to_string();
    drop(process);
    let proc = std::path::Path::new("/proc").join(&pid);
    let still_running = proc.exists();
    if still_running {
        std::process::Command::new("kill").args(["-KILL", &pid]).status()?;
    }
    assert!(!still_running);
    Ok(())
}

#[test]
fn round3_from_town_stops_on_planner_budget() {
    for row in ["Q01a", "Q01b", "Q01c", "Q03a"] {
        let reason = "search limit of 20 tiles hit after 20 settled";
        let result = q03a_route::<()>(row, Err(reason.into()));
        assert!(result.is_err_and(|error| error.to_string() == format!("{row} from-town route: {reason}")));
    }
}

async fn use_door_with_cooldown(
    client: &mut oteryn_dev_client::DevClientSession,
    placement: &[u8],
    revision: u64,
) -> TestResult<UseDisposition> {
    retry_door_use(
        client,
        revision,
        async |client, revision| Ok(client.use_object(placement, revision).await?.disposition),
        async |client| {
            client.service_liveness(Duration::from_millis(200)).await?;
            Ok(())
        },
    ).await
}

async fn retry_door_use<C>(
    client: &mut C,
    revision: u64,
    mut send: impl AsyncFnMut(&mut C, u64) -> TestResult<UseDisposition>,
    mut service: impl AsyncFnMut(&mut C) -> TestResult,
) -> TestResult<UseDisposition> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    loop {
        let disposition = send(client, revision).await?;
        if disposition != UseDisposition::Exhausted || tokio::time::Instant::now() >= deadline {
            return Ok(disposition);
        }
        service(client).await?;
    }
}

#[tokio::test]
async fn round3_door_use_retry() -> TestResult {
    let mut state = (0, Vec::new());
    let opened = retry_door_use(
        &mut state, 4,
        async |state, revision| {
            state.1.push(revision);
            Ok(UseDisposition::Committed)
        },
        async |_| Err("unexpected liveness wait when opening the door".into()),
    ).await?;
    assert_eq!(opened, UseDisposition::Committed);
    let restored = retry_door_use(
        &mut state, 5,
        async |state, revision| {
            state.1.push(revision);
            Ok(if state.0 == 0 { UseDisposition::Exhausted } else { UseDisposition::Committed })
        },
        async |state| { state.0 += 1; Ok(()) },
    ).await?;
    assert_eq!(restored, UseDisposition::Committed);
    assert_eq!(state, (1, vec![4, 5, 5]));
    Ok(())
}
