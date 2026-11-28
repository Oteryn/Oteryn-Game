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

fn unrelated_inventory(
    mut inventory: oteryn_protocol_oteryn::item_view::CharacterInventory,
    affected_definitions: &[u32],
) -> oteryn_protocol_oteryn::item_view::CharacterInventory {
    inventory
        .entries
        .retain(|item| !affected_definitions.contains(&item.item_definition_ref.get()));
    durable_inventory(inventory)
}

type ChestContents = std::collections::BTreeMap<String, Vec<(String, String, i16, i64)>>;

async fn forge_chests(db: &mut FixtureDb, clients: &SeamClients<'_>) -> TestResult<ChestContents> {
    let [character] = clients.characters else {
        return Err("Forge chest check requires one character".into());
    };
    let rows: Vec<(String, Option<String>, Option<String>, i16, i64)> = sqlx::query_as(
        "SELECT chest.item_instance_id::text, child.item_instance_id::text,
                child.definition_production_key, coalesce(tier.tier,0)::smallint,
                coalesce(child.quantity,0)::bigint
         FROM game_item_container_entries e
         JOIN game_item_instances chest ON chest.item_instance_id=e.item_instance_id
         LEFT JOIN game_item_nested_entries nested ON nested.parent_item_instance_id=chest.item_instance_id
         LEFT JOIN game_item_instances child ON child.item_instance_id=nested.item_instance_id
         LEFT JOIN game_item_tiers tier ON tier.item_instance_id=child.item_instance_id
         WHERE e.character_id=$1::uuid AND chest.lifecycle=1
           AND chest.definition_production_key='oteryn:item.tibia.i37561'
         ORDER BY chest.item_instance_id,child.item_instance_id",
    ).bind(uuid_text(character)).fetch_all(db).await?;
    let mut chests = ChestContents::new();
    for (chest, child, definition, tier, count) in rows {
        let entries = chests.entry(chest).or_default();
        if let Some((child, definition)) = child.zip(definition) {
            entries.push((child, definition, tier, count));
        }
    }
    Ok(chests)
}

fn chest_delivery_matches(
    before: &ChestContents,
    after: &ChestContents,
    expected: Option<&str>,
) -> bool {
    let added = after
        .iter()
        .filter(|(id, _)| !before.contains_key(*id))
        .collect::<Vec<_>>();
    before
        .iter()
        .all(|(id, items)| after.get(id) == Some(items))
        && match (expected, added.as_slice()) {
            (None, []) => true,
            (Some(expected), [(_, children)]) => matches!(
                children.as_slice(),
                [(_, definition, 1, 1)] if definition == expected
            ),
            _ => false,
        }
}

#[test]
fn p18_chest_checks_exact_nested_result() {
    let before = ChestContents::new();
    let mut after = ChestContents::new();
    let dagger = "oteryn:item.tibia.i3267";
    after.insert(
        "new chest".into(),
        vec![("receiver".into(), dagger.into(), 0, 1)],
    );
    assert!(!chest_delivery_matches(&before, &after, Some(dagger)));
    after.get_mut("new chest").expect("chest")[0].2 = 1;
    assert!(chest_delivery_matches(&before, &after, Some(dagger)));
    assert!(!chest_delivery_matches(&before, &after, None));
}

fn valid_history_ordinals(history: &[serde_json::Value]) -> bool {
    history.iter().rev().enumerate().all(|(index, entry)| {
        let first = history.last().and_then(|entry| entry["ordinal"].as_u64());
        first.is_some_and(|first| {
            first > 0
                && (history.len() == 50 || first == 1)
                && first.checked_add(index as u64) == entry["ordinal"].as_u64()
        })
    })
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
    let command_id = client.next_command_id();
    evidence(&format!(
        "P18_FORGE_DISPATCH operation={operation:?} command_id={command_id} last_frame={}",
        client.last_server_sequence()
    ));
    let reply = client
        .session_mut()
        .gated_command(CAPABILITY_ITEM_FORGE_V1, COMMAND_TYPE_ITEM_FORGE, &payload)
        .await?;
    verdict::Harness::global().progress("forge-command-reply");
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
    if entries.len() != applied.len() || !valid_history_ordinals(entries) {
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

// Match the existing dispatch PostgreSQL occurrence fixture. Selecting an input
// does not write the decision root or a reservation, or submit a Forge command.
async fn fusion_occurrence(
    clients: &SeamClients<'_>,
    mut occurrence: [u8; 16],
    success: bool,
) -> TestResult<[u8; 16]> {
    use oteryn_simulation_determinism::{
        DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
    };
    use sha2::Digest;

    let [character] = clients.characters else {
        return Err("Fusion fixture requires one dedicated character".into());
    };
    let mut connection = sqlx::PgConnection::connect(clients.url).await?;
    let seed: Vec<u8> =
        sqlx::query_scalar("SELECT secret FROM game_forge_decision_root WHERE singleton")
            .fetch_one(&mut connection)
            .await?;
    let root = GameplayDecisionRoot::from_bytes(
        sha2::Sha256::new()
            .chain_update(seed)
            .chain_update(character)
            .finalize()
            .into(),
    );
    for counter in 0_u32..100_000 {
        occurrence[9..13].copy_from_slice(&counter.to_be_bytes());
        let candidate = DecisionOccurrenceId::from_bytes(occurrence);
        let draw = |index| {
            deterministic_decision_u64(&root, candidate, "forge_fusion", index)
                .map_err(|error| format!("Forge fixture decision error: {error:?}"))
        };
        if (draw(0)? % 100 < 50) == success && draw(2)? % 10_001 < 7400 {
            let outcome = if success { "success" } else { "failure" };
            evidence(&format!(
                "FIXTURE row=P18.full forced fusion roll={outcome}"
            ));
            return Ok(occurrence);
        }
    }
    Err("bounded Forge fixture occurrence search exhausted".into())
}

async fn fusion_case(
    dedicated: &SeamClients<'_>,
    expected_success: bool,
    expected_prior_history: &[&Observation],
) -> TestResult<Observation> {
    let tag = if expected_success { 0xda } else { 0xd7 };
    let mut fixture = connect_character(dedicated, tag, 0).await?;
    super::client_row_p18::prepare_station_travel(&fixture, "P18.full").await?;
    character_prep::grant_item(
        &fixture,
        character_prep::item_id(FUSION_PAIR)?,
        if expected_success { 1 } else { 2 },
    )
    .await?;
    character_prep::grant_item(&fixture, 3043, 3).await?;
    logout(&mut fixture).await?;
    drop(fixture);
    if expected_success {
        let [character] = dedicated.characters else {
            return Err("Fusion fixture requires one dedicated character".into());
        };
        let mut db = sqlx::PgConnection::connect(dedicated.url).await?;
        let mut tx = sqlx::Connection::begin(&mut db).await?;
        fixture_dust(&mut tx, &uuid_text(character), 100).await?;
        tx.commit().await?;
        evidence("FIXTURE row=P18.full input=forge_dust before=0 after=100 cap=100");
    } else {
        super::client_row_p18::seed_dust("P18.full", dedicated).await?;
    }

    let mut client = connect_character(dedicated, tag + 1, 0).await?;
    super::client_row_p18::open_station(&mut client, dedicated, !expected_prior_history.is_empty())
        .await?;
    let session = client.join_snapshot().game_session_id;
    let first_history = command(&mut client, Operation::QueryHistory, [0; 16], None, 50).await?;
    validate_history("step-2", &first_history, expected_prior_history)?;

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
    let mut bank = sqlx::PgConnection::connect(dedicated.url).await?;
    if bank_balance(&mut bank, dedicated).await? != 0 {
        return Err("standard Fusion requires an empty bank".into());
    }
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
    let mut affected = coins
        .iter()
        .map(|(definition, _)| definition.get())
        .collect::<Vec<_>>();
    affected.push(pair_ref.get());
    if expected_success {
        affected.push(provisioning::item_reference(&definitions, 37561)?.get());
    }
    let chests_before = forge_chests(&mut bank, dedicated).await?;
    let before = unrelated_inventory(
        client
            .inventory()
            .cloned()
            .ok_or("Fusion pre-state missing")?,
        &affected,
    );
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
            fusion_occurrence(
                dedicated,
                occurrence(session, u8::try_from(index + 1)?),
                expected_success,
            )
            .await?,
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
            if success != expected_success
                || bonus != 0
                || observed.body["operation"] != "fusion"
                || observed.body.get("reason") != Some(&serde_json::Value::Null)
            {
                return Err(format!("invalid Fusion outcome: {}", observed.body).into());
            }
            let target = u8::from(success);
            let source: Option<u8> = None;
            let (dust_paid, gold_paid) = (100, 25_000);
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
            // A successful result is checked in its new chest below. Only a
            // failed Fusion leaves a loose dagger available as a Forge operand.
            let expected_tiers: &[u8] = if success { &[] } else { &[0] };
            let live_dust = super::client_row_p18::dust(&mut client).await?;
            let live_gold = carried_gold(&client, &coins);
            let live_bank = bank_balance(&mut bank, dedicated).await?;
            let live_tiers = dagger_tiers(&mut client, pair_ref.get()).await?;
            if live_dust != (100 - dust_paid, 100)
                || live_gold != gold_before - gold_paid
                || live_bank != 0
                || live_tiers != expected_tiers
                || client
                    .inventory()
                    .cloned()
                    .map(|inventory| unrelated_inventory(inventory, &affected))
                    .as_ref()
                    != Some(&before)
            {
                return Err(format!(
                    "Fusion live mismatch: dust={live_dust:?} gold={live_gold} expected_gold={} bank={live_bank} loose_tiers={live_tiers:?} expected_loose_tiers={expected_tiers:?}",
                    gold_before - gold_paid,
                )
                .into());
            }
            fusion_outcome = Some((expected_tiers, success, dust_paid, gold_paid));
        }
        observations.push(observed);
    }
    client.service_liveness(Duration::from_millis(490)).await?;
    let history = command(&mut client, Operation::QueryHistory, [0; 16], None, 50).await?;
    let applied_observations = expected_prior_history
        .iter()
        .copied()
        .chain(observations.iter())
        .collect::<Vec<_>>();
    validate_history("step-6", &history, &applied_observations)?;
    let retained = client
        .inventory()
        .cloned()
        .ok_or("step-6 inventory absent")?;
    let chests_after = forge_chests(&mut bank, dedicated).await?;
    let chest_ref = provisioning::item_reference(&definitions, 37561)?;
    if super::full_e2ef7::units(&client, chest_ref) != u64::try_from(chests_after.len())? {
        return Err("Fusion live inventory did not project the carried Exaltation Chest".into());
    }
    if !chest_delivery_matches(
        &chests_before,
        &chests_after,
        expected_success.then_some(FUSION_PAIR),
    ) {
        return Err("standard Fusion did not deliver the exact Exaltation Chest result".into());
    }
    let retained_balance = super::client_row_p18::dust(&mut client).await?;
    logout(&mut client).await?;
    drop(client);
    let mut fresh = connect_character(dedicated, tag + 2, 0).await?;
    if fresh.join_snapshot().game_session_id == session
        || fresh.inventory().cloned().map(durable_inventory) != Some(durable_inventory(retained))
    {
        return Err("step-7 fresh admission session/inventory durability mismatch".into());
    }
    let (expected_tiers, succeeded, dust_paid, gold_paid) =
        fusion_outcome.ok_or("funded valid-pair Fusion was never exercised")?;
    if forge_chests(&mut bank, dedicated).await? != chests_after
        || bank_balance(&mut bank, dedicated).await? != 0
        || carried_gold(&fresh, &coins) != gold_before - gold_paid
        || dagger_tiers(&mut fresh, pair_ref.get()).await? != expected_tiers
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
        "P18_FUSION_CHECK status=PASS succeeded={succeeded} dust_paid={dust_paid} gold_paid={gold_paid} loose_dagger_tiers={expected_tiers:?} exact_chest={succeeded} relog=true"
    ));
    observations
        .pop()
        .ok_or_else(|| "missing Fusion observation".into())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let dedicated = character_prep::dedicated_character(clients, "NODE_BOOT_Q_P18_ACCOUNT").await?;
    let dedicated = dedicated.clients(clients);
    let failure = fusion_case(&dedicated, false, &[]).await?;
    fusion_case(&dedicated, true, &[&failure]).await?;

    let mut failures = Vec::new();
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
    let (balance, limit): (i64, i16) = sqlx::query_as(
        "SELECT balance,dust_limit FROM game_character_forge_dust WHERE character_id=$1::uuid FOR UPDATE")
        .bind(character).fetch_one(&mut *tx).await?;
    if spend > i64::from(limit) {
        return Err("Forge funding exceeds the live dust cap".into());
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
           CASE $5 WHEN 'SPEND' THEN 'proficiency' ELSE 'creature_kill' END,
           game_character_uuid_v7(),game_character_uuid_v7(),$4,0,$2,
           CASE WHEN $5='GAIN' THEN $2+$4 ELSE $2-$4 END,$3::smallint,
           $3::smallint
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

pub(super) async fn bank_balance(tx: &mut FixtureDb, clients: &SeamClients<'_>) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT coalesce((SELECT balance FROM game_account_bank_balances
        WHERE account_id=$1::uuid AND world_id=$2::uuid),0)::bigint",
    )
    .bind(&clients.accounts[0])
    .bind(uuid_text(&clients.world))
    .fetch_one(tx)
    .await?)
}

// Apply the rollout switch with the same exclusive fence as the production
// activation transaction, before entering the offline funding transaction.
async fn activate_type2_fixture(db: &mut FixtureDb) -> TestResult {
    use oteryn_gameplay_durability::durability::item_mint_audit::TYPE2_AUDIT_ACTIVATION_FENCE;

    let mut tx = sqlx::Connection::begin(db).await?;
    sqlx::query("SET LOCAL lock_timeout = '5s'")
        .execute(&mut *tx)
        .await?;
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(TYPE2_AUDIT_ACTIVATION_FENCE)
        .execute(&mut *tx)
        .await?;
    let activated = sqlx::query(
        "INSERT INTO game_type2_audit_activation VALUES (1,now()) ON CONFLICT DO NOTHING",
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();
    tx.commit().await?;
    if activated == 1 {
        evidence("FIXTURE row=P18.full input=type2_audit_activation fence=exclusive");
    }
    Ok(())
}

async fn fixture_bank(
    tx: &mut FixtureDb,
    clients: &SeamClients<'_>,
    character: &str,
    amount: i64,
) -> TestResult<i64> {
    let mut before = bank_balance(tx, clients).await?;
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
            amount,balance_before,balance_after,acting_character_id,previous_entry_id)
          SELECT game_character_uuid_v7(),transaction_id,$2::uuid,$3::uuid,1,$6,$7,$7+$6,$1::uuid,
            (SELECT last_entry_id FROM game_account_bank_balances WHERE account_id=$2::uuid AND world_id=$3::uuid)
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
    Ok(before)
}

async fn raise_cap_for_funding(
    clients: &SeamClients<'_>,
    db: &mut FixtureDb,
    character: &str,
    required: u64,
    tag: u8,
) -> TestResult {
    let current: i16 = sqlx::query_scalar(
        "SELECT dust_limit FROM game_character_forge_dust WHERE character_id=$1::uuid",
    )
    .bind(character)
    .fetch_one(&mut *db)
    .await?;
    if u64::try_from(current)? >= required {
        return Ok(());
    }
    let mut client = connect_character(clients, tag, 0).await?;
    super::client_row_p18::open_station(&mut client, clients, true).await?;
    let session = client.join_snapshot().game_session_id;
    let baseline = command(&mut client, Operation::QueryHistory, [0; 16], None, 50).await?;
    let mut expected_history = baseline
        .body
        .as_array()
        .cloned()
        .ok_or("invalid cap history baseline")?;
    if baseline.status != CommandStatus::Accepted || !valid_history_ordinals(&expected_history) {
        return Err("invalid cap history baseline".into());
    }
    let mut final_limit = u64::try_from(current)?;
    for limit in (final_limit..required).step_by(20) {
        let stage = (limit - 100) / 20;
        let cost = 25 + stage;
        final_limit = limit + 20;
        let mut tx = sqlx::Connection::begin(&mut *db).await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        if fixture_dust(&mut tx, character, i64::try_from(cost)?).await? != (cost, limit) {
            return Err("Forge cap progression funding mismatch".into());
        }
        tx.commit().await?;
        evidence(&format!(
            "FIXTURE row=P18.full input=dust_for_cap_upgrade dust={cost} cap={limit}"
        ));
        client.service_liveness(Duration::from_millis(490)).await?;
        let result = command(
            &mut client,
            Operation::DustLimit,
            occurrence(session, u8::try_from(stage)?),
            None,
            0,
        )
        .await?;
        if result.status != CommandStatus::Accepted
            || result.body
                != serde_json::json!({
                    "operation":"dust_limit","succeeded":true,"result":{"dust":0,"dust_limit":final_limit}
                })
            || super::client_row_p18::dust(&mut client).await? != (0, final_limit)
        {
            return Err(format!(
                "live Forge cap upgrade {limit} → {} failed: {}",
                final_limit,
                result.body
            )
            .into());
        }
        let next = expected_history
            .first()
            .and_then(|entry| entry["ordinal"].as_u64())
            .unwrap_or(0)
            .checked_add(1)
            .ok_or("cap history ordinal overflow")?;
        expected_history.insert(0, serde_json::json!({"ordinal":next,"receipt":result.body}));
        expected_history.truncate(50);
    }
    logout(&mut client).await?;
    let mut fresh = connect_character(clients, tag.wrapping_add(1), 0).await?;
    if fresh.join_snapshot().game_session_id == session
        || super::client_row_p18::dust(&mut fresh).await? != (0, final_limit)
    {
        return Err("live Forge cap progression did not persist".into());
    }
    let history = command(&mut fresh, Operation::QueryHistory, [0; 16], None, 50).await?;
    if history.status != CommandStatus::Accepted
        || history.body != serde_json::Value::Array(expected_history)
    {
        return Err("live Forge cap upgrade history did not persist".into());
    }
    evidence("P18_CAP_HISTORY_CHECK status=PASS relog=true");
    logout(&mut fresh).await?;
    Ok(())
}

async fn run_case(clients: &SeamClients<'_>, case: &ForgeCase, tag: u8) -> TestResult {
    let [character] = clients.characters else {
        return Err("Forge case needs the dedicated character".into());
    };
    let character = uuid_text(character);
    let mut grant = connect_character(clients, tag, 0).await?;
    super::client_row_p18::prepare_station_travel(&grant, "P18.full").await?;
    if let Some(pair) = case.pair {
        for (item, _) in pair {
            character_prep::grant_item(&grant, item, 1).await?;
        }
    }
    if case.cores > 0 {
        character_prep::grant_item(&grant, 37110, u32::try_from(case.cores)?).await?;
    }
    if case.operation == Operation::SliversToCores {
        for count in [20, 19, 11] {
            character_prep::grant_item(&grant, 37109, count).await?;
        }
    }
    logout(&mut grant).await?;
    drop(grant);

    let mut db = sqlx::PgConnection::connect(clients.url).await?;
    raise_cap_for_funding(
        clients,
        &mut db,
        &character,
        u64::try_from(case.dust)?,
        tag - 64,
    )
    .await?;
    if case.gold > 0 {
        activate_type2_fixture(&mut db).await?;
    }
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
    // As in the native Forge PG fixture, seed historical input ledgers offline.
    // The transaction ends before the live session and cannot alter its writes.
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    let funded = fixture_dust(&mut tx, &character, case.dust).await?;
    let gold = fixture_bank(&mut tx, clients, &character, case.gold).await?;
    sqlx::query("SET LOCAL session_replication_role=origin")
        .execute(&mut *tx)
        .await?;
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
    super::client_row_p18::open_station(&mut client, clients, true).await?;
    let session = client.join_snapshot().game_session_id;
    let definitions = provisioning::item_index()?;
    let coins = [(3031, 1), (3035, 100), (3043, 10_000)]
        .into_iter()
        .map(|(id, value)| Ok((provisioning::item_reference(&definitions, id)?, value)))
        .collect::<TestResult<Vec<_>>>()?;
    // Native fees consume carried coins before the account's bank balance.
    let carried_before = i64::try_from(carried_gold(&client, &coins))?;
    let carried_debit = carried_before.min(case.gold);
    let expected_carried = carried_before - carried_debit;
    let expected_bank = gold - (case.gold - carried_debit);
    let sliver = provisioning::item_reference(&definitions, 37109)?;
    let core = provisioning::item_reference(&definitions, 37110)?;
    let counts = |client: &DevClientSession| {
        (
            super::full_e2ef7::units(client, sliver),
            super::full_e2ef7::units(client, core),
        )
    };
    let (slivers, cores) = counts(&client);
    if case.operation == Operation::SliversToCores && slivers < 50 {
        logout(&mut client).await?;
        return Err(format!("fixture supplied {slivers} slivers, expected at least 50").into());
    }
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
        if quote.selection != selection
            || (
                quote.target_tier,
                quote.success_percent,
                quote.protection_percent,
            ) != (1, 100, 0)
            || (
                u64::from(quote.resources.dust),
                u64::from(quote.resources.dust_limit),
            ) != funded
            || quote.resources.carried_gold != u64::try_from(carried_before)?
            || quote.resources.bank_gold != u64::try_from(gold)?
            || !quote.resources.bank_spendable
            || (quote.resources.slivers, quote.resources.cores) != (slivers, cores)
        {
            return Err(format!(
                "incorrect Forge quote selection, tiers, odds or resources: {quote:?}"
            )
            .into());
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

    let mut affected = Vec::new();
    if carried_debit != 0 {
        affected.extend(coins.iter().map(|(definition, _)| definition.get()));
    }
    if matches!(
        case.operation,
        Operation::DustToSlivers | Operation::SliversToCores
    ) {
        affected.push(sliver.get());
    }
    if case.cores != 0 || case.operation == Operation::SliversToCores {
        affected.push(core.get());
    }
    let chests_before = forge_chests(&mut db, clients).await?;
    let expected_chest = case
        .pair
        .map(|pair| format!("oteryn:item.tibia.i{}", pair[0].0));
    if expected_chest.is_some() {
        affected.push(provisioning::item_reference(&definitions, 37561)?.get());
        affected.extend(target_ref);
    }
    affected.extend(donor_ref);
    let before = unrelated_inventory(
        client
            .inventory()
            .cloned()
            .ok_or("Forge pre-state missing")?,
        &affected,
    );
    let history_before = command(&mut client, Operation::QueryHistory, [0; 16], None, 50).await?;
    if history_before.status != CommandStatus::Accepted {
        return Err("starting Forge history command was refused".into());
    }
    let history_before = history_before
        .body
        .as_array()
        .ok_or("invalid starting Forge history")?;
    if !valid_history_ordinals(history_before) {
        return Err("invalid starting Forge history ordinals".into());
    }
    client.service_liveness(Duration::from_millis(490)).await?;
    let result = command(&mut client, case.operation, occurrence(session, 1), pair, 0).await?;
    let (expected_dust, expected_cap) = (
        funded.0 - u64::try_from(case.dust)?,
        funded.1 + 20 * u64::from(case.operation == Operation::DustLimit),
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
    let chests_after = forge_chests(&mut db, clients).await?;
    let chest_ref = provisioning::item_reference(&definitions, 37561)?;
    if super::full_e2ef7::units(&client, chest_ref) != u64::try_from(chests_after.len())? {
        return Err("Forge live inventory did not project the carried Exaltation Chest".into());
    }
    if !chest_delivery_matches(&chests_before, &chests_after, expected_chest.as_deref())
        || unrelated_inventory(retained.clone(), &affected) != before
        || i64::try_from(carried_gold(&client, &coins))? != expected_carried
        || bank_balance(&mut db, clients).await? != expected_bank
    {
        return Err("Forge changed unrelated inventory or debited the wrong gold source".into());
    }
    logout(&mut client).await?;
    drop(client);
    let mut fresh = connect_character(clients, tag + 2, 0).await?;
    let persisted = command(&mut fresh, Operation::QueryHistory, [0; 16], None, 50).await?;
    let history = persisted
        .body
        .as_array()
        .ok_or("invalid relog Forge history")?;
    let actual_dust = super::client_row_p18::dust(&mut fresh).await?;
    let actual_bank = bank_balance(&mut db, clients).await?;
    let actual_carried = i64::try_from(carried_gold(&fresh, &coins))?;
    let actual_counts = counts(&fresh);
    let inventory_same = fresh.join_snapshot().game_session_id != session
        && fresh.inventory().cloned().map(durable_inventory) == Some(durable_inventory(retained));
    // The exact tier-one result is checked inside the new chest and compared after relog.
    // Both original operands must be absent from loose custody.
    let tiers_ok = if let (Some(target), Some(donor)) = (target_ref, donor_ref) {
        dagger_tiers(&mut fresh, target).await?.is_empty()
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
        || forge_chests(&mut db, clients).await? != chests_after
        || actual_bank != expected_bank
        || actual_carried != expected_carried
        || actual_counts != (expected_slivers, expected_cores)
        || !inventory_same
        || !tiers_ok
        || persisted.status != CommandStatus::Accepted
        || history.len() != (history_before.len() + 1).min(50)
        || !valid_history_ordinals(history)
        || history.get(1..) != Some(&history_before[..history_before.len().min(49)])
        || history[0]["receipt"] != expected_receipt
    {
        return Err(format!("expected {expected_receipt}, observed {observed}; relog dust={actual_dust:?} bank={actual_bank} carried={actual_carried} materials={actual_counts:?} tiers_ok={tiers_ok} inventory_same={inventory_same}; {}",
            quote_error.unwrap_or_default()).into());
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

pub(super) async fn preview(
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

pub(super) async fn dagger_tiers(
    client: &mut DevClientSession,
    definition: u32,
) -> TestResult<Vec<u8>> {
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
                preview::Payload::Quote(quote)
                    if quote.selection == selection
                        && (quote.cost.gold, quote.cost.dust, quote.cost.cores)
                            == (25_000, 100, 0)
                        && (
                            quote.target_tier,
                            quote.success_percent,
                            quote.protection_percent,
                        ) == (1, 50, 0) =>
                {
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
    const JOURNEY_BUDGET: Duration = Duration::from_secs(720);
    let started = std::time::Instant::now();
    evidence(&format!(
        "P18_BUDGET row=P18.full journey_s={}",
        JOURNEY_BUDGET.as_secs()
    ));
    let result = verdict::indexed_timeout(JOURNEY_BUDGET, verdict::observe(journey(clients))).await;
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
    fn forge_history_rejects_duplicate_missing_and_swapped_ordinals() {
        use super::valid_history_ordinals;
        use serde_json::json;

        assert!(valid_history_ordinals(&[
            json!({"ordinal":2}),
            json!({"ordinal":1})
        ]));
        for invalid in [
            [json!({"ordinal":1}), json!({"ordinal":1})],
            [json!({}), json!({"ordinal":1})],
            [json!({"ordinal":1}), json!({"ordinal":2})],
        ] {
            assert!(!valid_history_ordinals(&invalid));
        }
    }

    #[test]
    fn forge_inventory_keeps_unrelated_entries_in_the_comparison() {
        use super::unrelated_inventory;
        use oteryn_protocol_oteryn::item_view::{CharacterInventory, ItemEntry};
        use std::num::{NonZeroU32, NonZeroU64};

        let original = CharacterInventory::default();
        let mut changed = original.clone();
        changed.entries.push(ItemEntry {
            appearance_id: 1,
            handle: NonZeroU64::MIN,
            item_definition_ref: NonZeroU32::MIN,
            count: NonZeroU32::MIN,
            sub_type: 0,
        });
        assert_ne!(
            unrelated_inventory(changed.clone(), &[2]),
            unrelated_inventory(original.clone(), &[2])
        );
        assert_eq!(
            unrelated_inventory(changed, &[1]),
            unrelated_inventory(original, &[1])
        );
    }

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
