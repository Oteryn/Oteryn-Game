//! Opt-in house catalogue sweep. Select with NODE_BOOT_E2E_ONLY=house_sweep.
//! OTERYN_E2E_HOUSE_SWEEP_BATCH=start:len slices the key-sorted authored catalogue.
//!
//! Oracles: content/houses contains the source ID, coordinates and rent. Compiled membership
//! and door markers come from crates/gameplay/world/src/map/facts.rs:715 and :738.
//! The live auction projection is owned by crates/gameplay/durability/src/durability/
//! house_management/query.rs:135. Settlement and first rent are owned by
//! crates/gameplay/durability/src/durability/house_ownership/settlement.rs:130.
//! The fixture observes ownership, bids and rent without mutating the auction deadline.

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

fn compiled_house(world: &BundleWorld, house: &House) -> TestResult {
    let id = house
        .provenance
        .as_ref()
        .ok_or("missing house source ID")?
        .engine_house
        .house_id;
    let facts = world.facts();
    if house.tiles.is_empty() {
        return Err("no authored house tiles".into());
    }
    for &at in &house.tiles {
        if facts.house_id(position(at)) != Some(id) {
            return Err(format!("compiled house {id} missing tile {at:?}").into());
        }
    }
    for &at in &house.doors {
        if facts.house_door(position(at), id).is_none() {
            return Err(format!("compiled house {id} missing door {at:?}").into());
        }
    }
    if house.doors.is_empty() {
        return Err("no authored house door".into());
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
const BID_MAXIMUM: u64 = 100;
const UNCONTESTED_PRICE: u64 = 0;

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

fn open_auction(row: &panel::Row, rent: u64, before: u64, after: u64) -> TestResult {
    let earliest = before
        .checked_add(SEVEN_DAYS_MS)
        .ok_or("auction deadline overflow")?;
    let latest = after
        .checked_add(SEVEN_DAYS_MS)
        .ok_or("auction deadline overflow")?;
    if row.state != 2
        || row.owned
        || row.rent != rent
        || row.maximum != Some(BID_MAXIMUM)
        || row.price != UNCONTESTED_PRICE
        || row
            .ends
            .is_none_or(|ends| !(earliest..=latest).contains(&ends))
    {
        return Err(format!(
            "expected first-bid deadline {earliest}..={latest} and rent {rent}, observed {row:?}"
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

async fn first_rent_ledger(clients: &SeamClients<'_>, house: &House) -> TestResult {
    let mut db = sqlx::PgConnection::connect(clients.url).await?;
    let (price, rent, rent_entries, price_entries, rent_amount): (i64, i64, i64, i64, i64) =
        sqlx::query_as(
            "SELECT o.price, o.first_rent,
                    count(e.entry_id) FILTER (WHERE e.kind=10),
                    count(e.entry_id) FILTER (WHERE e.kind=9),
                    coalesce(sum(e.amount) FILTER (WHERE e.kind=10),0)::bigint
             FROM game_house_properties p
             JOIN game_house_operations o ON o.operation_id=p.acquisition_receipt_ref
               AND o.world_id=p.world_id AND o.house_key=p.house_key
             LEFT JOIN game_account_bank_entries e ON e.house_operation_id=o.operation_id
             WHERE p.world_id=encode($1,'hex')::uuid AND p.house_key=$2
               AND o.kind=2 AND o.character_id=encode($3,'hex')::uuid
             GROUP BY o.operation_id",
        )
        .bind(clients.world.as_slice())
        .bind(&house.identity.key)
        .bind(clients.characters[0].as_slice())
        .fetch_one(&mut db)
        .await?;
    let expected = i64::try_from(house.rent_gold)?;
    if (price, rent, rent_entries, price_entries, rent_amount) != (0, expected, 1, 0, expected) {
        return Err(format!("wrong first-rent ledger: price={price}, rent={rent}, rent entries={rent_entries}, price entries={price_entries}, rent amount={rent_amount}").into());
    }
    Ok(())
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
) -> TestResult {
    use oteryn_dev_client::client_runtime::house_transfer as transfer;
    let key = &house.identity.key;
    let entry = position((house.entrance.x, house.entrance.y, house.entrance.z));
    let house_id = house
        .provenance
        .as_ref()
        .ok_or("missing house source ID")?
        .engine_house
        .house_id;
    let expected_door = [(0, -1), (1, 0), (0, 1), (-1, 0)]
        .into_iter()
        .find_map(|(dx, dy)| {
            let at = TilePos {
                x: entry.x.checked_add_signed(dx)?,
                y: entry.y.checked_add_signed(dy)?,
                floor: entry.floor,
            };
            world.facts().house_door(at, house_id).map(|_| at)
        })
        .ok_or("authored entrance has no adjoining numbered door")?;
    let mut db = sqlx::PgConnection::connect(clients.url).await?;
    for (index, expected_grants) in [(0, (1, 1)), (1, (0, 0))] {
        let grants: (i64, i64) = sqlx::query_as(
            "SELECT count(*), count(*) FILTER (WHERE NOT content_fenced)
             FROM game_house_access(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid)",
        )
        .bind(clients.world.as_slice())
        .bind(key)
        .bind(clients.characters[index].as_slice())
        .fetch_one(&mut db)
        .await?;
        if grants != expected_grants {
            return Err(format!(
                "unexpected house access grants {grants:?} for character index {index}"
            )
            .into());
        }
    }
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
    if ack.house.as_deref() != Some(key)
        || ack.position != wire_position(expected_door)
        || ack.world != clients.world
        || ack.channel != clients.channel
    {
        return Err(format!("owner handoff missed the numbered entrance door: {ack:?}").into());
    }
    if exit_ack.house.is_some()
        || exit_ack.position != wire_position(entry)
        || exit_ack.world != clients.world
        || exit_ack.channel != clients.channel
        || tile(owner.world_spatial().actor_position)? != entry
    {
        return Err(format!("owner return missed the Channel entrance: {exit_ack:?}").into());
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
    walk(client, world, &path).await
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

async fn sweep_house(
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    house: &House,
    session: &mut Option<DevClientSession>,
    prepared: &mut bool,
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
        compiled_house(world, house)?;
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
    if !*prepared {
        let Some(unprepared) = session.take() else {
            rows.emit(Err("bidder session absent".into()), "preparation");
            return;
        };
        match provisioning::provision_fixture(unprepared, clients, 0, 100).await {
            Ok(client) => {
                *session = Some(client);
                *prepared = true;
            }
            Err(error) => {
                rows.emit(Err(error), "preparation");
                return;
            }
        }
    }
    let Some(client) = session.as_mut() else {
        rows.emit(Err("bidder session absent".into()), "preparation");
        return;
    };
    let bid_result = async {
        let balance = bank(client).await?;
        let reserve = BID_MAXIMUM.checked_add(house.rent_gold).ok_or("escrow overflow")?;
        let mut db = sqlx::PgConnection::connect(clients.url).await?;
        let start = database_ms(&mut db).await?;
        bid(client, clients.world, house, BID_MAXIMUM).await?;
        let finish = database_ms(&mut db).await?;
        let row = query(client, clients.world, house).await?;
        open_auction(&row, house.rent_gold, start, finish)?;
        let ends = row.ends.ok_or("first bid has no deadline")?;
        let after = bank(client).await?;
        if balance.checked_sub(after) != Some(reserve) {
            return Err(format!("bank {balance}->{after}, expected bid escrow {reserve}").into());
        }
        let escrow: i64 = sqlx::query_scalar(
            "SELECT b.escrow_gold FROM game_house_auctions a JOIN game_house_bids b USING(auction_id)
             WHERE a.world_id=encode($1,'hex')::uuid AND a.house_key=$2 AND NOT a.settled
               AND b.character_id=encode($3,'hex')::uuid AND b.state=1",
        ).bind(clients.world.as_slice()).bind(&house.identity.key)
            .bind(clients.characters[0].as_slice()).fetch_one(&mut db).await?;
        if u64::try_from(escrow)? != reserve {
            return Err(format!("durable bid escrow {escrow}, expected {reserve}").into());
        }
        Ok::<_, Box<dyn std::error::Error>>((balance, ends))
    }
    .await;
    let (before, ends) = match bid_result {
        Ok(result) => {
            rows.emit(Ok("first-bid-deadline-and-reserved-escrow".into()), "");
            result
        }
        Err(error) => {
            rows.emit(Err(error), "bid");
            return;
        }
    };
    let transfer = tokio::time::timeout(Duration::from_secs(15), async {
        let mut db = sqlx::PgConnection::connect(clients.url).await?;
        loop {
            let current = query(client, clients.world, house).await?;
            if current.owned && current.state == 3 && current.ends.is_none() {
                let (stored_deadline, deadline_floor, settled_at): (i64, i64, i64) =
                    sqlx::query_as(
                        "SELECT (extract(epoch FROM a.ends_at)*1000)::bigint,
                            floor(extract(epoch FROM a.ends_at)*1000)::bigint, evidence.occurred_at
                     FROM game_house_properties p
                     JOIN game_house_operations o ON o.operation_id=p.acquisition_receipt_ref
                     JOIN game_house_auctions a ON a.auction_id=o.auction_id AND a.settled
                     JOIN game_house_audit_outbox evidence ON evidence.operation_id=o.operation_id
                     WHERE p.world_id=encode($1,'hex')::uuid AND p.house_key=$2 AND o.kind=2",
                    )
                    .bind(clients.world.as_slice())
                    .bind(&house.identity.key)
                    .fetch_one(&mut db)
                    .await?;
                if u64::try_from(stored_deadline)? != ends || settled_at < deadline_floor {
                    return Err(
                        "auction settled before its original deadline or the deadline changed"
                            .into(),
                    );
                }
                return Ok::<_, Box<dyn std::error::Error>>(());
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    })
    .await
    .unwrap_or_else(|_| {
        Err(
            "natural auction settlement was not observed within the 15s qualification budget"
                .into(),
        )
    });
    if !rows.emit(
        transfer.map(|()| "bidder-is-effective-owner".into()),
        "settlement",
    ) {
        return;
    }
    if has_world {
        let door_result = doors(clients, client, world, house).await;
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
        first_rent_ledger(clients, house).await?;
        Ok(format!("first-rent-{}", house.rent_gold))
    }
    .await;
    rows.emit(rent, "rent_charge");
}

pub(super) async fn run(clients: &SeamClients<'_>, world: &BundleWorld) -> TestResult {
    let batch = match std::env::var(BATCH_VARIABLE) {
        Ok(text) => house_batch(Some(&text)),
        Err(std::env::VarError::NotPresent) => house_batch(None),
        Err(error) => Err(format!("invalid {BATCH_VARIABLE}: {error}").into()),
    }?;
    let (start, len) = batch;
    let houses = houses(&directory())?;
    let end = start
        .checked_add(len)
        .filter(|end| *end <= houses.len())
        .ok_or_else(|| {
            format!(
                "{BATCH_VARIABLE} slice {start}:{len} exceeds {} houses",
                houses.len()
            )
        })?;
    evidence(&format!(
        "house_sweep batch start={start} len={len} selected={} total={} mode=seeded",
        end - start,
        houses.len()
    ));
    let mut tally = Tally::default();
    let mut session = None;
    let mut prepared = false;
    for house in &houses[start..end] {
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
        if tokio::time::timeout(
            Duration::from_secs(90),
            sweep_house(
                clients,
                world,
                house,
                &mut session,
                &mut prepared,
                &mut rows,
            ),
        )
        .await
        .is_err()
        {
            rows.emit(
                Err("house entry exceeded its 90s budget".into()),
                "deadline",
            );
            session = None;
        }
        if rows.emitted < STEPS.len() {
            rows.block_rest();
        }
    }
    let release = if let Some(mut client) = session {
        logout_retrying_busy(&mut client)
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
            })
    } else {
        Ok(())
    };
    evidence(&format!(
        "house_sweep summary entries={} total={} rows_pass={} rows_fail={} mode=seeded",
        end - start,
        houses.len(),
        tally.pass,
        tally.fail
    ));
    release?;
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
    fn auction_projection_requires_live_deadline_and_exact_rent() -> TestResult {
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
                ends_ms: Some(101 + SEVEN_DAYS_MS),
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
        open_auction(listing, 50, 100, 101)?;
        assert!(open_auction(listing, 51, 100, 101).is_err());
        assert!(open_auction(listing, 50, 102, 103).is_err());
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
