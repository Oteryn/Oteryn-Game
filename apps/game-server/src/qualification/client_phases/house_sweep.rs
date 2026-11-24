//! Opt-in house catalogue sweep. Select with NODE_BOOT_E2E_ONLY=house_sweep.
//! OTERYN_E2E_HOUSE_SWEEP_BATCH=start:len slices the key-sorted authored catalogue.
//!
//! Oracles: content/houses contains the source ID, coordinates and rent. Compiled membership
//! and door markers come from crates/gameplay/world/src/map/facts.rs:715 and :738.
//! The live auction projection is owned by crates/gameplay/durability/src/durability/
//! house_management/query.rs:135. Settlement and first rent are owned by
//! crates/gameplay/durability/src/durability/house_ownership/settlement.rs:130.
//! Ownership is observed via the same live query used to observe the bid. The fixture never
//! inserts ownership, bid or rent results. Once the real bid is visible, the fixture
//! seeds an elapsed deadline for settlement-only coverage after checking the original
//! seven-day deadline and proxy price (docs/reference/tibia-manual/houses.md:38).

use super::*;
use oteryn_dev_client::{
    DevClientSession,
    client_runtime::{house as panel, server_mechanic::CommandStatus},
};
use oteryn_protocol_oteryn::house as wire;
use serde::Deserialize;

const BATCH_VARIABLE: &str = "OTERYN_E2E_HOUSE_SWEEP_BATCH";
const BATCH_DEFAULT: (usize, usize) = (0, 3);
const STEPS: [&str; 6] = ["world", "auction", "bid", "transfer", "doors", "rent"];

#[derive(Deserialize)]
struct Key {
    key: String,
}

#[derive(Deserialize)]
struct Source {
    engine_house: EngineHouse,
}

#[derive(Deserialize)]
struct EngineHouse {
    house_id: u32,
}

#[derive(Deserialize)]
struct House {
    identity: Key,
    town: Key,
    kind: String,
    provenance: Option<Source>,
    doors: Vec<(u16, u16, i8)>,
    tiles: Vec<(u16, u16, i8)>,
    rent_gold: u64,
    entrance: Entrance,
}

#[derive(Deserialize)]
struct Entrance {
    x: u16,
    y: u16,
    z: i8,
}

#[derive(Deserialize)]
struct Shard {
    houses: Vec<House>,
}

fn house_batch(value: Option<&str>) -> TestResult<(usize, usize)> {
    let Some(text) = value else {
        return Ok(BATCH_DEFAULT);
    };
    let number = |part: &str| {
        if part.bytes().all(|byte| byte.is_ascii_digit()) {
            part.parse::<usize>().ok()
        } else {
            None
        }
    };
    text.split_once(':')
        .and_then(|(start, len)| Some((number(start)?, number(len)?)))
        .filter(|&(_, len)| len > 0)
        .ok_or_else(|| {
            format!("invalid {BATCH_VARIABLE}={text:?}: expected start:len with len > 0").into()
        })
}

fn houses(directory: &Path) -> TestResult<Vec<House>> {
    let mut houses = Vec::new();
    for file in std::fs::read_dir(directory)? {
        let file = file?;
        if file
            .file_name()
            .to_str()
            .is_some_and(|name| name.starts_with("houses-") && name.ends_with(".json"))
        {
            let shard: Shard = serde_json::from_reader(std::fs::File::open(file.path())?)?;
            houses.extend(shard.houses);
        }
    }
    houses.sort_unstable_by(|left, right| left.identity.key.cmp(&right.identity.key));
    if houses.is_empty()
        || houses
            .windows(2)
            .any(|pair| pair[0].identity.key == pair[1].identity.key)
    {
        return Err("house catalogue is empty or contains duplicate keys".into());
    }
    Ok(houses)
}

fn directory() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/houses")
}

#[derive(Default)]
struct Tally {
    pass: usize,
    fail: usize,
}

struct Rows<'a> {
    key: &'a str,
    tally: &'a mut Tally,
    emitted: usize,
    lap: std::time::Instant,
}

impl Rows<'_> {
    fn emit(&mut self, result: TestResult<String>, cause: &str) -> bool {
        let step = STEPS[self.emitted];
        self.emitted += 1;
        let seconds = self.lap.elapsed().as_secs_f64();
        self.lap = std::time::Instant::now();
        let (status, detail, cause) = match result {
            Ok(detail) => {
                self.tally.pass += 1;
                ("PASS", detail, String::new())
            }
            Err(error) => {
                self.tally.fail += 1;
                ("FAIL", error.to_string(), format!(" cause={cause}"))
            }
        };
        let detail: String = detail
            .chars()
            .take(400)
            .map(|c| {
                if c.is_ascii_alphanumeric() || "._:-,/".contains(c) {
                    c
                } else {
                    '-'
                }
            })
            .collect();
        evidence(&format!(
            "e2e_all scenario=HS.{}.{step} status={status} seconds={seconds:.3} mode=seeded evidence={detail}{cause}",
            self.key
        ));
        status == "PASS"
    }

    fn block_rest(&mut self) {
        let root = STEPS[self.emitted - 1];
        while self.emitted < STEPS.len() {
            self.emit(Err(format!("blocked-by-{root}").into()), "blocked");
        }
    }
}

fn position((x, y, floor): (u16, u16, i8)) -> TilePos {
    TilePos {
        x,
        y,
        floor: -floor,
    }
}

struct PinnedHouseTile {
    walkable: bool,
    door: bool,
    number: Option<u32>,
}

type PinnedHouses =
    std::collections::BTreeMap<u32, std::collections::BTreeMap<TilePos, PinnedHouseTile>>;

fn pinned_houses(
    bytes: &[u8],
    generation: &QualificationWorld,
    houses: &[House],
) -> TestResult<PinnedHouses> {
    let ids: std::collections::BTreeSet<_> = houses
        .iter()
        .filter_map(|house| {
            house
                .provenance
                .as_ref()
                .map(|source| source.engine_house.house_id)
        })
        .collect();
    let mut pinned = PinnedHouses::new();
    let mut door_palette = Vec::new();
    oteryn_world_bundle::bundle::visit(bytes, |manifest, sector| {
        if door_palette.is_empty() {
            door_palette = manifest
                .palette
                .iter()
                .map(|entry| {
                    crate::map::boot::native_item_definition(Some(&generation.native), &entry.key)
                        .is_some_and(|item| item.door)
                })
                .collect();
        }
        for tile in sector
            .tiles
            .into_iter()
            .filter(|tile| ids.contains(&tile.house))
        {
            let pos = TilePos {
                x: tile.x,
                y: tile.y,
                floor: sector.floor,
            };
            let top = tile.items.iter().filter(|item| item.depth == 0);
            let ground = generation
                .bundle
                .base()
                .tile(pos.x, pos.y, pos.floor)
                .is_some_and(|tile| tile.walkable());
            let walkable = ground
                && !top
                    .clone()
                    .any(|item| generation.bundle.facts().blocks(item.palette));
            let mut doors =
                top.filter(|item| door_palette.get(item.palette as usize) == Some(&true));
            let door = doors.clone().next().is_some();
            let number = doors
                .find_map(|item| item.attrs.door.filter(|&number| number != 0))
                .map(u32::from);
            pinned.entry(tile.house).or_default().insert(
                pos,
                PinnedHouseTile {
                    walkable,
                    door,
                    number,
                },
            );
        }
        Ok(())
    })?;
    Ok(pinned)
}

fn compiled_house(world: &BundleWorld, house: &House, pinned: &PinnedHouses) -> TestResult {
    let id = house
        .provenance
        .as_ref()
        .map(|source| source.engine_house.house_id);
    let source = id.and_then(|id| pinned.get(&id));
    let facts = world.facts();
    let entrance = position((house.entrance.x, house.entrance.y, house.entrance.z));
    let entry_doors: Vec<_> = source
        .into_iter()
        .flatten()
        .filter(|(pos, tile)| {
            tile.door
                && pos.floor == entrance.floor
                && u32::from(pos.x.abs_diff(entrance.x)) + u32::from(pos.y.abs_diff(entrance.y))
                    == 1
        })
        .collect();
    let has_numbered_entry = entry_doors.iter().any(|(_, tile)| tile.number.is_some());
    let has_walkable_ingress = source.into_iter().flatten().any(|(pos, tile)| {
        tile.walkable
            && pos.floor == entrance.floor
            && u32::from(pos.x.abs_diff(entrance.x)) + u32::from(pos.y.abs_diff(entrance.y)) == 1
    });
    let required = |pos: TilePos| {
        entry_doors.iter().any(|(door, tile)| {
            **door == pos
                && (tile.number.is_some()
                    || (!has_numbered_entry && !has_walkable_ingress && entry_doors.len() == 1))
        })
    };
    let catalogue_tiles = house
        .tiles
        .iter()
        .filter(|&&at| {
            let at = position(at);
            facts.house_id(at) != id
                && !source
                    .and_then(|tiles| tiles.get(&at))
                    .is_some_and(|tile| tile.walkable)
        })
        .count();
    let catalogue_doors = house
        .doors
        .iter()
        .filter(|&&at| {
            let at = position(at);
            id.and_then(|id| facts.house_door(at, id)).is_none() && !required(at)
        })
        .count();
    evidence(&format!(
        "house_sweep INFO house={} catalogue_only_tiles={catalogue_tiles} catalogue_only_doors={catalogue_doors}",
        house.identity.key
    ));
    let Some(id) = id else {
        return Ok(());
    };
    for (&at, tile) in source.into_iter().flatten() {
        if tile.walkable && facts.house_id(at) != Some(id) {
            return Err(format!("compiled house {id} missing walkable tile {at:?}").into());
        }
    }
    for &(at, tile) in &entry_doors {
        let at = *at;
        if required(at) && (tile.number.is_none() || facts.house_door(at, id) != tile.number) {
            return Err(format!("compiled house {id} missing numbered entry door {at:?}").into());
        }
    }
    Ok(())
}

// Use the same catalogue command and page decoder as the shipped house panel
// (crates/client-ui/src/features/0049-house/protocol.rs:26). The separate auction-order
// extension is optional and the shipped client does not negotiate it.
async fn query(
    client: &mut DevClientSession,
    world: [u8; 16],
    house: &House,
) -> TestResult<panel::Row> {
    let mut next = None::<String>;
    // The owner limits queries to 1024 houses, and the wire limits pages to 20.
    for _ in 0..1024_usize.div_ceil(wire::PAGE_SIZE) {
        let command = panel::query(world, &house.town.key, next.as_deref())?;
        let reply = client
            .session_mut()
            .gated_command(command.capability, command.kind, &command.payload)
            .await?;
        if reply.status != CommandStatus::Accepted {
            return Err(format!("auction list status {:?}", reply.status).into());
        }
        let page = panel::page(&reply.payload)?;
        if let Some(row) = page
            .rows
            .into_iter()
            .find(|row| row.house == house.identity.key)
        {
            return Ok(row);
        }
        let Some(continuation) = page.cursor else {
            return Err("authored house missing from auction list".into());
        };
        if next.is_some_and(|previous| continuation <= previous) {
            return Err("auction cursor did not advance".into());
        }
        next = Some(continuation);
    }
    Err("auction pagination exceeded maximum catalogue size".into())
}

const SEVEN_DAYS_MS: u64 = 7 * 24 * 60 * 60 * 1_000;
const UNCONTESTED_PRICE: u64 = 0;
const BID_MAXIMUM: u64 = 100;

fn vacant_listing(row: &panel::Row, rent: u64) -> TestResult {
    if row.state != 1
        || row.owned
        || row.rent != rent
        || row.ends.is_some()
        || row.maximum.is_some()
        || row.price != UNCONTESTED_PRICE
    {
        return Err(format!("expected vacant listing with rent {rent}, observed {row:?}").into());
    }
    Ok(())
}

fn opened_listing(row: &panel::Row, rent: u64, before: u64, after: u64) -> TestResult {
    let deadline = before + SEVEN_DAYS_MS..=after + SEVEN_DAYS_MS;
    if row.state != 2
        || row.owned
        || row.rent != rent
        || row.maximum != Some(BID_MAXIMUM)
        || row.price != UNCONTESTED_PRICE
        || row.ends.is_none_or(|ends| !deadline.contains(&ends))
    {
        return Err(format!(
            "expected uncontested first bid with deadline in {deadline:?}, observed {row:?}"
        )
        .into());
    }
    Ok(())
}

async fn database_ms(db: &mut sqlx::PgConnection) -> TestResult<u64> {
    let millis: i64 =
        sqlx::query_scalar("SELECT (extract(epoch FROM clock_timestamp())*1000)::bigint")
            .fetch_one(db)
            .await?;
    Ok(u64::try_from(millis)?)
}

async fn bank(client: &mut DevClientSession) -> TestResult<u64> {
    use oteryn_protocol_oteryn::atelier::resources as money;
    let reply = client
        .session_mut()
        .gated_command(
            money::CAPABILITY_ATELIER_RESOURCES_V1,
            money::COMMAND_TYPE_ATELIER_RESOURCES_QUERY,
            &[],
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("bank query status {:?}", reply.status).into());
    }
    Ok(money::decode_reply(&reply.payload)
        .map_err(|error| format!("bank query decoding: {error:?}"))?
        .bank_gold)
}

async fn bid(
    client: &mut DevClientSession,
    world: [u8; 16],
    house: &House,
    maximum: u64,
) -> TestResult {
    let command = panel::mutation(world, &house.identity.key, 0, &panel::Action::Bid(maximum))?;
    let reply = client
        .session_mut()
        .gated_command(command.capability, command.kind, &command.payload)
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("bid command status {:?}", reply.status).into());
    }
    let (disposition, _) = panel::result(&reply.payload)?;
    if disposition != wire::HouseDisposition::Ok as u64 {
        return Err(format!("bid disposition {disposition}").into());
    }
    Ok(())
}

async fn seed_elapsed_deadline(clients: &SeamClients<'_>, house: &House) -> TestResult {
    let mut db = sqlx::PgConnection::connect(clients.url).await?;
    let changed = sqlx::query(
        "UPDATE game_house_auctions SET ends_at=clock_timestamp()-interval '1 second'
         WHERE world_id=encode($1,'hex')::uuid AND house_key=$2 AND NOT settled",
    )
    .bind(clients.world.as_slice())
    .bind(&house.identity.key)
    .execute(&mut db)
    .await?
    .rows_affected();
    if changed != 1 {
        return Err(format!("expected one active auction to expire, found {changed}").into());
    }
    Ok(())
}

fn adjoining_door(pinned: &PinnedHouses, house: &House, entrance: TilePos) -> Option<TilePos> {
    let id = house.provenance.as_ref()?.engine_house.house_id;
    let source = pinned.get(&id)?;
    // The entrance contract uses the first numbered cardinal neighbour, N/E/S/W.
    [(0, -1), (1, 0), (0, 1), (-1, 0)]
        .into_iter()
        .find_map(|(dx, dy)| {
            let at = TilePos {
                x: entrance.x.checked_add_signed(dx)?,
                y: entrance.y.checked_add_signed(dy)?,
                floor: entrance.floor,
            };
            source
                .get(&at)
                .filter(|tile| tile.door && tile.number.is_some())
                .map(|_| at)
        })
}

fn wire_position(at: TilePos) -> [i32; 3] {
    [i32::from(at.x), i32::from(at.y), -i32::from(at.floor)]
}

// The server seals House doors out of the ordinary Channel DoorBook
// (crates/gameplay/world/src/map/facts.rs:592). Opening means taking the real
// House scope handoff from the authored entrance (house_transfer.rs:199 and :441).
async fn doors(
    clients: &SeamClients<'_>,
    owner: &mut DevClientSession,
    world: &BundleWorld,
    house: &House,
    pinned: &PinnedHouses,
) -> TestResult {
    use oteryn_dev_client::client_runtime::house_transfer as transfer;
    let key = &house.identity.key;
    let entry = position((house.entrance.x, house.entrance.y, house.entrance.z));
    let expected_door = adjoining_door(pinned, house, entry)
        .ok_or("authored entrance has no adjoining pinned numbered door")?;
    let request = |exit| {
        transfer::encode_house_transfer_intent(&transfer::HouseTransferIntent {
            exit,
            house: key.to_owned(),
        })
        .map_err(|error| format!("house door encoding: {error:?}"))
    };
    let enter = request(false)?;
    let exit = request(true)?;
    let mut outsider = connect_character(clients, 0x68, 1).await?;
    let outcome: TestResult = async {
        approach(&mut outsider, world, entry).await?;
        let denied = outsider
            .session_mut()
            .gated_command(
                transfer::CAPABILITY_HOUSE_TRANSFER_V1,
                transfer::COMMAND_TYPE_HOUSE_TRANSFER,
                &enter,
            )
            .await?;
        if denied.status != CommandStatus::Rejected
            || !denied.payload.is_empty()
            || tile(outsider.world_spatial().actor_position)? != entry
        {
            return Err(format!(
                "non-owner door disposition {:?}, expected rejection in Channel",
                denied.status
            )
            .into());
        }
        Ok(())
    }
    .await;
    let release = logout_retrying_busy(&mut outsider).await;
    outcome?;
    let (result, _, _, _) = release?;
    if !matches!(
        result,
        Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    ) {
        return Err("non-owner logout refused".into());
    }
    approach(owner, world, entry).await?;
    let opened = owner
        .session_mut()
        .gated_command(
            transfer::CAPABILITY_HOUSE_TRANSFER_V1,
            transfer::COMMAND_TYPE_HOUSE_TRANSFER,
            &enter,
        )
        .await?;
    if opened.status != CommandStatus::Accepted {
        return Err(format!(
            "owner door disposition {:?}, expected Accepted",
            opened.status
        )
        .into());
    }
    let ack = transfer::decode_house_transfer_ack(&opened.payload)
        .map_err(|error| format!("owner door acknowledgement: {error:?}"))?;
    let left = owner
        .session_mut()
        .gated_command(
            transfer::CAPABILITY_HOUSE_TRANSFER_V1,
            transfer::COMMAND_TYPE_HOUSE_TRANSFER,
            &exit,
        )
        .await?;
    if left.status != CommandStatus::Accepted {
        return Err(format!("owner exit disposition {:?}", left.status).into());
    }
    let exit_ack = transfer::decode_house_transfer_ack(&left.payload)
        .map_err(|error| format!("owner exit acknowledgement: {error:?}"))?;
    if ack.house.as_deref() != Some(key) || ack.position != wire_position(expected_door) {
        return Err(format!(
            "owner entered at {:?}, expected adjoining door {:?}",
            ack.position,
            wire_position(expected_door)
        )
        .into());
    }
    if exit_ack.house.is_some() || exit_ack.position != wire_position(entry) {
        return Err(format!(
            "owner exited at {:?}, expected authored entrance {:?}",
            exit_ack.position,
            wire_position(entry)
        )
        .into());
    }
    Ok(())
}

async fn approach(
    client: &mut DevClientSession,
    world: &BundleWorld,
    entrance: TilePos,
) -> TestResult {
    let from = tile(client.world_spatial().actor_position)?;
    let path =
        route(world, from, |at| at == entrance).ok_or("no walk route to authored entrance")?;
    let mut at = from;
    for direction in path {
        let expected = destination(world, at, direction).ok_or("route has no expected landing")?;
        // The session retries only TooEarly movement pacing. A Rejected
        // command reaches the assertion on its first attempt.
        let outcome = client.session_mut().step_retrying(direction).await?;
        let actual = tile(client.world_spatial().actor_position)?;
        if outcome.disposition != StepDisposition::Moved || actual != expected {
            return Err(format!("entrance walk {direction:?} from {at:?}: {:?}, landed {actual:?}, expected {expected:?}", outcome.disposition).into());
        }
        at = actual;
    }
    Ok(())
}

fn first_rent(before: u64, after: u64, price: u64, rent: u64) -> TestResult {
    if before.checked_sub(after) != price.checked_add(rent) || price.checked_add(rent).is_none() {
        return Err(format!(
            "bank {before}->{after}, expected charge of price {price} plus first rent {rent}"
        )
        .into());
    }
    Ok(())
}

// The maximum accepted house-bid escrow in bid.rs and the guildhall bid port.
const SEEDED_BANK_GOLD: i64 = 999_999_999_999;

async fn seed_bidder(
    clients: &SeamClients<'_>,
    account: &str,
    house: &House,
) -> TestResult<[u8; 16]> {
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    let mut tx = database.begin().await?;
    let character: Vec<u8> = sqlx::query_scalar(
        "SELECT uuid_send(character_id) FROM game_character_roots
         WHERE account_id=$1::uuid AND world_id=encode($2,'hex')::uuid",
    )
    .bind(account)
    .bind(clients.world.as_slice())
    .fetch_one(&mut *tx)
    .await?;

    // These are fixture-only historical records. Restore triggers before any live command.
    tx.execute("SET LOCAL session_replication_role = replica")
        .await?;
    let bank = sqlx::query(include_str!("house_sweep_fixture.sql"))
        .bind(account)
        .bind(clients.world.as_slice())
        .bind(clients.channel.as_slice())
        .bind(clients.scope_generation.to_string())
        .bind(SEEDED_BANK_GOLD)
        .execute(&mut *tx)
        .await?;
    if bank.rows_affected() != 1 {
        return Err("house sweep bank fixture expected one opening balance".into());
    }
    if house.kind == "guildhall" {
        let guild = sqlx::query(include_str!("house_sweep_guild.sql"))
            .bind(account)
            .bind(clients.world.as_slice())
            .execute(&mut *tx)
            .await?;
        if guild.rows_affected() != 1 {
            return Err("house sweep guild fixture expected one active guild leader".into());
        }
    }
    tx.commit().await?;
    let character = character
        .try_into()
        .map_err(|_| "invalid bidder Character id")?;
    evidence(&format!(
        "house_sweep preparation house={} mode=seeded bank_gold={SEEDED_BANK_GOLD} premium=platform_grant guild={}",
        house.identity.key,
        house.kind == "guildhall",
    ));
    Ok(character)
}

async fn sweep_house(
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    house: &House,
    pinned: &PinnedHouses,
    session: &mut Option<DevClientSession>,
    rows: &mut Rows<'_>,
) {
    if session.is_none() {
        match connect_character(clients, 0x67, 0).await {
            Ok(client) => *session = Some(client),
            Err(error) => {
                rows.emit(Err(error), "admission");
                return;
            }
        }
    }
    let Some(client) = session.as_mut() else {
        return;
    };
    let world_check = (|| {
        let map = client
            .session_mut()
            .world_map_snapshot()
            .ok_or("live map snapshot absent")?;
        if map.header.bundle_digest != world.base().digest() {
            return Err("live map bundle differs from compiled map".into());
        }
        compiled_house(world, house, pinned)?;
        Ok(format!(
            "compiled-tiles-{}-doors-{}",
            house.tiles.len(),
            house.doors.len()
        ))
    })();
    let has_world = rows.emit(world_check, "compiled_world");
    let auction = async {
        let row = query(client, clients.world, house).await?;
        vacant_listing(&row, house.rent_gold)?;
        Ok("live-vacant-listing".into())
    }
    .await;
    if !rows.emit(auction, "auction") {
        return;
    }
    let Some(unprepared) = session.take() else {
        rows.emit(Err("bidder session absent".into()), "preparation");
        return;
    };
    match provisioning::provision_fixture(unprepared, clients, 0, 100).await {
        Ok(client) => *session = Some(client),
        Err(error) => {
            rows.emit(Err(error), "preparation");
            return;
        }
    }
    let Some(client) = session.as_mut() else {
        rows.emit(Err("bidder session absent".into()), "preparation");
        return;
    };
    let bid_result = async {
        let balance = bank(client).await?;
        let reserve = BID_MAXIMUM.checked_add(house.rent_gold).ok_or("escrow overflow")?;
        if reserve > balance {
            return Err("seeded bank balance cannot cover the bid and first rent".into());
        }
        let mut db = sqlx::PgConnection::connect(clients.url).await?;
        let start = database_ms(&mut db).await?;
        bid(client, clients.world, house, BID_MAXIMUM).await?;
        let finish = database_ms(&mut db).await?;
        let listing = query(client, clients.world, house).await?;
        opened_listing(&listing, house.rent_gold, start, finish)?;
        let after = bank(client).await?;
        if balance.checked_sub(after) != Some(reserve) {
            return Err(format!("bank {balance}->{after}, expected escrow {reserve} (maximum {BID_MAXIMUM} plus rent {})", house.rent_gold).into());
        }
        Ok::<_, Box<dyn std::error::Error>>(balance)
    }
    .await;
    let before = match bid_result {
        Ok(balance) => {
            rows.emit(Ok("first-bid-deadline-price-and-bank-escrow".into()), "");
            balance
        }
        Err(error) => {
            rows.emit(Err(error), "bid");
            return;
        }
    };
    let transfer = tokio::time::timeout(Duration::from_secs(15), async {
        seed_elapsed_deadline(clients, house).await?;
        loop {
            let current = query(client, clients.world, house).await?;
            if current.owned && current.state == 3 && current.ends.is_none() {
                return Ok::<_, Box<dyn std::error::Error>>(());
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    })
    .await
    .unwrap_or_else(|_| {
        Err("auction settlement did not project winning bidder as owner within 15s".into())
    });
    if !rows.emit(
        transfer.map(|()| "settlement-only;bidder-is-effective-owner".into()),
        "settlement",
    ) {
        return;
    }
    if has_world {
        let door_result = doors(clients, client, world, house, pinned).await;
        rows.emit(
            door_result.map(|()| "non-owner-rejected-owner-entered".into()),
            "door_access",
        );
    } else {
        rows.emit(Err("blocked-by-world".into()), "blocked");
    }
    let rent = async {
        first_rent(
            before,
            bank(client).await?,
            UNCONTESTED_PRICE,
            house.rent_gold,
        )?;
        Ok(format!("first-rent-{}", house.rent_gold))
    }
    .await;
    rows.emit(rent, "rent_charge");
}

pub(super) async fn run(
    clients: &SeamClients<'_>,
    bytes: &[u8],
    generation: &QualificationWorld,
) -> TestResult {
    let world = &generation.bundle;
    let batch = match std::env::var(BATCH_VARIABLE) {
        Ok(text) => house_batch(Some(&text)),
        Err(std::env::VarError::NotPresent) => house_batch(None),
        Err(error) => Err(format!("invalid {BATCH_VARIABLE}: {error}").into()),
    }?;
    let (start, len) = batch;
    let houses = houses(&directory())?;
    let end = start.saturating_add(len).min(houses.len());
    if start >= end {
        return Err(format!(
            "{BATCH_VARIABLE} start {start} is beyond {} houses",
            houses.len()
        )
        .into());
    }
    evidence(&format!(
        "house_sweep batch start={start} len={len} selected={} total={} mode=seeded settlement=settlement-only",
        end - start,
        houses.len()
    ));
    let pinned = pinned_houses(bytes, generation, &houses[start..end])?;
    let mut tally = Tally::default();
    let account_list = required("NODE_BOOT_HOUSE_SWEEP_ACCOUNTS")?;
    let accounts: Vec<_> = account_list.split(',').collect();
    let unique: std::collections::BTreeSet<_> = accounts.iter().collect();
    if accounts.len() != end - start
        || accounts.len() != unique.len()
        || accounts.iter().any(|account| {
            crate::node::uuid_bytes(account).is_none()
                || clients.accounts.iter().any(|existing| existing == account)
        })
    {
        return Err("house sweep requires one distinct Platform Account per selected house".into());
    }
    let mut release_error = None;
    for (house, account) in houses[start..end].iter().zip(accounts) {
        let key = house
            .identity
            .key
            .strip_prefix("oteryn:content.house.")
            .unwrap_or(&house.identity.key);
        verdict::Harness::global().progress("house-sweep-entry");
        let mut rows = Rows {
            key,
            tally: &mut tally,
            emitted: 0,
            lap: std::time::Instant::now(),
        };
        let mut session = None;
        if tokio::time::timeout(Duration::from_secs(90), async {
            let character = match seed_bidder(clients, account, house).await {
                Ok(character) => character,
                Err(error) => {
                    rows.emit(Err(error), "preparation");
                    return;
                }
            };
            let accounts = [account.to_owned(), clients.accounts[1].clone()];
            let characters = [character, clients.characters[1]];
            let entry = SeamClients {
                accounts: &accounts,
                characters: &characters,
                ..*clients
            };
            sweep_house(&entry, world, house, &pinned, &mut session, &mut rows).await;
        })
        .await
        .is_err()
        {
            rows.emit(
                Err("house entry exceeded its 90s budget".into()),
                "deadline",
            );
        }
        if let Some(mut client) = session {
            let release = logout_retrying_busy(&mut client)
                .await
                .and_then(|(result, _, _, _)| {
                    if matches!(
                        result,
                        Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
                    ) {
                        Ok(())
                    } else {
                        Err("bidder logout refused".into())
                    }
                });
            if let Err(error) = release {
                release_error.get_or_insert(error);
            }
        }
        if rows.emitted < STEPS.len() {
            rows.block_rest();
        }
    }
    evidence(&format!(
        "house_sweep summary entries={} total={} rows_pass={} rows_fail={} mode=seeded",
        end - start,
        houses.len(),
        tally.pass,
        tally.fail
    ));
    if let Some(error) = release_error {
        return Err(error);
    }
    if tally.fail > 0 {
        return Err(format!("house sweep has {} FAIL rows", tally.fail).into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn batches_require_unsigned_numbers_and_positive_length() -> TestResult {
        assert_eq!(house_batch(None)?, (0, 3));
        assert_eq!(house_batch(Some("19:20"))?, (19, 20));
        for malformed in ["", "0", "+0:3", " 0:3", "0:0", "0:3:4", "0:"] {
            assert!(house_batch(Some(malformed)).is_err(), "{malformed}");
        }
        Ok(())
    }

    #[test]
    fn content_census_contains_all_shards_in_key_order() -> TestResult {
        let catalogue = houses(&directory())?;
        assert_eq!(catalogue.len(), 995);
        assert!(
            catalogue
                .windows(2)
                .all(|pair| pair[0].identity.key < pair[1].identity.key)
        );
        Ok(())
    }

    #[test]
    fn auction_projection_requires_exact_deadline_proxy_price_and_rent() -> TestResult {
        let bytes = wire::encode_house_page(
            &[wire::HouseSummary {
                details: None,
                house: "h".into(),
                kind: wire::HouseKind::PrivateHouse,
                state: 2,
                rent: 50,
                revision: 1,
                acl_revision: 1,
                owned: false,
                move_out_ms: None,
                price: 0,
                ends_ms: Some(SEVEN_DAYS_MS + 101),
                own_maximum: Some(BID_MAXIMUM),
            }],
            None,
            true,
        )
        .map_err(|error| format!("{error:?}"))?;
        let page = panel::page(&bytes)?;
        let [listing] = page.rows.as_slice() else {
            return Err("expected one house row".into());
        };
        opened_listing(listing, 50, 100, 102)?;
        assert!(opened_listing(listing, 51, 100, 102).is_err());
        assert!(opened_listing(listing, 50, 102, 104).is_err());
        assert!(opened_listing(listing, 50, 98, 100).is_err());
        let mut listing = page.rows.into_iter().next().ok_or("missing listing")?;
        listing.price = 1;
        assert!(opened_listing(&listing, 50, 100, 102).is_err());
        listing.price = 0;
        listing.state = 1;
        listing.maximum = None;
        listing.ends = None;
        vacant_listing(&listing, 50)?;
        listing.ends = Some(SEVEN_DAYS_MS + 101);
        assert!(vacant_listing(&listing, 50).is_err());
        Ok(())
    }

    #[test]
    fn first_rent_requires_exact_bank_debit() -> TestResult {
        first_rent(150, 95, 5, 50)?;
        assert!(first_rent(150, 145, 5, 50).is_err());
        assert!(first_rent(150, 95, u64::MAX, 1).is_err());
        Ok(())
    }
}
