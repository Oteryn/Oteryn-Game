//! G26: house sale escrow/authority (cmd 6129, cap 49).
//! Reads the house-sale catalogue, and for a house the character does not own sends a real
//! transfer offer (action 3) to the other provisioned character: the server must not change the
//! catalogue. No player route grants a house in a run (ownership is settled by Control-side auction
//! settlement), so the row ends FAIL `needed:owned-house` after the refusal is proven.
use super::*;
use oteryn_dev_client::DevClientSession;
use sqlx::Connection;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = tokio::time::timeout(Duration::from_secs(120), journey(clients)).await;
    let (status, detail) = match result {
        Ok(Ok(detail)) => ("PASS", token(&detail)),
        Ok(Err(error)) => ("FAIL", token(&error.to_string())),
        Err(_) => ("FAIL", "step=journey;observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=G26.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn token(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join("-")
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=logout observed=refused".into());
    }
    Ok(())
}

async fn character_name(clients: &SeamClients<'_>, index: usize) -> TestResult<String> {
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    let name: String = sqlx::query_scalar(
        "SELECT name FROM game_character_roots WHERE character_id = encode($1, 'hex')::uuid",
    )
    .bind(clients.characters[index].as_slice())
    .fetch_one(&mut database)
    .await?;
    database.close().await?;
    Ok(name)
}

use oteryn_protocol_oteryn::house_sale as sale;

async fn catalogue(client: &mut DevClientSession) -> TestResult<sale::HouseSaleReply> {
    let payload = sale::encode_request(&sale::HouseSaleRequest::catalogue(String::new()))
        .map_err(|error| format!("step=encode observed={error:?}"))?;
    let response = client
        .session_mut()
        .gated_command(sale::CAPABILITY, sale::COMMAND, &payload)
        .await?;
    if response.status != CommandStatus::Accepted {
        return Err(format!("step=catalogue observed=status:{:?}", response.status).into());
    }
    sale::decode_reply(&response.payload)
        .map_err(|error| format!("step=catalogue-decode observed={error:?}").into())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let own = character_name(clients, 1).await?;
    let peer = character_name(clients, 0).await?;
    let mut client = connect_world(clients, 0xd6).await?;
    if !client.selected_capabilities().contains(&sale::CAPABILITY) {
        return Err(format!(
            "step=admit observed=capability-{}-not-selected",
            sale::CAPABILITY
        )
        .into());
    }
    let before = catalogue(&mut client).await?;
    let owned = before.rows.iter().filter(|row| row.owner == own).count();
    let mut refusal = "no-house-rows".to_owned();
    if let Some(row) = before.rows.iter().find(|row| row.owner != own) {
        let request = sale::HouseSaleRequest {
            action: 3,
            house: row.house.clone(),
            expected_revision: row.revision.max(1),
            due_at_unix_ms: before.now_unix_ms + 86_400_000,
            recipient: peer,
            price: 1_000,
            after_house: String::new(),
        };
        let payload = sale::encode_request(&request)
            .map_err(|error| format!("step=offer-encode observed={error:?}"))?;
        let response = client
            .session_mut()
            .gated_command(sale::CAPABILITY, sale::COMMAND, &payload)
            .await?;
        refusal = format!("status:{:?}", response.status);
        let after = catalogue(&mut client).await?;
        if format!("{:?}", after.rows) != format!("{:?}", before.rows) {
            return Err(format!("step=offer-not-owner observed={refusal};catalogue-changed;needed=refusal-without-change").into());
        }
    }
    logout(&mut client).await?;
    if owned == 0 {
        return Err(format!("step=sale-flow observed=houses:{};owned:0;offer-by-non-owner:{refusal};needed:owned-house", before.rows.len()).into());
    }
    Err(format!(
        "step=sale-flow observed=owned:{owned};needed:escrow-and-recipient-drive-not-implemented"
    )
    .into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evidence_tokens_cover_empty_and_whitespace() {
        assert_eq!(token(""), "");
        assert!(!token("a b\tc\nd").chars().any(char::is_whitespace));
        assert_eq!(token("source:path.rs:12"), "source:path.rs:12");
    }
}
