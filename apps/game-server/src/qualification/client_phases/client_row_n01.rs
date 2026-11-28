//! N01 client replay capture: the house owner updates the guest list through the real HOUSE-WIRE-1
//! command, and the same update is replayed with the now-stale ACL revision. Client owner:
//! `apps/client/src/features/0049-house` (`prepare` and the `Panel::apply` reducer in client-ui).
//!
//! Captured: the selected catalogue page after the winning bid (the page the client request is built
//! from), the exact request the server accepted, the accepted ACL reply and the stale refusal.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::{CommandStatus, house, raw_feature};
use oteryn_protocol_oteryn::house as wire;

const TOWN_KEY: &str = "oteryn:content.area.city.thais";

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn capture(name: &str, bytes: &[u8]) -> TestResult {
    let root = required("E2E_ALL_CAPTURE_DIR")?;
    std::fs::write(Path::new(&root).join(name), bytes)?;
    Ok(())
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

async fn observed_exchange(
    client: &mut DevClientSession,
    command: raw_feature::FeatureCommand,
) -> TestResult<oteryn_dev_client::client_runtime::server_mechanic::ServerMechanicResponse> {
    let capability = command.capability;
    let kind = command.kind;
    evidence(&format!(
        "N01_WIRE direction=request capability={capability} command={kind} payload={}",
        hex(&command.payload)
    ));
    let reply = send(client, command).await?;
    evidence(&format!(
        "N01_WIRE direction=reply capability={capability} command={kind} status={:?} payload={}",
        reply.status,
        hex(&reply.payload)
    ));
    Ok(reply)
}

fn observe_page(command: u32, page: &house::Page) {
    evidence(&format!(
        "N01_PAGE command={command} rows={} cursor={:?}",
        page.rows.len(),
        page.cursor
    ));
    for row in &page.rows {
        evidence(&format!(
            "N01_PROPERTY command={command} house={} kind={} state={} owned={} price={} ends={:?} maximum={:?}",
            row.house, row.kind, row.state, row.owned, row.price, row.ends, row.maximum
        ));
    }
}

async fn guests(
    client: &mut DevClientSession,
    world: [u8; 16],
    house_key: &str,
) -> TestResult<wire::HouseAclSnapshot> {
    let request = house::acl_query(
        world,
        TOWN_KEY,
        &wire::HouseAclRead {
            house: house_key.to_owned(),
            selector: wire::HouseAclSelector::Guest,
        },
    )
    .map_err(|error| format!("guest list query encode {error}"))?;
    let reply = send(client, request).await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("guest list query status {:?}", reply.status).into());
    }
    wire::decode_house_acl_snapshot(&reply.payload)
        .map_err(|error| format!("guest list decode {error:?}").into())
}

async fn catalogue_page(
    client: &mut DevClientSession,
    world: [u8; 16],
    cursor: Option<&str>,
) -> TestResult<(house::Page, Vec<u8>)> {
    let request =
        house::query(world, TOWN_KEY, cursor).map_err(|error| format!("query encode {error}"))?;
    let reply = observed_exchange(client, request).await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("catalogue status {:?}", reply.status).into());
    }
    let page = house::page(&reply.payload).map_err(|error| format!("catalogue decode {error}"))?;
    observe_page(wire::COMMAND_TYPE_HOUSE_QUERY, &page);
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
    let (mut page, mut page_bytes) = catalogue_page(&mut client, world_bytes, None).await?;
    let auction_order_selected = client
        .selected_capabilities()
        .contains(&wire::CAPABILITY_HOUSE_AUCTION_ORDER_V1);
    evidence(&format!(
        "N01_AUCTION_ORDER selected={auction_order_selected}"
    ));
    if auction_order_selected {
        let mut request = house::query(world_bytes, TOWN_KEY, None)
            .map_err(|error| format!("auction query encode {error}"))?;
        request.capability = wire::CAPABILITY_HOUSE_AUCTION_ORDER_V1;
        request.kind = wire::COMMAND_TYPE_HOUSE_AUCTION_LIST;
        let reply = observed_exchange(&mut client, request).await?;
        match house::page(&reply.payload) {
            Ok(page) => observe_page(wire::COMMAND_TYPE_HOUSE_AUCTION_LIST, &page),
            Err(error) => evidence(&format!("N01_AUCTION_ORDER decode={error:?}")),
        }
    }
    let from = tile(client.world_spatial().actor_position)?;
    let mut chosen = None;
    let mut page_cursor = None;
    let mut preceding_pages = Vec::new();
    // Six wire pages bound the search to at most 120 catalogue records.
    'pages: for page_index in 0..6 {
        for row in page.rows {
            if row.kind != wire::HouseKind::PrivateHouse as u64 || row.state != 1 || row.owned {
                evidence(&format!(
                    "N01_CANDIDATE house={} status=rejected reason=not_vacant_private kind={} state={} owned={}",
                    row.house, row.kind, row.state, row.owned
                ));
                continue;
            }
            let at = match entrance(&row.house) {
                Ok(at) => at,
                Err(error) => {
                    evidence(&format!(
                        "N01_CANDIDATE house={} status=rejected reason=entrance_lookup error={error}",
                        row.house
                    ));
                    continue;
                }
            };
            let approach = |candidate: TilePos| {
                candidate.floor == at.floor
                    && u32::from(candidate.x.abs_diff(at.x)) + u32::from(candidate.y.abs_diff(at.y))
                        == 1
            };
            if let Some(path) = route(world, from, approach) {
                evidence(&format!(
                    "N01_CANDIDATE house={} status=selected entrance={at:?} steps={}",
                    row.house,
                    path.len()
                ));
                chosen = Some((row, path));
                break 'pages;
            }
            evidence(&format!(
                "N01_CANDIDATE house={} status=rejected reason=no_walkable_approach from={from:?} entrance={at:?}",
                row.house
            ));
        }
        let Some(next_cursor) = page.cursor else {
            break;
        };
        if page_index == 5 {
            break;
        }
        if page_cursor
            .as_ref()
            .is_some_and(|cursor| cursor >= &next_cursor)
        {
            return Err("house catalogue cursor did not advance".into());
        }
        preceding_pages.push(page_bytes);
        page_cursor = Some(next_cursor);
        (page, page_bytes) =
            catalogue_page(&mut client, world_bytes, page_cursor.as_deref()).await?;
    }
    let (private, path) =
        chosen.ok_or("no reachable vacant private house in six catalogue pages")?;
    walk(&mut client, world, &path).await?;
    let maximum = private.price.max(private.rent).max(1);
    let bid = house::mutation(
        world_bytes,
        &private.house,
        private.revision,
        &house::Action::Bid(maximum),
    )
    .map_err(|error| format!("bid encode {error}"))?;
    evidence(&format!(
        "N01_BID house={} maximum={maximum}",
        private.house
    ));
    let bid_reply = observed_exchange(&mut client, bid).await?;
    let (bid_disposition, _) = house::result(&bid_reply.payload)
        .map_err(|error| format!("bid decode {error}; status {:?}", bid_reply.status))?;
    if bid_reply.status != CommandStatus::Accepted || bid_disposition != 1 {
        return Err(format!(
            "bid disposition {bid_disposition} status {:?}",
            bid_reply.status
        )
        .into());
    }

    let (page, page_bytes) =
        catalogue_page(&mut client, world_bytes, page_cursor.as_deref()).await?;
    let current = page
        .rows
        .iter()
        .find(|row| row.house == private.house)
        .ok_or("owned house missing from the selected page")?;
    if !current.owned {
        return Err("bid has not settled into house ownership".into());
    }
    let guest = clients.characters[1];
    let before = guests(&mut client, world_bytes, &private.house).await?;
    if before.revision != current.acl_revision || before.members.contains(&guest) {
        return Err("guest-list pre-state diverges from the owned catalogue page".into());
    }
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
    let committed = guests(&mut client, world_bytes, &private.house).await?;
    if committed.revision != acl_revision || committed.members != [guest] {
        return Err("accepted guest edit did not persist the exact guest list".into());
    }
    let stale_reply = send(&mut client, acl).await?;
    let (stale_disposition, _) = house::result(&stale_reply.payload)
        .map_err(|error| format!("stale decode {error}; status {:?}", stale_reply.status))?;
    if stale_reply.status != CommandStatus::Accepted || stale_disposition != 3 {
        return Err(format!("stale reply d={stale_disposition}").into());
    }

    let after_stale = guests(&mut client, world_bytes, &private.house).await?;
    if after_stale != committed {
        return Err("stale guest edit changed the guest list".into());
    }
    let logout = client.session_mut().logout().await?;
    if logout.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("N01 logout refused: {logout:?}").into());
    }
    drop(client);
    let mut fresh = connect_character(clients, 0xac, 0).await?;
    if guests(&mut fresh, world_bytes, &private.house).await? != committed {
        return Err("guest list after relog differs from the committed list".into());
    }
    let expected = format!(
        "world={}\nhouse={}\nmember={}\npage_acl_revision={}\npreceding_pages={}\nacl_disposition={acl_disposition}\n\
         acl_revision={acl_revision}\nstale_disposition={stale_disposition}\n",
        hex(&world_bytes),
        private.house,
        hex(&guest),
        current.acl_revision,
        preceding_pages.len(),
    );
    for (index, bytes) in preceding_pages.iter().enumerate() {
        capture(&format!("n01-preceding-{index}.bin"), bytes)?;
    }
    capture("n01-page.bin", &page_bytes)?;
    capture("n01-acl-request.bin", &acl_request)?;
    capture("n01-acl.bin", &acl_reply.payload)?;
    capture("n01-stale.bin", &stale_reply.payload)?;
    capture("n01-expected.txt", expected.as_bytes())?;
    drop(fresh);
    Ok(format!(
        "house={}|acl_revision={}->{acl_revision}|stale={stale_disposition}",
        private.house, current.acl_revision
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
