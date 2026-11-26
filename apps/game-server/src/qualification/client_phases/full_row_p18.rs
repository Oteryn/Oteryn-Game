//! P18 Exaltation Forge full-row journey.
//!
//! Costs and tiers follow Global's Equipment Upgrade reference:
//! https://tibia.fandom.com/wiki/Equipment_Upgrade
//! Each case is prepared offline and submits one live command before relog.

use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::forge_preview as preview;
use oteryn_protocol_oteryn::item_forge::{
    CAPABILITY_ITEM_FORGE_V1, COMMAND_TYPE_ITEM_FORGE, Operation, Request,
};
use std::num::NonZeroU64;

#[derive(Debug)]
struct Observation {
    operation: Operation,
    status: CommandStatus,
    body: serde_json::Value,
    changed: bool,
}

fn occurrence(session: oteryn_protocol_oteryn::GameSessionId, ordinal: u8) -> [u8; 16] {
    use sha2::Digest;
    let digest = sha2::Sha256::new()
        .chain_update(b"E2E-P18/occurrence")
        .chain_update(session.as_bytes())
        .chain_update([ordinal])
        .finalize();
    let mut value = [0; 16];
    value.copy_from_slice(&digest[..16]);
    value[6] = (value[6] & 15) | 0x70;
    value[8] = (value[8] & 63) | 0x80;
    value
}

const FUSION_PAIR: &str = "oteryn:item.tibia.i3267";

/// Item handles are issued by each session. Compare the complete carried view
/// after removing only those handle values.
pub(super) fn durable_inventory(
    mut inventory: oteryn_protocol_oteryn::item_view::CharacterInventory,
) -> oteryn_protocol_oteryn::item_view::CharacterInventory {
    for item in inventory
        .entries
        .iter_mut()
        .chain(&mut inventory.main_backpack)
        .chain(inventory.equipment.iter_mut().map(|entry| &mut entry.item))
    {
        item.handle = NonZeroU64::MIN;
    }
    inventory
}

async fn observe_inventory(
    expect_change: bool,
    mut poll: impl AsyncFnMut(Duration) -> TestResult<bool>,
) -> TestResult<bool> {
    // The receipt precedes the inventory projection, which may complete on a
    // subsequent server refresh. Only read from the existing session here.
    let slices = if expect_change { 20 } else { 1 };
    for _ in 0..slices {
        if poll(Duration::from_millis(50)).await? {
            return Ok(true);
        }
    }
    Ok(false)
}

async fn command(
    client: &mut DevClientSession,
    operation: Operation,
    occurrence: [u8; 16],
    pair: Option<(NonZeroU64, NonZeroU64)>,
    history_limit: u8,
) -> TestResult<Observation> {
    let before = client
        .inventory()
        .cloned()
        .ok_or("inventory frame missing")?;
    let payload = Request {
        operation,
        occurrence,
        first_item: pair.map(|value| value.0),
        second_item: pair.map(|value| value.1),
        boost: false,
        protect: false,
        history_limit,
    }
    .encode()
    .ok_or("P18 request was not canonical")?;
    let reply = client
        .session_mut()
        .gated_command(CAPABILITY_ITEM_FORGE_V1, COMMAND_TYPE_ITEM_FORGE, &payload)
        .await?;
    let body = serde_json::from_slice::<serde_json::Value>(&reply.payload)
        .unwrap_or(serde_json::Value::Null);
    let expect_change = operation.changes_inventory()
        && reply.status == CommandStatus::Accepted
        && body["applied"] == true;
    let changed = observe_inventory(expect_change, async |duration| {
        client.service_liveness(duration).await?;
        Ok(client.inventory() != Some(&before))
    })
    .await?;
    Ok(Observation {
        operation,
        status: reply.status,
        body,
        changed,
    })
}

fn validate(observation: &Observation) -> TestResult<bool> {
    if observation.status != CommandStatus::Accepted || !observation.body.is_object() {
        return Err(format!(
            "step-{:?} observed status={:?} payload={}",
            observation.operation, observation.status, observation.body
        )
        .into());
    }
    let applied = observation.body["applied"].as_bool().unwrap_or(false);
    if applied != observation.changed && observation.operation.changes_inventory() {
        return Err(format!(
            "step-{:?} reply/delta mismatch applied={applied} inventory_changed={}",
            observation.operation, observation.changed
        )
        .into());
    }
    if !applied
        && (observation.body["reason"].as_str().is_none() || observation.changed)
        && observation.operation != Operation::QueryHistory
    {
        return Err(format!(
            "step-{:?} refusal lacked reason or changed inventory: {}",
            observation.operation, observation.body
        )
        .into());
    }
    Ok(applied)
}

fn validate_history(step: &str, observation: &Observation, applied: &[&Observation]) -> TestResult {
    if observation.status != CommandStatus::Accepted {
        return Err(format!(
            "{step} HISTORY observed status={:?} payload={}",
            observation.status, observation.body
        )
        .into());
    }
    let entries = observation.body.as_array().ok_or_else(|| {
        format!(
            "{step} HISTORY payload was not an array: {}",
            observation.body
        )
    })?;
    if entries.len() != applied.len() {
        return Err(format!(
            "{step} HISTORY receipt count mismatch expected={} observed={} payload={}",
            applied.len(),
            entries.len(),
            observation.body
        )
        .into());
    }
    for (entry, expected) in entries.iter().zip(applied.iter().rev()) {
        if entry["receipt"] != expected.body {
            return Err(format!(
                "{step} HISTORY receipt mismatch for {:?}: observed={} expected={}",
                expected.operation, entry, expected.body
            )
            .into());
        }
    }
    Ok(())
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step-7 ordinary logout refused".into());
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let dedicated = character_prep::dedicated_character(clients, "NODE_BOOT_Q_P18_ACCOUNT").await?;
    let dedicated = dedicated.clients(clients);
    let mut fixture = connect_character(&dedicated, 0xd7, 0).await?;
    character_prep::grant_item(&fixture, character_prep::item_id(FUSION_PAIR)?, 2).await?;
    character_prep::grant_item(&fixture, 3043, 3).await?;
    logout(&mut fixture).await?;
    drop(fixture);
    super::client_row_p18::seed_dust("P18.full", &dedicated).await?;

    let mut client = connect_character(&dedicated, 0xd8, 0).await?;
    let session = client.join_snapshot().game_session_id;
    let first_history = command(&mut client, Operation::QueryHistory, [0; 16], None, 50).await?;
    validate_history("step-2", &first_history, &[])?;

    let definitions = provisioning::item_index()?;
    let revision = definitions
        .revision(FUSION_PAIR)
        .ok_or("forge dagger definition missing")?;
    let pair_ref = definitions
        .definition_ref("Item", FUSION_PAIR, revision)
        .ok_or("forge dagger reference missing")?;
    let inventory = client
        .inventory()
        .ok_or("forge fixture inventory missing")?;
    let pair_count = inventory
        .entries
        .iter()
        .filter(|entry| entry.item_definition_ref == pair_ref && entry.count.get() == 1)
        .count();
    if pair_count != 2 {
        return Err(format!("forge fixture expected two single daggers, got {pair_count}").into());
    }

    let coins = [(3031, 1), (3035, 100), (3043, 10_000)]
        .into_iter()
        .map(|(id, worth)| {
            let key = format!("oteryn:item.tibia.i{id}");
            let revision = definitions
                .revision(&key)
                .ok_or("Forge coin definition missing")?;
            let reference = definitions
                .definition_ref("Item", &key, revision)
                .ok_or("Forge coin reference missing")?;
            Ok((reference, worth))
        })
        .collect::<TestResult<Vec<_>>>()?;
    let gold_before = carried_gold(&client, &coins);
    if gold_before < 30_000 || dagger_tiers(&mut client, pair_ref.get()).await? != [0, 0] {
        return Err(
            "valid-pair Fusion fixture lacks two tier-zero daggers or its gold and dust".into(),
        );
    }
    let initial_dust = super::client_row_p18::dust(&mut client).await?;
    if initial_dust != (100, 100) {
        return Err(format!(
            "funded Fusion expected fixture dust 100/100, observed {initial_dust:?}"
        )
        .into());
    }
    let mut fusion_outcome = None;
    let operations = [Operation::Fusion];
    let mut observations = Vec::new();
    for (index, operation) in operations.into_iter().enumerate() {
        let pair = if operation.requires_pair() {
            let preferred = (operation == Operation::Fusion).then_some(pair_ref.get());
            let Some(pair) = quoted_pair(&mut client, operation, preferred).await? else {
                return Err("seeded Fusion pair missing".into());
            };
            Some(pair)
        } else {
            None
        };
        // The shared Forge cooldown applies after an evaluated refusal too.
        client.service_liveness(Duration::from_millis(490)).await?;
        let observed = command(
            &mut client,
            operation,
            occurrence(session, u8::try_from(index + 1)?),
            pair,
            0,
        )
        .await?;
        let did_apply = validate(&observed)?;
        if operation == Operation::Fusion {
            if !did_apply {
                return Err(format!(
                    "valid-pair Fusion refused with admitted dust {initial_dust:?}: {}",
                    observed.body
                )
                .into());
            }
            let success = observed.body["succeeded"]
                .as_bool()
                .ok_or("Fusion success flag missing")?;
            let bonus = observed.body["bonus"]
                .as_u64()
                .ok_or("Fusion bonus missing")?;
            if (!success && bonus != 0) || bonus > 7 || !observed.body["reason"].is_null() {
                return Err(format!("invalid Fusion outcome: {}", observed.body).into());
            }
            let target = u8::from(success);
            let source = match bonus {
                4 | 5 => Some(0),
                6 => Some(1),
                _ => None,
            };
            let (dust_paid, gold_paid) = (
                if bonus == 1 { 0 } else { 100 },
                if bonus == 3 { 0 } else { 25_000 },
            );
            let paid = &observed.body["result"];
            let (first, second) = pair.ok_or("Fusion pair missing")?;
            if *paid
                != serde_json::json!({
                    "first_item":first.get(),"first_tier":target,"second_item":second.get(),"second_tier":source,
                    "dust":100-dust_paid,"dust_limit":100,"gold_spent":gold_paid,"cores_spent":0,
                    "slivers_spent":0,"slivers_created":0,"cores_created":0
                })
            {
                return Err(format!("Fusion exact receipt mismatch: {paid}").into());
            }
            let mut expected_tiers = vec![target];
            expected_tiers.extend(source);
            expected_tiers.sort_unstable();
            if super::client_row_p18::dust(&mut client).await? != (100 - dust_paid, 100)
                || carried_gold(&client, &coins) != gold_before - gold_paid
                || dagger_tiers(&mut client, pair_ref.get()).await? != expected_tiers
            {
                return Err("Fusion exact live gold, dust, or dagger tiers mismatch".into());
            }
            fusion_outcome = Some((expected_tiers, success, dust_paid, gold_paid));
        }
        observations.push(observed);
    }
    client.service_liveness(Duration::from_millis(490)).await?;
    let history = command(&mut client, Operation::QueryHistory, [0; 16], None, 50).await?;
    let applied_observations = observations
        .iter()
        .filter(|observation| observation.body["applied"] == true)
        .collect::<Vec<_>>();
    validate_history("step-6", &history, &applied_observations)?;
    let retained = client
        .inventory()
        .cloned()
        .ok_or("step-6 inventory absent")?;
    let retained_balance = super::client_row_p18::dust(&mut client).await?;
    logout(&mut client).await?;
    drop(client);
    let mut fresh = connect_character(&dedicated, 0xd9, 0).await?;
    if fresh.join_snapshot().game_session_id == session
        || fresh.inventory().cloned().map(durable_inventory) != Some(durable_inventory(retained))
    {
        return Err("step-7 fresh admission session/inventory durability mismatch".into());
    }
    let (expected_tiers, succeeded, dust_paid, gold_paid) =
        fusion_outcome.ok_or("funded valid-pair Fusion was never exercised")?;
    if dagger_tiers(&mut fresh, pair_ref.get()).await? != expected_tiers
        || super::client_row_p18::dust(&mut fresh).await? != retained_balance
    {
        return Err("Fusion dagger tiers or Forge balance did not persist across relog".into());
    }
    let fresh_history = command(&mut fresh, Operation::QueryHistory, [0; 16], None, 50).await?;
    logout(&mut fresh).await?;
    validate_history("step-7", &fresh_history, &applied_observations)?;
    if fresh_history.body != history.body {
        return Err(format!("step-7 fresh HISTORY mismatch: {fresh_history:?}").into());
    }
    evidence(&format!(
        "P18_FUSION_CHECK status=PASS succeeded={succeeded} dust_paid={dust_paid} gold_paid={gold_paid} durable_dagger_tiers={expected_tiers:?}"
    ));
    let mut failures = Vec::new();
    if !succeeded {
        failures.push("Fusion: first attempt rolled a natural failure".to_owned());
    }
    for (ordinal, case) in CASES.iter().enumerate() {
        if let Err(error) = run_case(&dedicated, case, 0xe0 + u8::try_from(ordinal)? * 3).await {
            let failure = format!("{:?}: {error}", case.operation);
            evidence(&format!("P18_CASE_FAIL {failure}"));
            failures.push(failure);
        }
    }
    if !failures.is_empty() {
        return Err(failures.join("; ").into());
    }
    Ok(())
}

struct ForgeCase {
    operation: Operation,
    /// (Tibia item ID, input tier), target first.
    pair: Option<[(u32, u8); 2]>,
    gold: i64,
    dust: i64,
    cores: u64,
}

const CASES: [ForgeCase; 6] = [
    ForgeCase {
        operation: Operation::Transfer,
        pair: Some([(34091, 0), (28716, 2)]),
        gold: 20_000_000,
        dust: 100,
        cores: 1,
    },
    ForgeCase {
        operation: Operation::DustToSlivers,
        pair: None,
        gold: 0,
        dust: 60,
        cores: 0,
    },
    ForgeCase {
        operation: Operation::SliversToCores,
        pair: None,
        gold: 0,
        dust: 0,
        cores: 0,
    },
    ForgeCase {
        operation: Operation::DustLimit,
        pair: None,
        gold: 0,
        dust: 25,
        cores: 0,
    },
    ForgeCase {
        operation: Operation::ConvergenceFusion,
        pair: Some([(28717, 0), (34090, 0)]),
        gold: 55_000_000,
        dust: 130,
        cores: 0,
    },
    ForgeCase {
        operation: Operation::ConvergenceTransfer,
        pair: Some([(43876, 0), (43885, 1)]),
        gold: 65_000_000,
        dust: 160,
        cores: 1,
    },
];

type FixtureDb = sqlx::PgConnection;

async fn fixture_dust(tx: &mut FixtureDb, character: &str, spend: i64) -> TestResult<(u64, u64)> {
    sqlx::query("INSERT INTO game_character_forge_dust(character_id,balance,dust_limit) VALUES($1::uuid,0,100) ON CONFLICT DO NOTHING")
        .bind(character).execute(&mut *tx).await?;
    let (mut balance, mut limit): (i64, i16) = sqlx::query_as(
        "SELECT balance,dust_limit FROM game_character_forge_dust WHERE character_id=$1::uuid FOR UPDATE")
        .bind(character).fetch_one(&mut *tx).await?;
    // Historical cap prerequisites must follow the deployed ledger's constraint.
    // The live DustLimit case separately checks Global's +1 transition.
    while spend > i64::from(limit) {
        let cost = 25 + i64::from((limit - 100) / 20);
        if balance < cost {
            append_dust(tx, character, balance, limit, cost - balance, "GAIN").await?;
            balance = cost;
        }
        append_dust(tx, character, balance, limit, cost, "LIMIT_RAISE").await?;
        balance -= cost;
        limit += 20;
    }
    if balance < spend {
        append_dust(tx, character, balance, limit, spend - balance, "GAIN").await?;
    } else if balance > spend {
        append_dust(tx, character, balance, limit, balance - spend, "SPEND").await?;
    }
    Ok((u64::try_from(spend)?, u64::try_from(limit)?))
}

async fn append_dust(
    tx: &mut FixtureDb,
    character: &str,
    before: i64,
    limit: i16,
    amount: i64,
    kind: &str,
) -> TestResult {
    sqlx::query(
        "WITH e AS (
        INSERT INTO game_character_forge_dust_entries
          (entry_id,character_id,previous_entry_id,kind,cause,cause_occurrence_id,transaction_id,
           amount,lost_amount,balance_before,balance_after,dust_limit_before,dust_limit_after)
        SELECT game_character_uuid_v7(),character_id,last_entry_id,
           $5,
           CASE $5 WHEN 'LIMIT_RAISE' THEN 'forge_dust_limit'
                   WHEN 'SPEND' THEN 'proficiency' ELSE 'creature_kill' END,
           game_character_uuid_v7(),game_character_uuid_v7(),$4,0,$2,
           CASE WHEN $5='GAIN' THEN $2+$4 ELSE $2-$4 END,$3,
           CASE WHEN $5='LIMIT_RAISE' THEN $3+20 ELSE $3 END
        FROM game_character_forge_dust WHERE character_id=$1::uuid
        RETURNING entry_id,balance_after,dust_limit_after)
        UPDATE game_character_forge_dust SET balance=e.balance_after,dust_limit=e.dust_limit_after,
        last_entry_id=e.entry_id FROM e WHERE character_id=$1::uuid",
    )
    .bind(character)
    .bind(before)
    .bind(limit)
    .bind(amount)
    .bind(kind)
    .execute(tx)
    .await?;
    Ok(())
}

async fn bank_balance(tx: &mut FixtureDb, clients: &SeamClients<'_>) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT coalesce((SELECT balance FROM game_account_bank_balances
        WHERE account_id=$1::uuid AND world_id=$2::uuid),0)::bigint",
    )
    .bind(&clients.accounts[0])
    .bind(uuid_text(&clients.world))
    .fetch_one(tx)
    .await?)
}

async fn fixture_bank(
    tx: &mut FixtureDb,
    clients: &SeamClients<'_>,
    character: &str,
    amount: i64,
) -> TestResult<i64> {
    let mut before = bank_balance(tx, clients).await?;
    // Use the bank PG fixture's seed pattern in this offline fixture transaction.
    // Every live fee runs in a new session with all triggers active.
    if amount == 0 {
        return Ok(before);
    }
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    let mut remaining = amount;
    while remaining > 0 {
        let chunk = remaining.min(20_000_000);
        sqlx::query("WITH op AS (
          INSERT INTO game_account_bank_operations(operation_occurrence_id,transaction_id,event_id,kind,
            request_binding,acting_character_id,account_id,world_id,channel_id,runtime_scope_ownership_generation,
            amount,outcome,occurred_at,envelope_sha256,committed_at,planned_platinum_item_instance_id,
            planned_gold_item_instance_id,backpack_item_instance_id)
          SELECT game_character_uuid_v7(),game_character_uuid_v7(),game_character_uuid_v7(),1,
            sha256('p18 fixture'),$1::uuid,$2::uuid,$3::uuid,$4::uuid,$5::text::numeric,
            $6,0,1,sha256('p18 fixture'),0,game_character_uuid_v7(),game_character_uuid_v7(),item_instance_id
          FROM game_item_container_slots WHERE character_id=$1::uuid RETURNING transaction_id),
          entry AS (
          INSERT INTO game_account_bank_entries(entry_id,transaction_id,account_id,world_id,kind,
            amount,balance_before,balance_after,acting_character_id)
          SELECT game_character_uuid_v7(),transaction_id,$2::uuid,$3::uuid,1,$6,$7,$7+$6,$1::uuid
          FROM op RETURNING entry_id,balance_after)
          INSERT INTO game_account_bank_balances(account_id,world_id,balance,last_entry_id)
          SELECT $2::uuid,$3::uuid,balance_after,entry_id FROM entry
          ON CONFLICT (account_id,world_id) DO UPDATE
          SET balance=excluded.balance,last_entry_id=excluded.last_entry_id")
            .bind(character).bind(&clients.accounts[0]).bind(uuid_text(&clients.world))
            .bind(uuid_text(&clients.channel)).bind(clients.scope_generation.to_string())
            .bind(chunk).bind(before).execute(&mut *tx).await?;
        before += chunk;
        remaining -= chunk;
    }
    sqlx::query("SET LOCAL session_replication_role=origin")
        .execute(tx)
        .await?;
    Ok(before)
}

async fn run_case(clients: &SeamClients<'_>, case: &ForgeCase, tag: u8) -> TestResult {
    let [character] = clients.characters else {
        return Err("Forge case needs the dedicated character".into());
    };
    let character = uuid_text(character);
    let mut grant = connect_character(clients, tag, 0).await?;
    if let Some(pair) = case.pair {
        for (item, _) in pair {
            character_prep::grant_item(&grant, item, 1).await?;
        }
    }
    if case.cores > 0 {
        character_prep::grant_item(&grant, 37110, u32::try_from(case.cores)?).await?;
    }
    if case.operation == Operation::SliversToCores {
        for count in [20, 20, 10] {
            character_prep::grant_item(&grant, 37109, count).await?;
        }
    }
    logout(&mut grant).await?;
    drop(grant);

    let mut db = sqlx::PgConnection::connect(clients.url).await?;
    let mut tx = sqlx::Connection::begin(&mut db).await?;
    if let Some(pair) = case.pair {
        for (item, tier) in pair {
            if tier > 0 {
                let result = sqlx::query("INSERT INTO game_item_tiers(item_instance_id,tier)
                    SELECT i.item_instance_id,$3 FROM game_item_container_entries e
                    JOIN game_item_instances i USING (item_instance_id)
                    WHERE e.character_id=$1::uuid AND i.definition_production_key=$2 AND i.lifecycle=1 AND i.quantity=1")
                    .bind(&character).bind(format!("oteryn:item.tibia.i{item}"))
                    .bind(i16::from(tier)).execute(&mut *tx).await?;
                if result.rows_affected() != 1 {
                    return Err(
                        format!("expected one item {item} to receive fixture tier {tier}").into(),
                    );
                }
            }
        }
    }
    let funded = fixture_dust(&mut tx, &character, case.dust).await?;
    let gold = fixture_bank(&mut tx, clients, &character, case.gold).await?;
    tx.commit().await?;
    evidence(&format!(
        "FIXTURE row=P18.full operation={:?} item_id_tier_count={:?} gold_added={} dust={}/{} cores_added={} slivers_added={}",
        case.operation,
        case.pair
            .map(|pair| pair.map(|(item, tier)| (item, tier, 1))),
        case.gold,
        funded.0,
        funded.1,
        case.cores,
        if case.operation == Operation::SliversToCores {
            50
        } else {
            0
        }
    ));

    let mut client = connect_character(clients, tag + 1, 0).await?;
    let session = client.join_snapshot().game_session_id;
    let definitions = provisioning::item_index()?;
    let coins = [(3031, 1), (3035, 100), (3043, 10_000)]
        .into_iter()
        .map(|(id, value)| Ok((provisioning::item_reference(&definitions, id)?, value)))
        .collect::<TestResult<Vec<_>>>()?;
    // Native fees consume carried coins before the account's bank balance.
    let gold_before = gold + i64::try_from(carried_gold(&client, &coins))?;
    let sliver = provisioning::item_reference(&definitions, 37109)?;
    let core = provisioning::item_reference(&definitions, 37110)?;
    let counts = |client: &DevClientSession| {
        (
            super::full_e2ef7::units(client, sliver),
            super::full_e2ef7::units(client, core),
        )
    };
    let (slivers, cores) = counts(&client);
    if super::client_row_p18::dust(&mut client).await? != funded
        || bank_balance(&mut db, clients).await? != gold
        || cores < case.cores
    {
        return Err("live admission did not observe exact Forge funding".into());
    }
    let mut target_ref = None;
    let mut donor_ref = None;
    let mut quote_error = None;
    let pair = if let Some([target, donor]) = case.pair {
        let inventory = forge_items(&mut client).await?;
        let target = (
            provisioning::item_reference(&definitions, target.0)?.get(),
            target.1,
        );
        let donor = (
            provisioning::item_reference(&definitions, donor.0)?.get(),
            donor.1,
        );
        target_ref = Some(target.0);
        donor_ref = Some(donor.0);
        let find = |identity| {
            let matches = inventory
                .iter()
                .filter(|item| (item.definition_ref, item.tier) == identity)
                .collect::<Vec<_>>();
            match matches.as_slice() {
                [item] => Ok(*item),
                _ => Err("missing or ambiguous fixture Forge input"),
            }
        };
        let (first, second) = (find(target)?, find(donor)?);
        if !eligible_pair(case.operation, first, second) {
            return Err("fixture pair is ineligible".into());
        }
        let selection = preview::Selection::Pair {
            operation: case.operation,
            first: first.handle,
            second: second.handle,
            boost: false,
            protect: false,
        };
        let preview::Payload::Quote(quote) = preview(&mut client, selection).await? else {
            return Err("exact fixture pair did not receive a quote".into());
        };
        if quote.selection != selection {
            return Err("quote selection mismatch".into());
        }
        if (
            u64::from(quote.cost.dust),
            quote.cost.gold,
            u64::from(quote.cost.cores),
        ) != (
            u64::try_from(case.dust)?,
            u64::try_from(case.gold)?,
            case.cores,
        ) {
            quote_error = Some(format!(
                "Global quote mismatch: expected gold={} dust={} cores={}, observed {:?}",
                case.gold, case.dust, case.cores, quote.cost
            ));
        }
        Some((first.handle, second.handle))
    } else {
        None
    };

    let history_before = command(&mut client, Operation::QueryHistory, [0; 16], None, 50).await?;
    let history_before = history_before
        .body
        .as_array()
        .ok_or("invalid starting Forge history")?;
    client.service_liveness(Duration::from_millis(490)).await?;
    let result = command(&mut client, case.operation, occurrence(session, 1), pair, 0).await?;
    let (expected_dust, expected_cap) = (
        funded.0 - u64::try_from(case.dust)?,
        funded.1 + u64::from(case.operation == Operation::DustLimit),
    );
    let (expected_slivers, expected_cores) = match case.operation {
        Operation::DustToSlivers => (slivers + 3, cores),
        Operation::SliversToCores => (
            slivers.checked_sub(50).ok_or("fixture needs 50 slivers")?,
            cores + 1,
        ),
        _ => (slivers, cores - case.cores),
    };
    let expected_receipt = if case.operation == Operation::DustLimit {
        serde_json::json!({"operation":"dust_limit","succeeded":true,"result":{"dust":expected_dust,"dust_limit":expected_cap}})
    } else {
        let operation = match case.operation {
            Operation::Transfer => "transfer",
            Operation::ConvergenceFusion => "convergence_fusion",
            Operation::ConvergenceTransfer => "convergence_transfer",
            Operation::DustToSlivers => "dust_to_slivers",
            Operation::SliversToCores => "slivers_to_cores",
            _ => return Err("invalid independent Forge case".into()),
        };
        serde_json::json!({
            "operation":operation,"succeeded":true,"applied":true,"reason":null,"bonus":0,
            "result":{"first_item":pair.map(|p|p.0.get()),"first_tier":pair.map(|_|1),
            "second_item":pair.map(|p|p.1.get()),"second_tier":null,
            "dust":expected_dust,"dust_limit":expected_cap,"gold_spent":case.gold,
            "cores_spent":case.cores,"slivers_spent":slivers.saturating_sub(expected_slivers),
            "slivers_created":expected_slivers.saturating_sub(slivers),
            "cores_created":expected_cores.saturating_sub(cores)}
        })
    };

    let retained = client
        .inventory()
        .cloned()
        .ok_or("Forge outcome inventory missing")?;
    logout(&mut client).await?;
    drop(client);
    let mut fresh = connect_character(clients, tag + 2, 0).await?;
    let persisted = command(&mut fresh, Operation::QueryHistory, [0; 16], None, 50).await?;
    let history = persisted
        .body
        .as_array()
        .ok_or("invalid relog Forge history")?;
    let actual_dust = super::client_row_p18::dust(&mut fresh).await?;
    let actual_gold =
        bank_balance(&mut db, clients).await? + i64::try_from(carried_gold(&fresh, &coins))?;
    let actual_counts = counts(&fresh);
    let inventory_same = fresh.join_snapshot().game_session_id != session
        && fresh.inventory().cloned().map(durable_inventory) == Some(durable_inventory(retained));
    let tiers_ok = if let (Some(target), Some(donor)) = (target_ref, donor_ref) {
        dagger_tiers(&mut fresh, target).await? == [1]
            && dagger_tiers(&mut fresh, donor).await?.is_empty()
    } else {
        true
    };
    logout(&mut fresh).await?;

    let observed = &result.body;
    if quote_error.is_some()
        || result.status != CommandStatus::Accepted
        || (case.operation.changes_inventory() && !result.changed)
        || *observed != expected_receipt
        || actual_dust != (expected_dust, expected_cap)
        || actual_gold != gold_before - case.gold
        || actual_counts != (expected_slivers, expected_cores)
        || !inventory_same
        || !tiers_ok
        || persisted.status != CommandStatus::Accepted
        || history.len() != history_before.len() + 1
        || history.get(1..) != Some(history_before.as_slice())
        || history[0]["receipt"] != expected_receipt
    {
        return Err(format!("expected {expected_receipt}, observed {observed}; relog dust={actual_dust:?} gold_delta={} materials={actual_counts:?} tiers_ok={tiers_ok} inventory_same={inventory_same}; {}",
            gold_before - actual_gold, quote_error.unwrap_or_default()).into());
    }
    evidence(&format!(
        "P18_CASE_CHECK operation={:?} status=PASS relog=true",
        case.operation
    ));
    Ok(())
}

fn eligible_pair(operation: Operation, first: &preview::Item, second: &preview::Item) -> bool {
    if first.handle == second.handle
        || first.classification == 0
        || first.classification != second.classification
        || first.container
        || second.container
    {
        return false;
    }
    match operation {
        Operation::Fusion => {
            first.definition_ref == second.definition_ref
                && first.tier == second.tier
                && first.tier < first.maximum_tier
        }
        Operation::ConvergenceFusion => {
            first.classification == 4
                && first.tier == second.tier
                && first.tier < first.maximum_tier
        }
        Operation::Transfer => {
            first.tier == 0 && second.tier >= 2 && second.tier - 1 <= first.maximum_tier
        }
        Operation::ConvergenceTransfer => {
            first.classification == 4
                && first.tier == 0
                && second.tier > 0
                && second.tier <= first.maximum_tier
        }
        _ => false,
    }
}

async fn preview(
    client: &mut DevClientSession,
    selection: preview::Selection,
) -> TestResult<preview::Payload> {
    let request = preview::Request {
        nonce: 1,
        selection,
    };
    let bytes = request
        .encode()
        .ok_or("step=forge-preview invalid request")?;
    let reply = client
        .session_mut()
        .gated_command(
            preview::CAPABILITY_FORGE_PREVIEW_V1,
            preview::COMMAND_TYPE_FORGE_PREVIEW,
            &bytes,
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!(
            "step=forge-preview selection={selection:?} status={:?}",
            reply.status
        )
        .into());
    }
    let reply = preview::Reply::decode(&reply.payload).ok_or("step=forge-preview invalid reply")?;
    if reply.nonce != request.nonce {
        return Err("step=forge-preview mismatched nonce".into());
    }
    Ok(reply.payload)
}

async fn forge_items(client: &mut DevClientSession) -> TestResult<Vec<preview::Item>> {
    let mut items = Vec::new();
    let mut after = 0;
    loop {
        let preview::Payload::Page(page) =
            preview(client, preview::Selection::Browse { after }).await?
        else {
            return Err("step=forge-preview need=carried-item-page".into());
        };
        if page.after != after || items.len() + page.items.len() > 512 {
            return Err("step=forge-preview invalid page or inventory bound exceeded".into());
        }
        items.extend(page.items);
        match page.next {
            None => break,
            Some(next) if next > after => after = next,
            _ => return Err("step=forge-preview cursor did not advance".into()),
        }
    }
    Ok(items)
}

async fn dagger_tiers(client: &mut DevClientSession, definition: u32) -> TestResult<Vec<u8>> {
    let mut tiers: Vec<_> = forge_items(client)
        .await?
        .into_iter()
        .filter(|item| item.definition_ref == definition)
        .map(|item| item.tier)
        .collect();
    tiers.sort_unstable();
    Ok(tiers)
}

fn carried_gold(client: &DevClientSession, coins: &[(std::num::NonZeroU32, u64)]) -> u64 {
    coins
        .iter()
        .map(|&(definition, worth)| super::full_e2ef7::units(client, definition) * worth)
        .sum()
}

async fn quoted_pair(
    client: &mut DevClientSession,
    operation: Operation,
    definition: Option<u32>,
) -> TestResult<Option<(NonZeroU64, NonZeroU64)>> {
    let items = forge_items(client).await?;
    for first in items
        .iter()
        .filter(|item| definition.is_none_or(|id| item.definition_ref == id))
    {
        for second in items
            .iter()
            .filter(|second| eligible_pair(operation, first, second))
        {
            let selection = preview::Selection::Pair {
                operation,
                first: first.handle,
                second: second.handle,
                boost: false,
                protect: false,
            };
            match preview(client, selection).await? {
                preview::Payload::Quote(quote) if quote.selection == selection => {
                    return Ok(Some((first.handle, second.handle)));
                }
                preview::Payload::Refused => {
                    return Err(
                        format!("step=forge-preview refused eligible {operation:?} pair").into(),
                    );
                }
                _ => return Err("step=forge-preview mismatched pair quote".into()),
            }
        }
    }
    Ok(None)
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(240), verdict::observe(journey(clients)))
            .await;
    let result = match result {
        Ok(result) => result,
        Err(_) => Err("P18 journey timeout after real native commands".into()),
    };
    match result {
        Ok(()) => evidence(&format!(
            "e2e_all scenario=P18.full status=PASS seconds={:.3}",
            started.elapsed().as_secs_f64()
        )),
        Err(error) => {
            evidence(&format!("P18_GAP {error}"));
            evidence(&format!(
                "e2e_all scenario=P18.full status=FAIL seconds={:.3} evidence={}",
                started.elapsed().as_secs_f64(),
                error
                    .to_string()
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join("_")
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn forge_pair_prerequisites_follow_the_selected_operation() -> TestResult {
        use oteryn_protocol_oteryn::forge_preview::Item;
        let first = Item {
            handle: NonZeroU64::MIN,
            definition_ref: 1,
            appearance: 1,
            name: "dagger".into(),
            classification: 1,
            tier: 0,
            maximum_tier: 1,
            container: false,
        };
        let mut second = Item {
            handle: NonZeroU64::new(2).ok_or("second handle")?,
            ..first.clone()
        };
        assert!(!eligible_pair(Operation::Fusion, &first, &first));
        assert!(eligible_pair(Operation::Fusion, &first, &second));
        second.tier = 1;
        assert!(!eligible_pair(Operation::Transfer, &first, &second));
        let first = Item {
            classification: 4,
            maximum_tier: 10,
            ..first
        };
        second.classification = 4;
        second.maximum_tier = 10;
        assert!(!eligible_pair(
            Operation::ConvergenceFusion,
            &first,
            &second
        ));
        assert!(eligible_pair(
            Operation::ConvergenceTransfer,
            &first,
            &second
        ));
        second.tier = 2;
        assert!(eligible_pair(Operation::Transfer, &first, &second));
        Ok(())
    }

    use super::*;

    #[tokio::test]
    async fn applied_forge_observes_delayed_inventory_publication() -> TestResult {
        let mut serviced = Duration::ZERO;
        assert!(
            observe_inventory(true, async |duration| {
                serviced += duration;
                Ok(serviced >= Duration::from_millis(150))
            })
            .await?
        );
        Ok(())
    }

    #[tokio::test]
    async fn applied_forge_without_inventory_publication_stays_unchanged() -> TestResult {
        assert!(!observe_inventory(true, async |_| Ok(false)).await?);
        assert!(observe_inventory(false, async |_| Ok(true)).await?);
        Ok(())
    }

    #[test]
    fn occurrences_are_v7_session_scoped() -> TestResult {
        let mut first_id = [1; 16];
        first_id[6] = 0x71;
        first_id[8] = 0x81;
        let mut second_id = [2; 16];
        second_id[6] = 0x72;
        second_id[8] = 0x82;
        let first = oteryn_protocol_oteryn::GameSessionId::decode(&first_id)?;
        let second = oteryn_protocol_oteryn::GameSessionId::decode(&second_id)?;
        assert_eq!(occurrence(first, 1), occurrence(first, 1));
        assert_ne!(occurrence(first, 1), occurrence(first, 2));
        assert_ne!(occurrence(first, 1), occurrence(second, 1));
        oteryn_protocol_oteryn::GameSessionId::decode(&occurrence(first, 1))?;
        Ok(())
    }
}
