// Shared BANK-1 cases (migration 0071). Any wrapper that provides the same path-loaded crate root
// as `bank_postgres.rs`, and the Bestiary harness, can include this file.
//
// The harness bootstraps Character 41 (Account 40) on a live Channel session (50) of World 42 /
// Channel 43 held by node 1. The bank fixture (the main backpack, coin stacks, other characters
// and a seeded balance) is written by the migration owner with the guards off, as other item
// cases do; every bank operation then runs through the fenced entry points or, for the crash and
// guard cases, the same transaction body as the runtime role.

use crate::bestiary_postgres_harness::{
    CHANNEL, CHARACTER, Harness, SESSION, TestResult, WORLD, configured_admin, debug, id, runtime,
};
use crate::domain::CharacterId;
use crate::domain::currency::Coin;
use crate::durability::bank::{
    BankCoinDirection, BankCoinFacts, BankDepositRequest, BankError, BankLedgerKind,
    BankOperationIdentity, BankOperationOccurrence, BankOperationOutcome, BankOperationRecord,
    BankRequest, BankResult, BankTransferRequest, BankWithdrawRequest,
    bank_operation_in_transaction,
};
use crate::durability::bank_audit::{BankCauseV1, decode_bank_envelope};
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_transfer::{
    CurrentCharacterItemFence, ItemDefinitionFacts, ItemStackClass,
};
use crate::foundation::{
    ChannelId, ConnectionGeneration, GameSessionId, RuntimeScopeRefV1, ScopeOwnershipGeneration,
    WorldId,
};
use sqlx::PgPool;

const ACCOUNT: u8 = 40;
const BACKPACK: u8 = 90;
/// Bank Friend: another Account of the same World.
const FRIEND: u8 = 61;
const FRIEND_ACCOUNT: u8 = 60;
/// Alt Hero: another character of the acting Account.
const ALT: u8 = 62;
/// Far Hero: another Account's character of another World.
const FAR: u8 = 63;
const FAR_ACCOUNT: u8 = 64;
const FAR_WORLD: u8 = 65;
const GOLD: &str = "oteryn:item.tibia.i3031";
const PLATINUM: &str = "oteryn:item.tibia.i3035";
const CRYSTAL: &str = "oteryn:item.tibia.i3043";
const BACKPACK_KEY: &str = "oteryn:item.tibia.i2854";
const OCCURRED_AT: i64 = 1_790_000_000_000;
const BALANCE_MAX: u64 = 999_999_999_999;

/// A UUIDv7 distinct per (seed, salt).
fn salted(seed: u8, salt: u8) -> [u8; 16] {
    let mut bytes = id(seed);
    bytes[1] = salt;
    bytes
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn uuid(seed: u8) -> String {
    format!("'{}'::uuid", hex(&id(seed)))
}

fn sqlstate(error: &sqlx::Error) -> String {
    error
        .as_database_error()
        .and_then(|error| error.code())
        .map_or_else(|| format!("{error}"), |code| code.into_owned())
}

fn definition(key: &str) -> TypedDefinitionRef {
    TypedDefinitionRef {
        family: "Item".into(),
        production_key: key.into(),
        revision_ref: "rev-1".into(),
    }
}

fn coins() -> BankCoinFacts {
    BankCoinFacts {
        gold: definition(GOLD),
        platinum: definition(PLATINUM),
        crystal: definition(CRYSTAL),
        backpack: ItemDefinitionFacts {
            definition: definition(BACKPACK_KEY),
            stack: ItemStackClass::NonStackable,
            container_capacity: Some(20),
            container_slot_equip_pattern: true,
        },
    }
}

fn fence() -> TestResult<CurrentCharacterItemFence> {
    Ok(CurrentCharacterItemFence {
        character_id: CharacterId::from_bytes(id(CHARACTER)).map_err(debug)?,
        game_session_id: GameSessionId::decode(&id(SESSION)).map_err(debug)?,
        connection_generation: ConnectionGeneration::new(1).map_err(debug)?,
        character_lease_generation: 1,
        runtime_scope: RuntimeScopeRefV1::channel(
            WorldId::decode(&id(WORLD)).map_err(debug)?,
            ChannelId::decode(&id(CHANNEL)).map_err(debug)?,
        ),
        scope_ownership_generation: ScopeOwnershipGeneration::new(1).map_err(debug)?,
    })
}

/// Occurrence `n`: TransactionId salt 1, EventId salt 2, output slots salts 4..6.
fn identity(n: u8) -> TestResult<BankOperationIdentity> {
    Ok(BankOperationIdentity {
        occurrence: BankOperationOccurrence::from_bytes(salted(n, 7)).map_err(debug)?,
        transaction_id: salted(n, 1),
        event_id: salted(n, 2),
        occurred_at_unix_ms: OCCURRED_AT,
        server_build_id: "oteryn-game-server-test".into(),
    })
}

fn deposit(n: u8, amount: u64) -> TestResult<BankDepositRequest> {
    Ok(BankDepositRequest {
        identity: identity(n)?,
        amount,
        coins: coins(),
        change_item_instance_ids: [salted(n, 5), salted(n, 6)],
    })
}

fn withdraw(n: u8, amount: u64) -> TestResult<BankWithdrawRequest> {
    Ok(BankWithdrawRequest {
        identity: identity(n)?,
        amount,
        coins: coins(),
        output_item_instance_ids: [salted(n, 4), salted(n, 5), salted(n, 6)],
    })
}

fn transfer(n: u8, amount: u64, recipient: u8, key: &str) -> TestResult<BankTransferRequest> {
    Ok(BankTransferRequest {
        identity: identity(n)?,
        amount,
        recipient_character_id: CharacterId::from_bytes(id(recipient)).map_err(debug)?,
        recipient_name_key: key.into(),
    })
}

/// Fixture statements as the migration owner with every trigger off.
async fn seed(pool: &PgPool, script: &str) -> TestResult {
    let mut tx = pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await?;
    sqlx::raw_sql(sqlx::AssertSqlSafe(script.to_owned()))
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

fn item(seed: u8, key: &str, quantity: u32) -> String {
    format!(
        "INSERT INTO game_item_instances(item_instance_id, world_id, definition_family, \
           definition_production_key, definition_revision_ref, quantity, lifecycle, \
           minted_transaction_id) \
         VALUES ({}, {}, 'Item', '{key}', 'rev-1', {quantity}, 1, {});",
        uuid(seed),
        uuid(WORLD),
        uuid(seed.wrapping_add(1)),
    )
}

/// The equipped main backpack of Character 41 and the three other characters.
async fn seed_bank(harness: &Harness) -> TestResult {
    seed(
        &harness.pool,
        &format!(
            "{backpack} \
             INSERT INTO game_item_container_slots(character_id, item_instance_id, world_id, \
               placed_transaction_id) VALUES ({character}, {backpack_id}, {world}, {placed}); \
             INSERT INTO game_character_account_guards VALUES ({friend_account}), ({far_account}); \
             INSERT INTO game_character_roots VALUES \
               ({friend}, {friend_account}, {world}, 1, 1, 'profile-1', 'ruleset-1', 'content-1', \
                'starter-1', 'Bank Friend'), \
               ({alt}, {account}, {world}, 1, 1, 'profile-1', 'ruleset-1', 'content-1', \
                'starter-1', 'Alt Hero'), \
               ({far}, {far_account}, {far_world}, 1, 1, 'profile-1', 'ruleset-1', 'content-1', \
                'starter-1', 'Far Hero');",
            backpack = item(BACKPACK, BACKPACK_KEY, 1),
            character = uuid(CHARACTER),
            backpack_id = uuid(BACKPACK),
            world = uuid(WORLD),
            placed = uuid(BACKPACK + 2),
            friend = uuid(FRIEND),
            friend_account = uuid(FRIEND_ACCOUNT),
            alt = uuid(ALT),
            account = uuid(ACCOUNT),
            far = uuid(FAR),
            far_account = uuid(FAR_ACCOUNT),
            far_world = uuid(FAR_WORLD),
        ),
    )
    .await
}

/// A live coin stack in a new direct backpack entry at `ordinal`. Seed `seed` uses the ids
/// `seed` (item), `seed + 1` (minted) and `seed + 2` (placed).
async fn stack(
    harness: &Harness,
    seed_id: u8,
    key: &str,
    quantity: u32,
    ordinal: u64,
) -> TestResult {
    seed(
        &harness.pool,
        &format!(
            "{} INSERT INTO game_item_container_entries(item_instance_id, world_id, character_id, \
               parent_item_instance_id, placement_ordinal, placed_transaction_id) \
             VALUES ({}, {}, {}, {}, {ordinal}, {});",
            item(seed_id, key, quantity),
            uuid(seed_id),
            uuid(WORLD),
            uuid(CHARACTER),
            uuid(BACKPACK),
            uuid(seed_id.wrapping_add(2)),
        ),
    )
    .await
}

/// A balance of `balance` on `account`, as one seeded deposit operation and entry by `character`.
async fn seed_balance(harness: &Harness, account: u8, character: u8, balance: u64) -> TestResult {
    let operation = format!("'{}'::uuid", hex(&salted(account, 9)));
    let entry = format!("'{}'::uuid", hex(&salted(account, 10)));
    seed(
        &harness.pool,
        &format!(
            "INSERT INTO game_account_bank_operations(operation_occurrence_id, transaction_id, \
               event_id, kind, request_binding, acting_character_id, account_id, world_id, \
               channel_id, runtime_scope_ownership_generation, amount, outcome, occurred_at, \
               envelope_sha256, committed_at, planned_platinum_item_instance_id, \
               planned_gold_item_instance_id, backpack_item_instance_id) \
             VALUES ('{occ}'::uuid, {operation}, '{event}'::uuid, 1, sha256('seed'), {character}, \
               {account}, {world}, {channel}, 1, 1, 0, 1, sha256('seed'), 0, '{p}'::uuid, \
               '{g}'::uuid, {backpack}); \
             INSERT INTO game_account_bank_entries(entry_id, transaction_id, account_id, \
               world_id, kind, amount, balance_before, balance_after, acting_character_id) \
             VALUES ({entry}, {operation}, {account}, {world}, 1, {balance}, 0, {balance}, \
               {character}); \
             INSERT INTO game_account_bank_balances VALUES ({account}, {world}, {balance}, {entry});",
            occ = hex(&salted(account, 11)),
            event = hex(&salted(account, 12)),
            p = hex(&salted(account, 13)),
            g = hex(&salted(account, 14)),
            character = uuid(character),
            account = uuid(account),
            world = uuid(WORLD),
            channel = uuid(CHANNEL),
            backpack = uuid(BACKPACK),
        ),
    )
    .await
}

/// Every durable fact a bank operation may change.
async fn snapshot(pool: &PgPool) -> TestResult<String> {
    Ok(sqlx::query_scalar(
        "SELECT concat_ws('|', \
           (SELECT string_agg(concat_ws(':', item_instance_id, quantity, lifecycle, \
                     last_transaction_id), ',' ORDER BY item_instance_id) \
              FROM game_item_instances), \
           (SELECT string_agg(concat_ws(':', item_instance_id, placement_ordinal), ',' \
                     ORDER BY placement_ordinal) FROM game_item_container_entries), \
           (SELECT string_agg(concat_ws(':', account_id, balance, last_entry_id), ',' \
                     ORDER BY account_id) FROM game_account_bank_balances), \
           (SELECT count(*) FROM game_account_bank_operations), \
           (SELECT count(*) FROM game_account_bank_entries), \
           (SELECT count(*) FROM game_account_bank_coin_lines), \
           (SELECT count(*) FROM game_account_bank_audit_outbox), \
           (SELECT string_agg(character_revision::text, ',' ORDER BY character_id) \
              FROM game_character_roots))",
    )
    .fetch_one(pool)
    .await?)
}

/// Values only: snapshot without balance rows (a refusal may add a value-neutral zero row) and
/// without the operation count (a refusal records its row).
async fn value_snapshot(pool: &PgPool) -> TestResult<String> {
    Ok(sqlx::query_scalar(
        "SELECT concat_ws('|', \
           (SELECT string_agg(concat_ws(':', item_instance_id, quantity, lifecycle, \
                     last_transaction_id), ',' ORDER BY item_instance_id) \
              FROM game_item_instances), \
           (SELECT string_agg(concat_ws(':', item_instance_id, placement_ordinal), ',' \
                     ORDER BY placement_ordinal) FROM game_item_container_entries), \
           (SELECT string_agg(concat_ws(':', account_id, balance, last_entry_id), ',' \
                     ORDER BY account_id) FROM game_account_bank_balances WHERE balance > 0), \
           (SELECT count(*) FROM game_account_bank_entries), \
           (SELECT count(*) FROM game_account_bank_coin_lines), \
           (SELECT count(*) FROM game_account_bank_audit_outbox))",
    )
    .fetch_one(pool)
    .await?)
}

async fn balance(pool: &PgPool, account: u8) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT coalesce((SELECT balance FROM game_account_bank_balances \
           WHERE account_id = encode($1,'hex')::uuid AND world_id = encode($2,'hex')::uuid), 0)",
    )
    .bind(id(account).as_slice())
    .bind(id(WORLD).as_slice())
    .fetch_one(pool)
    .await?)
}

/// (production key, quantity, lifecycle, placement ordinal) of every coin, highest ordinal first.
async fn backpack(pool: &PgPool) -> TestResult<Vec<(String, i64, String)>> {
    Ok(sqlx::query_as(
        "SELECT i.definition_production_key, i.quantity, e.placement_ordinal::text \
           FROM game_item_container_entries e \
           JOIN game_item_instances i ON i.item_instance_id = e.item_instance_id \
          WHERE e.parent_item_instance_id = encode($1,'hex')::uuid \
          ORDER BY e.placement_ordinal DESC",
    )
    .bind(id(BACKPACK).as_slice())
    .fetch_all(pool)
    .await?)
}

fn committed(outcome: BankOperationOutcome) -> TestResult<BankOperationRecord> {
    match outcome {
        BankOperationOutcome::Committed(record) => Ok(record),
        other => Err(format!("expected a fresh commit, got {other:?}").into()),
    }
}

/// The stored event of `n`, through both registered gates.
async fn event(pool: &PgPool, n: u8) -> TestResult<crate::durability::bank_audit::BankOperationV1> {
    let envelope: Vec<u8> = sqlx::query_scalar(
        "SELECT envelope FROM game_account_bank_audit_outbox WHERE event_id = encode($1,'hex')::uuid",
    )
    .bind(salted(n, 2).as_slice())
    .fetch_one(pool)
    .await?;
    Ok(decode_bank_envelope(&envelope).map_err(debug)?.1)
}

#[test]
fn deposit_withdraw_and_transfer_commit_entries_lines_and_one_event_each() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "bank_flow", false).await?;
        // Opened before the fixture adds roots outside the Character authority records.
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        seed_bank(&harness).await?;
        // Display order (highest ordinal first): the crystal, then 30 gold.
        stack(&harness, 100, GOLD, 30, 1).await?;
        stack(&harness, 103, CRYSTAL, 1, 2).await?;
        let (root, node) = (&harness.root, &harness.node);
        let revisions = harness.root_revision().await?;

        // Deposit 2,350: 30 gold, then the crystal; 7,680 change as 76 platinum and 80 gold.
        let record = committed(
            root.bank_deposit(&authority, node, fence()?, deposit(1, 2_350)?)
                .await
                .map_err(debug)?,
        )?;
        assert_eq!(record.result, BankResult::Ok);
        let [entry] = record.entries[..] else {
            return Err("one deposit entry".into());
        };
        assert_eq!(
            (entry.kind, entry.amount, entry.balance_before, entry.balance_after),
            (BankLedgerKind::Deposit, 2_350, 0, 2_350)
        );
        assert_eq!(
            record
                .coin_lines
                .iter()
                .map(|l| (l.direction, l.coin, l.quantity_before, l.quantity_after, l.placement_ordinal))
                .collect::<Vec<_>>(),
            vec![
                (BankCoinDirection::Input, Coin::Gold, 30, 0, 1),
                (BankCoinDirection::Input, Coin::Crystal, 1, 0, 2),
                (BankCoinDirection::Output, Coin::Platinum, 0, 76, 3),
                (BankCoinDirection::Output, Coin::Gold, 0, 80, 4),
            ]
        );
        assert_eq!(balance(&harness.pool, ACCOUNT).await?, 2_350);
        assert_eq!(
            backpack(&harness.pool).await?,
            vec![
                (GOLD.into(), 80, "4".into()),
                (PLATINUM.into(), 76, "3".into()),
            ]
        );
        let payload = event(&harness.pool, 1).await?;
        assert!(matches!(
            payload.cause.and_then(|cause| cause.cause),
            Some(BankCauseV1::Conversion(_))
        ));
        assert_eq!((payload.coin_lines.len(), payload.value_lines.len()), (4, 1));

        // Withdraw 1,234 as 12 platinum and 34 gold after every entry.
        let record = committed(
            root.bank_withdraw(&authority, node, fence()?, withdraw(2, 1_234)?)
                .await
                .map_err(debug)?,
        )?;
        assert_eq!(
            record
                .coin_lines
                .iter()
                .map(|l| (l.coin, l.quantity_after, l.placement_ordinal))
                .collect::<Vec<_>>(),
            vec![(Coin::Platinum, 12, 5), (Coin::Gold, 34, 6)]
        );
        assert_eq!(record.entries[0].balance_after, 1_116);
        assert_eq!(event(&harness.pool, 2).await?.coin_lines.len(), 2);

        // Transfer 1,000 to another Account: two entries that name each other, committed together.
        let record = committed(
            root.bank_transfer(&authority, node, fence()?, transfer(3, 1_000, FRIEND, "bankfriend")?)
                .await
                .map_err(debug)?,
        )?;
        let [out, into] = record.entries[..] else {
            return Err("two transfer entries".into());
        };
        assert_eq!(
            (out.kind, out.account_id, out.balance_before, out.balance_after),
            (BankLedgerKind::TransferOut, id(ACCOUNT), 1_116, 116)
        );
        assert_eq!(
            (into.kind, into.account_id, into.balance_before, into.balance_after),
            (BankLedgerKind::TransferIn, id(FRIEND_ACCOUNT), 0, 1_000)
        );
        let pair: (bool, bool) = sqlx::query_as(
            "SELECT bool_and(o.counterpart_entry_id = i.entry_id), \
                    bool_and(i.counterpart_entry_id = o.entry_id) \
               FROM game_account_bank_entries o JOIN game_account_bank_entries i \
                 ON i.transaction_id = o.transaction_id AND i.kind = 4 \
              WHERE o.kind = 3",
        )
        .fetch_one(&harness.pool)
        .await?;
        assert_eq!(pair, (true, true));
        assert_eq!(balance(&harness.pool, ACCOUNT).await?, 116);
        assert_eq!(balance(&harness.pool, FRIEND_ACCOUNT).await?, 1_000);
        assert_eq!(event(&harness.pool, 3).await?.value_lines.len(), 2);
        // Each balance equals its latest entry; the chain links every entry of an Account.
        let chained: bool = sqlx::query_scalar(
            "SELECT bool_and(b.balance = e.balance_after) \
               AND (SELECT count(*) FROM game_account_bank_entries WHERE previous_entry_id IS NOT NULL) = 2 \
               FROM game_account_bank_balances b \
               JOIN game_account_bank_entries e ON e.entry_id = b.last_entry_id",
        )
        .fetch_one(&harness.pool)
        .await?;
        assert!(chained);
        // No CharacterRevision advance.
        assert_eq!(harness.root_revision().await?, revisions);

        // Replay: the same occurrence and binding return the first outcome and write nothing.
        let before = snapshot(&harness.pool).await?;
        let replayed = root
            .bank_deposit(&authority, node, fence()?, deposit(1, 2_350)?)
            .await
            .map_err(debug)?;
        let BankOperationOutcome::Replayed(replayed) = replayed else {
            return Err("a replay must not commit again".into());
        };
        assert_eq!(replayed.entries[0], entry);
        assert_eq!(replayed.coin_lines.len(), 4);
        let replayed = root
            .bank_transfer(&authority, node, fence()?, transfer(3, 1_000, FRIEND, "bankfriend")?)
            .await
            .map_err(debug)?;
        assert!(matches!(replayed, BankOperationOutcome::Replayed(ref r) if r.entries == [out, into]));
        // A changed binding conflicts.
        for changed in [
            root.bank_deposit(&authority, node, fence()?, deposit(1, 2_351)?)
                .await,
            root.bank_withdraw(&authority, node, fence()?, withdraw(1, 2_350)?)
                .await,
        ] {
            assert!(matches!(changed, Err(BankError::ConflictingOccurrence)), "{changed:?}");
        }
        assert_eq!(snapshot(&harness.pool).await?, before);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

#[test]
fn limits_admit_their_maximum_and_refuse_one_more_before_any_write() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "bank_limits", false).await?;
        // Opened before the fixture adds roots outside the Character authority records.
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        seed_bank(&harness).await?;
        // 20 stacks of 100 crystal coins: a 20,000,000 deposit consumes them all.
        for n in 0..20_u8 {
            stack(&harness, 100 + 3 * n, CRYSTAL, 100, u64::from(n) + 1).await?;
        }
        seed_balance(&harness, ACCOUNT, CHARACTER, BALANCE_MAX - 20_000_000 - 1).await?;
        let (root, node) = (&harness.root, &harness.node);

        // Amounts outside their rows are refused before any read or write.
        let before = snapshot(&harness.pool).await?;
        for refused in [
            root.bank_deposit(&authority, node, fence()?, deposit(1, 20_000_001)?)
                .await,
            root.bank_deposit(&authority, node, fence()?, deposit(1, 0)?)
                .await,
            root.bank_withdraw(&authority, node, fence()?, withdraw(1, 1_010_000)?)
                .await,
            root.bank_transfer(
                &authority,
                node,
                fence()?,
                transfer(1, BALANCE_MAX + 1, FRIEND, "bankfriend")?,
            )
            .await,
        ] {
            assert!(
                matches!(refused, Err(BankError::InvalidInput)),
                "{refused:?}"
            );
        }
        assert_eq!(snapshot(&harness.pool).await?, before);

        // The deposit maximum: 20 input stacks, no change.
        let record = committed(
            root.bank_deposit(&authority, node, fence()?, deposit(2, 20_000_000)?)
                .await
                .map_err(debug)?,
        )?;
        assert_eq!(record.coin_lines.len(), 20);
        assert_eq!(record.entries[0].balance_after, BALANCE_MAX - 1);
        assert!(backpack(&harness.pool).await?.is_empty());

        // The balance maximum: one more gold reaches it, the next is BALANCE_LIMIT.
        stack(&harness, 200, GOLD, 2, 30).await?;
        let record = committed(
            root.bank_deposit(&authority, node, fence()?, deposit(3, 1)?)
                .await
                .map_err(debug)?,
        )?;
        assert_eq!(record.entries[0].balance_after, BALANCE_MAX);
        let values = value_snapshot(&harness.pool).await?;
        let refused = committed(
            root.bank_deposit(&authority, node, fence()?, deposit(4, 1)?)
                .await
                .map_err(debug)?,
        )?;
        assert_eq!(refused.result, BankResult::BalanceLimit);
        assert!(refused.entries.is_empty() && refused.coin_lines.is_empty());
        assert_eq!(value_snapshot(&harness.pool).await?, values);

        // The transfer maximum: the whole balance to another Account; one more is invalid.
        let record = committed(
            root.bank_transfer(
                &authority,
                node,
                fence()?,
                transfer(5, BALANCE_MAX, FRIEND, "bankfriend")?,
            )
            .await
            .map_err(debug)?,
        )?;
        assert_eq!(record.entries[1].balance_after, BALANCE_MAX);
        assert_eq!(balance(&harness.pool, ACCOUNT).await?, 0);
        // A credit above the recipient's maximum: RECIPIENT_CANNOT_RECEIVE_TRANSFERS. Two more
        // crystal stacks give the sender 2,000,000 (the last gold coin and the change of 1).
        stack(&harness, 210, CRYSTAL, 100, 31).await?;
        stack(&harness, 213, CRYSTAL, 100, 32).await?;
        let record = committed(
            root.bank_deposit(&authority, node, fence()?, deposit(6, 2_000_000)?)
                .await
                .map_err(debug)?,
        )?;
        assert_eq!(record.entries[0].balance_after, 2_000_000);
        let values = value_snapshot(&harness.pool).await?;
        let refused = committed(
            root.bank_transfer(
                &authority,
                node,
                fence()?,
                transfer(7, 1, FRIEND, "bankfriend")?,
            )
            .await
            .map_err(debug)?,
        )?;
        assert_eq!(refused.result, BankResult::RecipientCannotReceiveTransfers);
        assert_eq!(value_snapshot(&harness.pool).await?, values);

        // The withdrawal maximum: 100 crystal, 99 platinum and 99 gold in three new stacks.
        let record = committed(
            root.bank_withdraw(&authority, node, fence()?, withdraw(8, 1_009_999)?)
                .await
                .map_err(debug)?,
        )?;
        assert_eq!(
            record
                .coin_lines
                .iter()
                .map(|l| (l.coin, l.quantity_after))
                .collect::<Vec<_>>(),
            vec![(Coin::Crystal, 100), (Coin::Platinum, 99), (Coin::Gold, 99)]
        );
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

#[test]
fn typed_refusals_write_only_their_operation_row_and_replay() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "bank_refusals", false).await?;
        // Opened before the fixture adds roots outside the Character authority records.
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        seed_bank(&harness).await?;
        stack(&harness, 100, GOLD, 50, 1).await?;
        seed_balance(&harness, ACCOUNT, CHARACTER, 500).await?;
        let (root, node) = (&harness.root, &harness.node);
        let values = value_snapshot(&harness.pool).await?;

        let cases: Vec<(u8, BankRequest, BankResult)> = vec![
            (
                10,
                BankRequest::Deposit(deposit(10, 51)?),
                BankResult::InsufficientCoins,
            ),
            (
                11,
                BankRequest::Withdraw(withdraw(11, 501)?),
                BankResult::InsufficientBalance,
            ),
            (
                12,
                BankRequest::Transfer(transfer(12, 501, FRIEND, "bankfriend")?),
                BankResult::InsufficientBalance,
            ),
            (
                13,
                BankRequest::Transfer(transfer(13, 1, FRIEND, "otherfriend")?),
                BankResult::UnknownRecipient,
            ),
            (
                14,
                BankRequest::Transfer(transfer(14, 1, FAR, "farhero")?),
                BankResult::UnknownRecipient,
            ),
            (
                15,
                BankRequest::Transfer(transfer(15, 1, 77, "nobody")?),
                BankResult::UnknownRecipient,
            ),
            (
                16,
                BankRequest::Transfer(transfer(16, 1, ALT, "althero")?),
                BankResult::SameAccount,
            ),
            (
                17,
                BankRequest::Transfer(transfer(17, 1, CHARACTER, "fixturehero")?),
                BankResult::SameAccount,
            ),
        ];
        for (n, request, expected) in cases {
            let authority = &authority;
            let run = |request: BankRequest| async move {
                match request {
                    BankRequest::Deposit(r) => {
                        root.bank_deposit(authority, node, fence()?, r).await
                    }
                    BankRequest::Withdraw(r) => {
                        root.bank_withdraw(authority, node, fence()?, r).await
                    }
                    BankRequest::Transfer(r) => {
                        root.bank_transfer(authority, node, fence()?, r).await
                    }
                }
                .map_err(|error| -> Box<dyn std::error::Error> { debug(error).into() })
            };
            let record = committed(run(request.clone()).await?)?;
            assert_eq!(record.result, expected, "case {n}");
            assert!(
                record.entries.is_empty() && record.coin_lines.is_empty(),
                "case {n}"
            );
            // The refusal replays as itself.
            let replayed = run(request).await?;
            assert!(
                matches!(replayed, BankOperationOutcome::Replayed(ref r) if r.result == expected),
                "case {n}: {replayed:?}"
            );
            assert_eq!(value_snapshot(&harness.pool).await?, values, "case {n}");
        }
        // A full backpack: no room for a withdrawal.
        for n in 0..19_u8 {
            stack(&harness, 110 + 3 * n, PLATINUM, 1, 10 + u64::from(n)).await?;
        }
        let values = value_snapshot(&harness.pool).await?;
        let record = committed(
            root.bank_withdraw(&authority, node, fence()?, withdraw(20, 1)?)
                .await
                .map_err(debug)?,
        )?;
        assert_eq!(record.result, BankResult::NoRoom);
        assert_eq!(value_snapshot(&harness.pool).await?, values);
        let outcomes: Vec<i16> = sqlx::query_scalar(
            "SELECT outcome FROM game_account_bank_operations WHERE outcome <> 0 ORDER BY outcome",
        )
        .fetch_all(&harness.pool)
        .await?;
        assert_eq!(outcomes, vec![2, 2, 3, 4, 6, 6, 6, 8, 8]);

        // A stale fence is rejected and writes nothing.
        let before = snapshot(&harness.pool).await?;
        let mut stale = fence()?;
        stale.connection_generation = ConnectionGeneration::new(2).map_err(debug)?;
        let rejected = root
            .bank_withdraw(&authority, node, stale, withdraw(21, 1)?)
            .await;
        assert!(
            matches!(rejected, Err(BankError::AuthorityRejected)),
            "{rejected:?}"
        );
        assert_eq!(snapshot(&harness.pool).await?, before);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

/// One transaction body as the runtime role, as the entry point runs it after its recovery
/// fence and admission locks.
async fn runtime_transaction(
    harness: &Harness,
    request: &BankRequest,
) -> TestResult<(
    sqlx::Transaction<'static, sqlx::Postgres>,
    BankOperationOutcome,
)> {
    let mut tx = harness.pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *tx)
        .await?;
    let outcome = bank_operation_in_transaction(&mut tx, &harness.node, &fence()?, request)
        .await
        .map_err(debug)?;
    Ok((tx, outcome))
}

#[test]
fn a_crash_at_any_step_leaves_nothing_and_an_ambiguous_commit_resolves_by_replay() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "bank_crash", false).await?;
        // Opened before the fixture adds roots outside the Character authority records.
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        seed_bank(&harness).await?;
        stack(&harness, 100, GOLD, 30, 1).await?;
        stack(&harness, 103, CRYSTAL, 1, 2).await?;
        let before = snapshot(&harness.pool).await?;
        let request = BankRequest::Deposit(deposit(1, 2_350)?);

        // The whole body ran but never committed: a rollback, then a lost connection.
        let (tx, outcome) = runtime_transaction(&harness, &request).await?;
        assert!(matches!(outcome, BankOperationOutcome::Committed(_)));
        tx.rollback().await?;
        assert_eq!(snapshot(&harness.pool).await?, before);
        let (mut tx, _) = runtime_transaction(&harness, &request).await?;
        let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
            .fetch_one(&mut *tx)
            .await?;
        sqlx::query("SELECT pg_terminate_backend($1)")
            .bind(pid)
            .execute(&harness.pool)
            .await?;
        assert!(tx.commit().await.is_err());
        assert_eq!(snapshot(&harness.pool).await?, before);

        // An abort after each write step: every statement belongs to the one transaction, so a
        // failure at any point leaves no partial row. Each prefix is cut by a forced error.
        for table in [
            "game_account_bank_operations",
            "game_account_bank_entries",
            "game_account_bank_balances",
            "game_account_bank_coin_lines",
            "game_account_bank_audit_outbox",
        ] {
            let (mut tx, _) = runtime_transaction(&harness, &request).await?;
            let rows: i64 =
                sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {table}")))
                    .fetch_one(&mut *tx)
                    .await?;
            assert!(rows > 0, "{table} written in the transaction");
            assert!(sqlx::query("SELECT 1/0").execute(&mut *tx).await.is_err());
            drop(tx);
            assert_eq!(snapshot(&harness.pool).await?, before, "{table}");
        }

        // The commit succeeds but its outcome is lost: the entry point replays it.
        let (tx, outcome) = runtime_transaction(&harness, &request).await?;
        tx.commit().await?;
        let first = committed(outcome)?;
        let after = snapshot(&harness.pool).await?;
        let BankOperationOutcome::Replayed(replayed) = harness
            .root
            .bank_deposit(&authority, &harness.node, fence()?, deposit(1, 2_350)?)
            .await
            .map_err(debug)?
        else {
            return Err("an ambiguous commit must resolve by replay".into());
        };
        assert_eq!(replayed, first);
        assert_eq!(snapshot(&harness.pool).await?, after);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

/// As the runtime role, run the writer body for `request` (when given), then `tamper`, and expect
/// the commit to be refused by the deferred guard raising `expected`, with nothing left behind.
async fn expect_guard(
    harness: &Harness,
    case: &str,
    request: Option<&BankRequest>,
    tamper: &str,
    expected: &str,
) -> TestResult {
    let before = snapshot(&harness.pool).await?;
    let mut tx = match request {
        Some(request) => runtime_transaction(harness, request).await?.0,
        None => {
            let mut tx = harness.pool.begin().await?;
            sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
                .execute(&mut *tx)
                .await?;
            tx
        }
    };
    let result = match sqlx::raw_sql(sqlx::AssertSqlSafe(tamper.to_owned()))
        .execute(&mut *tx)
        .await
    {
        Ok(_) => tx.commit().await,
        Err(error) => Err(error),
    };
    let message = result
        .as_ref()
        .err()
        .and_then(|error| error.as_database_error())
        .map(|error| {
            (
                error.code().unwrap_or_default().into_owned(),
                error.message().to_owned(),
            )
        });
    match message {
        Some((code, message)) if code == "23514" && message.contains(expected) => {}
        other => return Err(format!("{case}: expected 23514 {expected:?}, got {other:?}").into()),
    }
    if snapshot(&harness.pool).await? != before {
        return Err(format!("{case}: a refused transaction changed durable state").into());
    }
    Ok(())
}

#[test]
fn every_deferred_guard_refuses_a_hand_written_row_and_the_runtime_cannot_delete() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "bank_guards", false).await?;
        seed_bank(&harness).await?;
        stack(&harness, 100, GOLD, 30, 1).await?;
        stack(&harness, 103, CRYSTAL, 1, 2).await?;
        stack(&harness, 106, GOLD, 5, 3).await?;
        seed_balance(&harness, ACCOUNT, CHARACTER, 5_000).await?;
        let account = uuid(ACCOUNT);
        let world = uuid(WORLD);
        let deposit_request = BankRequest::Deposit(deposit(1, 20)?);
        let withdraw_request = BankRequest::Withdraw(withdraw(2, 1_234)?);
        let transfer_request = BankRequest::Transfer(transfer(3, 100, FRIEND, "bankfriend")?);
        let tx_of = |n: u8| format!("'{}'::uuid", hex(&salted(n, 1)));

        // The balance equals its latest entry.
        expect_guard(&harness, "a balance moved without an entry", Some(&deposit_request),
            &format!("UPDATE game_account_bank_balances SET balance = balance + 1 \
                       WHERE account_id = {account};"), "a bank balance must equal its latest ledger entry").await?;
        // Each entry's before is the previous entry's after, and it is reached by the balance.
        expect_guard(&harness, "an entry appended to a committed operation", None,
            &format!("INSERT INTO game_account_bank_entries(entry_id, transaction_id, account_id, \
                        world_id, kind, amount, balance_before, balance_after, previous_entry_id, \
                        acting_character_id) \
                      SELECT '{}'::uuid, '{}'::uuid, {account}, {world}, 2, 1, balance, balance - 1, \
                        last_entry_id, {} FROM game_account_bank_balances \
                       WHERE account_id = {account}; \
                      UPDATE game_account_bank_balances SET balance = balance - 1, \
                        last_entry_id = '{}'::uuid WHERE account_id = {account};",
                hex(&salted(99, 3)), hex(&salted(ACCOUNT, 9)), uuid(CHARACTER), hex(&salted(99, 3))),
            "must chain from the previous entry").await?;
        // The chain itself: a before other than the previous entry's after.
        expect_guard(&harness, "an entry off the chain", None,
            &format!("INSERT INTO game_account_bank_entries(entry_id, transaction_id, account_id, \
                        world_id, kind, amount, balance_before, balance_after, previous_entry_id, \
                        acting_character_id) \
                      SELECT '{}'::uuid, '{}'::uuid, {account}, {world}, 2, 1, balance + 7, balance + 6, \
                        last_entry_id, {} FROM game_account_bank_balances \
                       WHERE account_id = {account};",
                hex(&salted(99, 4)), hex(&salted(ACCOUNT, 9)), uuid(CHARACTER)),
            "must chain from the previous entry").await?;
        // An operation has exactly the entries and lines its kind needs.
        expect_guard(&harness, "a deposit with a second entry", Some(&deposit_request),
            &format!("INSERT INTO game_account_bank_entries(entry_id, transaction_id, account_id, \
                        world_id, kind, amount, balance_before, balance_after, previous_entry_id, \
                        acting_character_id) \
                      SELECT '{}'::uuid, {}, {account}, {world}, 2, 1, balance, balance - 1, \
                        last_entry_id, {} FROM game_account_bank_balances \
                       WHERE account_id = {account}; \
                      UPDATE game_account_bank_balances SET balance = balance - 1, \
                        last_entry_id = '{}'::uuid WHERE account_id = {account};",
                hex(&salted(99, 4)), tx_of(1), uuid(CHARACTER), hex(&salted(99, 4))), "exactly its one ledger entry").await?;
        // A deposit's input worth minus its change equals its credit: one more output coin,
        // admitted by the item proofs as a line of this operation, breaks the conservation.
        let extra_output = |n: u8, ordinal: u8, coin: Coin| {
            let item = format!("'{}'::uuid", hex(&salted(97, n)));
            format!(
                "INSERT INTO game_item_instances(item_instance_id, world_id, definition_family, \
                   definition_production_key, definition_revision_ref, quantity, lifecycle, \
                   minted_transaction_id) \
                 VALUES ({item}, {world}, 'Item', '{key}', 'rev-1', 1, 1, {tx}); \
                 INSERT INTO game_item_container_entries(item_instance_id, world_id, character_id, \
                   parent_item_instance_id, placement_ordinal, placed_transaction_id) \
                 VALUES ({item}, {world}, {character}, {backpack}, 50, {tx}); \
                 INSERT INTO game_account_bank_coin_lines(transaction_id, line_ordinal, direction, \
                   item_instance_id, coin_worth, placement_ordinal, quantity_before, quantity_after) \
                 VALUES ({tx}, {ordinal}, 2, {item}, {worth}, 50, 0, 1);",
                tx = tx_of(n),
                key = coin.production_key(),
                worth = coin.worth(),
                character = uuid(CHARACTER),
                backpack = uuid(BACKPACK),
            )
        };
        expect_guard(&harness, "a deposit with an output beyond its change", Some(&deposit_request),
            &extra_output(1, 3, Coin::Platinum), "input worth minus its change must equal its credit").await?;
        // A withdrawal's output worth equals its debit.
        expect_guard(&harness, "a withdrawal minting one more coin", Some(&withdraw_request),
            &extra_output(2, 3, Coin::Crystal), "output worth must equal its debit").await?;
        // A transfer's two entries commit together, and nothing else.
        expect_guard(&harness, "a transfer with a third entry", Some(&transfer_request),
            &format!("INSERT INTO game_account_bank_entries(entry_id, transaction_id, account_id, \
                        world_id, kind, amount, balance_before, balance_after, previous_entry_id, \
                        acting_character_id) \
                      SELECT '{}'::uuid, {}, {account}, {world}, 1, 1, balance, balance + 1, \
                        last_entry_id, {} FROM game_account_bank_balances \
                       WHERE account_id = {account}; \
                      UPDATE game_account_bank_balances SET balance = balance + 1, \
                        last_entry_id = '{}'::uuid WHERE account_id = {account};",
                hex(&salted(99, 5)), tx_of(3), uuid(CHARACTER), hex(&salted(99, 5))), "two entries together on different accounts").await?;
        // A refused operation changes nothing.
        let before = snapshot(&harness.pool).await?;
        let mut tx = harness.pool.begin().await?;
        sqlx::query("SET LOCAL ROLE oteryn_game_runtime").execute(&mut *tx).await?;
        let result = sqlx::raw_sql(sqlx::AssertSqlSafe(format!(
            "INSERT INTO game_account_bank_operations(operation_occurrence_id, transaction_id, \
               event_id, kind, request_binding, acting_character_id, account_id, world_id, \
               channel_id, runtime_scope_ownership_generation, amount, outcome, occurred_at, \
               committed_at) \
             VALUES ('{}'::uuid, '{}'::uuid, '{}'::uuid, 3, sha256('x'), {}, {account}, {world}, \
               {}, 1, 1, 0, 1, 0);",
            hex(&salted(98, 7)), hex(&salted(98, 1)), hex(&salted(98, 2)), uuid(CHARACTER),
            uuid(CHANNEL),
        ))).execute(&mut *tx).await;
        assert!(result.is_err(), "an OK outcome needs its envelope digest (CHECK)");
        drop(tx);
        let mut tx = harness.pool.begin().await?;
        sqlx::query("SET LOCAL ROLE oteryn_game_runtime").execute(&mut *tx).await?;
        sqlx::raw_sql(sqlx::AssertSqlSafe(format!(
            "INSERT INTO game_account_bank_operations(operation_occurrence_id, transaction_id, \
               event_id, kind, request_binding, acting_character_id, account_id, world_id, \
               channel_id, runtime_scope_ownership_generation, amount, recipient_character_id, \
               recipient_name_key, outcome, occurred_at, committed_at) \
             VALUES ('{}'::uuid, '{}'::uuid, '{}'::uuid, 3, sha256('x'), {}, {account}, {world}, \
               {}, 1, 1, {}, 'bankfriend', 2, 1, 0); \
             INSERT INTO game_account_bank_balances VALUES ({}, {world}, 0, NULL) \
               ON CONFLICT DO NOTHING;",
            hex(&salted(98, 7)), hex(&salted(98, 1)), hex(&salted(98, 2)), uuid(CHARACTER),
            uuid(CHANNEL), uuid(FRIEND), uuid(FRIEND_ACCOUNT),
        ))).execute(&mut *tx).await?;
        sqlx::raw_sql(sqlx::AssertSqlSafe(format!(
            "INSERT INTO game_account_bank_entries(entry_id, transaction_id, account_id, world_id, \
               kind, amount, balance_before, balance_after, acting_character_id) \
             VALUES ('{}'::uuid, '{}'::uuid, {}, {world}, 1, 1, 0, 1, {}); \
             UPDATE game_account_bank_balances SET balance = 1, last_entry_id = '{}'::uuid \
              WHERE account_id = {};",
            hex(&salted(98, 3)), hex(&salted(98, 1)), uuid(FRIEND_ACCOUNT), uuid(FRIEND),
            hex(&salted(98, 3)), uuid(FRIEND_ACCOUNT),
        ))).execute(&mut *tx).await?;
        let refused = tx.commit().await;
        assert!(matches!(&refused, Err(error) if sqlstate(error) == "23514"), "a refused operation with an entry: {refused:?}");
        assert_eq!(snapshot(&harness.pool).await?, before);

        // The runtime role never deletes, nor rewrites the ledger.
        for statement in [
            "DELETE FROM game_account_bank_balances",
            "DELETE FROM game_account_bank_entries",
            "DELETE FROM game_account_bank_operations",
            "DELETE FROM game_account_bank_coin_lines",
            "DELETE FROM game_account_bank_audit_outbox",
            "UPDATE game_account_bank_entries SET amount = amount",
            "UPDATE game_account_bank_operations SET outcome = outcome",
            "TRUNCATE game_account_bank_balances",
        ] {
            let mut tx = harness.pool.begin().await?;
            sqlx::query("SET LOCAL ROLE oteryn_game_runtime").execute(&mut *tx).await?;
            let result = sqlx::query(sqlx::AssertSqlSafe(statement)).execute(&mut *tx).await;
            assert!(
                matches!(&result, Err(error) if sqlstate(error) == "42501"),
                "{statement}: {result:?}"
            );
        }
        // Not even the owner deletes a balance or rewrites an entry.
        for statement in [
            "DELETE FROM game_account_bank_balances",
            "UPDATE game_account_bank_entries SET amount = amount",
        ] {
            let result = sqlx::query(sqlx::AssertSqlSafe(statement)).execute(&harness.pool).await;
            assert!(
                matches!(&result, Err(error) if sqlstate(error) == "23514"),
                "{statement}: {result:?}"
            );
        }
        harness.cleanup().await
    })
}
