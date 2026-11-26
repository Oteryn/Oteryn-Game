//! G20 Forge preview over a dedicated character carrying two tier-zero swords
//! and the exact sliver cost of one core. The previews must leave custody intact.

use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::{CommandStatus, item_view::CharacterInventory};
use oteryn_protocol_oteryn::{forge_preview as wire, item_forge::Operation};

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(90), verdict::observe(journey(clients))).await;
    let (status, detail) = match result {
        Ok(Ok(())) => (
            "PASS",
            "forge-preview-cursor-funded-and-empty-dust-exact-custody-after-relog".to_owned(),
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

#[derive(Debug, PartialEq, Eq, sqlx::FromRow)]
struct ForgeCustody {
    backpack: String,
    items: serde_json::Value,
    dust: Option<serde_json::Value>,
    gold: i64,
    slivers: i64,
    swords: i64,
    entry_count: i64,
}

async fn forge_custody(
    connection: &mut sqlx::PgConnection,
    character: &[u8; 16],
) -> TestResult<ForgeCustody> {
    let custody: ForgeCustody = sqlx::query_as(
        "WITH backpack AS (
            SELECT item_instance_id FROM game_item_container_slots
            WHERE character_id = encode($1, 'hex')::uuid
        ), entries AS (
            SELECT i.item_instance_id, e.placement_ordinal, i.quantity, i.lifecycle,
                   i.definition_production_key AS definition, COALESCE(t.tier, 0) AS tier
            FROM backpack b
            JOIN game_item_container_entries e ON e.parent_item_instance_id = b.item_instance_id
            JOIN game_item_instances i ON i.item_instance_id = e.item_instance_id
            LEFT JOIN game_item_tiers t ON t.item_instance_id = i.item_instance_id
            WHERE e.character_id = encode($1, 'hex')::uuid
            ORDER BY e.placement_ordinal
            LIMIT 101
        )
        SELECT b.item_instance_id::text AS backpack,
               COALESCE(jsonb_agg(to_jsonb(e) ORDER BY e.placement_ordinal)
                   FILTER (WHERE e.item_instance_id IS NOT NULL), '[]'::jsonb) AS items,
               (SELECT to_jsonb(d) FROM game_character_forge_dust d
                   WHERE d.character_id = encode($1, 'hex')::uuid) AS dust,
               COALESCE(sum(e.quantity * CASE e.definition
                   WHEN 'oteryn:item.tibia.i3031' THEN 1
                   WHEN 'oteryn:item.tibia.i3035' THEN 100
                   WHEN 'oteryn:item.tibia.i3043' THEN 10000 ELSE 0 END)
                   FILTER (WHERE e.lifecycle = 1), 0)::bigint AS gold,
               COALESCE(sum(e.quantity) FILTER (WHERE e.lifecycle = 1
                   AND e.definition = 'oteryn:item.tibia.i37109'), 0)::bigint AS slivers,
               count(*) FILTER (WHERE e.lifecycle = 1 AND e.quantity = 1 AND e.tier = 0
                   AND e.definition = 'oteryn:item.tibia.i3264') AS swords,
               count(e.item_instance_id) AS entry_count
        FROM backpack b LEFT JOIN entries e ON TRUE
        GROUP BY b.item_instance_id",
    )
    .bind(character.as_slice())
    .fetch_one(connection)
    .await?;
    if custody.entry_count > 100 {
        return Err("step=custody backpack exceeds snapshot bound".into());
    }
    Ok(custody)
}

async fn preview_unchanged(
    client: &mut DevClientSession,
    nonce: u64,
    selection: wire::Selection,
    inventory: &CharacterInventory,
    connection: &mut sqlx::PgConnection,
    character: &[u8; 16],
    custody: &ForgeCustody,
) -> TestResult<wire::Payload> {
    let reply = preview(client, nonce, selection).await?;
    let observed = forge_custody(connection, character).await?;
    if client.inventory() != Some(inventory) || check_custody(custody, &observed).is_err() {
        return Err(format!(
            "step=preview nonce={nonce} changed inventory or durable Forge custody"
        )
        .into());
    }
    Ok(reply)
}

fn check_custody(expected: &ForgeCustody, observed: &ForgeCustody) -> TestResult {
    if observed != expected {
        return Err("durable Forge custody changed".into());
    }
    Ok(())
}

fn check_first_page(page: &wire::Page) -> TestResult {
    if page.after != 0 || page.next.is_some() {
        return Err(format!(
            "step=browse requested after=0 with no continuation, observed after={} next={:?}",
            page.after, page.next
        )
        .into());
    }
    Ok(())
}

const FUNDED_DUST: u32 = 30;
const INITIAL_DUST_LIMIT: u32 = 100;
const FIRST_DUST_LIMIT_COST: u32 = 25;
const NEXT_DUST_LIMIT: u32 = 120;

fn check_funded_dust_limit(reply: wire::Payload) -> TestResult {
    let expected = wire::ResourceQuote {
        operation: Operation::DustLimit,
        full_resources: false,
        before: wire::Resources {
            dust: FUNDED_DUST,
            dust_limit: INITIAL_DUST_LIMIT,
            ..Default::default()
        },
        after_dust: FUNDED_DUST - FIRST_DUST_LIMIT_COST,
        after_limit: NEXT_DUST_LIMIT,
        after_slivers: 0,
        after_cores: 0,
    };
    if reply != wire::Payload::Resource(expected) {
        return Err(
            format!("step=funded-dust-limit observed={reply:?} expected={expected:?}").into(),
        );
    }
    Ok(())
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
const STARTER_GOLD: i64 = 50;

fn browse(
    client: &DevClientSession,
    reply: wire::Payload,
    custody: &ForgeCustody,
) -> TestResult<(wire::Page, [wire::Item; 2], wire::Item)> {
    let wire::Payload::Page(page) = reply else {
        return Err("step=browse observed=not-a-page".into());
    };
    check_first_page(&page)?;
    let gold = custody.gold;
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
    connection: &mut sqlx::PgConnection,
    character: &[u8; 16],
    custody: &ForgeCustody,
) -> TestResult<wire::Resources> {
    let inventory = client
        .inventory()
        .ok_or("step=preview missing inventory")?
        .clone();
    macro_rules! read_preview {
        ($nonce:expr, $selection:expr) => {
            preview_unchanged(
                client, $nonce, $selection, &inventory, connection, character, custody,
            )
            .await?
        };
    }
    let reply = read_preview!(1, wire::Selection::Browse { after: 0 });
    let (page, [first, second], bag) = browse(client, reply, custody)?;
    match read_preview!(2, wire::Selection::Effects(first.handle)) {
        wire::Payload::Effects(effects)
            if effects.item == first && effects.tiers == ["0", "1 (0.50% Onslaught)"] => {}
        other => return Err(format!("step=effects observed={other:?}").into()),
    }
    let refused = read_preview!(3, wire::Selection::Effects(bag.handle));
    if refused != wire::Payload::Refused {
        return Err(format!("step=container-effects observed={refused:?}").into());
    }
    if page.resources.dust != 0
        || page.resources.dust_limit != INITIAL_DUST_LIMIT
        || page.resources.slivers != SLIVERS_PER_CORE
        || page.resources.cores != 0
    {
        return Err(format!(
            "step=resources wrong fixture balances: {:?}",
            page.resources
        )
        .into());
    }
    let refused = read_preview!(4, wire::Selection::Resource(Operation::DustLimit));
    if refused != wire::Payload::Refused {
        return Err(format!("step=empty-dust-limit observed={refused:?}").into());
    }
    match read_preview!(5, wire::Selection::Resource(Operation::SliversToCores)) {
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
    match read_preview!(6, selection) {
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
        "two-tier-zero-swords,50-slivers,funded-case-30-dust",
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
    let character = &clients.characters[0];
    let mut connection = sqlx::PgConnection::connect(clients.url).await?;
    let custody = forge_custody(&mut connection, character).await?;
    if custody.gold != STARTER_GOLD
        || custody.slivers != i64::try_from(SLIVERS_PER_CORE)?
        || custody.swords != 2
    {
        return Err(format!(
            "step=pre-state observed swords={} slivers={} gold={} expected swords=2 slivers={SLIVERS_PER_CORE} gold={STARTER_GOLD}",
            custody.swords, custody.slivers, custody.gold
        ).into());
    }
    let outcome = verdict::observe(body(&mut client, &mut connection, character, &custody)).await;
    let closed = verdict::observe(finish(&mut client)).await;
    let resources = outcome?;
    closed?;
    drop(client);

    verify_relog(&clients, &mut connection, active, &custody, resources).await?;
    let credited = seed_funded_dust(&mut connection, &clients).await?;
    let funded = forge_custody(&mut connection, character).await?;
    let mut custody_with_dust = custody;
    custody_with_dust.dust.clone_from(&funded.dust);
    check_custody(&custody_with_dust, &funded)?;
    if credited != i64::from(FUNDED_DUST)
        || funded
            .dust
            .as_ref()
            .and_then(|dust| dust["balance"].as_u64())
            != Some(u64::from(FUNDED_DUST))
        || funded
            .dust
            .as_ref()
            .and_then(|dust| dust["dust_limit"].as_u64())
            != Some(u64::from(INITIAL_DUST_LIMIT))
    {
        return Err("step=funded-pre-state missing exact dust credit and initial limit".into());
    }

    let mut client = connect_character(&clients, 0x82, 0).await?;
    let funded_session = client.join_snapshot().game_session_id;
    let inventory = client
        .inventory()
        .ok_or("step=funded missing inventory")?
        .clone();
    let quote = verdict::observe(preview_unchanged(
        &mut client,
        1,
        wire::Selection::Resource(Operation::DustLimit),
        &inventory,
        &mut connection,
        character,
        &funded,
    ))
    .await
    .and_then(check_funded_dust_limit);
    let closed = verdict::observe(finish(&mut client)).await;
    quote?;
    closed?;
    drop(client);
    verify_relog(
        &clients,
        &mut connection,
        funded_session,
        &funded,
        wire::Resources {
            dust: FUNDED_DUST,
            ..resources
        },
    )
    .await
}

async fn seed_funded_dust(
    connection: &mut sqlx::PgConnection,
    clients: &SeamClients<'_>,
) -> TestResult<i64> {
    let mut tx = connection.begin().await?;
    let credited = sqlx::query_scalar(include_str!(
        "../../../../../tools/qualification/node_boot/g20_forge_dust.sql"
    ))
    .bind(clients.characters[0].as_slice())
    .bind(clients.world.as_slice())
    .bind(i64::from(FUNDED_DUST))
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(credited)
}

async fn verify_relog(
    clients: &SeamClients<'_>,
    connection: &mut sqlx::PgConnection,
    previous: oteryn_protocol_oteryn::GameSessionId,
    custody: &ForgeCustody,
    resources: wire::Resources,
) -> TestResult {
    let mut fresh = connect_character(clients, 0x82, 0).await?;
    if fresh.join_snapshot().game_session_id == previous {
        return Err("step=read-only-relog reused original session".into());
    }
    let inventory = fresh
        .inventory()
        .ok_or("step=relog missing inventory")?
        .clone();
    let durable = verdict::observe(preview_unchanged(
        &mut fresh,
        1,
        wire::Selection::Browse { after: 0 },
        &inventory,
        connection,
        &clients.characters[0],
        custody,
    ))
    .await
    .and_then(|reply| browse(&fresh, reply, custody));
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
    fn first_page_rejects_wrong_cursor_and_unexpected_continuation() -> TestResult {
        let mut page = wire::Page {
            after: 0,
            next: None,
            items: Vec::new(),
            resources: wire::Resources::default(),
        };
        check_first_page(&page)?;
        page.after = 1;
        assert!(check_first_page(&page).is_err());
        page.after = 0;
        page.next = Some(1);
        assert!(check_first_page(&page).is_err());
        Ok(())
    }

    #[test]
    fn funded_dust_limit_requires_the_exact_positive_quote() -> TestResult {
        assert!(check_funded_dust_limit(wire::Payload::Refused).is_err());
        let mut quote = wire::ResourceQuote {
            operation: Operation::DustLimit,
            full_resources: false,
            before: wire::Resources {
                dust: 30,
                dust_limit: 100,
                ..Default::default()
            },
            after_dust: 5,
            after_limit: 120,
            after_slivers: 0,
            after_cores: 0,
        };
        check_funded_dust_limit(wire::Payload::Resource(quote))?;
        quote.after_dust = 30;
        assert!(check_funded_dust_limit(wire::Payload::Resource(quote)).is_err());
        quote.after_dust = 5;
        quote.after_limit = 100;
        assert!(check_funded_dust_limit(wire::Payload::Resource(quote)).is_err());
        Ok(())
    }

    #[test]
    fn custody_rejects_equivalent_replacements_and_moved_slots() -> TestResult {
        let snapshot = || ForgeCustody {
            backpack: "backpack-a".into(),
            items: serde_json::json!([
                {"item_instance_id":"sword-a","placement_ordinal":1,"quantity":1,"tier":0},
                {"item_instance_id":"sword-b","placement_ordinal":2,"quantity":1,"tier":0},
                {"item_instance_id":"slivers-a","placement_ordinal":3,"quantity":50,"tier":0},
            ]),
            dust: None,
            gold: 50,
            slivers: 50,
            swords: 2,
            entry_count: 3,
        };
        let expected = snapshot();
        check_custody(&expected, &snapshot())?;
        let mut replaced_swords = snapshot();
        replaced_swords.items[0]["item_instance_id"] = "sword-c".into();
        replaced_swords.items[1]["item_instance_id"] = "sword-d".into();
        assert!(check_custody(&expected, &replaced_swords).is_err());
        let mut replaced_slivers = snapshot();
        replaced_slivers.items[2]["item_instance_id"] = "slivers-b".into();
        assert!(check_custody(&expected, &replaced_slivers).is_err());
        let mut moved = snapshot();
        moved.items[0]["placement_ordinal"] = 4.into();
        assert!(check_custody(&expected, &moved).is_err());
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
