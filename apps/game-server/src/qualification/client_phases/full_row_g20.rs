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
    let result = verdict::indexed_timeout(Duration::from_secs(90), verdict::observe(journey(clients))).await;
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

fn resource_matches(operation: Operation, payload: &wire::Payload) -> bool {
    matches!(payload, wire::Payload::Resource(quote) if quote.operation == operation)
}

fn resource_transition(operation: Operation, resources: wire::Resources) -> Option<wire::ResourceQuote> {
    use crate::domain::forge::ForgeDustResources;
    use crate::domain::forge_dust::{DustLimit, ForgeDustBalance};
    let mut expected = wire::ResourceQuote {
        operation, full_resources: true, before: resources,
        after_dust: resources.dust, after_limit: resources.dust_limit,
        after_slivers: resources.slivers, after_cores: resources.cores,
    };
    match operation {
        Operation::DustLimit => {
            let limit = DustLimit::new(u16::try_from(resources.dust_limit).ok()?).ok()?;
            let balance = ForgeDustBalance::new(u16::try_from(resources.dust).ok()?, limit).ok()?;
            let next = balance.raise_limit().ok()?.after;
            expected.after_dust = u32::from(next.balance());
            expected.after_limit = u32::from(next.limit().get());
            // Command 668's limit projection deliberately omits unrelated balances.
            expected.full_resources = false;
            expected.before = wire::Resources { dust: resources.dust, dust_limit: resources.dust_limit, ..Default::default() };
            expected.after_slivers = 0;
            expected.after_cores = 0;
        }
        Operation::DustToSlivers | Operation::SliversToCores => {
            let source = ForgeDustResources { dust: u64::from(resources.dust), slivers: resources.slivers, cores: resources.cores };
            let next = if operation == Operation::DustToSlivers { source.dust_to_slivers() } else { source.slivers_to_core() }.ok()?;
            expected.after_dust = u32::try_from(next.dust).ok()?;
            expected.after_slivers = next.slivers;
            expected.after_cores = next.cores;
        }
        _ => return None,
    }
    Some(expected)
}

fn fusion_cost_matches(quote: &wire::Quote, cost: wire::Cost, tier: u8) -> bool {
    quote.success_percent == oteryn_gameplay_items::forge::FORGE_BASE_SUCCESS_PERCENT
        && quote.protection_percent == 0
        && quote.target_tier == tier
        && quote.cost == cost
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
    for (nonce, operation) in [
        (3, Operation::DustLimit),
        (4, Operation::DustToSlivers),
        (5, Operation::SliversToCores),
    ] {
        let observed = preview(client, nonce, wire::Selection::Resource(operation)).await?;
        let expected = resource_transition(operation, page.resources);
        let valid = match expected {
            Some(expected) => resource_matches(operation, &observed)
                && matches!(&observed, wire::Payload::Resource(quote) if *quote == expected),
            None => operation != Operation::DustLimit && observed == wire::Payload::Refused,
        };
        if !valid {
            return Err(format!("step=resource-{operation:?} observed={observed:?};needed={expected:?}").into());
        }
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
    let first = &page.items[one];
    let second = &page.items[two];
    let tier = first.tier.checked_add(1).ok_or("step=pair observed=invalid-tier")?;
    let price = oteryn_gameplay_items::forge::FORGE_REFERENCE_PRICES.iter()
        .find(|row| row.classification == first.classification && row.tier == tier)
        .ok_or("step=pair observed=no-authored-fusion-price")?;
    let cost = wire::Cost { gold: price.gold, dust: oteryn_gameplay_items::forge::NORMAL_FUSION_DUST, cores: 0 };
    match preview(client, 6, selection).await? {
        wire::Payload::Quote(quote)
            if quote.selection == selection && quote.first == *first && quote.second == *second
                && quote.resources == page.resources && fusion_cost_matches(&quote, cost, tier) =>
        {
            Ok(())
        }
        other => Err(format!("step=pair observed={other:?}").into()),
    }
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let mut client = connect_world(clients, 0x82).await?;
    let outcome = verdict::observe(body(&mut client, clients)).await;
    let closed = verdict::observe(finish(&mut client)).await;
    outcome?;
    closed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_preview_cannot_echo_a_different_operation() {
        let quote = wire::ResourceQuote {
            operation: Operation::DustToSlivers,
            full_resources: true, before: wire::Resources::default(),
            after_dust: 0, after_limit: 0, after_slivers: 0, after_cores: 0,
        };
        assert!(!resource_matches(Operation::DustLimit, &wire::Payload::Resource(quote)));
    }

    #[test]
    fn fusion_preview_cannot_quote_the_wrong_cost() -> TestResult {
        let item = |handle| -> TestResult<wire::Item> {
            Ok(wire::Item {
                handle: oteryn_protocol_oteryn::item_view::ItemHandle::new(handle).ok_or("handle")?,
                definition_ref: 1, appearance: 1, name: "sword".into(), classification: 1,
                tier: 0, maximum_tier: 1, container: false,
            })
        };
        let first = item(1)?;
        let second = item(2)?;
        let cost = wire::Cost { gold: 25_000, dust: 100, cores: 0 };
        let mut quote = wire::Quote {
            selection: wire::Selection::Pair { operation: Operation::Fusion, first: first.handle,
                second: second.handle, boost: false, protect: false },
            first, second, resources: wire::Resources::default(),
            cost, target_tier: 1, success_percent: 50, protection_percent: 0,
        };
        assert!(fusion_cost_matches(&quote, cost, 1));
        quote.cost.gold = 1;
        assert!(!fusion_cost_matches(&quote, cost, 1));
        Ok(())
    }

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
