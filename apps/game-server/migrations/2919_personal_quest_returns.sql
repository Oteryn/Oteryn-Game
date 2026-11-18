-- A source STEP can arm one return against the same Character's active stage.
-- Crystal 201189f8 mission06_run_like_wolf.lua:137-163.
CREATE TABLE game_quest_personal_returns (
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id) ON DELETE CASCADE,
    return_key TEXT NOT NULL CHECK (octet_length(return_key) BETWEEN 1 AND 128),
    occurrence UUID NOT NULL UNIQUE DEFAULT game_character_uuid_v7()
        CHECK (game_character_is_uuid_v7(occurrence)),
    world_id UUID NOT NULL,
    map_revision TEXT NOT NULL CHECK (map_revision ~ '^sha256:[0-9a-f]{64}$'),
    binding BYTEA NOT NULL CHECK (octet_length(binding) = 32),
    due_at TIMESTAMPTZ NOT NULL,
    committed_revision NUMERIC(20,0),
    settled_xact_id xid8,
    PRIMARY KEY (character_id, return_key),
    CHECK ((committed_revision IS NULL) = (settled_xact_id IS NULL))
);

-- Preserve the existing shared-switch proof and admit the independent durable
-- personal timer only when it settled in the receipt's own transaction.
CREATE OR REPLACE FUNCTION game_quest_timer_receipt_proven() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.cause_kind = 'server_timer' THEN
        IF NEW.transition_key LIKE 'oteryn:quest-personal-return/%' THEN
            IF NOT EXISTS (
                SELECT 1 FROM game_quest_personal_returns p
                WHERE p.character_id = NEW.character_id AND p.occurrence = NEW.cause_id
                    AND p.return_key = NEW.transition_key AND p.due_at <= clock_timestamp()
                    AND p.committed_revision = NEW.committed_character_revision
                    AND p.settled_xact_id = NEW.created_xact_id
            ) THEN
                RAISE EXCEPTION 'personal quest return receipt lacks its same-XID timer proof'
                    USING ERRCODE = '23514';
            END IF;
        ELSE
            IF NOT EXISTS (
                SELECT 1 FROM game_quest_timer_observations o
                WHERE o.character_id = NEW.character_id AND o.lease_id = NEW.cause_id
                    AND o.recipe = NEW.transition_key AND NEW.cause_ordinal = 0
                    AND o.committed_revision = NEW.committed_character_revision
                    AND o.created_xact_id = NEW.created_xact_id
            ) THEN
                RAISE EXCEPTION 'server timer receipt lacks its original lease proof'
                    USING ERRCODE = '23514';
            END IF;
            PERFORM game_quest_timer_complete(NEW.cause_id, NEW.transition_key);
        END IF;
    END IF;
    RETURN NULL;
END $$;

REVOKE ALL ON game_quest_personal_returns FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE, DELETE ON game_quest_personal_returns TO oteryn_game_runtime;
GRANT SELECT ON game_quest_personal_returns TO oteryn_game_control;

-- A source-authored bank bonus is a separate credit chained after the ordinary
-- coin conversion. The two bank rows and quest stage prove the same confirmation.
ALTER TABLE game_account_bank_entries
    ADD COLUMN quest_deposit_transaction_id UUID UNIQUE
        REFERENCES game_account_bank_operations(transaction_id) DEFERRABLE INITIALLY DEFERRED;

DO $$
DECLARE body TEXT; constraint_name TEXT;
BEGIN
    FOREACH constraint_name IN ARRAY ARRAY['game_account_bank_entries_one_reference',
        'game_account_bank_entries_guild_reference','game_account_bank_entries_house_value'] LOOP
        SELECT pg_get_expr(conbin,conrelid) INTO STRICT body FROM pg_constraint
            WHERE conrelid='game_account_bank_entries'::regclass AND conname=constraint_name;
        EXECUTE format('ALTER TABLE game_account_bank_entries DROP CONSTRAINT %I',constraint_name);
        CASE constraint_name
        WHEN 'game_account_bank_entries_one_reference' THEN
            IF strpos(body,'num_nonnulls(')=0 THEN RAISE EXCEPTION 'missing bank entry owner proof'; END IF;
            body:=replace(body,'num_nonnulls(','num_nonnulls(quest_deposit_transaction_id,');
        WHEN 'game_account_bank_entries_guild_reference' THEN
            body:='CASE WHEN quest_deposit_transaction_id IS NOT NULL THEN kind IS NULL AND guild_kind IS NULL AND market_kind IS NULL AND house_rent_kind IS NULL AND bazaar_kind IS NULL ELSE ('||body||') END';
        ELSE
            body:='CASE WHEN quest_deposit_transaction_id IS NOT NULL THEN recipient_character_id IS NULL AND counterpart_entry_id IS NULL AND escrow_before IS NULL AND escrow_after IS NULL AND balance_after=balance_before+amount ELSE ('||body||') END';
        END CASE;
        EXECUTE format('ALTER TABLE game_account_bank_entries ADD CONSTRAINT %I CHECK (%s)',constraint_name,body);
    END LOOP;
END $$;

CREATE FUNCTION game_quest_bank_deposit_credit_proven() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM game_account_bank_operations o
        JOIN game_npc_bank_confirmations n USING(operation_occurrence_id)
        CROSS JOIN LATERAL (SELECT convert_from(n.frozen_request,'UTF8')::jsonb->'quest_bonus' AS bonus) f
        JOIN game_character_quest_receipts q
          ON q.character_id=n.character_id AND q.cause_kind='command'
         AND q.cause_id=n.game_session_id AND q.cause_ordinal=n.command_id
         AND q.transition_key=f.bonus #>> '{rule,key}'
        JOIN game_account_bank_entries d ON d.transaction_id=o.transaction_id AND d.kind=1
        WHERE o.transaction_id=NEW.quest_deposit_transaction_id AND o.kind=1 AND o.outcome=0
          AND o.created_xact_id=pg_current_xact_id() AND q.created_xact_id=pg_current_xact_id()
          AND NEW.created_xact_id=pg_current_xact_id() AND n.completed
          AND n.npc_key=f.bonus #>> '{rule,npc}'
          AND n.character_id=NEW.acting_character_id AND q.quest_key=f.bonus->>'quest'
          AND o.account_id=NEW.account_id AND o.world_id=NEW.world_id
          AND q.content_revision=f.bonus->>'content_revision'
          AND q.track_keys=ARRAY[f.bonus #>> '{rule,track}']
          AND q.values_before=ARRAY[(f.bonus #>> '{rule,before}')::bigint]
          AND q.values_after=ARRAY[(f.bonus #>> '{rule,after}')::bigint]
          AND NEW.amount=(f.bonus #>> '{rule,gold}')::bigint
          AND d.entry_id=NEW.previous_entry_id AND d.balance_after=NEW.balance_before
    ) THEN
        RAISE EXCEPTION 'Quest deposit bonus must share its conversion and stage receipt'
            USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;

CREATE FUNCTION game_quest_bank_deposit_stage_proven() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM game_npc_bank_confirmations n
        JOIN game_account_bank_operations o USING(operation_occurrence_id)
        JOIN game_account_bank_entries b ON b.quest_deposit_transaction_id=o.transaction_id
        WHERE n.game_session_id=NEW.cause_id AND n.command_id=NEW.cause_ordinal
          AND n.character_id=NEW.character_id AND b.created_xact_id=pg_current_xact_id()
    ) THEN
        RAISE EXCEPTION 'Quest deposit stage requires its bank credit' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;

CREATE CONSTRAINT TRIGGER quest_bank_deposit_credit_proven
AFTER INSERT ON game_account_bank_entries DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW WHEN (NEW.quest_deposit_transaction_id IS NOT NULL)
EXECUTE FUNCTION game_quest_bank_deposit_credit_proven();

CREATE CONSTRAINT TRIGGER quest_bank_deposit_stage_proven
AFTER INSERT ON game_character_quest_receipts DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW WHEN (NEW.transition_key LIKE 'oteryn:quest-transition/bank-deposit/%')
EXECUTE FUNCTION game_quest_bank_deposit_stage_proven();

DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_quest_bank_deposit_credit_proven() SET search_path=pg_catalog,%I,pg_temp',current_schema());
    EXECUTE format('ALTER FUNCTION game_quest_bank_deposit_stage_proven() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_quest_bank_deposit_credit_proven(), game_quest_bank_deposit_stage_proven() FROM PUBLIC;
