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

fn entrance(key: &str) -> TestResult<TilePos> {
    for body in [
        include_str!("../../../../../content/houses/houses-00000-00499.json"),
        include_str!("../../../../../content/houses/houses-00500-00994.json"),
    ] {
        let value: serde_json::Value = serde_json::from_str(body)?;
        let Some(houses) = value["houses"].as_array() else {
            return Err("House catalogue has no houses array".into());
        };
        if let Some(record) = houses
            .iter()
            .find(|record| record["identity"]["key"].as_str() == Some(key))
        {
            return Ok(TilePos {
                x: u16::try_from(record["entrance"]["x"].as_u64().ok_or("entrance x")?)?,
                y: u16::try_from(record["entrance"]["y"].as_u64().ok_or("entrance y")?)?,
                floor: -i8::try_from(record["entrance"]["z"].as_i64().ok_or("entrance z")?)?,
            });
        }
    }
    Err(format!("House catalogue missing {key}").into())
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

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world_id = WorldId::decode(&clients.world)?;
    let world = load_world(bytes, &pin, world_id, ChannelId::decode(&clients.channel)?)?;
    let world_bytes = *world_id.as_bytes();
    // Separate Accounts bid and own so the ACL probe never depends on settlement.
    let mut client = connect_character(clients, 0xab, 0).await?;
    let (initial, _) = first_page(&mut client, world_bytes).await?;
    let from = tile(client.world_spatial().actor_position)?;
    let mut chosen = None;
    for row in initial
        .rows
        .iter()
        .rev()
        .filter(|row| row.kind == 1 && matches!(row.state, 1 | 2) && !row.owned)
    {
        let at = entrance(&row.house)?;
        if let Some(path) = route(world, from, |candidate| candidate == at) {
            chosen = Some((row.clone(), path));
            break;
        }
    }
    let (private, path) = chosen.ok_or("no reachable private house on the first page")?;
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
    let (page, page_bytes) = first_page(&mut client, world_bytes).await?;
    let current = page
        .rows
        .iter()
        .find(|row| row.house == acl_house && row.owned)
        .ok_or("owner ACL house missing from the first page")?;
    let guest = clients.characters[0];
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
        &house::Action::Guests(vec![guest]),
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
