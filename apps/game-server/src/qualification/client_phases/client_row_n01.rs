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
        evidence(&format!("N01_CLIENT_CAPTURE_WRITE_ERROR file={name} error={error}"));
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

async fn first_page(client: &mut DevClientSession, world: [u8; 16]) -> TestResult<(house::Page, Vec<u8>)> {
    let request = house::query(world, TOWN_KEY, None).map_err(|error| format!("query encode {error}"))?;
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

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world_id = WorldId::decode(&clients.world)?;
    let world = load_world(bytes, &pin, world_id, ChannelId::decode(&clients.channel)?)?;
    let world_bytes = *world_id.as_bytes();
    // Character 0 owns the house; S05 uses character 1 and the first private house of the page.
    let mut client = connect_character(clients, 0xab, 0).await?;
    let (initial, _) = first_page(&mut client, world_bytes).await?;
    let from = tile(client.world_spatial().actor_position)?;
    let mut chosen = None;
    for row in initial.rows.iter().rev().filter(|row| row.kind == 1) {
        let at = entrance(&row.house)?;
        if let Some(path) = route(world, from, |candidate| candidate == at) {
            chosen = Some((row.clone(), path));
            break;
        }
    }
    let (private, path) = chosen.ok_or("no reachable private house on the first page")?;
    walk(&mut client, world, &path).await?;
    let maximum = private.price.max(private.rent).max(1);
    let bid = house::mutation(world_bytes, &private.house, private.revision, &house::Action::Bid(maximum))
        .map_err(|error| format!("bid encode {error}"))?;
    let bid_reply = send(&mut client, bid).await?;
    let (bid_disposition, _) = house::result(&bid_reply.payload)
        .map_err(|error| format!("bid decode {error}; status {:?}", bid_reply.status))?;
    if bid_reply.status != CommandStatus::Accepted || bid_disposition != 1 {
        return Err(format!("bid disposition {bid_disposition} status {:?}", bid_reply.status).into());
    }

    let (page, page_bytes) = first_page(&mut client, world_bytes).await?;
    let current = page
        .rows
        .iter()
        .find(|row| row.house == private.house)
        .ok_or("owned house missing from the first page")?;
    let guest = clients.characters[1];
    let acl = house::mutation(
        world_bytes,
        &private.house,
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

    let expected = format!(
        "world={}\nhouse={}\nmember={}\npage_acl_revision={}\nacl_disposition={acl_disposition}\n\
         acl_revision={acl_revision}\nstale_disposition={stale_disposition}\n",
        hex(&world_bytes),
        private.house,
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
        private.house, current.acl_revision
    ))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(150), verdict::observe(journey(clients))).await {
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
