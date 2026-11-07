-- QENGINE-2: the existing quest, carried custody, appearance and backpack MINT
-- owners participate in one transaction. Their original proof arms stay intact.
CREATE TABLE game_quest_reward_sources (
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    cause_id UUID NOT NULL,
    cause_ordinal NUMERIC(20,0) NOT NULL CHECK(cause_ordinal BETWEEN 1 AND 18446744073709551615),
    transition_key TEXT NOT NULL CHECK(octet_length(transition_key) BETWEEN 1 AND 128),
    plan JSONB NOT NULL CHECK(jsonb_typeof(plan)='object' AND octet_length(plan::text)<=16384),
    mint_transaction_id UUID UNIQUE REFERENCES game_reward_claim_mint_receipts(transaction_id)
        DEFERRABLE INITIALLY DEFERRED,
    grant_keys TEXT[] NOT NULL CHECK(cardinality(grant_keys)<=3),
    created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
    PRIMARY KEY(character_id,cause_id,cause_ordinal,transition_key),
    FOREIGN KEY(character_id,cause_id,cause_ordinal,transition_key)
        REFERENCES game_character_quest_receipts(character_id,cause_id,cause_ordinal,transition_key)
        DEFERRABLE INITIALLY DEFERRED
);

CREATE FUNCTION game_quest_reward_complete() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE expected JSONB; actual BIGINT; account UUID; grant_key TEXT; addon INTEGER;
BEGIN
    IF NEW.created_xact_id<>pg_current_xact_id() OR NEW.plan->>'key' IS DISTINCT FROM NEW.transition_key
        OR NOT EXISTS(SELECT 1 FROM game_character_quest_receipts r
            WHERE r.character_id=NEW.character_id AND r.cause_id=NEW.cause_id
                AND r.cause_ordinal=NEW.cause_ordinal AND r.transition_key=NEW.transition_key
                AND r.quest_key=NEW.plan->>'quest' AND r.cause_kind='command'
                AND r.created_xact_id=pg_current_xact_id()) THEN
        RAISE EXCEPTION 'quest reward lacks same-transaction quest receipt' USING ERRCODE='23514';
    END IF;
    FOR expected IN SELECT value FROM jsonb_array_elements(NEW.plan->'take') LOOP
        SELECT coalesce(sum(c.quantity),0) INTO actual FROM game_quest_callback_consumptions c
            WHERE c.character_id=NEW.character_id AND c.cause_id=NEW.cause_id
                AND c.cause_ordinal=NEW.cause_ordinal AND c.transition_key=NEW.transition_key
                AND c.before_item->>'definition_production_key'=expected->>'item'
                AND c.location_kind IN('container','nested','backpack','equipment')
                AND c.replacement IS NULL AND c.created_xact_id=pg_current_xact_id();
        IF actual IS DISTINCT FROM (expected->>'count')::bigint THEN
            RAISE EXCEPTION 'quest reward must consume exact carried item count' USING ERRCODE='23514';
        END IF;
    END LOOP;
    IF EXISTS(SELECT 1 FROM game_quest_callback_consumptions c
        WHERE c.character_id=NEW.character_id AND c.cause_id=NEW.cause_id
            AND c.cause_ordinal=NEW.cause_ordinal AND c.transition_key=NEW.transition_key
            AND NOT EXISTS(SELECT 1 FROM jsonb_array_elements(NEW.plan->'take') e
                WHERE e->>'item'=c.before_item->>'definition_production_key')) THEN
        RAISE EXCEPTION 'quest reward consumed an unauthored input' USING ERRCODE='23514';
    END IF;
    IF (NEW.plan->'give'='null'::jsonb) IS DISTINCT FROM (NEW.mint_transaction_id IS NULL)
        OR (NEW.mint_transaction_id IS NOT NULL AND NOT EXISTS(
            SELECT 1 FROM game_reward_claim_mint_receipts m JOIN game_item_instances i
                ON i.item_instance_id=m.item_instance_id
            WHERE m.transaction_id=NEW.mint_transaction_id AND m.character_id=NEW.character_id
                AND m.game_session_id=NEW.cause_id AND m.command_id=NEW.cause_ordinal
                AND m.claim_production_key=NEW.transition_key AND m.created_xact_id=pg_current_xact_id()
                AND i.definition_production_key=NEW.plan #>> '{give,item}'
                AND m.quantity=(NEW.plan #>> '{give,count}')::bigint)) THEN
        RAISE EXCEPTION 'quest reward lacks exact native backpack MINT' USING ERRCODE='23514';
    END IF;
    SELECT account_id INTO STRICT account FROM game_character_roots WHERE character_id=NEW.character_id;
    -- Account unlocks can already exist from another Character. They still
    -- satisfy the reward, but must never be replaced with a second receipt.
    IF NEW.plan->'outfit'<>'null'::jsonb THEN
        FOR addon IN 0..2 LOOP
            IF addon<>0 AND ((NEW.plan #>> '{outfit,addons}')::integer & (1 << (addon-1)))=0 THEN
                CONTINUE;
            END IF;
            grant_key:=(NEW.plan #>> '{outfit,outfit}') || '/quest-grant/' || addon;
            IF NOT EXISTS(SELECT 1 FROM game_account_appearance_unlocks u
                JOIN game_account_appearance_unlock_receipts r USING(grant_occurrence_id,account_id,unlock_key)
                WHERE u.account_id=account AND u.unlock_key=grant_key
                    AND r.source_kind='quest'
                    AND r.content_revision='crystal-9f5a72c-quest-rewards-v1') THEN
                RAISE EXCEPTION 'quest outfit lacks compatible earned fact' USING ERRCODE='23514';
            END IF;
        END LOOP;
    END IF;
    FOREACH grant_key IN ARRAY NEW.grant_keys LOOP
        IF NOT EXISTS(SELECT 1 FROM generate_series(0,2) AS a(addon)
            WHERE (a.addon=0 OR ((NEW.plan #>> '{outfit,addons}')::integer & (1 << (a.addon-1)))<>0)
                AND grant_key=(NEW.plan #>> '{outfit,outfit}') || '/quest-grant/' || a.addon) THEN
            RAISE EXCEPTION 'quest outfit has an unauthored grant key' USING ERRCODE='23514';
        END IF;
        IF NOT EXISTS(SELECT 1 FROM game_account_appearance_unlocks u
            JOIN game_account_appearance_unlock_receipts r USING(grant_occurrence_id,account_id,unlock_key)
            WHERE u.account_id=account AND u.unlock_key=grant_key
                AND r.granting_character_id=NEW.character_id AND r.source_kind='quest'
                AND r.content_revision='crystal-9f5a72c-quest-rewards-v1'
                AND r.xmin=pg_current_xact_id()::text::xid) THEN
            RAISE EXCEPTION 'quest outfit lacks same-transaction earned fact' USING ERRCODE='23514';
        END IF;
    END LOOP;
    RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_quest_reward_complete AFTER INSERT ON game_quest_reward_sources
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_quest_reward_complete();
CREATE TRIGGER game_quest_reward_immutable BEFORE UPDATE OR DELETE ON game_quest_reward_sources
    FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_quest_reward_no_truncate BEFORE TRUNCATE ON game_quest_reward_sources
    EXECUTE FUNCTION game_item_reject_truncate();

-- Only the carried completion arm can accept a command cause, and only when
-- this source owner proves its exact authored consumption and quest receipt.
DO $migration$
DECLARE body TEXT; original TEXT:='r.cause_kind=''use'''; replacement TEXT;
BEGIN
    body:=pg_get_functiondef('game_quest_callback_complete()'::regprocedure);
    IF (length(body)-length(replace(body,original,'')))/length(original)<>1 THEN
        RAISE EXCEPTION 'unexpected carried quest completion body';
    END IF;
    replacement:=$arm$(r.cause_kind='use' OR (r.cause_kind='command' AND EXISTS(
        SELECT 1 FROM game_quest_reward_sources s WHERE s.character_id=NEW.character_id
            AND s.cause_id=NEW.cause_id AND s.cause_ordinal=NEW.cause_ordinal
            AND s.transition_key=NEW.transition_key AND s.created_xact_id=pg_current_xact_id())))$arm$;
    EXECUTE replace(body,original,replacement);
    EXECUTE format('ALTER FUNCTION game_quest_callback_complete() SET search_path=pg_catalog,%I,pg_temp',current_schema());
    EXECUTE format('ALTER FUNCTION game_quest_reward_complete() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;
REVOKE ALL ON game_quest_reward_sources FROM PUBLIC;
GRANT SELECT,INSERT ON game_quest_reward_sources TO oteryn_game_runtime;
GRANT SELECT ON game_quest_reward_sources TO oteryn_game_control;
REVOKE ALL ON FUNCTION game_quest_reward_complete() FROM PUBLIC;
