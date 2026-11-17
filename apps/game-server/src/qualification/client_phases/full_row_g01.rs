//! G01: house controls in the production session (apps/client feature 0049-house).
//! 1. Admit the node-boot character; require the HOUSE capability in the negotiated session.
//! 2. Drive the native house panel (`client_ui::house`): query each town catalogue page.
//! 3. Set the guest list on the owned house and require the same house to expose the committed
//!    ACL revision. Use the other fixture account to bid on an active auction.
//! 4. Log out, freshly admit, re-query and require the durable ACL revision/ownership to agree.
//!
//! No player route during the run grants a house (ownership is settled by Control-side auction
//! settlement) or funds a winning bid, so the row ends FAIL naming the missing precondition; it
//! never provisions state.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::{
    house::{self, Action},
    raw_feature,
};
use oteryn_dev_client::client_ui::house as panel_ui;

fn towns() -> TestResult<std::collections::BTreeSet<String>> {
    let mut towns = std::collections::BTreeSet::new();
    for source in [
        include_str!("../../../../../content/houses/houses-00000-00499.json"),
        include_str!("../../../../../content/houses/houses-00500-00994.json"),
    ] {
        let catalogue: serde_json::Value = serde_json::from_str(source)?;
        for house in catalogue["houses"].as_array().ok_or("step=catalogue observed=no-authored-houses")? {
            towns.insert(house["town"]["key"].as_str().ok_or("step=catalogue observed=no-town-key")?.to_owned());
        }
    }
    Ok(towns)
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = verdict::indexed_timeout(Duration::from_secs(75), verdict::observe(journey(clients))).await;
    let (status, detail) = match result {
        Ok(Ok(())) => (
            "PASS",
            "house-controls-durable-after-fresh-admission".to_owned(),
        ),
        Ok(Err(error)) => ("FAIL", token(&error.to_string())),
        Err(_) => ("FAIL", "step=journey;observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=G01.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn token(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join("-")
}

async fn send(
    client: &mut DevClientSession,
    panel: &mut panel_ui::Panel,
    action: Option<&Action>,
    next: bool,
) -> TestResult<raw_feature::GatedCommandResult> {
    let command = panel_ui::request(panel, action, next).map_err(|error| error.to_string())?;
    let reply = raw_feature::send(client.session_mut(), &command).await?;
    panel.apply(&command, &reply);
    Ok(reply)
}

async fn catalogue(
    panel: &mut panel_ui::Panel,
    towns: impl IntoIterator<Item = &str>,
    mut query: impl AsyncFnMut(&mut panel_ui::Panel, bool) -> TestResult,
    matches: impl Fn(&house::Row) -> bool,
) -> TestResult<Option<usize>> {
    for town in towns {
        panel.town = town.to_owned();
        let mut next = false;
        let mut cursors = std::collections::BTreeSet::new();
        loop {
            query(panel, next).await?;
            if let Some(index) = panel.page.rows.iter().position(&matches) {
                return Ok(Some(index));
            }
            let Some(cursor) = &panel.page.cursor else {
                break;
            };
            if !cursors.insert(cursor.clone()) {
                return Err(format!("step=query town={town} observed=repeated-cursor:{cursor}").into());
            }
            next = true;
        }
    }
    Ok(None)
}

async fn query(client: &mut DevClientSession, panel: &mut panel_ui::Panel, next: bool) -> TestResult {
    let reply = send(client, panel, None, next).await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("step=query town={} observed=status:{:?}", panel.town, reply.status).into());
    }
    let decoded = house::page(&reply.payload)?;
    if panel.page != decoded {
        return Err("step=query observed=catalogue-not-applied".into());
    }
    Ok(())
}

fn acl_advanced(
    id: &str,
    before: u64,
    disposition: u64,
    current: &[house::Row],
) -> Option<u64> {
    if disposition != oteryn_protocol_oteryn::house::HouseDisposition::Ok as u64 {
        return None;
    }
    current.iter().find(|row| row.owned && row.house == id)
        .map(|row| row.acl_revision).filter(|after| *after > before)
}

async fn house_legs(
    acl: impl AsyncFnOnce() -> TestResult,
    auction: impl AsyncFnOnce() -> TestResult,
) -> TestResult {
    acl().await?;
    auction().await
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=logout observed=refused".into());
    }
    Ok(())
}

fn guest_members_match(actual: &[[u8; 16]], expected: &[[u8; 16]]) -> bool {
    let mut actual = actual.to_vec();
    actual.sort_unstable();
    actual == expected
}

async fn guests(client: &mut DevClientSession, panel: &mut panel_ui::Panel) -> TestResult<house::HouseAclSnapshot> {
    let command = panel_ui::read_acl(panel, house::HouseAclSelector::Guest)?;
    let reply = raw_feature::send(client.session_mut(), &command).await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("step=guest-read observed={:?}", reply.status).into());
    }
    panel.apply(&command, &reply);
    panel.acl_snapshot.take().ok_or_else(|| "step=guest-read observed=no-acl".into())
}

async fn owned_acl(clients: &SeamClients<'_>, towns: &std::collections::BTreeSet<String>) -> TestResult {
    let mut client = connect_world(clients, 0xc1).await?;
    let mut panel = panel_ui::Panel::default();
    panel.admitted(clients.world, client.selected_capabilities());
    panel.index = catalogue(&mut panel, towns.iter().map(String::as_str), async |panel, next| {
        query(&mut client, panel, next).await
    }, |row| row.owned).await?.ok_or("step=acl needed=owned-house")?;
    let id = panel.page.rows[panel.index].house.clone();
    let town = panel.town.clone();

    let snapshot = guests(&mut client, &mut panel).await?;
    let peer = *clients.characters.first().ok_or("step=acl needed=fixture-peer")?;
    let mut members = snapshot.members;
    if members.contains(&peer) {
        members.retain(|member| *member != peer);
    } else {
        members.push(peer);
    }
    members.sort_unstable();
    let reply = send(&mut client, &mut panel, Some(&Action::Guests(members.clone())), false).await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("step=acl observed=status:{:?}", reply.status).into());
    }
    let (disposition, revision) = house::result(&reply.payload)?;
    let lookup = |row: &house::Row| row.owned && row.house == id;
    panel.index = catalogue(&mut panel, [&*town], async |panel, next| query(&mut client, panel, next).await, lookup).await?
        .ok_or("step=acl observed=owned-house-disappeared")?;
    if acl_advanced(&id, snapshot.revision, disposition, &panel.page.rows) != Some(revision) {
        return Err(format!("step=acl observed=disposition:{disposition},revision:{revision};needed=committed-advance-on-{id}").into());
    }
    let live = guests(&mut client, &mut panel).await?;
    if live.revision != revision || !guest_members_match(&live.members, &members) {
        return Err(format!("step=acl observed={live:?};needed=written-guest-list-on-{id}").into());
    }
    logout(&mut client).await?;
    drop(client);

    let mut fresh = connect_world(clients, 0xc2).await?;
    panel.admitted(clients.world, fresh.selected_capabilities());
    let index = catalogue(&mut panel, [&*town], async |panel, next| query(&mut fresh, panel, next).await, lookup).await?
        .ok_or("step=fresh observed=owned-house-disappeared")?;
    panel.index = index;
    let durable = panel.page.rows[index].acl_revision;
    let fresh_guests = guests(&mut fresh, &mut panel).await?;
    logout(&mut fresh).await?;
    if durable != revision || fresh_guests.revision != revision || !guest_members_match(&fresh_guests.members, &members) {
        return Err(format!("step=fresh observed=acl:{durable};needed={revision}").into());
    }
    Ok(())
}

async fn bid(clients: &SeamClients<'_>, towns: &std::collections::BTreeSet<String>) -> TestResult {
    // A personal housing slot disqualifies the owner from bidding on a second property.
    let mut bidder = connect_character(clients, 0xc1, 0).await?;
    let mut panel = panel_ui::Panel::default();
    panel.admitted(clients.world, bidder.selected_capabilities());
    panel.index = catalogue(&mut panel, towns.iter().map(String::as_str), async |panel, next| {
        query(&mut bidder, panel, next).await
    }, |row| row.ends.is_some() && row.kind == oteryn_protocol_oteryn::house::HouseKind::PrivateHouse as u64)
        .await?.ok_or("step=bid needed=active-personal-house-auction")?;
    let target = &panel.page.rows[panel.index];
    let maximum = target.price.max(target.maximum.unwrap_or(0)).checked_add(1)
        .ok_or("step=bid observed=maximum-out-of-range")?;
    let reply = send(&mut bidder, &mut panel, Some(&Action::Bid(maximum)), false).await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("step=bid observed=status:{:?}", reply.status).into());
    }
    let (disposition, _) = house::result(&reply.payload)?;
    logout(&mut bidder).await?;
    if disposition != oteryn_protocol_oteryn::house::HouseDisposition::Ok as u64 {
        return Err(format!("step=bid observed=disposition:{disposition};needed=eligible-funded-bidder").into());
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let towns = towns()?;
    house_legs(async || owned_acl(clients, &towns).await, async || bid(clients, &towns).await).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn catalogue_finds_owned_house_beyond_first_nonempty_page() -> TestResult {
        let mut panel = panel_ui::Panel::default();
        let found = catalogue(&mut panel, ["oteryn:content.area.city.thais", "oteryn:content.area.city.carlin"], async |panel, next| {
            panel.page = house::Page::default();
            let town = panel.town.rsplit('.').next();
            if town == Some("carlin") && next {
                panel.page.rows.push(house::Row { owned: true, ..house::Row::default() });
            } else {
                panel.page.rows.push(house::Row::default());
                if town == Some("carlin") {
                    panel.page.cursor = Some("later".into());
                }
            }
            Ok(())
        }, |row| row.owned).await?;
        assert_eq!(found, Some(0));
        assert!(panel.town.ends_with(".carlin"));
        Ok(())
    }

    #[test]
    fn acl_advance_requires_success_on_the_original_owned_house() {
        let reordered = [
            house::Row { house: "other".into(), acl_revision: 20, ..house::Row::default() },
            house::Row { house: "mine".into(), owned: true, acl_revision: 2, ..house::Row::default() },
        ];
        assert_eq!(acl_advanced("mine", 1, 2, &reordered), None);
        assert_eq!(acl_advanced("mine", 1, 1, &reordered), Some(2));
    }

    #[tokio::test]
    async fn owned_house_and_auction_both_exercise_their_mutations() -> TestResult {
        let acl = std::cell::Cell::new(false);
        let bid = std::cell::Cell::new(false);
        house_legs(async || { acl.set(true); Ok(()) }, async || { bid.set(true); Ok(()) }).await?;
        assert!(acl.get());
        assert!(bid.get());
        Ok(())
    }

    #[test]
    fn token_has_no_whitespace() {
        assert_eq!(token("step=a  observed=b\nc"), "step=a-observed=b-c");
    }
}

#[cfg(test)]
mod delta_tests {
    use super::*;

    #[test]
    fn acl_revision_bump_without_guest_write_fails() {
        assert!(!guest_members_match(&[[1; 16]], &[[2; 16]]));
    }
}
