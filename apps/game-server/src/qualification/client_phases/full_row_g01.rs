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

pub(super) const HOUSE: &str = "oteryn:content.house.upper_swamp_lane_4";

pub(super) async fn prepare_owned_house(
    clients: &SeamClients<'_>,
    tag: u8,
) -> TestResult<DevClientSession> {
    const AUCTION: &str = "01934f10-7c04-7021-805b-3b1122334491";

    async fn seed_auction(clients: &SeamClients<'_>) -> TestResult {
        let catalogue: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../content/houses/houses-00500-00994.json"
        ))?;
        let house = catalogue["houses"]
            .as_array()
            .ok_or("house fixture catalogue")?
            .iter()
            .find(|house| house["identity"]["key"] == HOUSE)
            .ok_or("house fixture authored property missing")?;
        let rent = house["rent_gold"]
            .as_i64()
            .ok_or("house fixture authored rent missing")?;
        if house["kind"] != "private_house" || rent <= 0 {
            return Err("house fixture requires an authored private property with rent".into());
        }
        let world = uuid_text(&clients.world);
        let channel = uuid_text(&clients.channel);
        let character = uuid_text(&clients.characters[0]);
        let mut database = sqlx::PgConnection::connect(clients.url).await?;
        let mut tx = sqlx::Connection::begin(&mut database).await?;
        let property = sqlx::query(
            "INSERT INTO game_house_properties(world_id,house_key,kind,state,rent_gold,content_digest)
             SELECT world_id,$3,'private_house',2,$4,server_artifact_digest
             FROM game_content_activations WHERE world_id=$1::text::uuid AND channel_id=$2::text::uuid
             ORDER BY activation_sequence DESC LIMIT 1",
        )
        .bind(&world)
        .bind(&channel)
        .bind(HOUSE)
        .bind(rent)
        .execute(&mut *tx)
        .await?;
        if property.rows_affected() != 1 {
            return Err("house fixture active content missing".into());
        }
        sqlx::query("INSERT INTO game_house_ownership VALUES($1::text::uuid,$2,NULL,1,game_character_uuid_v7())")
            .bind(&world).bind(HOUSE).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_house_auctions(auction_id,world_id,house_key,first_rent,ends_at,next_order)
                     VALUES($1::text::uuid,$2::text::uuid,$3,$4,clock_timestamp()-interval '1 second',2)")
            .bind(AUCTION).bind(&world).bind(HOUSE).bind(rent).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_house_bids VALUES($1::text::uuid,$2::text::uuid,$3::text::uuid,$4::text::uuid,1,$5,1,1,$5+1)")
            .bind(AUCTION).bind(&clients.accounts[0]).bind(&world).bind(&character).bind(rent).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(())
    }

    let mut client = connect_character(clients, tag, 0).await?;
    let world = WorldId::decode(&clients.world)?;
    let mut panel = panel_ui::Panel::default();
    panel.admitted(*world.as_bytes(), client.selected_capabilities());
    catalogue(&mut client, &mut panel).await?;
    if !panel
        .page
        .rows
        .iter()
        .any(|row| row.house == HOUSE && row.owned)
    {
        let (result, detail, _, _) = fixture_logout_retrying_busy(&mut client).await?;
        if result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
            return Err(format!("step=fixture-logout observed={detail}").into());
        }
        drop(client);
        let mut prepared = character_prep::Seeded {
            profile: "house-g01-build",
            reason: "authored house requires level 20",
        }
        .connect(clients, tag, 0)
        .await?;
        let level = prepared
            .actor_vitals()
            .ok_or("house fixture vitals missing")?;
        let (health, mana, _) = oteryn_gameplay_characters::vitals::maxima(
            "sorcerer",
            20,
            oteryn_protocol_oteryn::actor_spell::MAX_VITAL_POOL,
            oteryn_protocol_oteryn::actor_spell::MAX_SOUL,
        )
        .ok_or("house fixture has no source vitals")?;
        if (level.max_health, level.max_mana) != (health, mana) {
            return Err("step=fixture-admission observed=incorrect-level-20-vitals".into());
        }
        seed_auction(clients).await?;
        request_fixture_control(
            uuid_text(prepared.join_snapshot().game_session_id.as_bytes()),
            "--profile",
            "house-g01".into(),
        )
        .await?;
        catalogue(&mut prepared, &mut panel).await?;
        if !panel
            .page
            .rows
            .iter()
            .any(|row| row.house == HOUSE && row.owned)
        {
            return Err(
                "step=settlement observed=authored-house-not-owned-in-served-catalogue".into(),
            );
        }
        client = prepared;
    }
    evidence(&format!("G01_CHECK step=served-ownership status=PASS house={HOUSE}"));
    Ok(client)
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let mut client = connect_world(clients, 0xc1).await?;
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
    let logout = client.session_mut().logout().await?;
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
    let mut fresh = connect_world(clients, 0xc2).await?;
    let mut again = panel_ui::Panel::default();
    again.admitted(*world.as_bytes(), fresh.selected_capabilities());
    again.town = town.ok_or("step=fresh observed=no-town")?;
    send(&mut fresh, &mut again, None, false).await?;
    let durable = again
        .page
        .rows
        .get(owned.unwrap_or(0))
        .map(|row| row.acl_revision);
    let out = fresh.session_mut().logout().await?;
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
