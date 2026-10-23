-- DAILY-1: ordered lines and bindings join the native Inbox transaction.
CREATE TABLE game_character_daily_reward_lines (
    claim_id UUID NOT NULL REFERENCES game_character_daily_reward_claims(transaction_id),
    line SMALLINT NOT NULL CHECK (line BETWEEN 0 AND 19),
    kind TEXT NOT NULL CHECK (kind IN ('ITEM','PREY_WILDCARD','XP_BOOST')),
    definition TEXT CHECK (octet_length(definition) BETWEEN 1 AND 512),
    quantity INTEGER NOT NULL CHECK (quantity BETWEEN 1 AND 100),
    status TEXT NOT NULL CHECK (status IN ('PENDING','DELIVERED')),
    delivering_transaction_id UUID,
    item_instance_id UUID REFERENCES game_item_instances(item_instance_id),
    charges INTEGER CHECK (charges BETWEEN 1 AND 65535),
    PRIMARY KEY (claim_id,line),
    CHECK ((kind='ITEM')=(definition IS NOT NULL)),
    CHECK ((status='DELIVERED')=(delivering_transaction_id IS NOT NULL)),
    CHECK ((kind='ITEM' AND status='DELIVERED')=(item_instance_id IS NOT NULL))
);
CREATE TABLE game_item_bindings (
    item_instance_id UUID PRIMARY KEY REFERENCES game_item_instances(item_instance_id),
    world_id UUID NOT NULL,
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id) ON DELETE RESTRICT,
    cause TEXT NOT NULL CHECK (cause='DAILY_REWARD'),
    FOREIGN KEY(item_instance_id,world_id) REFERENCES game_item_instances(item_instance_id,world_id)
);
CREATE TABLE game_daily_reward_commands (
    game_session_id UUID NOT NULL,
    command_id NUMERIC(20,0) NOT NULL CHECK(command_id BETWEEN 1 AND 18446744073709551615),
    claim_id UUID NOT NULL UNIQUE REFERENCES game_character_daily_reward_claims(transaction_id),
    PRIMARY KEY(game_session_id,command_id)
);
CREATE TRIGGER daily_reward_commands_immutable BEFORE UPDATE OR DELETE OR TRUNCATE
    ON game_daily_reward_commands FOR EACH STATEMENT EXECUTE FUNCTION game_daily_reward_immutable();
CREATE TABLE game_account_daily_reward_jokers (
    account_id UUID NOT NULL REFERENCES game_character_account_guards(account_id),
    world_id UUID NOT NULL,
    jokers SMALLINT NOT NULL CHECK (jokers BETWEEN 0 AND 3),
    last_grant_month TEXT NOT NULL CHECK (last_grant_month ~ '^[0-9]{4}-[0-9]{2}$'),
    last_entry_id BIGINT NOT NULL CHECK (last_entry_id>0),
    PRIMARY KEY(account_id,world_id)
);
CREATE TABLE game_account_daily_reward_joker_entries (
    account_id UUID NOT NULL,
    world_id UUID NOT NULL,
    entry_id BIGINT NOT NULL CHECK(entry_id>0),
    claim_id UUID NOT NULL REFERENCES game_character_daily_reward_claims(transaction_id)
        DEFERRABLE INITIALLY DEFERRED,
    kind TEXT NOT NULL CHECK(kind IN ('GRANT','SPEND')),
    amount SMALLINT NOT NULL CHECK(amount BETWEEN 0 AND 3),
    balance_before SMALLINT NOT NULL CHECK(balance_before BETWEEN 0 AND 3),
    balance_after SMALLINT NOT NULL CHECK(balance_after BETWEEN 0 AND 3),
    PRIMARY KEY(account_id,world_id,entry_id),
    UNIQUE(claim_id,kind),
    CHECK(balance_after=balance_before + CASE kind WHEN 'GRANT' THEN amount ELSE -amount END)
);
ALTER TABLE game_account_daily_reward_jokers ADD FOREIGN KEY(account_id,world_id,last_entry_id)
    REFERENCES game_account_daily_reward_joker_entries(account_id,world_id,entry_id)
    DEFERRABLE INITIALLY DEFERRED;
CREATE TRIGGER daily_reward_lines_immutable BEFORE UPDATE OR DELETE OR TRUNCATE
    ON game_character_daily_reward_lines FOR EACH STATEMENT EXECUTE FUNCTION game_daily_reward_immutable();
CREATE TRIGGER daily_reward_bindings_immutable BEFORE UPDATE OR DELETE OR TRUNCATE
    ON game_item_bindings FOR EACH STATEMENT EXECUTE FUNCTION game_daily_reward_immutable();
CREATE TRIGGER daily_reward_joker_entries_immutable BEFORE UPDATE OR DELETE OR TRUNCATE
    ON game_account_daily_reward_joker_entries FOR EACH STATEMENT EXECUTE FUNCTION game_daily_reward_immutable();

-- Only the World-reset owner publishes a window. The positive DAILY window is
-- native epoch + 1: native epoch zero is the initial committed World generation.
-- Restart never inserts another row. Calendar labels remain Berlin 10:00.
CREATE FUNCTION game_daily_reward_reset_window() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
DECLARE instant BIGINT;
BEGIN
    IF NEW.reset_epoch=18446744073709551615 THEN RETURN NEW; END IF;
    instant:=floor(extract(epoch FROM NEW.activated_at)*1000)::bigint;
    INSERT INTO public.game_world_reset_history
        SELECT NEW.world_id,NEW.reset_epoch+1,
          ((NEW.activated_at AT TIME ZONE 'Europe/Berlin')-interval '10 hours')::date-DATE '1970-01-01',
          instant,instant;
    RETURN NEW;
END $$;
REVOKE ALL ON FUNCTION game_daily_reward_reset_window() FROM PUBLIC;
-- There can be multiple operational resets on the same civil day. Epoch is
-- the authority; a civil-day label is not a uniqueness key.
ALTER TABLE game_world_reset_history DROP CONSTRAINT game_world_reset_history_world_id_reset_day_key;
CREATE OR REPLACE FUNCTION game_world_reset_history_ordered() RETURNS trigger
LANGUAGE plpgsql SET search_path=pg_catalog,public AS $$
DECLARE previous public.game_world_reset_history%ROWTYPE;
BEGIN
    PERFORM pg_advisory_xact_lock(hashtextextended('oteryn:world-reset:' || NEW.world_id::text,0));
    SELECT * INTO previous FROM public.game_world_reset_history WHERE world_id=NEW.world_id
        ORDER BY reset_epoch DESC LIMIT 1;
    IF FOUND AND (NEW.reset_epoch<=previous.reset_epoch OR NEW.reset_day<previous.reset_day
        OR (NEW.reset_day=previous.reset_day AND NOT EXISTS(SELECT 1 FROM public.game_world_reset_epochs n
            WHERE n.world_id=NEW.world_id AND n.reset_epoch=NEW.reset_epoch-1
              AND floor(extract(epoch FROM n.activated_at)*1000)::bigint=NEW.reset_at_unix_ms))
        OR NEW.reset_at_unix_ms<previous.reset_at_unix_ms
        OR NEW.committed_at_unix_ms<previous.committed_at_unix_ms) THEN
        RAISE EXCEPTION 'World reset history must advance' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END $$;
CREATE TRIGGER daily_reward_reset_window AFTER INSERT ON game_world_reset_epochs
    FOR EACH ROW EXECUTE FUNCTION game_daily_reward_reset_window();
-- Upgrade an already initialized World too; process one ordered row at a time.
DO $$
DECLARE e public.game_world_reset_epochs%ROWTYPE; instant BIGINT;
BEGIN
    LOOP
        SELECT n.* INTO e FROM public.game_world_reset_epochs n WHERE n.reset_epoch<18446744073709551615
            AND NOT EXISTS(SELECT 1 FROM public.game_world_reset_history h WHERE h.world_id=n.world_id AND h.reset_epoch=n.reset_epoch+1)
            ORDER BY n.world_id,n.reset_epoch LIMIT 1;
        EXIT WHEN NOT FOUND;
        instant:=floor(extract(epoch FROM e.activated_at)*1000)::bigint;
        INSERT INTO public.game_world_reset_history VALUES(e.world_id,e.reset_epoch+1,
            ((e.activated_at AT TIME ZONE 'Europe/Berlin')-interval '10 hours')::date-DATE '1970-01-01',instant,instant);
    END LOOP;
END $$;

-- Deferred proof: a delivered ITEM line must name this claim's fresh native
-- Inbox MINT, its exact output ordinal, quantity, owner and permanent binding.
CREATE FUNCTION game_daily_reward_line_guard() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
DECLARE c public.game_character_daily_reward_claims%ROWTYPE;
BEGIN
    SELECT * INTO STRICT c FROM public.game_character_daily_reward_claims WHERE transaction_id=NEW.claim_id;
    IF NEW.kind='ITEM' AND (NEW.status<>'DELIVERED' OR NOT EXISTS(
        SELECT 1 FROM public.game_item_instances i
        JOIN public.game_item_bindings b USING(item_instance_id,world_id)
        JOIN public.game_item_character_inbox_locations l USING(item_instance_id,world_id,character_id)
        WHERE i.item_instance_id=NEW.item_instance_id AND b.character_id=c.character_id
          AND i.world_id=c.world_id AND i.quantity=NEW.quantity
          AND i.definition_production_key=NEW.definition
          AND i.minted_transaction_id=c.transaction_id
          AND NEW.delivering_transaction_id=c.transaction_id)) THEN
        RAISE EXCEPTION 'daily item requires atomic bound Inbox MINT' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER daily_reward_line_final AFTER INSERT ON game_character_daily_reward_lines
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_daily_reward_line_guard();

REVOKE ALL ON game_character_daily_reward_lines,game_item_bindings,
    game_account_daily_reward_jokers,game_account_daily_reward_joker_entries,game_daily_reward_commands FROM PUBLIC;
GRANT SELECT,INSERT ON game_character_daily_reward_lines,game_item_bindings,
    game_account_daily_reward_jokers,game_account_daily_reward_joker_entries,game_daily_reward_commands TO oteryn_game_runtime;
GRANT UPDATE(jokers,last_grant_month,last_entry_id) ON game_account_daily_reward_jokers TO oteryn_game_runtime;
REVOKE ALL ON FUNCTION game_daily_reward_line_guard() FROM PUBLIC;

-- Follow a bound item through its own custody tree, including a moved ancestor.
-- The carried-tree owner already caps depth at eight and total nodes at 500.
CREATE FUNCTION game_daily_reward_bound_custody(p_item UUID) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
DECLARE b public.game_item_bindings%ROWTYPE; current_item UUID; parent UUID; owner UUID; depth INTEGER;
BEGIN
    SELECT * INTO b FROM public.game_item_bindings WHERE item_instance_id=p_item;
    IF NOT FOUND THEN RETURN; END IF;
    IF EXISTS(SELECT 1 FROM public.game_npc_trade_lines WHERE item_instance_id=p_item AND line_kind IN(1,3)
        AND created_xact_id=pg_current_xact_id()) THEN
        RAISE EXCEPTION 'daily reward is not tradeable' USING ERRCODE='23514';
    END IF;
    IF EXISTS(SELECT 1 FROM public.game_item_instances WHERE item_instance_id=p_item AND lifecycle<>1) THEN
        -- Existing proof owners still validate the exact burn/merge/expiry. A
        -- retirement is never a license to sell a bound item.
        RETURN;
    END IF;
    current_item:=p_item;
    FOR depth IN 0..8 LOOP
        SELECT character_id INTO owner FROM (
            SELECT character_id FROM public.game_item_character_inbox_locations WHERE item_instance_id=current_item AND world_id=b.world_id
            UNION ALL SELECT character_id FROM public.game_item_character_depot_locations WHERE item_instance_id=current_item AND world_id=b.world_id
            UNION ALL SELECT character_id FROM public.game_item_container_entries WHERE item_instance_id=current_item AND world_id=b.world_id
            UNION ALL SELECT character_id FROM public.game_item_container_slots WHERE item_instance_id=current_item
            UNION ALL SELECT character_id FROM public.game_character_equipment_slots WHERE item_instance_id=current_item AND world_id=b.world_id
        ) owned LIMIT 1;
        IF owner IS NOT NULL THEN
            IF owner=b.character_id THEN RETURN; END IF;
            EXIT;
        END IF;
        SELECT parent_item_instance_id INTO parent FROM public.game_item_nested_entries WHERE item_instance_id=current_item AND world_id=b.world_id;
        IF parent IS NULL THEN EXIT; END IF;
        current_item:=parent;
    END LOOP;
    RAISE EXCEPTION 'daily reward cannot leave its Character and World' USING ERRCODE='23514';
END $$;

CREATE FUNCTION game_daily_reward_custody_guard() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
DECLARE item UUID; child UUID;
BEGIN
    -- A binding is inserted after native MINT, so eligibility must be checked
    -- here at deferred execution, never in an immediate trigger WHEN clause.
    -- Ordinary item traffic must not walk custody trees when DAILY is unused.
    IF NOT EXISTS(SELECT 1 FROM public.game_item_bindings) THEN RETURN NULL; END IF;
    IF TG_OP='DELETE' THEN item:=OLD.item_instance_id; ELSE item:=NEW.item_instance_id; END IF;
    FOR child IN WITH RECURSIVE tree(id,depth) AS (
        SELECT item,0 UNION ALL
        SELECT e.item_instance_id,t.depth+1 FROM tree t JOIN public.game_item_nested_entries e ON e.parent_item_instance_id=t.id WHERE t.depth<8
    ) SELECT b.item_instance_id FROM (SELECT id FROM tree LIMIT 501) affected
        JOIN public.game_item_bindings b ON b.item_instance_id=affected.id LOOP
        -- Keep the ancestor walk for bound descendants, but never invoke the
        -- owner/location proof for unrelated, unbound members of the tree.
        PERFORM public.game_daily_reward_bound_custody(child);
    END LOOP;
    RETURN NULL;
END $$;
DO $$
DECLARE relation TEXT;
BEGIN
    FOREACH relation IN ARRAY ARRAY['game_item_instances','game_item_character_inbox_locations',
        'game_item_character_depot_locations','game_item_container_entries','game_item_container_slots',
        'game_character_equipment_slots','game_item_nested_entries','game_item_ground_locations',
        'game_item_house_interior_locations','game_item_bindings'] LOOP
        EXECUTE format('CREATE CONSTRAINT TRIGGER daily_bound_custody AFTER INSERT OR UPDATE OR DELETE ON %I DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_daily_reward_custody_guard()',relation);
    END LOOP;
END $$;
-- Copy the binding at a split, and prohibit all unequal-binding merges. These
-- guards see the complete native move line set at commit, in either line order.
CREATE FUNCTION game_daily_reward_stack_guard() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
DECLARE operation SMALLINT; source public.game_item_bindings%ROWTYPE; line RECORD; equal_binding BOOLEAN;
BEGIN
    SELECT r.operation INTO operation FROM public.game_item_loop_move_receipts r WHERE transaction_id=NEW.transaction_id;
    IF operation NOT IN(2,3) THEN RETURN NULL; END IF;
    SELECT b.* INTO source FROM public.game_item_loop_move_lines l JOIN public.game_item_bindings b USING(item_instance_id)
        WHERE l.transaction_id=NEW.transaction_id AND l.from_kind<>0 LIMIT 1;
    IF NOT FOUND THEN RETURN NULL; END IF;
    FOR line IN SELECT item_instance_id,from_kind FROM public.game_item_loop_move_lines WHERE transaction_id=NEW.transaction_id ORDER BY line_ordinal LIMIT 3 LOOP
        IF operation=3 AND line.from_kind=0 THEN
            INSERT INTO public.game_item_bindings VALUES(line.item_instance_id,source.world_id,source.character_id,source.cause)
                ON CONFLICT(item_instance_id) DO NOTHING;
        END IF;
        SELECT EXISTS(SELECT 1 FROM public.game_item_bindings b WHERE b.item_instance_id=line.item_instance_id
            AND (b.world_id,b.character_id,b.cause)=(source.world_id,source.character_id,source.cause)) INTO equal_binding;
        IF NOT equal_binding THEN RAISE EXCEPTION 'daily reward stack bindings differ' USING ERRCODE='23514'; END IF;
    END LOOP;
    RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER daily_bound_stack AFTER INSERT ON game_item_loop_move_lines
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_daily_reward_stack_guard();
CREATE FUNCTION game_daily_reward_npc_guard() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
BEGIN
    IF NEW.line_kind IN(1,3) AND EXISTS(SELECT 1 FROM public.game_item_bindings WHERE item_instance_id=NEW.item_instance_id) THEN
        RAISE EXCEPTION 'daily reward is not tradeable' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END $$;
CREATE TRIGGER daily_bound_npc BEFORE INSERT ON game_npc_trade_lines FOR EACH ROW EXECUTE FUNCTION game_daily_reward_npc_guard();
REVOKE ALL ON FUNCTION game_daily_reward_bound_custody(UUID),game_daily_reward_custody_guard(),
    game_daily_reward_stack_guard(),game_daily_reward_npc_guard() FROM PUBLIC;

CREATE FUNCTION game_daily_reward_joker_guard() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
DECLARE previous SMALLINT; latest public.game_account_daily_reward_joker_entries%ROWTYPE; projected public.game_account_daily_reward_jokers%ROWTYPE;
BEGIN
    IF TG_TABLE_NAME='game_account_daily_reward_joker_entries' THEN
    IF NEW.entry_id=1 THEN previous:=0;
    ELSE
        SELECT balance_after INTO STRICT previous FROM public.game_account_daily_reward_joker_entries
            WHERE account_id=NEW.account_id AND world_id=NEW.world_id AND entry_id=NEW.entry_id-1;
    END IF;
    IF NEW.balance_before<>previous THEN RAISE EXCEPTION 'daily joker ledger discontinuity' USING ERRCODE='23514'; END IF;
    END IF;
    SELECT * INTO STRICT latest FROM public.game_account_daily_reward_joker_entries
        WHERE account_id=NEW.account_id AND world_id=NEW.world_id ORDER BY entry_id DESC LIMIT 1;
    SELECT * INTO STRICT projected FROM public.game_account_daily_reward_jokers WHERE account_id=NEW.account_id AND world_id=NEW.world_id;
    IF (projected.jokers,projected.last_entry_id) IS DISTINCT FROM (latest.balance_after,latest.entry_id) THEN
        RAISE EXCEPTION 'daily joker projection differs from ledger' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER daily_joker_state_final AFTER INSERT OR UPDATE ON game_account_daily_reward_jokers
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_daily_reward_joker_guard();
CREATE CONSTRAINT TRIGGER daily_joker_final AFTER INSERT ON game_account_daily_reward_joker_entries
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_daily_reward_joker_guard();
CREATE FUNCTION game_daily_reward_delivery_guard() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
DECLARE c public.game_character_daily_reward_claims%ROWTYPE; n INTEGER; total INTEGER; last_line INTEGER;
BEGIN
    SELECT * INTO STRICT c FROM public.game_character_daily_reward_claims WHERE transaction_id=NEW.claim_id;
    SELECT count(*),sum(quantity),max(line) INTO n,total,last_line FROM public.game_character_daily_reward_lines WHERE claim_id=NEW.claim_id;
    IF n<1 OR n>20 OR last_line<>n-1 OR total<>(CASE c.lane_day WHEN 1 THEN 5 WHEN 2 THEN 5 WHEN 4 THEN 10 WHEN 7 THEN 10 ELSE 1 END)
        OR EXISTS(SELECT 1 FROM public.game_character_daily_reward_lines WHERE claim_id=NEW.claim_id
            AND kind<>(CASE c.lane_day WHEN 3 THEN 'PREY_WILDCARD' WHEN 7 THEN 'XP_BOOST' ELSE 'ITEM' END)) THEN
        RAISE EXCEPTION 'daily claim requires its complete ordered reward plan' USING ERRCODE='23514';
    END IF;
    IF c.jokers_spent<>coalesce((SELECT amount FROM public.game_account_daily_reward_joker_entries WHERE claim_id=c.transaction_id AND kind='SPEND'),0) THEN
        RAISE EXCEPTION 'daily claim joker spend differs from ledger' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER daily_delivery_final AFTER INSERT ON game_daily_reward_commands
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_daily_reward_delivery_guard();
REVOKE ALL ON FUNCTION game_daily_reward_joker_guard(),game_daily_reward_delivery_guard() FROM PUBLIC;
