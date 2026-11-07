-- P04/P05: generated state extends the existing quest revision/receipt owner.
-- Currency balances and talisman levels stay in game_character_quest_tracks.
CREATE TABLE game_task_board_receipts (
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    world_id UUID NOT NULL,
    cause_kind TEXT NOT NULL CHECK (cause_kind IN ('command','creature_death')),
    cause_id UUID NOT NULL CHECK (game_character_is_uuid_v7(cause_id)),
    cause_ordinal NUMERIC(20,0) NOT NULL CHECK (cause_ordinal BETWEEN 0 AND 18446744073709551615),
    request_binding BYTEA NOT NULL CHECK (octet_length(request_binding)=32),
    reply JSONB NOT NULL CHECK (jsonb_typeof(reply)='object' AND octet_length(reply::text)<=65536),
    refusal JSONB NOT NULL,
    action JSONB NOT NULL,
    effects JSONB NOT NULL,
    committed_character_revision NUMERIC(20,0),
    created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
    PRIMARY KEY (character_id,cause_id,cause_ordinal),
    CHECK ((cause_kind='creature_death')=(cause_ordinal=0)),
    CHECK (committed_character_revision IS NULL OR committed_character_revision BETWEEN 2 AND 18446744073709551615)
);

CREATE TABLE game_character_task_boards (
    character_id UUID PRIMARY KEY REFERENCES game_character_roots(character_id),
    world_id UUID NOT NULL,
    body JSONB NOT NULL CHECK (jsonb_typeof(body)='object' AND octet_length(body::text)<=65536),
    committed_character_revision NUMERIC(20,0) NOT NULL CHECK (committed_character_revision BETWEEN 2 AND 18446744073709551615),
    cause_id UUID NOT NULL,
    cause_ordinal NUMERIC(20,0) NOT NULL,
    FOREIGN KEY (character_id,cause_id,cause_ordinal)
        REFERENCES game_task_board_receipts(character_id,cause_id,cause_ordinal)
        DEFERRABLE INITIALLY DEFERRED
);

-- Immutable fulfilment identity is separate from session-local CommandRef. This
-- is the Game entitlement grant seam; it neither offers nor bills Store coins.
CREATE TABLE game_task_board_expansion_deliveries (
    delivery_reference TEXT PRIMARY KEY CHECK (
        octet_length(delivery_reference) BETWEEN 1 AND 256
        AND delivery_reference ~ '^[A-Za-z0-9._:/-]+$'),
    character_id UUID NOT NULL,
    world_id UUID NOT NULL,
    cause_id UUID NOT NULL,
    cause_ordinal NUMERIC(20,0) NOT NULL,
    created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
    UNIQUE(character_id,cause_id,cause_ordinal),
    FOREIGN KEY(character_id,cause_id,cause_ordinal)
        REFERENCES game_task_board_receipts DEFERRABLE INITIALLY DEFERRED
);

-- Read the last committed server-save publication. The Monday calendar boundary
-- changes only when the World-reset owner advances its epoch, including DST.
CREATE FUNCTION game_task_board_period(p_world UUID) RETURNS BIGINT
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE epoch NUMERIC; at_time TIMESTAMPTZ; calendar_name TEXT;
BEGIN
    SELECT reset_epoch INTO epoch FROM game_world_reset_active WHERE world_id=p_world FOR SHARE;
    IF NOT FOUND THEN RETURN NULL; END IF;
    SELECT activated_at,calendar INTO at_time,calendar_name FROM game_world_reset_epochs
        WHERE world_id=p_world AND reset_epoch=epoch;
    IF calendar_name IS DISTINCT FROM 'TIBIA_SERVER_SAVE_BERLIN_06' THEN RETURN NULL; END IF;
    RETURN (date_trunc('week',(at_time AT TIME ZONE 'Europe/Berlin')-interval '6 hours')::date-DATE '1970-01-01')::bigint;
END $$;

CREATE FUNCTION game_task_board_state_row_guard() RETURNS TRIGGER
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP='INSERT' THEN RETURN NEW; END IF;
    IF TG_OP='UPDATE' AND NEW.character_id=OLD.character_id AND NEW.world_id=OLD.world_id
        AND NEW.committed_character_revision>OLD.committed_character_revision THEN RETURN NEW; END IF;
    RAISE EXCEPTION 'Task Board state must advance its quest receipt' USING ERRCODE='23514';
END $$;

CREATE FUNCTION game_task_board_state_guard() RETURNS TRIGGER
LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS(SELECT 1 FROM game_task_board_receipts r
        WHERE r.character_id=NEW.character_id AND r.world_id=NEW.world_id
        AND r.cause_id=NEW.cause_id AND r.cause_ordinal=NEW.cause_ordinal
        AND r.committed_character_revision=NEW.committed_character_revision
        AND r.refusal='null'::jsonb AND r.reply->'data'=NEW.body
        AND r.created_xact_id=pg_current_xact_id()) THEN
        RAISE EXCEPTION 'Task Board state has no matching same-transaction receipt' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;

CREATE FUNCTION game_task_board_receipt_guard() RETURNS TRIGGER
LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.created_xact_id<>pg_current_xact_id()
        OR ((NEW.reply->>'period')::bigint IS DISTINCT FROM game_task_board_period(NEW.world_id)
            AND NOT (NEW.cause_kind='creature_death' AND NEW.action ? 'CreditKill'
                AND game_task_board_period(NEW.world_id) IS NULL AND NEW.reply->>'period'='0'
                AND (NEW.reply->'data'->'weekly'='null'::jsonb OR NEW.reply->'data'->'weekly'->>'finished'='true')
                AND NEW.reply->'data'->'bounty'->>'state'<>'Active'
                AND NEW.committed_character_revision IS NULL))
        OR (NEW.reply->>'level')::bigint NOT BETWEEN 1 AND 4294967295 THEN
        RAISE EXCEPTION 'Task Board receipt has invalid reset or transaction provenance' USING ERRCODE='23514';
    END IF;
    IF NEW.committed_character_revision IS NOT NULL THEN
        IF NEW.refusal<>'null'::jsonb OR NOT EXISTS(
            SELECT 1 FROM game_character_quest_receipts q
            WHERE q.character_id=NEW.character_id AND q.cause_id=NEW.cause_id
            AND q.cause_ordinal=NEW.cause_ordinal
            AND q.transition_key='oteryn:taskboard/weekly/advance'
            AND q.quest_key='oteryn:taskboard/weekly'
            AND q.cause_kind=CASE WHEN NEW.cause_kind='command' THEN 'use' ELSE NEW.cause_kind END
            AND q.request_binding=NEW.request_binding
            AND q.committed_character_revision=NEW.committed_character_revision
            AND q.created_xact_id=pg_current_xact_id())
            OR NOT EXISTS(SELECT 1 FROM game_character_task_boards s
                WHERE s.character_id=NEW.character_id AND s.world_id=NEW.world_id
                AND s.committed_character_revision=NEW.committed_character_revision
                AND s.body=NEW.reply->'data') THEN
            RAISE EXCEPTION 'Task Board mutation is not in the quest revision chain' USING ERRCODE='23514';
        END IF;
    END IF;
    IF NEW.action='"GrantExpansion"'::jsonb AND NEW.refusal='null'::jsonb
        AND NOT EXISTS(SELECT 1 FROM game_task_board_expansion_deliveries d
            WHERE d.character_id=NEW.character_id AND d.world_id=NEW.world_id
            AND d.cause_id=NEW.cause_id AND d.cause_ordinal=NEW.cause_ordinal
            AND d.created_xact_id=pg_current_xact_id()) THEN
        RAISE EXCEPTION 'Task Board expansion requires a trusted delivery source' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;

CREATE FUNCTION game_task_board_expansion_delivery_guard() RETURNS TRIGGER
LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.created_xact_id<>pg_current_xact_id() OR NOT EXISTS(
        SELECT 1 FROM game_task_board_receipts r WHERE r.character_id=NEW.character_id
        AND r.world_id=NEW.world_id AND r.cause_id=NEW.cause_id AND r.cause_ordinal=NEW.cause_ordinal
        AND r.cause_kind='command' AND r.action='"GrantExpansion"'::jsonb
        AND r.refusal='null'::jsonb AND r.committed_character_revision IS NOT NULL
        AND r.reply->'data'->'expansion'='true'::jsonb
        AND r.created_xact_id=pg_current_xact_id()) THEN
        RAISE EXCEPTION 'Task Board expansion source lacks its successful entitlement receipt' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;

CREATE TRIGGER game_task_board_state_row_guard BEFORE INSERT OR UPDATE OR DELETE
    ON game_character_task_boards FOR EACH ROW EXECUTE FUNCTION game_task_board_state_row_guard();
CREATE CONSTRAINT TRIGGER game_task_board_state_guard AFTER INSERT OR UPDATE
    ON game_character_task_boards DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION game_task_board_state_guard();
CREATE CONSTRAINT TRIGGER game_task_board_receipt_guard AFTER INSERT
    ON game_task_board_receipts DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION game_task_board_receipt_guard();
CREATE TRIGGER game_task_board_receipts_immutable BEFORE UPDATE OR DELETE
    ON game_task_board_receipts FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_task_board_receipts_no_truncate BEFORE TRUNCATE
    ON game_task_board_receipts FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();
CREATE TRIGGER game_task_board_state_no_truncate BEFORE TRUNCATE
    ON game_character_task_boards FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();
CREATE CONSTRAINT TRIGGER game_task_board_expansion_delivery_guard AFTER INSERT
    ON game_task_board_expansion_deliveries DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION game_task_board_expansion_delivery_guard();
CREATE TRIGGER game_task_board_expansion_deliveries_immutable BEFORE UPDATE OR DELETE
    ON game_task_board_expansion_deliveries FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_task_board_expansion_deliveries_no_truncate BEFORE TRUNCATE
    ON game_task_board_expansion_deliveries FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();

DO $$
DECLARE signature TEXT;
BEGIN
    FOREACH signature IN ARRAY ARRAY['game_task_board_period(uuid)',
        'game_task_board_state_row_guard()','game_task_board_state_guard()',
        'game_task_board_receipt_guard()','game_task_board_expansion_delivery_guard()'] LOOP
        EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',signature,current_schema());
        EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',signature);
    END LOOP;
END $$;
REVOKE ALL ON game_task_board_receipts,game_character_task_boards FROM PUBLIC;
REVOKE ALL ON game_task_board_expansion_deliveries FROM PUBLIC;
GRANT SELECT,INSERT ON game_task_board_receipts TO oteryn_game_runtime;
GRANT SELECT,INSERT,UPDATE ON game_character_task_boards TO oteryn_game_runtime;
GRANT SELECT,INSERT ON game_task_board_expansion_deliveries TO oteryn_game_runtime;
GRANT EXECUTE ON FUNCTION game_task_board_period(UUID) TO oteryn_game_runtime;
