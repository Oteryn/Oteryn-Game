//! Q01: quest reward chests reached by real travel (FLOOR-E2E).
//!
//! Each row is one authored reward chest. The character starts wherever the durable world left
//! it, plans a route with `travel::Planner` (steps, doors, stairs, holes, ramps, ladders and a
//! carried rope), prints it, walks it with STEP and USE commands only, USEs the chest and must
//! then carry the exact reward item. A refused door (the level-35 door before Noble Armor for a
//! lower character), a refused floor change or a missing route is the row's FAIL with that
//! exact refusal; nothing is skipped or provisioned around.
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
pub(super) const CHESTS: [Chest; 5] = [
    chest("Q01a", "dark_armor", (33178, 31870, 12), 6028, 3383, 1, 0xf1),
    chest("Q01b", "double_hero", (33109, 31679, 13), 6032, 3039, 1, 0xf2),
    chest("Q01c", "double_hero", (33110, 31679, 13), 6033, 3093, 0, 0xf3),
    chest("Q01d", "noble_armor", (32451, 32048, 8), 6132, 3380, 1, 0xf4),
    chest("Q01e", "noble_armor", (32455, 32048, 8), 6133, 3385, 0, 0xf5),
];

static PLANNER: OnceLock<travel::Planner<'static>> = OnceLock::new();

fn planner(
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

fn carried(client: &oteryn_dev_client::DevClientSession, reference: std::num::NonZeroU32) -> usize {
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
    let mut client = connect_character(clients, chest.tag, chest.character).await?;
    let result = claim(&mut client, planner, chest, &label, ordinal, reference).await;
    let _ = client.session_mut().logout().await;
    result
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
    travel::travel_to(client, planner, label, chest.at, &travel::beside(chest.at)).await?;
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
