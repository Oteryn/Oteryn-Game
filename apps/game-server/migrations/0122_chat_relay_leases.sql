-- CHAT-2B: the durable relay lease of the World chat relay (decision CHAT-0,
-- `reviews/OTERYN_GAME_CHAT0_PLAYER_CHAT_DECISION_2026-09-30.md` §5, replay protection).
--
-- Scope of this migration only:
--   * `game_chat_relay_leases`: one row per World. A listener of the World relay acts on relay
--     lines only on a database-time reading at most the lease it stored here, and a restarted
--     listener starts past the highest lease stored, so a captured line cannot be admitted again
--     after a restart or a database clock step back (`CHAT0-RL-13`);
--   * the order guard: an UPDATE keeps the World and never lowers the lease. Writers store
--     `GREATEST(stored, new)`;
--   * grants: the runtime role may SELECT, INSERT and UPDATE, never DELETE; the control role may
--     SELECT.
--
-- No chat text, sender, recipient or key material is stored here (FND-02 §20).
--
-- Provisioning, outside this migration (CP decision D1400): the relay pins statement and
-- bind-parameter logging off with session `SET`s on every connection it opens, and refuses to
-- start when any of them fails. A superuser grants the login role the relay connects as (the
-- node's runtime role, a member of `oteryn_game_runtime`) the right to set them, once per cluster:
--
--   GRANT SET ON PARAMETER log_parameter_max_length, log_parameter_max_length_on_error,
--       log_statement, log_min_duration_statement TO <runtime login role>;
--
-- `log_parameter_max_length_on_error` is user-settable and needs no grant; the other three do.
-- The migration role need not be a superuser, so this migration does not grant it. Without the
-- grant the relay stays disabled (`chat_relay_disabled reason=log_pin_refused`).
--
-- Rollback: applied migrations are immutable, so rollback is a new migration that drops the
-- table, its triggers and its function. Only the chat relay reads the table.

CREATE TABLE game_chat_relay_leases (
    world_id UUID PRIMARY KEY CHECK (game_character_is_uuid_v7(world_id)),
    leased_until_micros BIGINT NOT NULL CHECK (leased_until_micros >= 0)
);

-- The lease of a World never moves back and never changes World.
CREATE FUNCTION game_chat_relay_lease_order_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.world_id IS DISTINCT FROM OLD.world_id
       OR NEW.leased_until_micros < OLD.leased_until_micros THEN
        RAISE EXCEPTION 'a chat relay lease keeps its World and never decreases'
            USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER game_chat_relay_lease_order BEFORE UPDATE
    ON game_chat_relay_leases
    FOR EACH ROW EXECUTE FUNCTION game_chat_relay_lease_order_guard();
CREATE TRIGGER game_chat_relay_leases_no_truncate BEFORE TRUNCATE
    ON game_chat_relay_leases
    FOR EACH STATEMENT EXECUTE FUNCTION game_character_reject_truncate();

DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_chat_relay_lease_order_guard() SET search_path = %I, pg_temp',
        current_schema());
END $$;

REVOKE ALL ON game_chat_relay_leases FROM PUBLIC;
REVOKE ALL ON FUNCTION game_chat_relay_lease_order_guard() FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE ON game_chat_relay_leases TO oteryn_game_runtime;
GRANT SELECT ON game_chat_relay_leases TO oteryn_game_control;
