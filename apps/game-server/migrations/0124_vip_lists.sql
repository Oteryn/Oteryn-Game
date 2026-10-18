-- VIP-1b (CP decision D1444): the durable VIP list of an account in one World (decision VIP0,
-- `docs/architecture/reviews/OTERYN_GAME_VIP0_VIP_LIST_DECISION_2026-09-30.md`
-- §5, §8, §9).
--
-- Scope of this migration only:
--   * two columns on `game_character_social_settings`: the character's shown/hidden VIP status
--     (VIP0 §5.3) and its consent revision, which every grant, end and withdrawal of a watcher's
--     consent advances (VIP0 §8.2, the presence owner's floor), and the PARTY baseline row of
--     every character that has none;
--   * `game_vip_lists`: one row per account and World, holding the list revision, the next entry
--     number and the persisted RL-07 command bucket;
--   * `game_vip_entries`, `game_vip_groups` and `game_vip_group_members`: entries, the custom
--     groups (numbers 4 to 8; 1 to 3 are the built-in groups) and memberships;
--   * `game_vip_consents` and the `game_vip_invitation_id` sequence: a pending invitation or an
--     accepted consent of a listed character to its watcher (VIP0 §5.1);
--   * `game_vip_command_receipts`: one immutable receipt per CommandRef. It is removed only with
--     its list;
--   * grants: the runtime role may read every VIP row and write only the columns the VIP writers
--     write. Receipts are insert-only. The control role may SELECT.
--
-- No presence, login state or chat text is stored here: presence stays in node memory and the
-- World chat relay (VIP0 §8). The trigger functions pin their search path and are not
-- executable by PUBLIC.
--
-- Rollback: applied migrations are immutable, so rollback is a new migration that drops the
-- tables, the sequence, the functions and the two settings columns. Only the VIP writers and
-- the VIP presence owner read them.

ALTER TABLE game_character_social_settings
    ADD COLUMN vip_status SMALLINT NOT NULL DEFAULT 1 CHECK (vip_status IN (1, 2)),
    ADD COLUMN vip_consent_revision NUMERIC(20, 0) NOT NULL DEFAULT 0
        CHECK (vip_consent_revision BETWEEN 0 AND 18446744073709551615);

-- A target must have its settings row to be listed. As 0046 did, every character created since
-- gets the accepted PARTY baseline (PARTYPVP0 §3: the defaults of an absent row); the admission
-- producer of `durability::vip` makes the row of every later character at its admission.
INSERT INTO game_character_social_settings (character_id, party_invites, channel_visibility,
    source_policy)
SELECT character_id, 1, 1, 'PARTYPVP0-PARTIES-AND-PVP-V1' FROM game_character_roots
ON CONFLICT (character_id) DO NOTHING;

CREATE TABLE game_vip_lists (
    account_id UUID NOT NULL REFERENCES game_character_account_guards(account_id),
    world_id UUID NOT NULL,
    revision NUMERIC(20, 0) NOT NULL CHECK (revision BETWEEN 1 AND 18446744073709551615),
    next_entry_no NUMERIC(20, 0) NOT NULL
        CHECK (next_entry_no BETWEEN 1 AND 18446744073709551615),
    rate_tokens SMALLINT NOT NULL CHECK (rate_tokens BETWEEN 0 AND 20),
    rate_at BIGINT NOT NULL CHECK (rate_at >= 0),
    PRIMARY KEY (account_id, world_id)
);

CREATE TABLE game_vip_entries (
    account_id UUID NOT NULL,
    world_id UUID NOT NULL,
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    entry_no NUMERIC(20, 0) NOT NULL CHECK (entry_no > 0),
    description TEXT NOT NULL DEFAULT '',
    icon SMALLINT NOT NULL DEFAULT 0,
    notify BOOLEAN NOT NULL DEFAULT FALSE,
    added_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    PRIMARY KEY (account_id, world_id, character_id),
    UNIQUE (account_id, world_id, entry_no),
    FOREIGN KEY (account_id, world_id) REFERENCES game_vip_lists ON DELETE CASCADE
);
-- The presence owner finds the watchers of a target by the target.
CREATE INDEX game_vip_entries_target ON game_vip_entries(character_id);

CREATE SEQUENCE game_vip_invitation_id NO CYCLE;

CREATE TABLE game_vip_consents (
    account_id UUID NOT NULL,
    world_id UUID NOT NULL,
    inviter_character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    character_id UUID NOT NULL,
    invitation_id BIGINT NOT NULL DEFAULT nextval('game_vip_invitation_id') UNIQUE
        CHECK (invitation_id > 0),
    consent SMALLINT NOT NULL CHECK (consent IN (1, 2)),
    invite_expires_at TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (account_id, world_id, inviter_character_id, character_id),
    CHECK (inviter_character_id <> character_id),
    FOREIGN KEY (account_id, world_id, character_id) REFERENCES game_vip_entries ON DELETE CASCADE
);
CREATE INDEX game_vip_consents_target ON game_vip_consents(character_id);
ALTER SEQUENCE game_vip_invitation_id OWNED BY game_vip_consents.invitation_id;

CREATE TABLE game_vip_groups (
    account_id UUID NOT NULL,
    world_id UUID NOT NULL,
    group_no SMALLINT NOT NULL CHECK (group_no BETWEEN 4 AND 8),
    name TEXT NOT NULL CHECK (char_length(name) BETWEEN 1 AND 25),
    PRIMARY KEY (account_id, world_id, group_no),
    FOREIGN KEY (account_id, world_id) REFERENCES game_vip_lists ON DELETE CASCADE
);
CREATE UNIQUE INDEX game_vip_group_names ON game_vip_groups(account_id, world_id, lower(name));

CREATE TABLE game_vip_group_members (
    account_id UUID NOT NULL,
    world_id UUID NOT NULL,
    character_id UUID NOT NULL,
    group_no SMALLINT NOT NULL CHECK (group_no BETWEEN 1 AND 8),
    PRIMARY KEY (account_id, world_id, character_id, group_no),
    FOREIGN KEY (account_id, world_id, character_id) REFERENCES game_vip_entries ON DELETE CASCADE
);

-- Receipt kinds: 1 withdraw, 2 reject, 3 revoke, 4 accept, 5 add, 6 remove, 7 group add,
-- 8 group rename, 9 group remove, 10 edit, 11 status. Results: 1 OK, 2 NOT_FOUND, 3 EXHAUSTED,
-- 4 LIST_FULL, 5 the add's UNKNOWN_NAME, 6 ALREADY_LISTED, 7 REJECTED, 8 GROUP_LIMIT,
-- 9 NAME_TAKEN. These are storage codes, not wire tags.
CREATE TABLE game_vip_command_receipts (
    account_id UUID NOT NULL,
    world_id UUID NOT NULL,
    game_session_id UUID NOT NULL,
    command_id NUMERIC(20, 0) NOT NULL CHECK (command_id > 0),
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    kind SMALLINT NOT NULL CHECK (kind BETWEEN 1 AND 11),
    binding BYTEA NOT NULL CHECK (octet_length(binding) = 32),
    result SMALLINT NOT NULL CHECK (result BETWEEN 1 AND 9),
    seconds SMALLINT,
    revision NUMERIC(20, 0) NOT NULL CHECK (revision BETWEEN 1 AND 18446744073709551615),
    committed_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    entry_no NUMERIC(20, 0) CHECK (entry_no BETWEEN 1 AND 18446744073709551615),
    group_no SMALLINT CHECK (group_no BETWEEN 4 AND 8),
    PRIMARY KEY (account_id, world_id, game_session_id, command_id),
    FOREIGN KEY (account_id, world_id) REFERENCES game_vip_lists ON DELETE CASCADE,
    CHECK ((kind = 5 AND result IN (1, 3, 4, 5, 6, 7))
        OR (kind IN (1, 2, 3, 4) AND result IN (1, 2, 3, 4))
        OR (kind = 6 AND result IN (1, 2, 3))
        OR (kind BETWEEN 7 AND 11 AND result IN (1, 2, 3, 7, 8, 9))),
    CHECK ((kind = 5 AND result IN (1, 6)) = (entry_no IS NOT NULL)),
    CHECK ((kind = 7 AND result = 1) = (group_no IS NOT NULL)),
    CHECK ((result = 3 AND seconds IS NOT NULL AND seconds BETWEEN 1 AND 3)
        OR (result <> 3 AND seconds IS NULL))
);

-- A receipt never changes. It is deleted only by the cascade of its list's deletion.
CREATE FUNCTION game_vip_receipt_immutable() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' AND NOT EXISTS (
        SELECT 1 FROM game_vip_lists
         WHERE account_id = OLD.account_id AND world_id = OLD.world_id
    ) THEN
        RETURN OLD;
    END IF;
    RAISE EXCEPTION 'immutable VIP receipt' USING ERRCODE = '23514';
END;
$$;
CREATE TRIGGER game_vip_receipt_immutable BEFORE UPDATE OR DELETE
    ON game_vip_command_receipts
    FOR EACH ROW EXECUTE FUNCTION game_vip_receipt_immutable();

CREATE TRIGGER game_vip_lists_no_truncate BEFORE TRUNCATE ON game_vip_lists
    FOR EACH STATEMENT EXECUTE FUNCTION game_character_reject_truncate();
CREATE TRIGGER game_vip_entries_no_truncate BEFORE TRUNCATE ON game_vip_entries
    FOR EACH STATEMENT EXECUTE FUNCTION game_character_reject_truncate();
CREATE TRIGGER game_vip_consents_no_truncate BEFORE TRUNCATE ON game_vip_consents
    FOR EACH STATEMENT EXECUTE FUNCTION game_character_reject_truncate();
CREATE TRIGGER game_vip_groups_no_truncate BEFORE TRUNCATE ON game_vip_groups
    FOR EACH STATEMENT EXECUTE FUNCTION game_character_reject_truncate();
CREATE TRIGGER game_vip_group_members_no_truncate BEFORE TRUNCATE ON game_vip_group_members
    FOR EACH STATEMENT EXECUTE FUNCTION game_character_reject_truncate();
CREATE TRIGGER game_vip_command_receipts_no_truncate BEFORE TRUNCATE
    ON game_vip_command_receipts
    FOR EACH STATEMENT EXECUTE FUNCTION game_character_reject_truncate();

DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_vip_receipt_immutable() SET search_path = pg_catalog, %I, pg_temp',
        current_schema());
END $$;
REVOKE ALL ON FUNCTION game_vip_receipt_immutable() FROM PUBLIC;

REVOKE ALL ON game_vip_lists, game_vip_entries, game_vip_consents, game_vip_groups,
    game_vip_group_members, game_vip_command_receipts
    FROM PUBLIC, oteryn_game_runtime, oteryn_game_control;
REVOKE ALL ON SEQUENCE game_vip_invitation_id FROM PUBLIC, oteryn_game_runtime, oteryn_game_control;

-- Exactly the writes of `durability::vip`; `durability::vip_presence` only reads.
GRANT SELECT, INSERT, UPDATE (revision, next_entry_no, rate_tokens, rate_at)
    ON game_vip_lists TO oteryn_game_runtime;
GRANT SELECT, INSERT, DELETE, UPDATE (description, icon, notify)
    ON game_vip_entries TO oteryn_game_runtime;
GRANT SELECT, INSERT, DELETE, UPDATE (consent) ON game_vip_consents TO oteryn_game_runtime;
GRANT SELECT, INSERT, DELETE, UPDATE (name) ON game_vip_groups TO oteryn_game_runtime;
GRANT SELECT, INSERT, DELETE ON game_vip_group_members TO oteryn_game_runtime;
GRANT SELECT, INSERT ON game_vip_command_receipts TO oteryn_game_runtime;
GRANT USAGE ON SEQUENCE game_vip_invitation_id TO oteryn_game_runtime;

GRANT SELECT ON game_vip_lists, game_vip_entries, game_vip_consents, game_vip_groups,
    game_vip_group_members, game_vip_command_receipts TO oteryn_game_control;
