-- Typed NPC trade continuations use the existing Quest obligation/recovery owner.
-- The trade transaction captures eligible persisted milestones before committing value;
-- the Quest transaction alone changes progress and consumes the original source proof.
ALTER TABLE game_character_quest_obligations
    ADD COLUMN source_kind TEXT NOT NULL DEFAULT 'reward_claim'
        CHECK (source_kind IN ('reward_claim','npc_trade')),
    ADD COLUMN source_trade_transaction_id UUID NULL REFERENCES game_npc_trades(transaction_id),
    ADD COLUMN source_binding BYTEA NULL,
    ADD COLUMN source_state JSONB NULL,
    ADD CONSTRAINT game_quest_obligation_source_shape CHECK (
        (source_kind='reward_claim' AND source_trade_transaction_id IS NULL AND source_binding IS NULL AND source_state IS NULL
            AND transition_key NOT LIKE 'oteryn:quest-trade/%')
        OR (source_kind='npc_trade' AND source_trade_transaction_id IS NOT NULL
            AND source_binding IS NOT NULL AND octet_length(source_binding)=32
            AND source_state IS NOT NULL AND jsonb_typeof(source_state)='object'
            AND transition_key LIKE 'oteryn:quest-trade/%'));
CREATE UNIQUE INDEX game_quest_obligation_trade_source
    ON game_character_quest_obligations(source_trade_transaction_id)
    WHERE source_trade_transaction_id IS NOT NULL;
ALTER TABLE game_character_quest_receipts DROP CONSTRAINT game_character_quest_receipts_cause_kind_check;
ALTER TABLE game_character_quest_receipts ADD CONSTRAINT game_character_quest_receipts_cause_kind_check
    CHECK (cause_kind IN ('command','use','step','claim_obligation','creature_death','trade_obligation'));

CREATE OR REPLACE FUNCTION game_character_quest_obligation_row_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        IF NEW.state = 'PENDING' THEN
            RETURN NEW;
        END IF;
    ELSIF TG_OP = 'UPDATE' THEN
        IF (NEW.claim_game_session_id, NEW.claim_command_id, NEW.character_id,
            NEW.transition_key, NEW.created_at, NEW.created_xact_id, NEW.source_kind, NEW.source_trade_transaction_id, NEW.source_binding, NEW.source_state)
           IS NOT DISTINCT FROM (OLD.claim_game_session_id, OLD.claim_command_id, OLD.character_id,
              OLD.transition_key, OLD.created_at, OLD.created_xact_id, OLD.source_kind, OLD.source_trade_transaction_id, OLD.source_binding, OLD.source_state)
           AND NEW.updated_at >= OLD.updated_at
           AND ((OLD.state = 'PENDING' AND NEW.state IN ('REFUSED', 'WAITING_MIGRATION'))
                OR (OLD.state = 'WAITING_MIGRATION' AND NEW.state = 'PENDING')
                OR (OLD.state = 'PENDING' AND NEW.state = 'CONSUMED' AND EXISTS (
                    SELECT 1 FROM game_character_quest_receipts r
                     WHERE r.character_id = OLD.character_id
                       AND r.cause_kind = CASE OLD.source_kind WHEN 'npc_trade' THEN 'trade_obligation' ELSE 'claim_obligation' END
                       AND r.cause_id = OLD.claim_game_session_id
                       AND r.cause_ordinal = OLD.claim_command_id
                       AND r.transition_key = OLD.transition_key
                       AND r.created_xact_id = pg_current_xact_id()))) THEN
            RETURN NEW;
        END IF;
    END IF;
    RAISE EXCEPTION 'quest obligation transition is not allowed' USING ERRCODE = '23514';
END;
$$;

CREATE OR REPLACE FUNCTION game_character_quest_obligation_proven() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    entry RECORD;
    actual BIGINT;
    count_keys INTEGER := 0;
BEGIN
    IF NEW.source_kind = 'npc_trade' THEN
        FOR entry IN SELECT key,value FROM jsonb_each(NEW.source_state) LOOP
            count_keys := count_keys + 1;
            IF jsonb_typeof(entry.value) <> 'number' OR entry.value::text !~ '^-?[0-9]+$' THEN
                RAISE EXCEPTION 'trade capture state must contain integer milestones' USING ERRCODE='23514';
            END IF;
            SELECT value INTO actual FROM game_character_quest_tracks
             WHERE character_id=NEW.character_id AND track_key=entry.key;
            IF NOT FOUND OR to_jsonb(actual) <> entry.value THEN
                RAISE EXCEPTION 'trade capture state is not the original persisted milestone' USING ERRCODE='23514';
            END IF;
        END LOOP;
        IF count_keys < 1 OR count_keys > 8 THEN
            RAISE EXCEPTION 'trade capture state bounds' USING ERRCODE='23514';
        END IF;
        IF NOT EXISTS (
            SELECT 1 FROM game_npc_trades t
             WHERE t.transaction_id = NEW.source_trade_transaction_id
               AND t.game_session_id = NEW.claim_game_session_id
               AND t.command_id = NEW.claim_command_id
               AND t.character_id = NEW.character_id
               AND t.created_xact_id = pg_current_xact_id()
               AND NEW.created_xact_id = t.created_xact_id) THEN
            RAISE EXCEPTION 'trade quest obligation must commit with its original NPC trade'
                USING ERRCODE = '23514';
        END IF;
        RETURN NULL;
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM game_reward_claim_mint_receipts r
         WHERE r.game_session_id = NEW.claim_game_session_id
           AND r.command_id = NEW.claim_command_id
           AND r.character_id = NEW.character_id
           AND r.created_xact_id = pg_current_xact_id()) THEN
        RAISE EXCEPTION 'quest obligation must commit with its reward-claim MINT receipt'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;

CREATE FUNCTION game_character_quest_trade_receipt_proven() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.cause_kind = 'trade_obligation' AND NOT EXISTS (
        SELECT 1 FROM game_character_quest_obligations o
        JOIN game_npc_trades t ON t.transaction_id=o.source_trade_transaction_id
         WHERE o.source_kind='npc_trade' AND o.state='CONSUMED'
           AND o.claim_game_session_id=NEW.cause_id AND o.claim_command_id=NEW.cause_ordinal
           AND o.character_id=NEW.character_id AND o.transition_key=NEW.transition_key
           AND t.character_id=o.character_id AND t.game_session_id=o.claim_game_session_id
           AND t.command_id=o.claim_command_id AND t.created_xact_id=o.created_xact_id) THEN
        RAISE EXCEPTION 'trade quest receipt must consume its captured trade obligation'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;
-- Preserve the existing schema-bound guard lookup policy after replacement and
-- apply the same policy to the new receipt guard (0056 Quest owner convention).
DO $$
DECLARE
    v_function TEXT;
BEGIN
    FOREACH v_function IN ARRAY ARRAY[
        'game_character_quest_obligation_row_guard()',
        'game_character_quest_obligation_proven()',
        'game_character_quest_trade_receipt_proven()'
    ] LOOP
        EXECUTE format('ALTER FUNCTION %s SET search_path = %I, pg_temp', v_function, current_schema());
    END LOOP;
END $$;

REVOKE ALL ON FUNCTION game_character_quest_trade_receipt_proven() FROM PUBLIC;
CREATE CONSTRAINT TRIGGER game_character_quest_trade_receipt_proven
    AFTER INSERT ON game_character_quest_receipts DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION game_character_quest_trade_receipt_proven();
