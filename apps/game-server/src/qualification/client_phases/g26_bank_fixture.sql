-- Retained opening balance for the dedicated buyer. The sale writes all escrow
-- entries through its own authority, with ordinary database guards enabled.
WITH operation AS (
    INSERT INTO game_account_bank_operations (
        operation_occurrence_id, transaction_id, event_id, kind, request_binding,
        acting_character_id, account_id, world_id, channel_id,
        runtime_scope_ownership_generation, amount, outcome, occurred_at,
        envelope_sha256, committed_at, planned_platinum_item_instance_id,
        planned_gold_item_instance_id, backpack_item_instance_id
    ) VALUES (
        game_character_uuid_v7(), game_character_uuid_v7(), game_character_uuid_v7(), 1,
        sha256('q_g26 opening balance'), encode($4,'hex')::uuid, $1::uuid,
        encode($2,'hex')::uuid, encode($3,'hex')::uuid, 1, $5, 0, 1,
        sha256('q_g26 opening balance'), 0, game_character_uuid_v7(),
        game_character_uuid_v7(), game_character_uuid_v7()
    ) RETURNING transaction_id, account_id, world_id, amount, acting_character_id
), entry AS (
    INSERT INTO game_account_bank_entries (
        entry_id, transaction_id, account_id, world_id, kind, amount,
        balance_before, balance_after, acting_character_id
    )
    SELECT game_character_uuid_v7(), transaction_id, account_id, world_id, 1, amount,
        0, amount, acting_character_id FROM operation
    RETURNING entry_id, account_id, world_id, balance_after
)
INSERT INTO game_account_bank_balances (account_id, world_id, balance, last_entry_id)
SELECT account_id, world_id, balance_after, entry_id FROM entry;
