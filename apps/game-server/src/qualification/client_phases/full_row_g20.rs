//! G20 Forge preview over a dedicated character carrying two tier-zero swords
//! and the exact sliver cost of one core. The previews must leave custody intact.

use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::item_view::CharacterInventory;
use oteryn_protocol_oteryn::{forge_preview as wire, item_forge::Operation};
use sha2::{Digest, Sha256};

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

async fn finish(client: &mut DevClientSession) -> TestResult {
    let (result, detail, attempts, _) = logout_once(client).await?;
    if result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) || attempts != 1 {
        return Err(format!("step=logout first attempt refused: {detail}").into());
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
    gate(client, CAPABILITY)?;
    let session = uuid_text(client.join_snapshot().game_session_id.as_bytes());
    let response = client
        .session_mut()
        .gated_command(CAPABILITY, wire::COMMAND_TYPE_FORGE_PREVIEW, &payload)
        .await?;
    if nonce == 2 && selection == wire::Selection::Resource(Operation::DustLimit) {
        evidence(&format!(
            "G20_FRAME game_session_id={session} command_id={} server_sequence={} status={} payload_sha256={}",
            response.command_id,
            response.server_sequence,
            response.status as u32,
            super::client_row_p20::hex(&Sha256::digest(&response.payload)),
        ));
        if let Some(directory) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
            let file = std::fs::File::create_new(Path::new(&directory).join("g20-funded.json"))?;
            serde_json::to_writer(
                file,
                &serde_json::json!({
                    "game_session_id": session,
                    "command_id": response.command_id,
                    "server_sequence": response.server_sequence,
                    "status": response.status as u32,
                    "payload": response.payload,
                }),
            )?;
        }
    }
    client.service_liveness(Duration::from_millis(50)).await?;
    require_mechanic_acceptance(response.status, "G20 Forge preview")?;
    let reply =
        wire::Reply::decode(&response.payload).ok_or("step=preview observed=decode-failed")?;
    if reply.nonce != nonce {
        return Err(format!("step=preview observed=nonce-{}-for-{nonce}", reply.nonce).into());
    }
    Ok(reply.payload)
}

#[derive(Clone, Debug, PartialEq, Eq, sqlx::FromRow)]
struct ForgeCustody {
    backpack: String,
    items: serde_json::Value,
    dust: Option<serde_json::Value>,
    other_custody: serde_json::Value,
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
        "WITH RECURSIVE owner AS (
            SELECT character_id, account_id, world_id FROM game_character_roots
            WHERE character_id = encode($1, 'hex')::uuid
        ), backpack AS (
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
        ), roots AS (
            SELECT b.item_instance_id, to_jsonb(b) AS placement FROM game_item_container_slots b JOIN owner o USING (character_id)
            UNION ALL
            SELECT e.item_instance_id, to_jsonb(e) FROM game_item_container_entries e JOIN owner o USING (character_id)
            UNION ALL
            SELECT e.item_instance_id, to_jsonb(e) FROM game_character_equipment_slots e JOIN owner o USING (character_id)
            UNION ALL
            SELECT d.item_instance_id, to_jsonb(d) FROM game_item_character_depot_locations d JOIN owner o USING (character_id)
            UNION ALL
            SELECT i.item_instance_id, to_jsonb(i) FROM game_item_character_inbox_locations i JOIN owner o USING (character_id)
        ), tree(item_instance_id, placement, path) AS (
            SELECT item_instance_id, placement, ARRAY[item_instance_id] FROM roots
            UNION ALL
            SELECT n.item_instance_id, to_jsonb(n), t.path || n.item_instance_id
            FROM tree t JOIN game_item_nested_entries n ON n.parent_item_instance_id = t.item_instance_id
            WHERE NOT n.item_instance_id = ANY(t.path)
        ), observed_items AS (
            SELECT t.item_instance_id, t.placement, to_jsonb(i) AS instance,
                   to_jsonb(tier_state) AS tier
            FROM tree t JOIN game_item_instances i USING (item_instance_id)
            LEFT JOIN game_item_tiers tier_state USING (item_instance_id)
            LIMIT 101
        )
        SELECT b.item_instance_id::text AS backpack,
               COALESCE(jsonb_agg(to_jsonb(e) ORDER BY e.placement_ordinal)
                   FILTER (WHERE e.item_instance_id IS NOT NULL), '[]'::jsonb) AS items,
               (SELECT to_jsonb(d) FROM game_character_forge_dust d
                   WHERE d.character_id = encode($1, 'hex')::uuid) AS dust,
               jsonb_build_object(
                   'items', (SELECT jsonb_agg(to_jsonb(i) ORDER BY item_instance_id) FROM observed_items i),
                   'dust_ledger', (SELECT jsonb_agg(to_jsonb(d) ORDER BY entry_id)
                       FROM game_character_forge_dust_entries d JOIN owner o USING (character_id)),
                   'stash', (SELECT jsonb_agg(to_jsonb(s) ORDER BY to_jsonb(s))
                       FROM game_character_stash_balances s JOIN owner o USING (character_id)),
                   'bank', (SELECT to_jsonb(b) FROM game_account_bank_balances b JOIN owner o USING (account_id, world_id)),
                   'bank_operations', (SELECT jsonb_agg(to_jsonb(p) ORDER BY operation_occurrence_id)
                       FROM game_account_bank_operations p JOIN owner o USING (account_id, world_id)),
                   'bank_entries', (SELECT jsonb_agg(to_jsonb(e) ORDER BY entry_id)
                       FROM game_account_bank_entries e JOIN owner o USING (account_id, world_id)),
                   'forge_cooldown', (SELECT to_jsonb(c) FROM game_forge_cooldowns c JOIN owner o USING (character_id)),
                   'forge_operations', (SELECT jsonb_agg(to_jsonb(p) ORDER BY to_jsonb(p))
                       FROM game_forge_operations p JOIN owner o USING (character_id)),
                   'forge_reservations', (SELECT jsonb_agg(to_jsonb(r) ORDER BY to_jsonb(r))
                       FROM game_forge_reservations r JOIN owner o USING (character_id)),
                   'forge_history', (SELECT jsonb_agg(to_jsonb(r) ORDER BY to_jsonb(r))
                       FROM game_forge_history r JOIN owner o USING (character_id)),
                   'dust_limit_reservations', (SELECT jsonb_agg(to_jsonb(r) ORDER BY to_jsonb(r))
                       FROM game_forge_dust_limit_reservations r JOIN owner o USING (character_id)),
                   'dust_limit_receipts', (SELECT jsonb_agg(to_jsonb(r) ORDER BY to_jsonb(r))
                       FROM game_forge_dust_limit_receipts r JOIN owner o USING (character_id)),
                   'dust_limit_history', (SELECT jsonb_agg(to_jsonb(r) ORDER BY to_jsonb(r))
                       FROM game_forge_dust_limit_history r JOIN owner o USING (character_id))
               ) AS other_custody,
               COALESCE(sum(e.quantity * CASE e.definition
                   WHEN 'oteryn:item.tibia.i3031' THEN 1
                   WHEN 'oteryn:item.tibia.i3035' THEN 100
                   WHEN 'oteryn:item.tibia.i3043' THEN 10000 ELSE 0 END)
                   FILTER (WHERE e.lifecycle = 1), 0)::bigint AS gold,
               COALESCE(sum(e.quantity) FILTER (WHERE e.lifecycle = 1
                   AND e.definition = 'oteryn:item.tibia.i37109'), 0)::bigint AS slivers,
               count(*) FILTER (WHERE e.lifecycle = 1 AND e.quantity = 1 AND e.tier = 0
                   AND e.definition = 'oteryn:item.tibia.i3264') AS swords,
               (SELECT count(*) FROM observed_items) AS entry_count
        FROM backpack b LEFT JOIN entries e ON TRUE
        GROUP BY b.item_instance_id",
    )
    .bind(character.as_slice())
    .fetch_one(connection)
    .await?;
    if custody.entry_count > 100 {
        return Err("step=custody character custody exceeds snapshot bound".into());
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

fn check_browse_suffix(first: &wire::Page, after: u64, reply: wire::Payload) -> TestResult {
    let wire::Payload::Page(page) = reply else {
        return Err("step=browse-cursor expected a page".into());
    };
    if page.after != after
        || page.next.is_some()
        || page.items != first.items[1..]
        || page.resources != first.resources
    {
        return Err(format!("step=browse-cursor after={after} observed={page:?}").into());
    }
    Ok(())
}

fn check_candidates(page: &wire::Page, expected: [u64; 3]) -> TestResult {
    let mut expected = expected;
    expected.sort_unstable();
    let actual: Vec<_> = page.items.iter().map(|item| item.handle.get()).collect();
    if actual != expected {
        return Err(format!(
            "step=browse candidate handles observed={actual:?} expected={expected:?}"
        )
        .into());
    }
    Ok(())
}

fn inventory_without_handles(inventory: &CharacterInventory) -> CharacterInventory {
    let mut inventory = inventory.clone();
    for entry in inventory
        .main_backpack
        .iter_mut()
        .chain(&mut inventory.entries)
        .chain(
            inventory
                .equipment
                .iter_mut()
                .map(|equipped| &mut equipped.item),
        )
    {
        entry.handle = std::num::NonZeroU64::MIN;
    }
    inventory
}

const FUNDED_DUST: u32 = 30;
const INITIAL_DUST_LIMIT: u32 = 100;
const FIRST_DUST_LIMIT_COST: u32 = 25;
// Global Dust Limit progression: https://tibia.fandom.com/wiki/Equipment_Upgrade?oldid=1205481
const NEXT_DUST_LIMIT: u32 = 101;

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
    if let wire::Payload::Resource(observed) = reply
        && observed.after_limit != expected.after_limit
    {
        return Err(format!(
            "step=funded-dust-limit observed={}->{} expected={}->{}",
            observed.before.dust_limit,
            observed.after_limit,
            expected.before.dust_limit,
            expected.after_limit,
        )
        .into());
    }
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

fn browse_metadata(item: &wire::Item, appearance: u16, definition_ref: u32) -> bool {
    let (name, container, classification, maximum_tier) = match appearance {
        2854 => ("backpack", true, 0, 0),
        SWORD => ("sword", false, 1, 1),
        _ => return false,
    };
    item.definition_ref == definition_ref
        && item.appearance == u32::from(appearance)
        && item.name == name
        && item.container == container
        && item.classification == classification
        && item.tier == 0
        && item.maximum_tier == maximum_tier
}

fn check_bank(resources: wire::Resources, bank: &serde_json::Value) -> TestResult {
    let gold = if bank.is_null() {
        0
    } else {
        bank["balance"]
            .as_u64()
            .ok_or("step=browse invalid SQL bank balance")?
    };
    // The dedicated q_g20 character has no starter-island restriction (BANK-0 §4.4).
    if resources.bank_gold != gold || !resources.bank_spendable {
        return Err(format!(
            "step=browse bank-gold={} expected={gold} bank-spendable={} expected=true",
            resources.bank_gold, resources.bank_spendable
        )
        .into());
    }
    Ok(())
}

fn browse(
    client: &DevClientSession,
    reply: wire::Payload,
    custody: &ForgeCustody,
) -> TestResult<(wire::Page, [wire::Item; 2], wire::Item)> {
    let wire::Payload::Page(page) = reply else {
        return Err("step=browse observed=not-a-page".into());
    };
    check_first_page(&page)?;
    check_bank(page.resources, &custody.other_custody["bank"])?;
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
        .find(|item| {
            item.handle == backpack.handle
                && backpack.appearance_id == 2854
                && backpack.count.get() == 1
                && browse_metadata(item, 2854, backpack.item_definition_ref.get())
        })
        .ok_or("step=browse carried backpack missing or wrong Forge metadata")?
        .clone();
    let swords = inventory
        .entries
        .iter()
        .filter(|item| item.appearance_id == SWORD)
        .collect::<Vec<_>>();
    let [one, two] = swords.as_slice() else {
        return Err("step=browse expected two fixture swords in inventory".into());
    };
    check_candidates(
        &page,
        [backpack.handle.get(), one.handle.get(), two.handle.get()],
    )?;
    let find = |entry: &oteryn_protocol_oteryn::item_view::ItemEntry| {
        page.items.iter().find(|item| {
            item.handle == entry.handle
                && entry.count.get() == 1
                && browse_metadata(item, SWORD, entry.item_definition_ref.get())
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
    let resources = check_quotes(
        client,
        connection,
        character,
        custody,
        &inventory,
        (page.resources, [first, second]),
        5,
    )
    .await?;
    let after = page.items[0].handle.get();
    check_browse_suffix(
        &page,
        after,
        read_preview!(7, wire::Selection::Browse { after }),
    )?;
    evidence(&format!("G20_BROWSE_CURSOR after={after} remaining=2"));
    Ok(resources)
}

async fn check_quotes(
    client: &mut DevClientSession,
    connection: &mut sqlx::PgConnection,
    character: &[u8; 16],
    custody: &ForgeCustody,
    inventory: &CharacterInventory,
    (resources, [first, second]): (wire::Resources, [wire::Item; 2]),
    nonce: u64,
) -> TestResult<wire::Resources> {
    macro_rules! read_preview {
        ($nonce:expr, $selection:expr) => {
            preview_unchanged(
                client, $nonce, $selection, inventory, connection, character, custody,
            )
            .await?
        };
    }
    match read_preview!(nonce, wire::Selection::Resource(Operation::SliversToCores)) {
        wire::Payload::Resource(quote)
            if quote.operation == Operation::SliversToCores
                && quote.full_resources
                && quote.before == resources
                && quote.after_dust == resources.dust
                && quote.after_limit == resources.dust_limit
                && quote.after_slivers == 0
                && resources.cores.checked_add(1) == Some(quote.after_cores) => {}
        other => return Err(format!("step=resource observed={other:?}").into()),
    }
    let selection = wire::Selection::Pair {
        operation: Operation::Fusion,
        first: first.handle,
        second: second.handle,
        boost: false,
        protect: false,
    };
    match read_preview!(nonce + 1, selection) {
        wire::Payload::Quote(quote)
            if quote.selection == selection
                && quote.first == first
                && quote.second == second
                && quote.resources == resources
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
            evidence(&format!(
                "G20_QUOTES fusion_bank_gold={} slivers_to_core_bank_gold={}",
                resources.bank_gold, resources.bank_gold,
            ));
            Ok(resources)
        }
        other => Err(format!("step=pair observed={other:?}").into()),
    }
}

const BANK_FIXTURE_GOLD: i64 = 37;

async fn seed_g20_bank_gold(
    connection: &mut sqlx::PgConnection,
    character: &[u8; 16],
    channel: &[u8; 16],
) -> TestResult {
    // Match the native Forge PostgreSQL bank fixture. Only the input deposit
    // bypasses ledger triggers in this disposable database.
    let mut transaction = connection.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *transaction)
        .await?;
    let seeded = sqlx::query(
        "WITH operation AS (
            INSERT INTO game_account_bank_operations (
                operation_occurrence_id, transaction_id, event_id, kind, request_binding,
                acting_character_id, account_id, world_id, channel_id,
                runtime_scope_ownership_generation, amount, outcome, occurred_at,
                envelope_sha256, committed_at, planned_platinum_item_instance_id,
                planned_gold_item_instance_id, backpack_item_instance_id
            )
            SELECT game_character_uuid_v7(), game_character_uuid_v7(), game_character_uuid_v7(),
                1, sha256('g20-bank-fixture'), c.character_id, c.account_id, c.world_id,
                encode($2, 'hex')::uuid, 1, $3, 0, 1, sha256('g20-bank-fixture'), 0,
                game_character_uuid_v7(), game_character_uuid_v7(), b.item_instance_id
            FROM game_character_roots c JOIN game_item_container_slots b USING (character_id)
            WHERE c.character_id = encode($1, 'hex')::uuid
            RETURNING transaction_id, account_id, world_id, acting_character_id
        ), entry AS (
            INSERT INTO game_account_bank_entries (
                entry_id, transaction_id, account_id, world_id, kind, amount,
                balance_before, balance_after, acting_character_id
            )
            SELECT game_character_uuid_v7(), transaction_id, account_id, world_id,
                1, $3, 0, $3, acting_character_id
            FROM operation RETURNING entry_id, account_id, world_id
        )
        INSERT INTO game_account_bank_balances (account_id, world_id, balance, last_entry_id)
        SELECT account_id, world_id, $3, entry_id FROM entry",
    )
    .bind(character.as_slice())
    .bind(channel.as_slice())
    .bind(BANK_FIXTURE_GOLD)
    .execute(&mut *transaction)
    .await?;
    if seeded.rows_affected() != 1 {
        return Err("step=fixture-bank expected one funded dedicated account".into());
    }
    transaction.commit().await?;
    evidence(&format!(
        "FIXTURE row=G20.full input=bank_gold balance={BANK_FIXTURE_GOLD}"
    ));
    Ok(())
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
    let character = &clients.characters[0];
    let mut connection = sqlx::PgConnection::connect(clients.url).await?;
    let custody = forge_custody(&mut connection, character).await?;
    if !custody.other_custody["bank"].is_null() {
        return Err("step=pre-state expected an unfunded bank account".into());
    }
    if custody.gold != STARTER_GOLD
        || custody.slivers != i64::try_from(SLIVERS_PER_CORE)?
        || custody.swords != 2
    {
        return Err(format!(
            "step=pre-state observed swords={} slivers={} gold={} expected swords=2 slivers={SLIVERS_PER_CORE} gold={STARTER_GOLD}",
            custody.swords, custody.slivers, custody.gold
        ).into());
    }
    let original_inventory = inventory_without_handles(
        client
            .inventory()
            .ok_or("step=pre-state missing inventory")?,
    );
    let outcome = verdict::observe(body(&mut client, &mut connection, character, &custody)).await;
    let closed = verdict::observe(finish(&mut client)).await;
    let resources = outcome?;
    closed?;
    drop(client);

    verify_relog(
        &clients,
        &mut connection,
        active,
        &custody,
        resources,
        &original_inventory,
    )
    .await?;
    let mut client = connect_character(&clients, 0x82, 0).await?;
    let funded_session = client.join_snapshot().game_session_id;
    request_fixture_control(
        uuid_text(funded_session.as_bytes()),
        "--profile",
        "g20-dust-input".into(),
    )
    .await?;
    seed_g20_bank_gold(&mut connection, character, &clients.channel).await?;
    let funded = forge_custody(&mut connection, character).await?;
    let balances = funded.dust.as_ref();
    let dust = balances
        .and_then(|row| row["balance"].as_u64())
        .unwrap_or(0);
    let limit = balances
        .and_then(|row| row["dust_limit"].as_u64())
        .unwrap_or(u64::from(INITIAL_DUST_LIMIT));
    if dust != u64::from(FUNDED_DUST) || limit != u64::from(INITIAL_DUST_LIMIT) {
        return Err(format!(
            "step=funded-pre-state observed=dust-{dust}-limit-{limit} required=dust-{FUNDED_DUST}-limit-{INITIAL_DUST_LIMIT}"
        ).into());
    }
    let mut expected_funded = custody.clone();
    expected_funded.dust = funded.dust.clone();
    let bank = &funded.other_custody["bank"];
    if bank["balance"].as_i64() != Some(BANK_FIXTURE_GOLD) {
        return Err("step=funded-pre-state expected the exact nonzero bank fixture balance".into());
    }
    expected_funded.other_custody["bank"] = bank.clone();
    for key in ["bank_operations", "bank_entries"] {
        let rows = &funded.other_custody[key];
        if !custody.other_custody[key].is_null() || rows.as_array().map(Vec::len) != Some(1) {
            return Err(format!("step=funded-pre-state expected one fixture {key} row").into());
        }
        expected_funded.other_custody[key] = rows.clone();
    }
    let ledger = &funded.other_custody["dust_ledger"];
    let Some([credit]) = ledger.as_array().map(Vec::as_slice) else {
        return Err("step=funded-pre-state expected exactly one fixture dust entry".into());
    };
    if credit["entry_id"] != funded.dust.as_ref().ok_or("missing fixture balance")?["last_entry_id"]
        || credit["balance_before"].as_u64() != Some(0)
        || credit["balance_after"].as_u64() != Some(dust)
        || credit["dust_limit_before"].as_u64() != Some(limit)
        || credit["dust_limit_after"].as_u64() != Some(limit)
    {
        return Err("step=funded-pre-state fixture ledger does not match its balance".into());
    }
    expected_funded.other_custody["dust_ledger"] = ledger.clone();
    check_custody(&expected_funded, &funded)?;
    evidence(&format!(
        "FIXTURE row=G20.full input=forge_dust balance={dust} limit={limit}"
    ));

    let inventory = client
        .inventory()
        .ok_or("step=funded missing inventory")?
        .clone();
    let funded_resources = wire::Resources {
        dust: FUNDED_DUST,
        bank_gold: u64::try_from(BANK_FIXTURE_GOLD)?,
        ..resources
    };
    let checks = verdict::observe(async {
        let page = preview_unchanged(
            &mut client,
            1,
            wire::Selection::Browse { after: 0 },
            &inventory,
            &mut connection,
            character,
            &funded,
        )
        .await?;
        let (page, pair, _) = browse(&client, page, &funded)?;
        if page.resources != funded_resources {
            return Err("step=funded browse before quote changed resources".into());
        }
        check_quotes(
            &mut client,
            &mut connection,
            character,
            &funded,
            &inventory,
            (page.resources, pair),
            4,
        )
        .await?;
        let quote = check_funded_dust_limit(
            preview_unchanged(
                &mut client,
                2,
                wire::Selection::Resource(Operation::DustLimit),
                &inventory,
                &mut connection,
                character,
                &funded,
            )
            .await?,
        );
        let page = preview_unchanged(
            &mut client,
            3,
            wire::Selection::Browse { after: 0 },
            &inventory,
            &mut connection,
            character,
            &funded,
        )
        .await?;
        if browse(&client, page, &funded)?.0.resources != funded_resources {
            return Err("step=funded browse after quote changed resources".into());
        }
        quote
    })
    .await;
    let closed = verdict::observe(finish(&mut client)).await;
    drop(client);
    let relog = async {
        closed?;
        verify_relog(
            &clients,
            &mut connection,
            funded_session,
            &funded,
            funded_resources,
            &original_inventory,
        )
        .await
    }
    .await;
    match (checks, relog) {
        (Err(quote), Err(relog)) => Err(format!("{quote}; relog also failed: {relog}").into()),
        (checks, relog) => checks.and(relog),
    }
}

async fn verify_relog(
    clients: &SeamClients<'_>,
    connection: &mut sqlx::PgConnection,
    previous: oteryn_protocol_oteryn::GameSessionId,
    custody: &ForgeCustody,
    resources: wire::Resources,
    original_inventory: &CharacterInventory,
) -> TestResult {
    let mut fresh = connect_character(clients, 0x82, 0).await?;
    if fresh.join_snapshot().game_session_id == previous {
        return Err("step=read-only-relog reused original session".into());
    }
    let inventory = fresh
        .inventory()
        .ok_or("step=relog missing inventory")?
        .clone();
    if &inventory_without_handles(&inventory) != original_inventory {
        return Err("step=relog inventory differs from original admission".into());
    }
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
    let actual = durable?.0.resources;
    if actual != resources {
        return Err("step=read-only-relog Forge previews changed resources".into());
    }
    closed?;
    evidence(&format!(
        "G20_RELOG dust={} dust_limit={} bank_gold={} bank_spendable={}",
        actual.dust, actual.dust_limit, actual.bank_gold, actual.bank_spendable,
    ));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn browse_suffix_rejects_a_repeated_first_page() -> TestResult {
        let first = wire::Page {
            after: 0,
            next: None,
            items: (0..3)
                .map(|value| wire::Item {
                    handle: std::num::NonZeroU64::MIN.saturating_add(value),
                    definition_ref: 1,
                    appearance: u32::from(SWORD),
                    name: "sword".into(),
                    container: false,
                    classification: 1,
                    tier: 0,
                    maximum_tier: 1,
                })
                .collect(),
            resources: wire::Resources::default(),
        };
        let mut repeated = first.clone();
        repeated.after = 1;
        assert!(check_browse_suffix(&first, 1, wire::Payload::Page(repeated.clone())).is_err());
        repeated.items.remove(0);
        check_browse_suffix(&first, 1, wire::Payload::Page(repeated))
    }

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
    fn browse_rejects_extra_missing_and_out_of_order_candidates() -> TestResult {
        let item = |handle| -> TestResult<wire::Item> {
            Ok(wire::Item {
                handle: std::num::NonZeroU64::new(handle).ok_or("test handle is zero")?,
                definition_ref: SWORD.into(),
                appearance: SWORD.into(),
                name: "Sword".into(),
                classification: 1,
                tier: 0,
                maximum_tier: 1,
                container: false,
            })
        };
        let mut page = wire::Page {
            after: 0,
            next: None,
            items: vec![item(1)?, item(2)?, item(3)?],
            resources: wire::Resources::default(),
        };
        let expected = [3, 1, 2];
        check_candidates(&page, expected)?;
        page.items.swap(1, 2);
        assert!(check_candidates(&page, expected).is_err());
        page.items.swap(1, 2);
        page.items.push(item(4)?);
        assert!(check_candidates(&page, expected).is_err());
        page.items.truncate(2);
        assert!(check_candidates(&page, expected).is_err());
        Ok(())
    }

    #[test]
    fn relog_inventory_ignores_session_handles_but_rejects_changed_quantities() -> TestResult {
        let first = CharacterInventory {
            entries: vec![oteryn_protocol_oteryn::item_view::ItemEntry {
                handle: std::num::NonZeroU64::MIN,
                item_definition_ref: std::num::NonZeroU32::MIN,
                appearance_id: SWORD,
                count: std::num::NonZeroU32::MIN,
                sub_type: 0,
            }],
            ..Default::default()
        };
        let mut fresh = first.clone();
        fresh.entries[0].handle = std::num::NonZeroU64::new(2).ok_or("test handle")?;
        assert_eq!(
            inventory_without_handles(&first),
            inventory_without_handles(&fresh)
        );
        fresh.entries[0].count = std::num::NonZeroU32::new(2).ok_or("test count")?;
        assert_ne!(
            inventory_without_handles(&first),
            inventory_without_handles(&fresh)
        );
        Ok(())
    }

    #[test]
    fn browse_bank_fields_require_independent_balance_and_eligibility() -> TestResult {
        let bank = serde_json::json!({"balance": 42});
        let mut resources = wire::Resources {
            bank_gold: 42,
            bank_spendable: true,
            ..Default::default()
        };
        check_bank(resources, &bank)?;
        resources.bank_gold = 43;
        assert!(check_bank(resources, &bank).is_err());
        resources.bank_gold = 42;
        resources.bank_spendable = false;
        assert!(check_bank(resources, &bank).is_err());
        resources.bank_gold = 0;
        assert!(check_bank(resources, &serde_json::Value::Null).is_err());
        resources.bank_spendable = true;
        check_bank(resources, &serde_json::Value::Null)
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
            after_limit: 101,
            after_slivers: 0,
            after_cores: 0,
        };
        check_funded_dust_limit(wire::Payload::Resource(quote))?;
        quote.after_dust = 30;
        assert!(check_funded_dust_limit(wire::Payload::Resource(quote)).is_err());
        quote.after_dust = 5;
        quote.after_limit = 120;
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
            other_custody: serde_json::json!({"dust_ledger":[],"items":[{"slot":"equipment","quantity":1}]}),
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
        let mut edited_equipment = snapshot();
        edited_equipment.other_custody["items"][0]["quantity"] = 2.into();
        assert!(check_custody(&expected, &edited_equipment).is_err());
        let mut appended_dust = snapshot();
        appended_dust.other_custody["dust_ledger"] = serde_json::json!([{"amount":1}]);
        assert!(check_custody(&expected, &appended_dust).is_err());
        for table in [
            "bank_operations",
            "bank_entries",
            "forge_history",
            "dust_limit_reservations",
            "dust_limit_receipts",
            "dust_limit_history",
        ] {
            let mut changed = snapshot();
            changed.other_custody[table] = serde_json::json!([{"unexpected": true}]);
            assert!(check_custody(&expected, &changed).is_err(), "{table}");
        }

        Ok(())
    }

    #[test]
    fn browse_rejects_drift_in_sword_and_backpack_metadata() {
        for (appearance, name, container, classification, maximum_tier) in [
            (SWORD, "sword", false, 1, 1),
            (2854, "backpack", true, 0, 0),
        ] {
            let expected = wire::Item {
                handle: std::num::NonZeroU64::MIN,
                definition_ref: 42,
                appearance: u32::from(appearance),
                name: name.into(),
                container,
                classification,
                tier: 0,
                maximum_tier,
            };
            assert!(browse_metadata(&expected, appearance, 42));
            for wrong in [
                wire::Item {
                    name: "wrong".into(),
                    ..expected.clone()
                },
                wire::Item {
                    appearance: 1,
                    ..expected.clone()
                },
                wire::Item {
                    definition_ref: 1,
                    ..expected.clone()
                },
                wire::Item {
                    container: !container,
                    ..expected.clone()
                },
                wire::Item {
                    classification: 2,
                    ..expected.clone()
                },
                wire::Item {
                    tier: 1,
                    ..expected.clone()
                },
                wire::Item {
                    maximum_tier: 2,
                    ..expected.clone()
                },
            ] {
                assert!(!browse_metadata(&wrong, appearance, 42));
            }
        }
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
