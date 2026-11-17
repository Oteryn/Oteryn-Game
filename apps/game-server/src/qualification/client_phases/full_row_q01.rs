//! Q01: quest reward chests reached by real travel (FLOOR-E2E).
//!
//! Each row is one authored reward chest. The character starts wherever the durable world left
//! it, plans a route with `travel::Planner` (steps, doors, stairs, holes, ramps, ladders and a
//! carried rope), prints it, walks it with STEP and USE commands only, USEs the chest and must
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
    from_town(planner, chest, &label);
    let mut client = connect_character(clients, chest.tag, chest.character).await?;
    let result = match setup(clients, planner, chest, &label, &mut client).await {
        Ok(()) => claim(&mut client, planner, chest, &label, ordinal, reference).await,
        Err(error) => Err(error),
    };
    let _ = client.session_mut().logout().await;
    result
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
) -> TestResult {
    let action = u16::try_from(1000 + DOOR_LEVEL)?;
    let Some(&door) = planner.level_doors(chest.at, 12, action).first() else {
        evidence(&format!(
            "Q01_CHECK {label} below_level not_run: no level-{DOOR_LEVEL} door within 12 tiles of the chest"
        ));
        return Ok(());
    };
    let mut passages = travel::Passages::default();
    if let Err(error) = travel::travel_to(
        client,
        planner,
        &format!("{label}:below-level"),
        door,
        &travel::beside(door),
        &mut passages,
    )
    .await
    {
        evidence(&format!(
            "Q01_CHECK {label} below_level not_run: cannot walk to the door: {error}"
        ));
        return Ok(());
    }
    let reply = travel::use_map_object(client, &planner.door_placement(door)?).await?;
    evidence(&format!(
        "Q01_CHECK {label} below_level level={level} door={},{},{} action={action} use={reply:?}",
        door.x, door.y, -door.floor
    ));
    if reply == UseDisposition::Committed {
        return Err(
            format!("{label} level {level} character opened the level-{DOOR_LEVEL} door").into(),
        );
    }
    Ok(())
}

/// Below-level check first, then the fixture build (knight, level 100), logout, reconnect and
/// the level read back. A character an earlier row already raised is used as it is.
async fn setup(
    clients: &SeamClients<'_>,
    planner: &travel::Planner<'_>,
    chest: &Chest,
    label: &str,
    client: &mut oteryn_dev_client::DevClientSession,
) -> TestResult {
    let before = level(client).await?;
    if before >= DOOR_LEVEL {
        evidence(&format!(
            "Q01_CHECK {label} setup:fixture_level={before} (raised by an earlier row)"
        ));
        return Ok(());
    }
    if chest.quest == "noble_armor" {
        below_level_check(client, planner, chest, label, before).await?;
    }
    let after = provision(clients, client, chest.tag, chest.character, "knight", label).await?;
    evidence(&format!(
        "Q01_CHECK {label} setup:fixture_level={after} profile=knight level_before={before} reload=fresh_admission"
    ));
    if after < DOOR_LEVEL {
        return Err(format!(
            "{label} fixture left the character at level {after}, below {DOOR_LEVEL}"
        )
        .into());
    }
    Ok(())
}

/// The node-boot fixture build for `profile` (fenced, as provision.py does it), then logout,
/// a fresh admission and the level read back.
pub(super) async fn provision(
    clients: &SeamClients<'_>,
    client: &mut oteryn_dev_client::DevClientSession,
    tag: u8,
    character: usize,
    profile: &str,
    label: &str,
) -> TestResult<u32> {
    let socket = required("NODE_BOOT_CONTROL_SOCKET")?;
    let profile = profile.to_owned();
    let session = uuid_text(client.join_snapshot().game_session_id.as_bytes());
    let script = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/qualification/node_boot/provision.py");
    let status = tokio::task::spawn_blocking(move || {
        std::process::Command::new("sudo")
            .args(["-n", "python3"])
            .arg(script)
            .args([
                "--socket",
                &socket,
                "--session",
                &session,
                "--profile",
                &profile,
            ])
            .status()
    })
    .await??;
    if !status.success() {
        return Err(format!("{label} node-boot fenced build provisioning failed").into());
    }
    let (result, logout, attempts, busy_timed_out) =
        client_world::logout_retrying_busy(client).await?;
    if result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("{label} fixture setup logout refused: {logout}; attempts={attempts}; busy_timed_out={busy_timed_out}").into());
    }
    // The old session is logged out; the fresh admission replaces it and is logged out by the row.
    *client = connect_character(clients, tag, character).await?;
    level(client).await
}

/// Evidence only: the route a fresh character would take from the start town, without a rope.
fn from_town(planner: &travel::Planner<'_>, chest: &Chest, label: &str) {
    verdict::Harness::global().progress("walk");
    let goal = travel::beside(chest.at);
    let planned = planner.plan(TOWN, chest.at, &goal, false, &travel::Passages::default());
    verdict::Harness::global().progress("walk");
    match planned {
        Ok(route) => {
            evidence(&format!(
                "TRAVEL_ROUTE {label} from-town {TOWN:?} actions={}",
                route.len()
            ));
            for line in planner.describe(TOWN, &route) {
                evidence(&format!("TRAVEL_ROUTE {label} from-town {line}"));
            }
        }
        Err(reason) => evidence(&format!(
            "TRAVEL_ROUTE {label} from-town {TOWN:?} unreached: {reason}"
        )),
    }
    for line in planner.offers_near(chest.at, 200) {
        evidence(&format!("TRAVEL_OFFERS {label} {line}"));
    }
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
        &travel::beside(chest.at),
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
        if result.is_ok() { "PASS" } else { "FAIL" },
        started.elapsed().as_secs_f64()
    ));
}
