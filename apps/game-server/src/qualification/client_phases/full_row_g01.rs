//! G01: house controls in the production session (apps/client feature 0049-house).
//! 1. Admit the node-boot character; require the HOUSE capability in the negotiated session.
//! 2. Drive the native house panel (`client_ui::house`): query each town catalogue page.
//! 3. Owned house: set the guest list through the panel, then re-query and require the ACL
//!    revision to move; auction row: bid and require an accepted, decodable result.
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

const TOWNS: [&str; 5] = ["thais", "carlin", "venore", "edron", "ab_dendriel"];

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = row_complete_with_timeout(Duration::from_secs(75), journey(clients)).await;
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

/// Query every town until a catalogue page carries rows; returns the town key used.
async fn catalogue(
    client: &mut DevClientSession,
    panel: &mut panel_ui::Panel,
) -> TestResult<Option<String>> {
    for town in TOWNS {
        panel.town = format!("oteryn:content.area.city.{town}");
        let reply = send(client, panel, None, false).await?;
        if reply.status != CommandStatus::Accepted {
            return Err(
                format!("step=query town={town} observed=status:{:?}", reply.status).into(),
            );
        }
        if !panel.page.rows.is_empty() {
            return Ok(Some(panel.town.clone()));
        }
    }
    Ok(None)
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let mut client = row_connect_world(clients, 0xc1).await?;
    if !client.selected_capabilities().contains(&49) {
        return Err("step=admit observed=house-capability-not-selected".into());
    }
    let world = WorldId::decode(&clients.world)?;
    let mut panel = panel_ui::Panel::default();
    panel.admitted(*world.as_bytes(), client.selected_capabilities());
    let town = catalogue(&mut client, &mut panel).await?;
    let rows = panel.page.rows.clone();
    let owned = rows.iter().position(|row| row.owned);
    let auction = rows.iter().position(|row| row.ends.is_some());
    let mut observed = format!("rows={}", rows.len());
    let mut revision_after = None;
    if let (Some(index), Some(town)) = (owned, town.as_ref()) {
        panel.index = index;
        let before = rows[index].acl_revision;
        let reply = send(
            &mut client,
            &mut panel,
            Some(&Action::Guests(Vec::new())),
            false,
        )
        .await?;
        let (disposition, _) = house::result(&reply.payload)?;
        panel.town.clone_from(town);
        send(&mut client, &mut panel, None, false).await?;
        let after = panel.page.rows.get(index).map(|row| row.acl_revision);
        observed.push_str(&format!(";acl:{disposition}:{before}->{after:?}"));
        revision_after = after.filter(|after| *after > before);
    } else if let Some(index) = auction {
        panel.index = index;
        let reply = send(&mut client, &mut panel, Some(&Action::Bid(1)), false).await?;
        let (disposition, revision) = house::result(&reply.payload)?;
        observed.push_str(&format!(";bid:{disposition}:rev{revision}"));
    }
    let logout = client.row_logout().await?;
    if logout.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err("step=logout observed=refused".into());
    }
    if owned.is_none() {
        return Err(format!(
            "step=house-controls observed={observed};needed=owned-house;settlement-only:crates/gameplay/durability/src/durability/house_ownership/settlement.rs"
        )
        .into());
    }
    let Some(revision) = revision_after else {
        return Err(format!("step=acl observed={observed};needed=acl-revision-advance").into());
    };
    let mut fresh = row_connect_world(clients, 0xc2).await?;
    let mut again = panel_ui::Panel::default();
    again.admitted(*world.as_bytes(), fresh.selected_capabilities());
    again.town = town.ok_or("step=fresh observed=no-town")?;
    send(&mut fresh, &mut again, None, false).await?;
    let durable = again
        .page
        .rows
        .get(owned.unwrap_or(0))
        .map(|row| row.acl_revision);
    let out = fresh.row_logout().await?;
    if out.result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err("step=logout observed=refused".into());
    }
    if durable != Some(revision) {
        return Err(format!("step=fresh observed=acl:{durable:?};needed={revision}").into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_has_no_whitespace() {
        assert_eq!(token("step=a  observed=b\nc"), "step=a-observed=b-c");
    }
}
