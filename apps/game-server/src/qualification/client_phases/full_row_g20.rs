//! G20 Forge preview over a dedicated character carrying two tier-zero swords
//! and the exact sliver cost of one core. The previews must leave custody intact.

use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::{forge_preview as wire, item_forge::Operation};

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(90), verdict::observe(journey(clients))).await;
    let (status, detail) = match result {
        Ok(Ok(())) => (
            "PASS",
            "forge-preview-page-effects-resources-pair-quote-and-relog".to_owned(),
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
        .bind(clients.characters[0].as_slice())
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

const SWORD: u16 = 3264;
const SLIVER: u32 = 37109;
const SLIVERS_PER_CORE: u64 = 50;

async fn browse(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
) -> TestResult<(wire::Page, [wire::Item; 2], wire::Item)> {
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
    let inventory = client.inventory().ok_or("step=browse missing inventory")?;
    let backpack = inventory
        .main_backpack
        .ok_or("step=browse missing backpack")?;
    let bag = page
        .items
        .iter()
        .find(|item| item.handle == backpack.handle && item.container)
        .ok_or("step=browse carried backpack absent from page")?
        .clone();
    let swords = inventory
        .entries
        .iter()
        .filter(|item| item.appearance_id == SWORD)
        .collect::<Vec<_>>();
    let [one, two] = swords.as_slice() else {
        return Err("step=browse expected two fixture swords in inventory".into());
    };
    let find = |entry: &oteryn_protocol_oteryn::item_view::ItemEntry| {
        page.items.iter().find(|item| {
            item.handle == entry.handle
                && item.definition_ref == entry.item_definition_ref.get()
                && item.appearance == u32::from(SWORD)
                && entry.count.get() == 1
                && !item.container
                && item.classification == 1
                && item.tier == 0
                && item.maximum_tier == 1
        })
    };
    let (Some(one), Some(two)) = (find(one), find(two)) else {
        return Err("step=browse fixture sword missing or wrong Forge metadata".into());
    };
    let pair = [one.clone(), two.clone()];
    let keys = pair
        .each_ref()
        .map(|item| (item.definition_ref, item.tier, item.container));
    if one.handle == two.handle || identical_pair(&keys) != Some((0, 1)) {
        return Err("step=pair fixture swords must be distinct identical candidates".into());
    }
    Ok((page, pair, bag))
}

async fn body(
    client: &mut DevClientSession,
    clients: &SeamClients<'_>,
) -> TestResult<wire::Resources> {
    let (page, [first, second], bag) = browse(client, clients).await?;
    match preview(client, 2, wire::Selection::Effects(first.handle)).await? {
        wire::Payload::Effects(effects)
            if effects.item == first && effects.tiers == ["0", "1 (0.50% Onslaught)"] => {}
        other => return Err(format!("step=effects observed={other:?}").into()),
    }
    let refused = preview(client, 3, wire::Selection::Effects(bag.handle)).await?;
    if refused != wire::Payload::Refused {
        return Err(format!("step=container-effects observed={refused:?}").into());
    }
    if page.resources.dust != 0 || page.resources.slivers != SLIVERS_PER_CORE {
        return Err(format!(
            "step=resources wrong fixture balances: {:?}",
            page.resources
        )
        .into());
    }
    let refused = preview(client, 4, wire::Selection::Resource(Operation::DustLimit)).await?;
    if refused != wire::Payload::Refused {
        return Err(format!("step=empty-dust-limit observed={refused:?}").into());
    }
    match preview(
        client,
        5,
        wire::Selection::Resource(Operation::SliversToCores),
    )
    .await?
    {
        wire::Payload::Resource(quote)
            if quote.operation == Operation::SliversToCores
                && quote.full_resources
                && quote.before == page.resources
                && quote.after_dust == page.resources.dust
                && quote.after_limit == page.resources.dust_limit
                && quote.after_slivers == 0
                && page.resources.cores.checked_add(1) == Some(quote.after_cores) => {}
        other => return Err(format!("step=resource observed={other:?}").into()),
    }
    let selection = wire::Selection::Pair {
        operation: Operation::Fusion,
        first: first.handle,
        second: second.handle,
        boost: false,
        protect: false,
    };
    match preview(client, 6, selection).await? {
        wire::Payload::Quote(quote)
            if quote.selection == selection
                && quote.first == first
                && quote.second == second
                && quote.resources == page.resources
                && quote.cost
                    == (wire::Cost {
                        gold: 25_000,
                        dust: 100,
                        cores: 0,
                    })
                && quote.success_percent == 50
                && quote.protection_percent == 0
                && quote.target_tier == first.tier + 1 =>
        {
            Ok(page.resources)
        }
        other => Err(format!("step=pair observed={other:?}").into()),
    }
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    character_prep::declare_seeded(
        "G20.full",
        "two-tier-zero-swords,50-slivers",
        "Forge preview",
    );
    let owner = character_prep::dedicated_character(clients, "NODE_BOOT_Q_G20_ACCOUNT").await?;
    let clients = owner.clients(clients);

    let mut client = connect_character(&clients, 0x82, 0).await?;
    let session = client.join_snapshot().game_session_id;
    character_prep::grant_item(&client, u32::from(SWORD), 2).await?;
    // Named fixture grants are capped at 20 and idempotent per item/count.
    for amount in [20, 19, 11] {
        character_prep::grant_item(&client, SLIVER, amount).await?;
    }
    finish(&mut client).await?;
    drop(client);

    let mut client = connect_character(&clients, 0x82, 0).await?;
    let active = client.join_snapshot().game_session_id;
    if active == session {
        return Err("step=fixture-relog reused original session".into());
    }
    let outcome = verdict::observe(body(&mut client, &clients)).await;
    let closed = verdict::observe(finish(&mut client)).await;
    let resources = outcome?;
    closed?;
    drop(client);

    let mut fresh = connect_character(&clients, 0x82, 0).await?;
    if fresh.join_snapshot().game_session_id == active {
        return Err("step=read-only-relog reused original session".into());
    }
    let durable = verdict::observe(browse(&mut fresh, &clients)).await;
    let closed = verdict::observe(finish(&mut fresh)).await;
    if durable?.0.resources != resources {
        return Err("step=read-only-relog Forge previews changed resources".into());
    }
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
