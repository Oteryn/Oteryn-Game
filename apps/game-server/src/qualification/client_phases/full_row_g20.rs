//! G20 forge preview (capability 152, command 6116).
//! 1. Admit the node-boot character; browse the forgeable candidates (page 0). The carried gold
//!    in the page resources must equal the durable carried gold read through read-only SQL.
//! 2. Ask for the effect table of the first candidate and for a resource transition quote; both
//!    must be answered with the matching payload and the echoed nonce.
//! 3. Ask for a fusion quote of two identical candidates; with no such pair carried the row fails
//!    naming that precondition. It never grants items.

use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::{forge_preview as wire, item_forge::Operation};

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = tokio::time::timeout(Duration::from_secs(90), journey(clients)).await;
    let (status, detail) = match result {
        Ok(Ok(())) => (
            "PASS",
            "forge-preview-page-effects-resources-and-pair-quote".to_owned(),
        ),
        Ok(Err(error)) => ("FAIL", compact(&error.to_string())),
        Err(_) => ("FAIL", "step=journey-observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=G20.full status={status} seconds={:.3} evidence={detail}",
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

async fn accepted(
    client: &mut DevClientSession,
    capability: u32,
    command: u32,
    payload: &[u8],
) -> TestResult<Vec<u8>> {
    let (status, payload) = send(client, capability, command, payload).await?;
    if status != CommandStatus::Accepted {
        return Err(format!(
            "step=command-{command} observed=status:{status:?} needed=accepted-reply"
        )
        .into());
    }
    Ok(payload)
}

async fn finish(client: &mut DevClientSession) -> TestResult {
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=logout observed=refused".into());
    }
    Ok(())
}

const CAPABILITY: u32 = wire::CAPABILITY_FORGE_PREVIEW_V1;

async fn preview(
    client: &mut DevClientSession,
    nonce: u64,
    selection: wire::Selection,
) -> TestResult<wire::Payload> {
    let payload = wire::Request { nonce, selection }
        .encode()
        .ok_or("step=preview observed=request-encode")?;
    let reply = accepted(
        client,
        CAPABILITY,
        wire::COMMAND_TYPE_FORGE_PREVIEW,
        &payload,
    )
    .await?;
    let reply = wire::Reply::decode(&reply).ok_or("step=preview observed=decode-failed")?;
    if reply.nonce != nonce {
        return Err(format!("step=preview observed=nonce-{}-for-{nonce}", reply.nonce).into());
    }
    Ok(reply.payload)
}

async fn carried_gold(clients: &SeamClients<'_>) -> TestResult<i64> {
    let mut connection = sqlx::PgConnection::connect(clients.url).await?;
    Ok(sqlx::query_scalar("SELECT COALESCE((SELECT sum(i.quantity * CASE i.definition_production_key WHEN 'oteryn:item.tibia.i3031' THEN 1 WHEN 'oteryn:item.tibia.i3035' THEN 100 WHEN 'oteryn:item.tibia.i3043' THEN 10000 ELSE 0 END)::bigint FROM game_item_container_entries e JOIN game_item_instances i USING(item_instance_id) WHERE e.character_id=encode($1,'hex')::uuid AND i.lifecycle=1),0)")
        .bind(clients.characters[1].as_slice())
        .fetch_one(&mut connection)
        .await?)
}

/// Two distinct positions holding the same definition at the same tier, neither a container.
fn identical_pair(keys: &[(u32, u8, bool)]) -> Option<(usize, usize)> {
    for (first, a) in keys.iter().enumerate() {
        if a.2 {
            continue;
        }
        if let Some(offset) = keys[first + 1..]
            .iter()
            .position(|b| !b.2 && b.0 == a.0 && b.1 == a.1)
        {
            return Some((first, first + 1 + offset));
        }
    }
    None
}

async fn body(client: &mut DevClientSession, clients: &SeamClients<'_>) -> TestResult {
    let wire::Payload::Page(page) =
        preview(client, 1, wire::Selection::Browse { after: 0 }).await?
    else {
        return Err("step=browse observed=not-a-page".into());
    };
    let gold = carried_gold(clients).await?;
    if i64::try_from(page.resources.carried_gold)? != gold {
        return Err(format!(
            "step=browse observed=carried-gold:{}-sql:{gold}",
            page.resources.carried_gold
        )
        .into());
    }
    let Some(first) = page.items.first() else {
        return Err("step=browse observed=no-candidates needed=forgeable-item-carried".into());
    };
    match preview(client, 2, wire::Selection::Effects(first.handle)).await? {
        wire::Payload::Effects(effects)
            if format!("{:?}", effects.item) == format!("{first:?}") => {}
        other => return Err(format!("step=effects observed={other:?}").into()),
    }
    let mut resource = false;
    for operation in [
        Operation::DustLimit,
        Operation::DustToSlivers,
        Operation::SliversToCores,
    ] {
        if !wire::resource_operation(operation) {
            continue;
        }
        match preview(client, 3, wire::Selection::Resource(operation)).await? {
            wire::Payload::Resource(_) => resource = true,
            other => return Err(format!("step=resource observed={other:?}").into()),
        }
        break;
    }
    if !resource {
        return Err("step=resource observed=no-resource-operation".into());
    }
    let keys: Vec<(u32, u8, bool)> = page
        .items
        .iter()
        .map(|item| (item.definition_ref, item.tier, item.container))
        .collect();
    let Some((one, two)) = identical_pair(&keys) else {
        return Err(format!(
            "step=pair observed=candidates:{} needed=two-identical-forgeable-items",
            keys.len()
        )
        .into());
    };
    let selection = wire::Selection::Pair {
        operation: Operation::Fusion,
        first: page.items[one].handle,
        second: page.items[two].handle,
        boost: false,
        protect: false,
    };
    match preview(client, 4, selection).await? {
        wire::Payload::Quote(quote)
            if quote.success_percent <= 100
                && quote.target_tier == page.items[one].tier.saturating_add(1) =>
        {
            Ok(())
        }
        other => Err(format!("step=pair observed={other:?}").into()),
    }
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let mut client = connect_world(clients, 0x82).await?;
    let outcome = body(&mut client, clients).await;
    let closed = finish(&mut client).await;
    outcome?;
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
    fn identical_pair_needs_same_definition_and_tier() {
        assert_eq!(
            identical_pair(&[(1, 0, false), (2, 0, false), (1, 0, false)]),
            Some((0, 2))
        );
        assert_eq!(identical_pair(&[(1, 0, false), (1, 1, false)]), None);
        assert_eq!(identical_pair(&[(1, 0, true), (1, 0, true)]), None);
        assert_eq!(identical_pair(&[]), None);
    }
}
