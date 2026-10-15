-- CHAR-CMD-CONSUMER-1: the Game receipt store of Platform character command intents
-- (`docs/contracts/OTERYN_GAME_CHARACTER_AUTHORITY_COMMANDS_V1.md` §5.2, §5.3, §6.1).
--
-- Scope of this migration only:
--   * `game_character_command_receipts`: one immutable receipt per `operation_id` (the
--     uniqueness scope is `(issuer_authority, operation_id)` and Game accepts one issuer, §3),
--     with the canonical binding of what was decided and the exact receipt bytes Game publishes
--     on every retry. Deterministic rejections are stored too, so a rejected `operation_id` can
--     never commit later;
--   * the §5.3 per-revision check: within a variant, at most one non-conflict receipt holds a
--     `source_revision`. A lower revision than one already decided is allowed;
--   * a committed create also names its Character, account and World and its audit identity;
--   * `game_character_creation_receipts`: every Character root's creation receipt, from the
--     operator bootstrap (`game_character_operation_receipts`) or a committed create command.
--     Character reads and the integrity check use it, so a command-created root is a root with
--     a creation receipt like any other;
--   * grants: the runtime role may SELECT and INSERT, never UPDATE or DELETE; the control role
--     may SELECT.
--
-- An `operation_id` is held by at most one of the two receipt tables; the deciding transaction
-- checks both under the operation advisory lock the bootstrap already takes.
--
-- Rollback: applied migrations are immutable, so rollback is a new migration that drops the
-- view and the table. A command-created Character root then has no creation receipt and the
-- integrity check refuses the store, so rollback is only for a store with no committed create.

CREATE TABLE game_character_command_receipts (
    operation_id UUID PRIMARY KEY CHECK (game_character_is_uuid_v7(operation_id)),
    command TEXT NOT NULL CHECK (command ~ '^[A-Za-z0-9_]{1,64}$'),
    -- 1 = PLATFORM_USER_CREATE. NULL when no supported intent was decoded.
    variant SMALLINT CHECK (variant = 1),
    source_revision NUMERIC(20,0)
        CHECK (source_revision BETWEEN 1 AND 18446744073709551615),
    command_binding BYTEA NOT NULL CHECK (octet_length(command_binding) BETWEEN 1 AND 1024),
    -- 1 = COMMITTED, 2 = REJECTED.
    outcome SMALLINT NOT NULL CHECK (outcome IN (1, 2)),
    result_code TEXT NOT NULL CHECK (result_code ~ '^CHAR_CMD_[A-Z_]{1,40}$'),
    recovery_generation NUMERIC(20,0) NOT NULL
        CHECK (recovery_generation BETWEEN 1 AND 18446744073709551615),
    decided_at BIGINT NOT NULL CHECK (decided_at > 0),
    receipt BYTEA NOT NULL CHECK (octet_length(receipt) BETWEEN 1 AND 1024),
    character_id UUID UNIQUE REFERENCES game_character_roots(character_id)
        CHECK (game_character_is_uuid_v7(character_id)),
    account_id UUID CHECK (game_character_is_uuid_v7(account_id)),
    world_id UUID CHECK (game_character_is_uuid_v7(world_id)),
    event_id UUID UNIQUE CHECK (game_character_is_uuid_v7(event_id)),
    transaction_id UUID UNIQUE CHECK (game_character_is_uuid_v7(transaction_id)),
    server_build_id TEXT CHECK (octet_length(server_build_id) BETWEEN 1 AND 128
        AND server_build_id !~ '[^A-Za-z0-9._:/+-]'),
    occurred_at BIGINT CHECK (occurred_at >= 0),
    CHECK ((variant IS NULL) = (source_revision IS NULL)),
    CHECK ((outcome = 1) = (result_code = 'CHAR_CMD_OK')),
    -- A committed receipt is a committed create of a decoded intent.
    CHECK (outcome = 2 OR variant IS NOT NULL),
    CHECK (
        (outcome = 1) = (character_id IS NOT NULL) AND (outcome = 1) = (account_id IS NOT NULL)
        AND (outcome = 1) = (world_id IS NOT NULL) AND (outcome = 1) = (event_id IS NOT NULL)
        AND (outcome = 1) = (transaction_id IS NOT NULL)
        AND (outcome = 1) = (server_build_id IS NOT NULL)
        AND (outcome = 1) = (occurred_at IS NOT NULL)
    )
);

-- §5.3: equal `source_revision` within a variant with another operation is a contradiction;
-- the newcomer's own receipt is the conflict, which this index does not count.
CREATE UNIQUE INDEX game_character_command_receipts_source_revision
    ON game_character_command_receipts (variant, source_revision)
    WHERE variant IS NOT NULL AND result_code <> 'CHAR_CMD_OPERATION_CONFLICT';

CREATE TRIGGER game_character_command_receipt_immutable BEFORE UPDATE OR DELETE
    ON game_character_command_receipts FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
CREATE TRIGGER game_character_command_receipts_no_truncate BEFORE TRUNCATE
    ON game_character_command_receipts EXECUTE FUNCTION game_character_reject_truncate();

CREATE VIEW game_character_creation_receipts AS
    SELECT operation_id, character_id, account_id, world_id, event_id, transaction_id,
        server_build_id, occurred_at
    FROM game_character_operation_receipts
    UNION ALL
    SELECT operation_id, character_id, account_id, world_id, event_id, transaction_id,
        server_build_id, occurred_at
    FROM game_character_command_receipts
    WHERE outcome = 1;

REVOKE ALL ON game_character_command_receipts, game_character_creation_receipts FROM PUBLIC;
GRANT SELECT, INSERT ON game_character_command_receipts TO oteryn_game_runtime;
GRANT SELECT ON game_character_creation_receipts TO oteryn_game_runtime;
GRANT SELECT ON game_character_command_receipts, game_character_creation_receipts
    TO oteryn_game_control;
