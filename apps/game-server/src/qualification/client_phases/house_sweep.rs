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
//! advances only the auction deadline in its disposable database, as in
//! apps/game-server/src/lifted_tests/auto/house_ownership_tests.rs:223.

use super::*;
use oteryn_dev_client::{DevClientSession, client_runtime::server_mechanic::CommandStatus};
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

#[derive(Debug, Default)]
struct Listing {
    key: String,
    state: u64,
    rent: Option<u64>,
    owned: bool,
    price: u64,
    ends: Option<u64>,
    own_maximum: Option<u64>,
}

fn push_varint(out: &mut Vec<u8>, mut value: u64) {
    while value >= 0x80 {
        out.push((value as u8 & 0x7f) | 0x80);
        value >>= 7;
    }
    out.push(value as u8);
}

fn push_bytes(out: &mut Vec<u8>, field: u64, value: &[u8]) {
    push_varint(out, (field << 3) | 2);
    push_varint(out, value.len() as u64);
    out.extend_from_slice(value);
}

fn read_varint(input: &[u8], cursor: &mut usize) -> TestResult<u64> {
    let mut value = 0_u64;
    for shift in (0..=63).step_by(7) {
        let byte = *input.get(*cursor).ok_or("truncated varint")?;
        *cursor += 1;
        value |= u64::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Ok(value);
        }
    }
    Err("overlong varint".into())
}

fn read_bytes<'a>(input: &'a [u8], cursor: &mut usize) -> TestResult<&'a [u8]> {
    let length = usize::try_from(read_varint(input, cursor)?)?;
    let end = cursor.checked_add(length).ok_or("field length overflow")?;
    let value = input.get(*cursor..end).ok_or("truncated field")?;
    *cursor = end;
    Ok(value)
}

fn listing(bytes: &[u8]) -> TestResult<Listing> {
    let mut row = Listing::default();
    let mut cursor = 0;
    while cursor < bytes.len() {
        let tag = read_varint(bytes, &mut cursor)?;
        if tag == 10 {
            row.key = std::str::from_utf8(read_bytes(bytes, &mut cursor)?)?.to_owned();
        } else {
            let value = read_varint(bytes, &mut cursor)?;
            match tag {
                16 => row.state = value,
                24 => row.rent = Some(value),
                40 => row.owned = value == 1,
                56 => row.price = value,
                64 => row.ends = Some(value),
                72 => row.own_maximum = Some(value),
                32 | 48 | 80 | 88 => {}
                _ => return Err(format!("unknown house listing field {tag}").into()),
            }
        }
    }
    Ok(row)
}

async fn query(
    client: &mut DevClientSession,
    world: [u8; 16],
    house: &House,
) -> TestResult<Listing> {
    let mut next = None::<String>;
    // The owner limits queries to 1024 houses, and the wire limits pages to 20.
    for _ in 0..1024_usize.div_ceil(wire::PAGE_SIZE) {
        let mut query = Vec::new();
        push_bytes(&mut query, 1, &world);
        push_bytes(&mut query, 2, house.town.key.as_bytes());
        if let Some(cursor) = &next {
            push_bytes(&mut query, 3, cursor.as_bytes());
        }
        wire::decode_house_query(&query).map_err(|error| format!("query encoding: {error:?}"))?;
        let reply = client
            .session_mut()
            .gated_command(
                wire::CAPABILITY_HOUSE_AUCTION_ORDER_V1,
                wire::COMMAND_TYPE_HOUSE_AUCTION_LIST,
                &query,
            )
            .await?;
        if reply.status != CommandStatus::Accepted {
            return Err(format!("auction list status {:?}", reply.status).into());
        }
        let bytes = &reply.payload;
        let mut cursor = 0;
        if read_varint(bytes, &mut cursor)? != 8
            || read_varint(bytes, &mut cursor)? != wire::HouseDisposition::Ok as u64
        {
            return Err("auction list disposition was not Ok".into());
        }
        let mut continuation = None;
        while cursor < bytes.len() {
            match read_varint(bytes, &mut cursor)? {
                26 => {
                    let row = listing(read_bytes(bytes, &mut cursor)?)?;
                    if row.key == house.identity.key {
                        return Ok(row);
                    }
                }
                34 => {
                    continuation =
                        Some(std::str::from_utf8(read_bytes(bytes, &mut cursor)?)?.to_owned())
                }
                _ => return Err("unknown auction page field".into()),
            }
        }
        if continuation.is_none() {
            return Err("authored house missing from auction list".into());
        }
        if continuation == next {
            return Err("auction cursor did not advance".into());
        }
        next = continuation;
    }
    Err("auction pagination exceeded maximum catalogue size".into())
}

fn open_auction(row: &Listing, rent: u64, now: u64) -> TestResult {
    if row.state != 2
        || row.owned
        || row.rent != Some(rent)
        || row.ends.is_none_or(|ends| ends <= now)
    {
        return Err(format!("expected open auction with rent {rent}, observed {row:?}").into());
    }
    Ok(())
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
    let mut amount = vec![8];
    push_varint(&mut amount, maximum);
    let mut target = Vec::new();
    push_bytes(&mut target, 1, &world);
    push_bytes(&mut target, 2, house.identity.key.as_bytes());
    let mut payload = Vec::new();
    push_bytes(&mut payload, 1, &amount);
    push_bytes(&mut payload, 8, &target);
    wire::decode_house_intent(&payload).map_err(|error| format!("bid encoding: {error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_HOUSE_V1,
            wire::COMMAND_TYPE_HOUSE_INTENT,
            &payload,
        )
        .await?;
    let mut cursor = 0;
    if reply.status != CommandStatus::Accepted || read_varint(&reply.payload, &mut cursor)? != 8 {
        return Err(format!("bid command status {:?}", reply.status).into());
    }
    let disposition = read_varint(&reply.payload, &mut cursor)?;
    if disposition != wire::HouseDisposition::Ok as u64 {
        return Err(format!("bid disposition {disposition}").into());
    }
    if query(client, world, house).await?.own_maximum != Some(maximum) {
        return Err("accepted bid missing from live auction projection".into());
    }
    Ok(())
}

async fn expire_auction(clients: &SeamClients<'_>, house: &House) -> TestResult {
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
    if left.status != CommandStatus::Accepted
        || !transfer::decode_house_transfer_ack(&left.payload).is_ok_and(|ack| ack.house.is_none())
    {
        return Err("owner could not return to the Channel".into());
    }
    if ack.house.as_deref() != Some(key)
        || !house
            .doors
            .iter()
            .any(|&(x, y, z)| ack.position == [i32::from(x), i32::from(y), i32::from(z)])
    {
        return Err(format!(
            "owner handoff did not land at an authored house door: {:?}",
            ack.position
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
        let now = u64::try_from(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_millis(),
        )?;
        open_auction(&row, house.rent_gold, now)?;
        Ok::<_, Box<dyn std::error::Error>>(row)
    }
    .await;
    let auction = match auction {
        Ok(row) => {
            rows.emit(Ok("live-open-auction".into()), "");
            row
        }
        Err(error) => {
            rows.emit(Err(error), "auction");
            return;
        }
    };
    if !*prepared {
        let Some(unprepared) = session.take() else {
            rows.emit(Err("bidder session absent".into()), "preparation");
            return;
        };
        match provisioning::provision_profile(unprepared, clients, 0, 100, "explorer").await {
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
        let maximum = auction.price.checked_add(1).ok_or("bid overflow")?;
        bid(client, clients.world, house, maximum).await?;
        Ok::<_, Box<dyn std::error::Error>>(balance)
    }
    .await;
    let before = match bid_result {
        Ok(balance) => {
            rows.emit(Ok("bid-confirmed-by-live-list".into()), "");
            balance
        }
        Err(error) => {
            rows.emit(Err(error), "bid");
            return;
        }
    };
    let transfer = tokio::time::timeout(Duration::from_secs(15), async {
        expire_auction(clients, house).await?;
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
        first_rent(before, bank(client).await?, auction.price, house.rent_gold)?;
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
    let end = start.saturating_add(len).min(houses.len());
    if start >= end {
        return Err(format!(
            "{BATCH_VARIABLE} start {start} is beyond {} houses",
            houses.len()
        )
        .into());
    }
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
        let listing = listing(&[10, 1, b'h', 16, 2, 24, 50, 40, 0, 64, 101])?;
        open_auction(&listing, 50, 100)?;
        assert!(open_auction(&listing, 51, 100).is_err());
        assert!(open_auction(&listing, 50, 101).is_err());
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
