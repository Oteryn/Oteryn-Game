//! N01 client replay capture: the house owner updates the guest list through the real HOUSE-WIRE-1
//! command, and the same update is replayed with the now-stale ACL revision. Client owner:
//! `apps/client/src/features/0049-house` (`prepare` and the `Panel::apply` reducer in client-ui).
//!
//! Captured: the first catalogue page after the winning bid (the page the client request is built
//! from), the exact request the server accepted, the accepted ACL reply and the stale refusal.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::{CommandStatus, house, raw_feature};

const TOWN_KEY: &str = "oteryn:content.area.city.thais";

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn capture(name: &str, bytes: &[u8]) {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    if let Err(error) = std::fs::write(Path::new(&root).join(name), bytes) {
        evidence(&format!(
            "N01_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"
        ));
    }
}

async fn send(
    client: &mut DevClientSession,
    command: raw_feature::FeatureCommand,
) -> TestResult<oteryn_dev_client::client_runtime::server_mechanic::ServerMechanicResponse> {
    Ok(client
        .session_mut()
        .gated_command(command.capability, command.kind, &command.payload)
        .await?)
}

async fn first_page(
    client: &mut DevClientSession,
    world: [u8; 16],
) -> TestResult<(house::Page, Vec<u8>)> {
    let request =
        house::query(world, TOWN_KEY, None).map_err(|error| format!("query encode {error}"))?;
    let reply = send(client, request).await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("catalogue status {:?}", reply.status).into());
    }
    let page = house::page(&reply.payload).map_err(|error| format!("catalogue decode {error}"))?;
    Ok((page, reply.payload))
}

fn houses() -> TestResult<Vec<serde_json::Value>> {
    let mut records = Vec::new();
    for body in [
        include_str!("../../../../../content/houses/houses-00000-00499.json"),
        include_str!("../../../../../content/houses/houses-00500-00994.json"),
    ] {
        let mut value: serde_json::Value = serde_json::from_str(body)?;
        let serde_json::Value::Array(houses) = value["houses"].take() else {
            return Err("House catalogue has no houses array".into());
        };
        records.extend(houses);
    }
    Ok(records)
}

fn entrance(record: &serde_json::Value) -> TestResult<TilePos> {
    Ok(TilePos {
        x: u16::try_from(record["entrance"]["x"].as_u64().ok_or("entrance x")?)?,
        y: u16::try_from(record["entrance"]["y"].as_u64().ok_or("entrance y")?)?,
        floor: -i8::try_from(record["entrance"]["z"].as_i64().ok_or("entrance z")?)?,
    })
}

async fn read_guests(
    client: &mut DevClientSession,
    world: [u8; 16],
    house_key: &str,
) -> TestResult<oteryn_protocol_oteryn::house::HouseAclSnapshot> {
    use oteryn_protocol_oteryn::house::{
        HouseAclRead, HouseAclSelector, decode_house_acl_snapshot,
    };
    let request = house::acl_query(
        world,
        TOWN_KEY,
        &HouseAclRead {
            house: house_key.to_owned(),
            selector: HouseAclSelector::Guest,
        },
    )
    .map_err(|error| format!("guest ACL query encode: {error}"))?;
    let reply = send(client, request).await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("guest ACL query refused: {:?}", reply.status).into());
    }
    decode_house_acl_snapshot(&reply.payload)
        .map_err(|error| format!("guest ACL query decode: {error:?}").into())
}

fn bid_visible(maximum: u64, listed: bool, listed_maximum: Option<u64>) -> bool {
    listed && listed_maximum == Some(maximum)
}

fn acl_visible(revision: u64, reread_revision: u64, guest_list_matches: bool) -> bool {
    revision == reread_revision && guest_list_matches
}

#[test]
fn house_bid_and_acl_require_authoritative_rereads() {
    assert!(!bid_visible(500, true, None));
    assert!(!bid_visible(500, true, Some(499)));
    assert!(bid_visible(500, true, Some(500)));
    assert!(!acl_visible(2, 2, false));
    assert!(!acl_visible(2, 1, true));
    assert!(acl_visible(2, 2, true));
}

fn separate_house_authorities(bid_house: &str, acl_house: &str, owns_acl: bool) -> bool {
    owns_acl && bid_house != acl_house
}

#[test]
fn auction_bid_does_not_supply_acl_ownership() {
    assert!(!separate_house_authorities("auction", "auction", false));
    assert!(!separate_house_authorities("auction", "other", false));
    assert!(separate_house_authorities("auction", "owned", true));
}

async fn seed_acl_owner(
    clients: &SeamClients<'_>,
    visible: &[String],
    bid_house: &str,
) -> TestResult<String> {
    use sqlx::{Connection as _, PgConnection};
    let world = clients.world.as_slice();
    let owner = clients.characters[1].as_slice();
    let mut connection = PgConnection::connect(clients.url).await?;
    let mut tx = connection.begin().await?;
    let (key, owned): (String, bool) = sqlx::query_as(
        "SELECT house_key, COALESCE(owner_character_id=encode($2,'hex')::uuid,false) AS owned          FROM game_house_properties WHERE world_id=encode($1,'hex')::uuid          AND house_key<>$3 AND house_key=ANY($4::text[]) AND kind='private_house'          AND (owner_character_id=encode($2,'hex')::uuid OR (state=1 AND owner_character_id IS NULL))          ORDER BY owned DESC, house_key LIMIT 1 FOR UPDATE",
    )
    .bind(world).bind(owner).bind(bid_house).bind(visible).fetch_one(&mut *tx).await?;
    if !owned {
        sqlx::query("WITH property AS (UPDATE game_house_properties p SET state=3,owner_character_id=r.character_id,owner_account_id=r.account_id,revision=p.revision+1,ownership_generation=p.ownership_generation+1,acquisition_receipt_ref=game_character_uuid_v7(),paid_until=statement_timestamp()+interval '30 days' FROM game_character_roots r WHERE p.world_id=encode($1,'hex')::uuid AND p.house_key=$2 AND p.state=1 AND r.character_id=encode($3,'hex')::uuid AND r.world_id=p.world_id RETURNING p.*), slot AS (INSERT INTO game_account_housing_slots(account_id,world_id,house_key) SELECT owner_account_id,world_id,house_key FROM property) UPDATE game_house_ownership a SET owner_character_id=p.owner_character_id,ownership_revision=p.ownership_generation,acquisition_receipt_ref=p.acquisition_receipt_ref FROM property p WHERE a.world_id=p.world_id AND a.house_key=p.house_key")
            .bind(world).bind(&key).bind(owner).execute(&mut *tx).await?;
    }
    let owns: bool = sqlx::query_scalar("SELECT owner_character_id=encode($3,'hex')::uuid FROM game_house_properties WHERE world_id=encode($1,'hex')::uuid AND house_key=$2")
        .bind(world).bind(&key).bind(owner).fetch_one(&mut *tx).await?;
    if !separate_house_authorities(bid_house, &key, owns) {
        return Err("auction and owner ACL fixtures are not distinct".into());
    }
    tx.commit().await?;
    Ok(key)
}

fn biddable(state: u64) -> bool {
    state == 2
}
fn guest_write(previous: &[[u8; 16]], guest: [u8; 16]) -> Vec<[u8; 16]> {
    if previous == [guest] {
        Vec::new()
    } else {
        vec![guest]
    }
}
#[test]
fn vacant_houses_are_excluded() {
    assert!(!biddable(1));
    assert!(biddable(2));
}
#[test]
fn guest_write_can_be_repeated() {
    assert!(guest_write(&[[1; 16]], [1; 16]).is_empty());
    assert_eq!(guest_write(&[], [1; 16]), vec![[1; 16]]);
}

async fn seed_open_auction(clients: &SeamClients<'_>, house: &str) -> TestResult {
    use sqlx::Connection as _;
    let mut connection = sqlx::PgConnection::connect(clients.url).await?;
    let opened = sqlx::query(
        "WITH property AS (
             UPDATE game_house_properties p SET state=2, revision=revision+1
             WHERE p.world_id=encode($1,'hex')::uuid AND p.house_key=$2
               AND p.kind='private_house' AND p.state=1 AND p.owner_character_id IS NULL
               AND NOT EXISTS (SELECT 1 FROM game_house_auctions a
                   WHERE a.world_id=p.world_id AND a.house_key=p.house_key AND NOT a.settled)
             RETURNING p.world_id,p.house_key,p.rent_gold
         )
         INSERT INTO game_house_auctions(auction_id,world_id,house_key,first_rent,ends_at)
         SELECT game_character_uuid_v7(),world_id,house_key,rent_gold,clock_timestamp()+interval '2 days'
         FROM property",
    )
    .bind(clients.world.as_slice())
    .bind(house)
    .execute(&mut connection)
    .await?;
    if opened.rows_affected() != 1 {
        return Err(
            format!("step=bid-fixture could not open the vacant auction for {house}").into(),
        );
    }
    Ok(())
}

fn bid_funding_target(maximum: u64, first_rent: u64) -> Option<u64> {
    maximum.checked_add(first_rent)
}

#[test]
fn fresh_bidder_needs_both_maximum_and_first_rent() {
    assert_eq!(bid_funding_target(750, 500), Some(1250));
    assert_eq!(bid_funding_target(u64::MAX, 1), None);
}

fn manifest_bidder_purse() -> TestResult<u64> {
    let requirements = crate::qualification_requirements::RowRequirements::load("N01")
        .ok_or("N01 manifest requirements absent")?;
    let crystals = requirements.items.get(&3043).copied().ok_or("N01 crystal purse absent")?;
    Ok(u64::from(crystals) * crate::domain::currency::Coin::Crystal.worth())
}

#[test]
fn n01_manifest_purse_covers_bid_and_rent_for_multiple_authored_houses() -> TestResult {
    let purse = manifest_bidder_purse()?;
    let eligible = houses()?.into_iter().filter(|record| {
        record["town"]["key"] == TOWN_KEY
            && record["kind"] == "private_house"
            && record["rent_gold"].as_u64().is_some_and(|rent| {
                bid_funding_target(rent.max(1), rent).is_some_and(|target| target <= purse)
            })
    }).count();
    assert!(eligible >= 2);
    Ok(())
}

async fn seed_house_anchors(clients: &SeamClients<'_>, candidates: &[(&str, i64)]) -> TestResult {
    use sqlx::Connection as _;
    let mut connection = sqlx::PgConnection::connect(clients.url).await?;
    let mut tx = connection.begin().await?;
    for (house, rent) in candidates {
        sqlx::query("WITH property AS (
            INSERT INTO game_house_properties(world_id,house_key,kind,rent_gold,content_digest)
            SELECT world_id,$3,'private_house',$4,server_artifact_digest
            FROM game_content_activations
            WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid
            ORDER BY activation_sequence DESC LIMIT 1
            ON CONFLICT DO NOTHING RETURNING *
        )
        INSERT INTO game_house_ownership(world_id,house_key,ownership_revision,acquisition_receipt_ref)
        SELECT world_id,house_key,ownership_generation,game_character_uuid_v7() FROM property")
            .bind(clients.world.as_slice()).bind(clients.channel.as_slice()).bind(house).bind(rent)
            .execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world_id = WorldId::decode(&clients.world)?;
    let mode = E2eMode::from_env()?;
    let npcs = if mode == E2eMode::Isolated {
        Some(oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?.npcs)
    } else {
        None
    };
    let world = load_world(bytes, &pin, world_id, ChannelId::decode(&clients.channel)?)?;
    let world_bytes = *world_id.as_bytes();
    // Separate Accounts bid and own so the ACL probe never depends on settlement.
    let mut client = connect_character(clients, 0xab, 0).await?;
    use sqlx::Connection as _;
    let mut connection = sqlx::PgConnection::connect(clients.url).await?;
    // Node boot leaves Premium delivery unactivated. The real bid owner admits
    // this configuration through its pre-delivery gate.
    let premium_delivered: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_premium_activation_latch)")
            .fetch_one(&mut connection)
            .await?;
    if premium_delivered && mode == E2eMode::Journey {
        return Err(Box::new(verdict::MissingServerRoute(concat!("source:", file!(), ":", line!()))));
    }
    let level: i64 = sqlx::query_scalar("SELECT level FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid")
        .bind(clients.characters[0].as_slice()).fetch_one(&mut connection).await?;
    if level < 20 {
        return Err(Box::new(verdict::MissingServerRoute(concat!("source:", file!(), ":", line!()))));
    }

    let isolated_funds = if let Some(npcs) = npcs {
        let amount = manifest_bidder_purse()?;
        Some(full_e2ef4::deposit_manifest_coins(&mut client, world, &npcs, amount).await?)
    } else {
        None
    };
    let (mut initial, _) = first_page(&mut client, world_bytes).await?;
    let from = tile(client.world_spatial().actor_position)?;
    let authored = houses()?;
    if initial.rows.iter().filter(|row| row.kind == 1).count() < 2 {
        if mode == E2eMode::Journey {
            evidence("N01_CHECK game-gap cause=reachable-house-anchors-unavailable");
            return Err(verdict::MissingServerRoute(concat!("source:", file!(), ":", line!())).into());
        }

        let mut candidates = Vec::new();
        for record in authored
            .iter()
            .filter(|record| record["town"]["key"] == TOWN_KEY && record["kind"] == "private_house")
        {
            let rent = record["rent_gold"].as_u64().ok_or("House rent absent")?;
            if isolated_funds.is_some_and(|funds| rent.saturating_mul(2) > funds) {
                continue;
            }
            let at = entrance(record)?;
            if route(world, from, |candidate| candidate == at).is_some() {
                candidates.push((
                    record["identity"]["key"]
                        .as_str()
                        .ok_or("House key absent")?,
                    record["rent_gold"].as_i64().ok_or("House rent absent")?,
                ));
                if candidates.len() == 2 {
                    break;
                }
            }
        }
        if candidates.len() != 2 {
            return Err("step=bid-fixture needs two reachable private houses".into());
        }
        seed_house_anchors(clients, &candidates).await?;
        initial = first_page(&mut client, world_bytes).await?.0;
    }
    let mut chosen = None;
    for state in [2, 1] {
        for row in initial
            .rows
            .iter()
            .rev()
            .filter(|row| row.kind == 1 && row.state == state && !row.owned)
        {
            let record = authored
                .iter()
                .find(|record| record["identity"]["key"] == row.house)
                .ok_or("House missing from authored catalogue")?;
            let rent = row.rent;
            let prior = row.maximum.map_or(0, |value| value.saturating_add(1));
            if isolated_funds.is_some_and(|funds| {
                bid_funding_target(prior.max(row.price).max(rent).max(1), rent)
                    .is_none_or(|target| target > funds)
            }) {
                continue;
            }
            let at = entrance(record)?;
            if let Some(path) = route(world, from, |candidate| candidate == at) {
                chosen = Some((row.clone(), path));
                break;
            }
        }
        if chosen.is_some() {
            break;
        }
    }
    let (mut private, path) = chosen.ok_or("no reachable private house on the first page")?;
    if !biddable(private.state) {
        if mode == E2eMode::Journey {
            evidence("N01_CHECK game-gap cause=no-open-house-auction");
            return Err(verdict::MissingServerRoute(concat!("source:", file!(), ":", line!())).into());
        }

        // An isolated catalogue can contain only vacant anchors. Install the open
        // auction fixture and require the fresh wire row before sending a bid.
        seed_open_auction(clients, &private.house).await?;
        private = first_page(&mut client, world_bytes)
            .await?
            .0
            .rows
            .into_iter()
            .find(|row| row.house == private.house && biddable(row.state))
            .ok_or("step=bid-fixture opened auction missing from catalogue")?;
    }
    walk(&mut client, world, &path).await?;
    let minimum = private.price.max(private.rent).max(1);
    let maximum = private
        .maximum
        .map(|prior| {
            prior
                .checked_add(1)
                .ok_or("prior bid maximum cannot be increased")
        })
        .transpose()?
        .map_or(minimum, |next| next.max(minimum));
    let first_rent: i64 = sqlx::query_scalar("SELECT first_rent FROM game_house_auctions
        WHERE world_id=encode($1,'hex')::uuid AND house_key=$2 AND NOT settled AND ends_at>clock_timestamp()")
        .bind(clients.world.as_slice()).bind(&private.house).fetch_one(&mut connection).await?;
    let funding = bid_funding_target(maximum, u64::try_from(first_rent)?)
        .ok_or("step=bid-fixture maximum plus first rent overflowed")?;
    if mode == E2eMode::Journey {
        let balance: Option<i64> = sqlx::query_scalar(
            "SELECT b.balance FROM game_account_bank_balances b
             JOIN game_character_roots c USING(account_id,world_id)
             WHERE c.character_id=encode($1,'hex')::uuid",
        )
        .bind(clients.characters[0].as_slice())
        .fetch_optional(&mut connection)
        .await?;
        if u64::try_from(balance.unwrap_or_default())? < funding {
            evidence("N01_CHECK game-gap cause=insufficient-player-earned-bank-funds");
            return Err(verdict::MissingServerRoute(concat!("source:", file!(), ":", line!())).into());
        }
    } else if isolated_funds.is_none_or(|funds| funds < funding) {
        return Err("N01 auction escrow exceeds the manifest purse".into());
    }
    let bid = house::mutation(
        world_bytes,
        &private.house,
        private.revision,
        &house::Action::Bid(maximum),
    )
    .map_err(|error| format!("bid encode {error}"))?;
    let bid_reply = send(&mut client, bid).await?;
    let (bid_disposition, _) = house::result(&bid_reply.payload)
        .map_err(|error| format!("bid decode {error}; status {:?}", bid_reply.status))?;
    if bid_reply.status != CommandStatus::Accepted || bid_disposition != 1 {
        return Err(format!(
            "bid disposition {bid_disposition} status {:?}",
            bid_reply.status
        )
        .into());
    }

    let (page, _) = first_page(&mut client, world_bytes).await?;
    let current = page
        .rows
        .iter()
        .find(|row| row.house == private.house)
        .ok_or("owned house missing from the first page")?;
    if !bid_visible(maximum, true, current.maximum) {
        return Err(format!(
            "bid not visible in the catalogue: expected maximum={maximum}; actual={current:?}"
        )
        .into());
    }
    let visible: Vec<_> = page.rows.iter().map(|row| row.house.clone()).collect();
    let logout = client.session_mut().logout().await?;
    if logout.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("bidder logout refused: {logout:?}").into());
    }
    drop(client);
    let acl_house = seed_acl_owner(clients, &visible, &private.house).await?;
    let mut client = connect_character(clients, 0xac, 1).await?;
    let guest = clients.characters[0];
    let previous = read_guests(&mut client, world_bytes, &acl_house).await?;
    if guest_write(&previous.members, guest).is_empty() {
        let clear = house::mutation(
            world_bytes,
            &acl_house,
            previous.revision,
            &house::Action::Guests(Vec::new()),
        )
        .map_err(|error| format!("guest ACL setup encode: {error}"))?;
        let response = send(&mut client, clear).await?;
        let (disposition, revision) = house::result(&response.payload)
            .map_err(|error| format!("guest ACL setup decode: {error}"))?;
        let checked = read_guests(&mut client, world_bytes, &acl_house).await?;
        if response.status != CommandStatus::Accepted
            || disposition != 1
            || revision <= previous.revision
            || checked.revision != revision
            || !checked.members.is_empty()
        {
            return Err(format!("guest ACL setup failed: {response:?}, {checked:?}").into());
        }
    }
    let (page, page_bytes) = first_page(&mut client, world_bytes).await?;
    let current = page
        .rows
        .iter()
        .find(|row| row.house == acl_house && row.owned)
        .ok_or("owner ACL house missing from the first page")?;
    let previous_guests = read_guests(&mut client, world_bytes, &acl_house).await?;
    if previous_guests.revision != current.acl_revision || previous_guests.members == [guest] {
        return Err(
            format!("guest ACL fixture cannot demonstrate a write: {previous_guests:?}").into(),
        );
    }
    let acl = house::mutation(
        world_bytes,
        &acl_house,
        current.acl_revision,
        &house::Action::Guests(guest_write(&previous_guests.members, guest)),
    )
    .map_err(|error| format!("acl encode {error}"))?;
    let acl_request = acl.payload.clone();
    let acl_reply = send(&mut client, acl.clone()).await?;
    let (acl_disposition, acl_revision) = house::result(&acl_reply.payload)
        .map_err(|error| format!("acl decode {error}; status {:?}", acl_reply.status))?;
    if acl_reply.status != CommandStatus::Accepted
        || acl_disposition != 1
        || acl_revision <= current.acl_revision
    {
        return Err(format!("acl reply d={acl_disposition} revision={acl_revision}").into());
    }
    let stale_reply = send(&mut client, acl).await?;
    let (stale_disposition, _) = house::result(&stale_reply.payload)
        .map_err(|error| format!("stale decode {error}; status {:?}", stale_reply.status))?;
    if stale_reply.status != CommandStatus::Accepted || stale_disposition != 3 {
        return Err(format!("stale reply d={stale_disposition}").into());
    }

    let guests = read_guests(&mut client, world_bytes, &acl_house).await?;
    if !acl_visible(acl_revision, guests.revision, guests.members == [guest]) {
        return Err(format!("guest ACL write not visible: {guests:?}").into());
    }
    let expected = format!(
        "world={}\nhouse={}\nmember={}\npage_acl_revision={}\nacl_disposition={acl_disposition}\n\
         acl_revision={acl_revision}\nstale_disposition={stale_disposition}\n",
        hex(&world_bytes),
        acl_house,
        hex(&guest),
        current.acl_revision,
    );
    capture("n01-page.bin", &page_bytes);
    capture("n01-acl-request.bin", &acl_request);
    capture("n01-acl.bin", &acl_reply.payload);
    capture("n01-stale.bin", &stale_reply.payload);
    capture("n01-expected.txt", expected.as_bytes());
    let _ = client.session_mut().logout().await;
    Ok(format!(
        "house={}|acl_revision={}->{acl_revision}|stale={stale_disposition}",
        acl_house, current.acl_revision
    ))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(150), verdict::observe(journey(clients)))
        .await
    {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=N01.client status=CAPTURED evidence={}",
            observed.replace(' ', "_")
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=N01.client status=FAIL evidence={}",
            error.to_string().replace(' ', "_")
        )),
        Err(_) => evidence("e2e_all scenario=N01.client status=FAIL evidence=timeout"),
    }
}
