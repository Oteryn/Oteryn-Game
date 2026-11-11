//! W18: podium/show-off sockets and house interactive cosmetics.
//!
//! Concrete player scenario, derived from W18 in the audit and scenario matrix and from
//! `gameplay_transport/house_podium.rs` plus its durable owner:
//! 1. Admit the node-boot character and send HOUSE QUERY for the character's real world and
//!    Thais.  Assert an accepted, decodable page.  If an open auction is returned, send HOUSE
//!    BID for that server-returned house and assert the result; if there is neither an owned
//!    house nor an auction, fail here with the observed page (the server needs an ordinary
//!    player-reachable auction or harness-provisioned ownership).
//! 2. Enter the owned house with HOUSE TRANSFER and assert the accepted scope transition.
//! 3. Acquire a podium through an ordinary buy/loot/pick-up/furnishing action, then obtain its
//!    item-instance UUID from a server reply.  Send PODIUM QUERY with that UUID and assert the
//!    initial projection.  Never synthesize an item UUID; if no legitimate route exposes one,
//!    fail at this step and name the missing acquisition/identity reply.
//! 4. Send PODIUM SET using the queried revision and a content key valid for the real podium
//!    kind.  Assert Accepted with revision + 1, then send PODIUM PROJECTION and assert kind,
//!    display, direction, key, and revision.
//! 5. Send the same stale PODIUM SET and assert the Changed refusal and an unchanged projection.
//! 6. Log out, freshly admit the same character, re-enter the house, query/projection using the
//!    previously server-issued UUID, and assert the durable display is identical.
//!
//! Steps after a blocked prerequisite are deliberately not fabricated.  The single row result
//! reports the first real player command/reply that prevents the complete lifecycle.

use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::house as house_wire;

const TOWN_KEY: &str = "oteryn:content.area.city.thais";

#[derive(Debug, Clone, PartialEq, Eq)]
struct HouseRow {
    key: String,
    state: u64,
    owned: bool,
    auction_ends_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HousePage {
    disposition: u64,
    rows: Vec<HouseRow>,
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

fn query_payload(world: [u8; 16]) -> Vec<u8> {
    let mut payload = Vec::new();
    push_bytes(&mut payload, 1, &world);
    push_bytes(&mut payload, 2, TOWN_KEY.as_bytes());
    push_bytes(&mut payload, 3, &[]);
    payload
}

fn bid_payload(world: [u8; 16], house: &str, maximum: u64) -> Vec<u8> {
    let mut bid = Vec::new();
    push_varint(&mut bid, 8);
    push_varint(&mut bid, maximum);
    let mut target = Vec::new();
    push_bytes(&mut target, 1, &world);
    push_bytes(&mut target, 2, house.as_bytes());
    let mut payload = Vec::new();
    push_bytes(&mut payload, 1, &bid);
    push_bytes(&mut payload, 8, &target);
    payload
}

fn decode_row(input: &[u8]) -> TestResult<HouseRow> {
    let (mut cursor, mut key, mut state, mut owned, mut ends) = (0, None, None, None, None);
    while cursor < input.len() {
        match read_varint(input, &mut cursor)? {
            10 => key = Some(std::str::from_utf8(read_bytes(input, &mut cursor)?)?.to_owned()),
            16 => state = Some(read_varint(input, &mut cursor)?),
            40 => owned = Some(read_varint(input, &mut cursor)? != 0),
            64 => ends = Some(read_varint(input, &mut cursor)?),
            tag if tag & 7 == 0 => {
                let _ = read_varint(input, &mut cursor)?;
            }
            tag if tag & 7 == 2 => {
                let _ = read_bytes(input, &mut cursor)?;
            }
            _ => return Err("unsupported house row wire type".into()),
        }
    }
    Ok(HouseRow {
        key: key.ok_or("house row key absent")?,
        state: state.ok_or("house row state absent")?,
        owned: owned.ok_or("house row ownership absent")?,
        auction_ends_ms: ends,
    })
}

fn decode_page(input: &[u8]) -> TestResult<HousePage> {
    let (mut cursor, mut disposition, mut rows) = (0, None, Vec::new());
    while cursor < input.len() {
        match read_varint(input, &mut cursor)? {
            8 => disposition = Some(read_varint(input, &mut cursor)?),
            26 => rows.push(decode_row(read_bytes(input, &mut cursor)?)?),
            34 => {
                let _ = read_bytes(input, &mut cursor)?;
            }
            _ => return Err("unsupported house page field".into()),
        }
    }
    Ok(HousePage {
        disposition: disposition.ok_or("house page disposition absent")?,
        rows,
    })
}

fn decode_result(input: &[u8]) -> TestResult<(u64, u64)> {
    let (mut cursor, mut disposition, mut revision) = (0, None, None);
    while cursor < input.len() {
        match read_varint(input, &mut cursor)? {
            8 => disposition = Some(read_varint(input, &mut cursor)?),
            16 => revision = Some(read_varint(input, &mut cursor)?),
            _ => return Err("unsupported house result field".into()),
        }
    }
    Ok((
        disposition.ok_or("house result disposition absent")?,
        revision.ok_or("house result revision absent")?,
    ))
}

async fn exchange(
    client: &mut DevClientSession,
    command: u32,
    payload: &[u8],
) -> TestResult<oteryn_dev_client::client_runtime::server_mechanic::ServerMechanicResponse> {
    Ok(client
        .session_mut()
        .gated_command(house_wire::CAPABILITY_HOUSE_V1, command, payload)
        .await?)
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let row_deadline = tokio::time::Instant::now() + Duration::from_secs(45);
    let mut client = connect_character(clients, 0xc8, 1).await?;
    let row_result: TestResult<_> = tokio::time::timeout_at(row_deadline, async {
    if !client
        .selected_capabilities()
        .contains(&house_wire::CAPABILITY_HOUSE_V1)
        || !client
            .selected_capabilities()
            .contains(&oteryn_protocol_oteryn::house_podium::CAPABILITY_HOUSE_PODIUM_V1)
    {
        return Err(format!(
            "step-1 admission missing house/podium capability; selected={:?}",
            client.selected_capabilities()
        )
        .into());
    }
    let world = WorldId::decode(&clients.world)?;
    let payload = query_payload(*world.as_bytes());
    // Decode our locally encoded command before sending it, as a native client would.
    house_wire::decode_house_query(&payload)
        .map_err(|error| format!("step-1 query encoding: {error:?}"))?;
    let response = exchange(&mut client, house_wire::COMMAND_TYPE_HOUSE_QUERY, &payload).await?;
    if response.status != CommandStatus::Accepted {
        return Err(format!(
            "step-1 HOUSE QUERY observed status={:?} payload={:?}; server needs an accepted house catalogue page",
            response.status, response.payload
        )
        .into());
    }
    let page = decode_page(&response.payload).map_err(|error| {
        format!(
            "step-1 HOUSE QUERY undecodable reply={:?}: {error}",
            response.payload
        )
    })?;
    if page.disposition != house_wire::HouseDisposition::Ok as u64 {
        return Err(format!(
            "step-1 HOUSE QUERY observed disposition={} rows={}; server needs an available house catalogue",
            page.disposition,
            page.rows.len()
        )
        .into());
    }
    evidence(&format!(
        "W18_CHECK step=1 command=HOUSE_QUERY status=Accepted disposition={} rows={} owned={}",
        page.disposition,
        page.rows.len(),
        page.rows.iter().filter(|row| row.owned).count()
    ));
    if let Some(owned) = page.rows.iter().find(|row| row.owned) {
        let mut transfer = vec![8, 1];
        push_bytes(&mut transfer, 2, owned.key.as_bytes());
        oteryn_protocol_oteryn::house_transfer::decode_house_transfer_intent(&transfer)
            .map_err(|error| format!("step-2 transfer encoding: {error:?}"))?;
        let response = client
            .session_mut()
            .gated_command(
                oteryn_protocol_oteryn::house_transfer::CAPABILITY_HOUSE_TRANSFER_V1,
                oteryn_protocol_oteryn::house_transfer::COMMAND_TYPE_HOUSE_TRANSFER,
                &transfer,
            )
            .await?;
        if response.status != CommandStatus::Accepted || response.payload.is_empty() {
            return Err(format!(
                "step-2 HOUSE TRANSFER house={} observed status={:?} payload={:?}; server needs an accepted house-scope acknowledgement",
                owned.key, response.status, response.payload
            )
            .into());
        }
        evidence(&format!(
            "W18_CHECK step=2 command=HOUSE_TRANSFER house={} status=Accepted ack_bytes={}",
            owned.key,
            response.payload.len()
        ));
        return Err(format!(
            "step-3 entered owned house={} but no ordinary acquisition/placement reply exposes a podium item UUID; server needs a player furnishing route with durable item identity",
            owned.key
        )
        .into());
    }
    let Some(auction) = page.rows.iter().find(|row| row.auction_ends_ms.is_some()) else {
        return Err(format!(
            "step-1 HOUSE QUERY observed Accepted/Ok with {} rows and 0 owned/open-auction houses; server needs an ordinary auction or harness-provisioned ownership",
            page.rows.len()
        )
        .into());
    };
    let bid = bid_payload(*world.as_bytes(), &auction.key, 1);
    house_wire::decode_house_intent(&bid)
        .map_err(|error| format!("step-1 bid encoding: {error:?}"))?;
    let response = exchange(&mut client, house_wire::COMMAND_TYPE_HOUSE_INTENT, &bid).await?;
    if response.status != CommandStatus::Accepted {
        return Err(format!(
            "step-1 HOUSE BID house={} maximum=1 observed status={:?} payload={:?}; server needs a terminal bid reply",
            auction.key, response.status, response.payload
        )
        .into());
    }
    let (disposition, revision) = decode_result(&response.payload).map_err(|error| {
        format!(
            "step-1 HOUSE BID house={} undecodable reply={:?}: {error}",
            auction.key, response.payload
        )
    })?;
    evidence(&format!(
        "W18_CHECK step=1 command=HOUSE_BID house={} maximum=1 status=Accepted disposition={} revision={revision}",
        auction.key, disposition
    ));
    Err(format!(
        "step-1 HOUSE BID house={} maximum=1 observed status=Accepted disposition={} revision={revision}; character still has no owned house/podium, server needs sufficient legitimately earned funds and a settled auction or harness-provisioned ownership",
        auction.key, disposition
    )
    .into())

    }).await.unwrap_or_else(|error| Err(error.into()));
    let logout_result = row_logout(&mut client).await;
    logout_result?;
    row_result
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = row_complete(journey(clients)).await;
    let failure = match result {
        Ok(Ok(())) => None,
        Ok(Err(error)) => Some(error.to_string()),
        Err(_) => Some("step-1 journey timeout after a real house command attempt".into()),
    };
    if let Some(error) = failure {
        evidence(&format!("W18_FAIL observed={error}"));
        evidence(&format!(
            "SEAM_EVIDENCE e2e_all scenario=W18.full status=FAIL seconds={:.3} evidence=step:prerequisite-or-mechanic-refused",
            started.elapsed().as_secs_f64()
        ));
    } else {
        evidence(&format!(
            "SEAM_EVIDENCE e2e_all scenario=W18.full status=PASS seconds={:.3}",
            started.elapsed().as_secs_f64()
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn house_query_and_bid_use_protocol_canonical_wire() -> TestResult {
        let mut world = [0x11; 16];
        world[6] = 0x71;
        world[8] = 0x91;
        let query = query_payload(world);
        assert_eq!(
            house_wire::decode_house_query(&query)
                .map_err(|error| format!("query decode: {error:?}"))?
                .world,
            world
        );
        let bid = bid_payload(world, "oteryn:content.house.main_street_1", 37);
        let decoded = house_wire::decode_house_intent(&bid)
            .map_err(|error| format!("bid decode: {error:?}"))?;
        assert_eq!(decoded.world, world);
        assert_eq!(decoded.house, "oteryn:content.house.main_street_1");
        assert_eq!(decoded.action, house_wire::HouseAction::Bid(37));
        Ok(())
    }

    #[test]
    fn house_page_parser_retains_acquisition_facts() -> TestResult {
        let mut row = Vec::new();
        push_bytes(&mut row, 1, b"oteryn:content.house.main_street_1");
        push_varint(&mut row, 16);
        push_varint(&mut row, 2);
        push_varint(&mut row, 40);
        push_varint(&mut row, 1);
        push_varint(&mut row, 64);
        push_varint(&mut row, 1234);
        let mut page = Vec::new();
        push_varint(&mut page, 8);
        push_varint(&mut page, house_wire::HouseDisposition::Ok as u64);
        push_bytes(&mut page, 3, &row);
        assert_eq!(
            decode_page(&page)?,
            HousePage {
                disposition: house_wire::HouseDisposition::Ok as u64,
                rows: vec![HouseRow {
                    key: "oteryn:content.house.main_street_1".into(),
                    state: 2,
                    owned: true,
                    auction_ends_ms: Some(1234),
                }],
            }
        );
        Ok(())
    }
}
