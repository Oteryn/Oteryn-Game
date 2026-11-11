//! G11 imbuement details query (capability 447, command 731).
//! 1. Admit the node-boot character; require capability 447 in the negotiated session.
//! 2. Query every carried item handle through the same payload the imbuement panel sends; a
//!    refusal for an item without an imbuement allowance is the correct answer and is skipped.
//! 3. For the first item with quotes, validate the echoed handle, the (family, tier) ordering and
//!    the bounded success chance, which only the server's imbuement catalogue can produce.
//! 4. Log out, freshly admit and re-query the same item definition: the quotes must be equal.
//!
//! The row never grants items; with no imbuable item carried it fails naming that precondition.

use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::ItemHandle;
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::imbuement_details as wire;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = row_complete_with_timeout(Duration::from_secs(90), journey(clients)).await;
    let (status, detail) = match result {
        Ok(Ok(())) => ("PASS", "imbuement-quotes-validated-and-durable".to_owned()),
        Ok(Err(error)) => ("FAIL", compact(&error.to_string())),
        Err(_) => ("FAIL", "step=journey-observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=G11.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn compact(value: &str) -> String {
    let joined = value.split_whitespace().collect::<Vec<_>>().join("-");
    joined.chars().take(400).collect()
}

fn gate(client: &DevClientSession, capability: u32) -> TestResult {
    if client.selected_capabilities().contains(&capability) {
        return Ok(());
    }
    Err(format!(
        "step=admit observed=capability-{capability}-not-selected needed=capability-{capability}-selected"
    )
    .into())
}

async fn send(
    client: &mut DevClientSession,
    capability: u32,
    command: u32,
    payload: &[u8],
) -> TestResult<(CommandStatus, Vec<u8>)> {
    gate(client, capability)?;
    let reply = client
        .session_mut()
        .gated_command(capability, command, payload)
        .await?;
    client.service_liveness(Duration::from_millis(50)).await?;
    Ok((reply.status, reply.payload))
}

async fn finish(client: &mut RowClient) -> TestResult {
    if client.row_logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=logout observed=refused".into());
    }
    Ok(())
}

const CAPABILITY: u32 = wire::CAPABILITY_IMBUEMENT_DETAILS_V1;

fn quotes_ok(quotes: &[wire::Quote]) -> bool {
    !quotes.is_empty()
        && quotes
            .windows(2)
            .all(|pair| (pair[0].family, pair[0].tier) < (pair[1].family, pair[1].tier))
        && quotes.iter().all(|quote| {
            quote.success_percent <= 100 && quote.materials.len() <= wire::MAX_MATERIALS
        })
}

fn carried(client: &DevClientSession) -> Vec<(ItemHandle, u32)> {
    client
        .inventory()
        .map(|inventory| {
            inventory
                .entries
                .iter()
                .map(|entry| (entry.handle, entry.item_definition_ref.get()))
                .collect()
        })
        .unwrap_or_default()
}

/// Returns the first carried item (optionally one definition only) that has imbuement quotes,
/// together with a summary of what every probed item answered.
async fn quotes_for(
    client: &mut DevClientSession,
    only: Option<u32>,
) -> TestResult<(Option<(u32, String)>, String)> {
    let mut observed = Vec::new();
    for (handle, definition) in carried(client).into_iter().take(32) {
        if only.is_some_and(|wanted| wanted != definition) {
            continue;
        }
        let (status, payload) = send(
            client,
            CAPABILITY,
            wire::COMMAND_TYPE_IMBUEMENT_DETAILS,
            &wire::encode_query(handle),
        )
        .await?;
        if status != CommandStatus::Accepted {
            observed.push(format!("{definition}:{status:?}"));
            continue;
        }
        let reply = wire::decode_reply(&payload)
            .map_err(|error| format!("step=decode observed={error:?}"))?;
        if format!("{:?}", reply.item) != format!("{handle:?}") {
            return Err("step=query observed=reply-for-another-item".into());
        }
        if reply.quotes.is_empty() {
            observed.push(format!("{definition}:no-quotes"));
            continue;
        }
        if !quotes_ok(&reply.quotes) {
            return Err(format!("step=quotes observed=invalid:{:?}", reply.quotes).into());
        }
        return Ok((
            Some((definition, format!("{:?}", reply.quotes))),
            observed.join(","),
        ));
    }
    Ok((None, observed.join(",")))
}

async fn body(
    client: &mut DevClientSession,
    _clients: &SeamClients<'_>,
) -> TestResult<(u32, String)> {
    match quotes_for(client, None).await? {
        (Some(found), _) => Ok(found),
        (None, observed) => Err(format!(
            "step=query observed={observed} needed=carried-item-with-imbuement-allowance"
        )
        .into()),
    }
}

async fn verify(
    client: &mut DevClientSession,
    _clients: &SeamClients<'_>,
    data: &(u32, String),
) -> TestResult {
    match quotes_for(client, Some(data.0)).await? {
        (Some((_, quotes)), _) if quotes == data.1 => Ok(()),
        (found, observed) => Err(format!(
            "step=relog observed={found:?}/{observed} needed=quotes-unchanged-after-fresh-admission"
        )
        .into()),
    }
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let mut client = row_connect_world(clients, 0x70).await?;
    let outcome = body(&mut client, clients).await;
    let closed = finish(&mut client).await;
    let data = outcome?;
    closed?;
    let mut fresh = row_connect_world(clients, 0x71).await?;
    let checked = verify(&mut fresh, clients, &data).await;
    let closed = finish(&mut fresh).await;
    checked?;
    closed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_is_one_bounded_token() {
        assert_eq!(
            compact("step=a  observed=b\nneeded=c"),
            "step=a-observed=b-needed=c"
        );
        assert!(!compact(&"x ".repeat(500)).contains(' '));
        assert!(compact(&"y".repeat(900)).len() <= 400);
    }

    #[test]
    fn quotes_must_be_ordered_and_bounded() {
        let quote = |family, tier, success_percent| wire::Quote {
            family,
            tier,
            gold: 1,
            requires_premium: false,
            recipe_available: true,
            success_percent,
            materials: Vec::new(),
        };
        assert!(quotes_ok(&[
            quote(1, 1, 90),
            quote(1, 2, 60),
            quote(2, 1, 100)
        ]));
        assert!(!quotes_ok(&[]));
        assert!(!quotes_ok(&[quote(1, 2, 90), quote(1, 1, 90)]));
        assert!(!quotes_ok(&[quote(1, 1, 101)]));
    }
}
