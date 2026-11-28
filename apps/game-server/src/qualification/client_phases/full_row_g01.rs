//! G01: house controls in the production session (apps/client feature 0049-house).
//! 1. Admit the node-boot character; require the HOUSE capability in the negotiated session.
//! 2. Drive the native house panel (`client_ui::house`) and find the served owned row.
//! 3. Set the guest list through the panel and re-query to prove the ACL revision advances.
//! 4. Log out, freshly admit, re-query and require the durable ACL revision/ownership to agree.
//!
//! The disposable qualification database starts this journey with the authored house settled.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::{
    house::{self, Action},
    raw_feature,
};
use oteryn_dev_client::client_ui::house as panel_ui;
use sqlx::Connection;

pub(super) const HOUSE: &str = "oteryn:content.house.upper_swamp_lane_4";

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(75), verdict::observe(journey(clients))).await;
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

pub(super) fn authored_house() -> TestResult<serde_json::Value> {
    let catalogue: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../content/houses/houses-00500-00994.json"
    ))?;
    let house = catalogue["houses"]
        .as_array()
        .and_then(|houses| {
            houses
                .iter()
                .find(|house| house["identity"]["key"] == HOUSE)
        })
        .ok_or("house fixture authored property missing")?;
    Ok(house.clone())
}

pub(super) async fn fixture_character(
    clients: &SeamClients<'_>,
    account: &str,
) -> TestResult<[u8; 16]> {
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    let characters: Vec<Vec<u8>> = sqlx::query_scalar(
        "SELECT uuid_send(character_id) FROM game_character_roots
         WHERE account_id=$1::text::uuid AND world_id=encode($2,'hex')::uuid LIMIT 2",
    )
    .bind(account)
    .bind(clients.world.as_slice())
    .fetch_all(&mut database)
    .await?;
    let [character] = characters.as_slice() else {
        return Err("expected one dedicated character in the qualification World".into());
    };
    Ok(character.as_slice().try_into()?)
}

/// Reproduce the post-settlement rows from S7 in the disposable World.
/// These three rows share the same acquisition receipt and content pin.
pub(super) async fn prepare_owned_house(clients: &SeamClients<'_>) -> TestResult {
    let house = authored_house()?;
    let rent = house["rent_gold"]
        .as_i64()
        .filter(|rent| *rent > 0 && house["kind"] == "private_house")
        .ok_or("house fixture requires authored private rent")?;
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    let mut tx = database.begin().await?;
    let seeded = sqlx::query(
        "WITH activation AS (
            SELECT world_id,server_artifact_digest FROM game_content_activations
            WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid
            ORDER BY activation_sequence DESC LIMIT 1
         ), property AS (
            INSERT INTO game_house_properties(world_id,house_key,kind,state,rent_gold,content_digest,
                owner_character_id,owner_account_id,acquisition_receipt_ref,paid_until)
            SELECT a.world_id,$3,'private_house',3,$4,a.server_artifact_digest,
                c.character_id,c.account_id,game_character_uuid_v7(),statement_timestamp()+interval '30 days'
            FROM activation a JOIN game_character_roots c USING(world_id)
            WHERE c.character_id=encode($5,'hex')::uuid AND c.account_id=$6::text::uuid
            ON CONFLICT (world_id,house_key) DO NOTHING RETURNING *
         ), slot AS (
            INSERT INTO game_account_housing_slots(account_id,world_id,house_key)
            SELECT owner_account_id,world_id,house_key FROM property
         )
         INSERT INTO game_house_ownership(world_id,house_key,owner_character_id,ownership_revision,acquisition_receipt_ref)
         SELECT world_id,house_key,owner_character_id,ownership_generation,acquisition_receipt_ref FROM property",
    )
    .bind(clients.world.as_slice())
    .bind(clients.channel.as_slice())
    .bind(HOUSE)
    .bind(rent)
    .bind(clients.characters[0].as_slice())
    .bind(&clients.accounts[0])
    .execute(&mut *tx)
    .await?;
    if seeded.rows_affected() == 0 {
        let owned: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM game_house_properties p
             WHERE p.world_id=encode($1,'hex')::uuid AND p.house_key=$2 AND p.state=3
               AND p.owner_character_id=encode($3,'hex')::uuid AND p.owner_account_id=$4::text::uuid
               AND p.content_digest=(SELECT server_artifact_digest FROM game_content_activations
                   WHERE world_id=p.world_id AND channel_id=encode($5,'hex')::uuid
                   ORDER BY activation_sequence DESC LIMIT 1))",
        )
        .bind(clients.world.as_slice())
        .bind(HOUSE)
        .bind(clients.characters[0].as_slice())
        .bind(&clients.accounts[0])
        .bind(clients.channel.as_slice())
        .fetch_one(&mut *tx)
        .await?;
        if !owned {
            return Err("house fixture did not establish settled ownership".into());
        }
    }
    tx.commit().await?;
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let guest = *clients.characters.first().ok_or("G01 guest character")?;
    let accounts = [required("NODE_BOOT_G01_ACCOUNT")?];
    let characters = [fixture_character(clients, &accounts[0]).await?];
    let clients = &SeamClients {
        accounts: &accounts,
        characters: &characters,
        ..*clients
    };
    prepare_owned_house(clients).await?;
    let mut client = connect_character(clients, 0xc1, 0).await?;
    if !client.selected_capabilities().contains(&49) {
        return Err("step=admit observed=house-capability-not-selected".into());
    }
    let world = WorldId::decode(&clients.world)?;
    let mut panel = panel_ui::Panel::default();
    panel.admitted(*world.as_bytes(), client.selected_capabilities());
    let authored = authored_house()?;
    let town = authored["town"]["key"]
        .as_str()
        .ok_or("house fixture town")?;
    panel.town = town.into();
    let query = send(&mut client, &mut panel, None, false).await?;
    if query.status != CommandStatus::Accepted {
        return Err(format!("step=catalogue observed={:?}", query.status).into());
    }
    panel.index = panel
        .page
        .rows
        .iter()
        .position(|row| row.house == HOUSE && row.owned)
        .ok_or("step=catalogue observed=authored-house-not-served-as-owned")?;
    let before = panel.page.rows[panel.index].acl_revision;
    evidence("G01_CHECK step=served-ownership status=PASS");
    let reply = send(
        &mut client,
        &mut panel,
        Some(&Action::Guests(vec![guest])),
        false,
    )
    .await?;
    let (disposition, revision) = house::result(&reply.payload)?;
    if reply.status != CommandStatus::Accepted
        || disposition != oteryn_protocol_oteryn::house::HouseDisposition::Ok as u64
        || before.checked_add(1) != Some(revision)
    {
        return Err(format!(
            "step=set-guests observed={:?}/{disposition}:revision:{before}->{revision}",
            reply.status
        )
        .into());
    }
    send(&mut client, &mut panel, None, false).await?;
    if !panel
        .page
        .rows
        .iter()
        .any(|row| row.house == HOUSE && row.owned && row.acl_revision == revision)
    {
        return Err("step=set-guests observed=acl-revision-missing-from-catalogue".into());
    }
    let (result, detail, _, _) = logout_retrying_busy(&mut client).await?;
    if result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("step=logout observed={detail}").into());
    }
    drop(client);
    let mut fresh = connect_character(clients, 0xc2, 0).await?;
    let mut again = panel_ui::Panel::default();
    again.admitted(*world.as_bytes(), fresh.selected_capabilities());
    again.town = town.into();
    send(&mut fresh, &mut again, None, false).await?;
    let durable = again
        .page
        .rows
        .iter()
        .find(|row| row.house == HOUSE && row.owned)
        .map(|row| row.acl_revision);
    let (result, detail, _, _) = logout_retrying_busy(&mut fresh).await?;
    if result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("step=logout observed={detail}").into());
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
