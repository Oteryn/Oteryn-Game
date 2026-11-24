-- Test-only historical deposit, using the same ledger/balance shape as
-- tests/support/bank_postgres_cases.rs::seed_balance.
WITH actor AS (
    SELECT account_id, character_id, world_id
    FROM game_character_roots
    WHERE account_id = $1::uuid AND world_id = encode($2, 'hex')::uuid
), deposit AS (
    INSERT INTO game_account_bank_operations(
        operation_occurrence_id, transaction_id, event_id, kind, request_binding,
        acting_character_id, account_id, world_id, channel_id, runtime_scope_ownership_generation,
        amount, outcome, occurred_at, envelope_sha256, committed_at,
        planned_platinum_item_instance_id, planned_gold_item_instance_id, backpack_item_instance_id
    )
    SELECT uuidv7(), uuidv7(), uuidv7(), 1, sha256('house sweep fixture'),
           character_id, account_id, world_id, encode($3, 'hex')::uuid, $4::text::numeric,
           1, 0, 1, sha256('house sweep fixture'), 0, uuidv7(), uuidv7(), uuidv7()
    FROM actor
    RETURNING transaction_id, acting_character_id, account_id, world_id
), credit AS (
    INSERT INTO game_account_bank_entries(
        entry_id, transaction_id, account_id, world_id, kind, amount,
        balance_before, balance_after, acting_character_id
    )
    SELECT uuidv7(), transaction_id, account_id, world_id, 1, $5, 0, $5, acting_character_id
    FROM deposit
    RETURNING entry_id, account_id, world_id, balance_after
)
INSERT INTO game_account_bank_balances(account_id, world_id, balance, last_entry_id)
SELECT account_id, world_id, balance_after, entry_id FROM credit
