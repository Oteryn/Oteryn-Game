-- P04: Task Board item/value participants extend existing custody owners. All
-- source proofs are immutable and require the Task Board command's quest receipt.
ALTER TABLE game_quest_callback_consumptions
    ADD COLUMN task_board_item_id INTEGER CHECK(task_board_item_id BETWEEN 1 AND 65535);
ALTER TABLE game_quest_callback_consumptions
    DROP CONSTRAINT game_quest_callback_consumptions_location_kind_check;
ALTER TABLE game_quest_callback_consumptions ADD CONSTRAINT game_quest_callback_consumptions_location_kind_check
    CHECK(location_kind IN('ground','container','nested','backpack','equipment','depot','depot_nested','depot_container'));

CREATE TABLE game_task_board_item_grants (
    item_instance_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(item_instance_id)),
    transaction_id UUID NOT NULL CHECK(game_character_is_uuid_v7(transaction_id)),
    character_id UUID NOT NULL,
    world_id UUID NOT NULL,
    cause_id UUID NOT NULL,
    cause_ordinal NUMERIC(20,0) NOT NULL CHECK(cause_ordinal>0),
    source_item_id INTEGER NOT NULL CHECK(source_item_id BETWEEN 1 AND 65535),
    definition_key TEXT NOT NULL,
    definition_revision TEXT NOT NULL,
    quantity BIGINT NOT NULL CHECK(quantity BETWEEN 1 AND 100),
    created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
    FOREIGN KEY(character_id,cause_id,cause_ordinal) REFERENCES game_task_board_receipts
        DEFERRABLE INITIALLY DEFERRED
);
CREATE INDEX game_task_board_item_grant_command ON game_task_board_item_grants(character_id,cause_id,cause_ordinal);
CREATE INDEX game_task_board_item_grant_transaction ON game_task_board_item_grants(transaction_id);

CREATE TABLE game_task_board_stash_sources (
    transaction_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(transaction_id)),
    entry_id UUID NOT NULL UNIQUE CHECK(game_character_is_uuid_v7(entry_id)),
    character_id UUID NOT NULL,
    world_id UUID NOT NULL,
    cause_id UUID NOT NULL,
    cause_ordinal NUMERIC(20,0) NOT NULL CHECK(cause_ordinal>0),
    source_item_id INTEGER NOT NULL CHECK(source_item_id BETWEEN 1 AND 65535),
    before_balance JSONB NOT NULL,
    amount BIGINT NOT NULL CHECK(amount>0),
    event_id UUID NOT NULL UNIQUE DEFAULT game_character_uuid_v7() CHECK(game_character_is_uuid_v7(event_id)),
    occurred_at BIGINT NOT NULL DEFAULT (floor(extract(epoch FROM statement_timestamp())*1000)::bigint) CHECK(occurred_at>0),
    created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
    FOREIGN KEY(character_id,cause_id,cause_ordinal) REFERENCES game_task_board_receipts
        DEFERRABLE INITIALLY DEFERRED
);
CREATE INDEX game_task_board_stash_source_command ON game_task_board_stash_sources(character_id,cause_id,cause_ordinal);

-- The existing stash owner binds its native envelope hash to its source. The
-- participant freezes that hash after the source allocated EventId/server time,
-- before inserting into the same existing economy audit outbox.
CREATE TABLE game_task_board_stash_audit_proofs (
    transaction_id UUID PRIMARY KEY REFERENCES game_task_board_stash_sources(transaction_id),
    event_id UUID NOT NULL UNIQUE REFERENCES game_character_stash_audit_outbox(event_id) DEFERRABLE INITIALLY DEFERRED,
    envelope_sha256 BYTEA NOT NULL CHECK(octet_length(envelope_sha256)=32),
    created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id()
);

-- Schema1 remains stow/withdraw; schema2 adds only Task Board consumption.
ALTER TABLE game_character_stash_audit_outbox
    DROP CONSTRAINT game_character_stash_audit_outbox_schema_revision_check;
ALTER TABLE game_character_stash_audit_outbox
    ADD CONSTRAINT game_character_stash_audit_outbox_schema_revision_check CHECK(schema_revision IN(1,2));

-- The current depot-search owner (0585, depot_search_retrieve.rs) traverses
-- sixteen levels and limits matching results after selecting the source key.
-- Every matching stack has at least one unit, so at most the requested units
-- are needed to prove a delivery. This bound does not cap the Character's depot.
-- A guard supplies one specific item and needs at most one returned location.
-- The root set deliberately excludes CharacterInbox, including its nested bags.
-- The reply projection supplies NULL key/maximum to aggregate all requested
-- definitions in one traversal, without clipping counts to delivery quantities.
CREATE FUNCTION game_task_board_depot_items(actor UUID,world UUID,item_key TEXT,maximum BIGINT,specific UUID)
RETURNS TABLE(item UUID,location JSONB,kind TEXT) LANGUAGE sql STABLE AS $$
    WITH RECURSIVE tree(item,world,location,kind,depth,path) AS (
        SELECT d.item_instance_id,d.world_id,to_jsonb(d),'depot',0,ARRAY[d.item_instance_id]
        FROM game_item_character_depot_locations d WHERE d.character_id=actor AND d.world_id=$2
        UNION ALL
        SELECT e.item,e.world,e.location,e.kind,t.depth+1,t.path||e.item FROM tree t JOIN (
            SELECT e.item_instance_id AS item,e.world_id AS world,e.parent_item_instance_id AS parent,
                to_jsonb(e) AS location,'depot_nested' AS kind FROM game_item_nested_entries e
            UNION ALL
            SELECT e.item_instance_id,e.world_id,e.parent_item_instance_id,to_jsonb(e),'depot_container'
                FROM game_item_container_entries e WHERE e.character_id=actor
        ) e ON e.parent=t.item AND e.world=t.world WHERE t.depth<16 AND NOT e.item=ANY(t.path)
    )
    SELECT t.item,t.location,t.kind FROM tree t JOIN game_item_instances i ON i.item_instance_id=t.item
        WHERE i.lifecycle=1 AND i.quantity>0 AND i.world_id=$2
          AND i.definition_family='Item' AND ($3 IS NULL OR i.definition_production_key=$3)
          AND ($5 IS NULL OR t.item=$5)
        ORDER BY t.item LIMIT CASE WHEN $4 IS NULL THEN NULL ELSE greatest($4,0) END
$$;

CREATE FUNCTION game_task_board_consume_instance(actor UUID,cause UUID,ordinal NUMERIC,
    source_id INTEGER,instance UUID,location_kind TEXT,location JSONB,amount BIGINT) RETURNS VOID
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE i game_item_instances%ROWTYPE; transaction UUID:=game_character_uuid_v7(); proof_bytes BYTEA;
BEGIN
    SELECT * INTO STRICT i FROM game_item_instances WHERE item_instance_id=instance FOR UPDATE;
    proof_bytes:=convert_to(jsonb_build_object('source','TASK_BOARD','character',actor,'cause',cause,
        'ordinal',ordinal,'source_item_id',source_id,'before',to_jsonb(i),'location',location,'amount',amount)::text,'UTF8');
    INSERT INTO game_quest_callback_consumptions(transaction_id,item_instance_id,character_id,cause_id,
        cause_ordinal,transition_key,before_item,before_ground,quantity,proof,proof_sha256,before_location,
        location_kind,replacement,task_board_item_id)
        VALUES(transaction,instance,actor,cause,ordinal,'oteryn:taskboard/weekly/advance',to_jsonb(i),NULL,
            amount::int,proof_bytes,sha256(proof_bytes),location,location_kind,NULL,source_id);
    UPDATE game_item_instances SET quantity=quantity-amount,lifecycle=CASE WHEN quantity=amount THEN 2 ELSE 1 END,
        last_transaction_id=transaction WHERE item_instance_id=instance AND lifecycle=1 AND quantity=i.quantity;
    IF NOT FOUND THEN RAISE EXCEPTION 'Task Board input changed under lock' USING ERRCODE='23514'; END IF;
    IF i.quantity=amount THEN
        CASE location_kind
            WHEN 'container' THEN DELETE FROM game_item_container_entries WHERE item_instance_id=instance;
            WHEN 'nested' THEN DELETE FROM game_item_nested_entries WHERE item_instance_id=instance;
            WHEN 'backpack' THEN PERFORM game_quest_callback_remove_backpack(instance);
            WHEN 'equipment' THEN DELETE FROM game_character_equipment_slots WHERE item_instance_id=instance;
            WHEN 'depot' THEN DELETE FROM game_item_character_depot_locations WHERE item_instance_id=instance;
            WHEN 'depot_nested' THEN DELETE FROM game_item_nested_entries WHERE item_instance_id=instance;
            WHEN 'depot_container' THEN DELETE FROM game_item_container_entries WHERE item_instance_id=instance;
            ELSE RAISE EXCEPTION 'unsupported Task Board custody' USING ERRCODE='23514';
        END CASE;
    END IF;
END $$;

CREATE FUNCTION game_task_board_consume_item(actor UUID,world UUID,cause UUID,ordinal NUMERIC,
    source_id INTEGER,item_key TEXT,item_revision TEXT,amount BIGINT) RETURNS BOOLEAN
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE available BIGINT; remaining BIGINT:=amount; take_count BIGINT; row_record RECORD;
    entry UUID; transaction UUID;
BEGIN
    IF amount<=0 OR amount>1000000000 THEN RETURN FALSE; END IF;
    -- Item locks precede stash balances, as in the existing stash owner.
    PERFORM i.item_instance_id FROM game_item_instances i WHERE i.item_instance_id IN (
        SELECT c.item FROM game_quest_callback_carried(actor,world) c
        UNION SELECT d.item FROM game_task_board_depot_items(actor,world,item_key,amount,NULL) d)
        AND i.definition_family='Item' AND i.definition_production_key=item_key
        ORDER BY i.item_instance_id FOR UPDATE;
    PERFORM 1 FROM game_character_stash_balances s WHERE s.character_id=actor AND s.world_id=world
        AND s.definition_key=item_key AND s.quantity>0 ORDER BY s.definition_revision LIMIT amount FOR UPDATE;
    SELECT coalesce(sum(least(quantity,amount)),0) INTO available FROM (
        SELECT i.quantity FROM game_quest_callback_carried(actor,world) c JOIN game_item_instances i ON i.item_instance_id=c.item
            WHERE i.lifecycle=1 AND i.definition_family='Item' AND i.definition_production_key=item_key
        UNION ALL SELECT s.quantity FROM (SELECT quantity FROM game_character_stash_balances WHERE character_id=actor
            AND world_id=world AND definition_key=item_key AND quantity>0 ORDER BY definition_revision LIMIT amount) s
        UNION ALL SELECT i.quantity FROM game_task_board_depot_items(actor,world,item_key,amount,NULL) d JOIN game_item_instances i ON i.item_instance_id=d.item
            WHERE i.lifecycle=1 AND i.definition_family='Item' AND i.definition_production_key=item_key
    ) counts;
    IF available<amount THEN RETURN FALSE; END IF;
    FOR row_record IN SELECT i.item_instance_id,i.quantity,c.kind,c.location
        FROM game_quest_callback_carried(actor,world) c JOIN game_item_instances i ON i.item_instance_id=c.item
        WHERE i.lifecycle=1 AND i.definition_family='Item' AND i.definition_production_key=item_key
        ORDER BY i.item_instance_id LOOP
        EXIT WHEN remaining=0;
        take_count:=least(remaining,row_record.quantity);
        PERFORM game_task_board_consume_instance(actor,cause,ordinal,source_id,row_record.item_instance_id,
            row_record.kind,row_record.location,take_count);
        remaining:=remaining-take_count;
    END LOOP;
    FOR row_record IN SELECT s.* FROM game_character_stash_balances s
        WHERE s.character_id=actor AND s.world_id=world AND s.definition_key=item_key AND s.quantity>0
        ORDER BY s.definition_revision LIMIT amount LOOP
        EXIT WHEN remaining=0;
        take_count:=least(remaining,row_record.quantity); entry:=game_character_uuid_v7(); transaction:=game_character_uuid_v7();
        INSERT INTO game_task_board_stash_sources(transaction_id,entry_id,character_id,world_id,cause_id,cause_ordinal,
            source_item_id,before_balance,amount) VALUES(transaction,entry,actor,world,cause,ordinal,source_id,to_jsonb(row_record),take_count);
        INSERT INTO game_character_stash_entries(entry_id,transaction_id,character_id,world_id,definition_key,
            definition_revision,amount,quantity_before,quantity_after,previous_entry_id)
            VALUES(entry,transaction,actor,world,row_record.definition_key,row_record.definition_revision,
                take_count,row_record.quantity,row_record.quantity-take_count,row_record.last_entry_id);
        UPDATE game_character_stash_balances SET quantity=quantity-take_count,last_entry_id=entry
            WHERE character_id=actor AND world_id=world AND definition_key=row_record.definition_key
              AND definition_revision=row_record.definition_revision;
        remaining:=remaining-take_count;
    END LOOP;
    FOR row_record IN SELECT i.item_instance_id,i.quantity,d.kind,d.location
        FROM game_task_board_depot_items(actor,world,item_key,amount,NULL) d JOIN game_item_instances i ON i.item_instance_id=d.item
        WHERE i.lifecycle=1 AND i.definition_family='Item' AND i.definition_production_key=item_key
        ORDER BY i.item_instance_id LOOP
        EXIT WHEN remaining=0;
        take_count:=least(remaining,row_record.quantity);
        PERFORM game_task_board_consume_instance(actor,cause,ordinal,source_id,row_record.item_instance_id,
            row_record.kind,row_record.location,take_count);
        remaining:=remaining-take_count;
    END LOOP;
    IF remaining<>0 THEN RAISE EXCEPTION 'Task Board delivery did not consume exact units' USING ERRCODE='23514'; END IF;
    RETURN TRUE;
END $$;

CREATE FUNCTION game_task_board_item_grant_guard() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
    IF NEW.created_xact_id<>pg_current_xact_id()
        OR NOT game_reward_claim_inbox_item_proven(NEW.item_instance_id,NEW.transaction_id)
        OR NOT EXISTS(SELECT 1 FROM game_reward_claim_inbox_mint_receipts m
            JOIN game_reward_claim_inbox_mint_lines l USING(transaction_id)
            JOIN game_task_board_receipts r ON r.character_id=NEW.character_id
                AND r.cause_id=NEW.cause_id AND r.cause_ordinal=NEW.cause_ordinal
            WHERE m.transaction_id=NEW.transaction_id AND m.game_session_id=NEW.cause_id
                AND m.command_id=NEW.cause_ordinal AND m.character_id=NEW.character_id
                AND m.world_id=NEW.world_id AND m.source_binding=r.request_binding
                AND m.created_xact_id=pg_current_xact_id() AND r.created_xact_id=pg_current_xact_id()
                AND r.refusal='null'::jsonb AND r.committed_character_revision IS NOT NULL
                AND r.action='"BuyTalisman"'::jsonb AND NEW.source_item_id=51978 AND NEW.quantity=1
                AND l.item_instance_id=NEW.item_instance_id AND l.source_line=NEW.source_item_id
                AND l.definition_family='Item' AND l.definition_production_key=NEW.definition_key
                AND l.definition_revision_ref=NEW.definition_revision AND l.quantity=NEW.quantity
                AND l.key_number=0 AND l.created_xact_id=pg_current_xact_id()) THEN
        RAISE EXCEPTION 'Task Board reward requires complete reserved Inbox mint and native audit' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;

CREATE FUNCTION game_task_board_stash_complete(transaction UUID) RETURNS BOOLEAN
LANGUAGE plpgsql AS $$
DECLARE s game_task_board_stash_sources%ROWTYPE; e game_character_stash_entries%ROWTYPE;
    b game_character_stash_balances%ROWTYPE;
BEGIN
    SELECT * INTO s FROM game_task_board_stash_sources WHERE transaction_id=transaction;
    IF NOT FOUND THEN RETURN FALSE; END IF;
    SELECT * INTO e FROM game_character_stash_entries WHERE entry_id=s.entry_id;
    SELECT * INTO b FROM game_character_stash_balances WHERE character_id=s.character_id AND world_id=s.world_id
        AND definition_key=s.before_balance->>'definition_key' AND definition_revision=s.before_balance->>'definition_revision';
    IF s.created_xact_id<>pg_current_xact_id() OR e.entry_id IS NULL OR e.created_xact_id<>pg_current_xact_id()
        OR e.transaction_id<>s.transaction_id OR e.character_id<>s.character_id OR e.world_id<>s.world_id
        OR e.definition_key<>s.before_balance->>'definition_key'
        OR e.definition_revision<>s.before_balance->>'definition_revision'
        OR e.amount<>s.amount OR e.quantity_before<>(s.before_balance->>'quantity')::bigint
        OR e.quantity_after<>e.quantity_before-e.amount
        OR e.previous_entry_id IS DISTINCT FROM (s.before_balance->>'last_entry_id')::uuid
        OR b.last_entry_id IS DISTINCT FROM e.entry_id OR b.quantity IS DISTINCT FROM e.quantity_after
        OR NOT EXISTS(SELECT 1 FROM game_task_board_receipts r
            WHERE r.character_id=s.character_id AND r.world_id=s.world_id
                AND r.cause_id=s.cause_id AND r.cause_ordinal=s.cause_ordinal
                AND r.cause_kind='command' AND r.action ? 'Deliver'
                AND r.refusal='null'::jsonb AND r.committed_character_revision IS NOT NULL
                AND r.created_xact_id=pg_current_xact_id())
        OR NOT EXISTS(SELECT 1 FROM game_character_stash_audit_outbox a
            JOIN game_task_board_stash_audit_proofs p USING(transaction_id,event_id)
            WHERE a.transaction_id=s.transaction_id AND a.event_id=s.event_id AND a.world_id=s.world_id
                AND a.occurred_at=s.occurred_at AND a.event_type_id=9 AND a.schema_revision=2
                AND a.retention_profile_id='ECONOMY_LEDGER_RETENTION_V1'
                AND a.envelope_sha256=p.envelope_sha256 AND p.created_xact_id=pg_current_xact_id()
                AND a.created_xact_id=pg_current_xact_id() AND a.publication_state=1 AND a.published_at IS NULL)
        OR EXISTS(SELECT 1 FROM game_character_stash_item_lines WHERE transaction_id=transaction)
        OR EXISTS(SELECT 1 FROM game_item_instances WHERE minted_transaction_id=transaction OR last_transaction_id=transaction) THEN
        RAISE EXCEPTION 'Task Board stash source must consume exact units without mint' USING ERRCODE='23514';
    END IF;
    RETURN TRUE;
END $$;

CREATE FUNCTION game_task_board_custody_totals() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE expected JSONB; actual BIGINT; consumed JSONB; granted JSONB; delivery JSONB;
BEGIN
    consumed:=CASE WHEN jsonb_typeof(NEW.effects)='object' THEN coalesce(NEW.effects->'consume_items','[]'::jsonb) ELSE '[]'::jsonb END;
    granted:=CASE WHEN jsonb_typeof(NEW.effects)='object' THEN coalesce(NEW.effects->'grant_items','[]'::jsonb) ELSE '[]'::jsonb END;
    IF consumed<>'[]'::jsonb AND (NEW.refusal<>'null'::jsonb OR NEW.committed_character_revision IS NULL
        OR NEW.cause_kind<>'command' OR NOT NEW.action ? 'Deliver') THEN
        RAISE EXCEPTION 'Task Board consumption requires a successful delivery' USING ERRCODE='23514';
    END IF;
    IF granted<>'[]'::jsonb AND (NEW.refusal<>'null'::jsonb OR NEW.committed_character_revision IS NULL
        OR NEW.cause_kind<>'command') THEN
        RAISE EXCEPTION 'Task Board grants require a successful purchase' USING ERRCODE='23514';
    END IF;
    IF NEW.refusal='null'::jsonb AND NEW.action ? 'Deliver' THEN
        delivery:=NEW.reply->'data'->'weekly'->'deliveries'->((NEW.action #>> '{Deliver,slot}')::integer);
        IF NEW.committed_character_revision IS NULL OR delivery->'delivered' IS DISTINCT FROM 'true'::jsonb
            OR consumed IS DISTINCT FROM jsonb_build_array(jsonb_build_object('item_id',delivery->'item_id','amount',delivery->'required')) THEN
            RAISE EXCEPTION 'Task Board delivery receipt requires its exact consumed items' USING ERRCODE='23514';
        END IF;
    END IF;
    IF NEW.refusal='null'::jsonb AND NEW.action='"BuyTalisman"'::jsonb
        AND (granted<>'[{"item_id":51978,"amount":1}]'::jsonb OR (NEW.effects->>'gold_cost')::bigint IS DISTINCT FROM 5000) THEN
        RAISE EXCEPTION 'Task Board talisman purchase requires exact fee and item' USING ERRCODE='23514';
    END IF;
    -- All thirteen item offers require the separate Item/House decoration-kit
    -- owner, including non-pickupable Cerberus (ANSWER 6544295916). They retain
    -- their sourced prices but cannot commit a raw Item grant or point debit.
    IF NEW.refusal='null'::jsonb AND (NEW.action #>> '{BuyShop,offer}')::integer BETWEEN 10 AND 22 THEN
        RAISE EXCEPTION 'Task Board decoration-kit purchase is unavailable' USING ERRCODE='23514';
    END IF;
    FOR expected IN SELECT value FROM jsonb_array_elements(consumed) LOOP
        SELECT coalesce(sum(quantity),0) INTO actual FROM (
            SELECT quantity::bigint FROM game_quest_callback_consumptions WHERE character_id=NEW.character_id
                AND cause_id=NEW.cause_id AND cause_ordinal=NEW.cause_ordinal
                AND transition_key='oteryn:taskboard/weekly/advance' AND task_board_item_id=(expected->>'item_id')::int
            UNION ALL SELECT amount FROM game_task_board_stash_sources WHERE character_id=NEW.character_id
                AND cause_id=NEW.cause_id AND cause_ordinal=NEW.cause_ordinal AND source_item_id=(expected->>'item_id')::int
        ) units;
        IF actual<>(expected->>'amount')::bigint THEN RAISE EXCEPTION 'Task Board delivery quantity mismatch' USING ERRCODE='23514'; END IF;
    END LOOP;
    FOR expected IN SELECT value FROM jsonb_array_elements(granted) LOOP
        SELECT coalesce(sum(quantity),0) INTO actual FROM game_task_board_item_grants
            WHERE character_id=NEW.character_id AND cause_id=NEW.cause_id AND cause_ordinal=NEW.cause_ordinal
                AND source_item_id=(expected->>'item_id')::int;
        IF actual<>(expected->>'amount')::bigint THEN RAISE EXCEPTION 'Task Board reward quantity mismatch' USING ERRCODE='23514'; END IF;
    END LOOP;
    IF EXISTS(SELECT 1 FROM game_task_board_item_grants g WHERE g.character_id=NEW.character_id
        AND g.cause_id=NEW.cause_id AND g.cause_ordinal=NEW.cause_ordinal AND NOT EXISTS(
            SELECT 1 FROM jsonb_array_elements(granted) v WHERE (v->>'item_id')::int=g.source_item_id))
        OR EXISTS(SELECT 1 FROM game_task_board_stash_sources s WHERE s.character_id=NEW.character_id
        AND s.cause_id=NEW.cause_id AND s.cause_ordinal=NEW.cause_ordinal AND NOT EXISTS(
            SELECT 1 FROM jsonb_array_elements(consumed) v WHERE (v->>'item_id')::int=s.source_item_id))
        OR EXISTS(SELECT 1 FROM game_quest_callback_consumptions c WHERE c.character_id=NEW.character_id
        AND c.cause_id=NEW.cause_id AND c.cause_ordinal=NEW.cause_ordinal AND c.transition_key='oteryn:taskboard/weekly/advance'
        AND NOT EXISTS(SELECT 1 FROM jsonb_array_elements(consumed) v WHERE (v->>'item_id')::int=c.task_board_item_id)) THEN
        RAISE EXCEPTION 'unexpected Task Board custody participant' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;

CREATE CONSTRAINT TRIGGER game_task_board_item_grant_guard AFTER INSERT ON game_task_board_item_grants
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_task_board_item_grant_guard();
CREATE CONSTRAINT TRIGGER game_task_board_custody_totals AFTER INSERT ON game_task_board_receipts
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_task_board_custody_totals();

-- The forward guard extensions below retain every pre-existing source arm.
DO $migration$
DECLARE n TEXT; definition TEXT; arm TEXT; begin_at INT; source_fk RECORD;
BEGIN
    FOR source_fk IN SELECT conname,conrelid FROM pg_constraint
        WHERE conrelid IN('game_character_stash_entries'::regclass,'game_character_stash_audit_outbox'::regclass)
          AND confrelid='game_character_stash_operations'::regclass LOOP
        EXECUTE format('ALTER TABLE %s DROP CONSTRAINT %I',source_fk.conrelid::regclass,source_fk.conname);
    END LOOP;
    FOREACH n IN ARRAY ARRAY['game_stash_admit','game_stash_balance_guard','game_quest_callback_admit',
        'game_quest_callback_complete','game_character_depot_remove','game_item_container_entry_removal_proven',
        'game_item_nested_entry_proven'] LOOP
        definition:=pg_get_functiondef((n||'()')::regprocedure);
        begin_at:=regexp_instr(definition,E'\\mBEGIN\\M',1,1,0,'i');
        IF begin_at=0 THEN RAISE EXCEPTION 'unexpected Task Board custody guard shape: %',n; END IF;
        CASE n
        WHEN 'game_stash_admit' THEN arm:=$arm$
            IF TG_TABLE_NAME='game_character_stash_entries' AND EXISTS(
                SELECT 1 FROM game_task_board_stash_sources s WHERE s.transaction_id=NEW.transaction_id
                  AND s.entry_id=NEW.entry_id AND s.created_xact_id=pg_current_xact_id()) THEN
                IF NOT EXISTS(SELECT 1 FROM game_task_board_stash_sources s JOIN game_character_stash_balances b
                    ON b.character_id=s.character_id AND b.world_id=s.world_id
                    AND b.definition_key=s.before_balance->>'definition_key'
                    AND b.definition_revision=s.before_balance->>'definition_revision'
                    WHERE s.transaction_id=NEW.transaction_id AND to_jsonb(b)=s.before_balance
                    AND NEW.character_id=s.character_id AND NEW.world_id=s.world_id
                    AND NEW.definition_key=b.definition_key AND NEW.definition_revision=b.definition_revision
                    AND NEW.quantity_before=b.quantity AND NEW.quantity_after=b.quantity-s.amount
                    AND NEW.amount=s.amount AND NEW.previous_entry_id=b.last_entry_id) THEN
                    RAISE EXCEPTION 'Task Board stash entry does not consume current balance' USING ERRCODE='23514';
                END IF;
                NEW.created_xact_id:=pg_current_xact_id(); RETURN NEW;
            END IF;
$arm$;
        WHEN 'game_stash_balance_guard' THEN arm:=$arm$
            IF TG_OP='UPDATE' AND EXISTS(SELECT 1 FROM game_task_board_stash_sources s
                JOIN game_character_stash_entries e ON e.entry_id=s.entry_id
                WHERE NEW.last_entry_id=s.entry_id AND s.created_xact_id=pg_current_xact_id()
                  AND e.created_xact_id=pg_current_xact_id() AND to_jsonb(OLD)=s.before_balance
                  AND (NEW.character_id,NEW.world_id,NEW.definition_key,NEW.definition_revision)=
                      (OLD.character_id,OLD.world_id,OLD.definition_key,OLD.definition_revision)
                  AND NEW.quantity=OLD.quantity-s.amount AND e.quantity_after=NEW.quantity
                  AND e.quantity_before=OLD.quantity AND e.previous_entry_id=OLD.last_entry_id) THEN RETURN NEW; END IF;
$arm$;
        WHEN 'game_quest_callback_admit' THEN arm:=$arm$
            IF NEW.location_kind IN('depot','depot_nested','depot_container') THEN
                SELECT * INTO i FROM game_item_instances WHERE item_instance_id=NEW.item_instance_id FOR UPDATE;
                IF NEW.transition_key<>'oteryn:taskboard/weekly/advance' OR NEW.task_board_item_id IS NULL
                    OR i.item_instance_id IS NULL OR to_jsonb(i) IS DISTINCT FROM NEW.before_item
                    OR i.lifecycle<>1 OR i.quantity<NEW.quantity OR i.definition_family<>'Item' OR NEW.replacement IS NOT NULL
                    OR NOT EXISTS(SELECT 1 FROM game_task_board_depot_items(NEW.character_id,i.world_id,i.definition_production_key,1,i.item_instance_id) d
                        WHERE d.item=i.item_instance_id AND d.kind=NEW.location_kind AND d.location=NEW.before_location)
                    OR EXISTS(SELECT 1 FROM game_item_keys WHERE item_instance_id=i.item_instance_id)
                    OR EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=i.item_instance_id)
                    OR EXISTS(SELECT 1 FROM game_item_texts WHERE item_instance_id=i.item_instance_id)
                    OR EXISTS(SELECT 1 FROM game_item_imbuements WHERE item_instance_id=i.item_instance_id)
                    OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=i.item_instance_id)
                    OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=i.item_instance_id) THEN
                    RAISE EXCEPTION 'Task Board input lacks plain owned depot custody' USING ERRCODE='23514';
                END IF;
                NEW.created_xact_id:=pg_current_xact_id(); RETURN NEW;
            END IF;
$arm$;
        WHEN 'game_quest_callback_complete' THEN arm:=$arm$
            IF NEW.location_kind IN('depot','depot_nested','depot_container') THEN
                SELECT * INTO i FROM game_item_instances WHERE item_instance_id=NEW.item_instance_id;
                IF NEW.created_xact_id<>pg_current_xact_id() OR NOT coalesce(game_quest_callback_after(NEW,i),false)
                    OR (i.quantity=0 AND (EXISTS(SELECT 1 FROM game_item_character_depot_locations WHERE item_instance_id=i.item_instance_id)
                        OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE item_instance_id=i.item_instance_id)
                        OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=i.item_instance_id)))
                    OR (i.quantity>0 AND NOT EXISTS(SELECT 1 FROM game_task_board_depot_items(NEW.character_id,i.world_id,i.definition_production_key,1,i.item_instance_id) d
                        WHERE d.item=i.item_instance_id AND d.location=NEW.before_location AND d.kind=NEW.location_kind))
                    OR NOT EXISTS(SELECT 1 FROM game_character_quest_receipts q WHERE q.character_id=NEW.character_id
                        AND q.cause_id=NEW.cause_id AND q.cause_ordinal=NEW.cause_ordinal AND q.transition_key=NEW.transition_key
                        AND q.cause_kind='use' AND q.created_xact_id=pg_current_xact_id()) THEN
                    RAISE EXCEPTION 'incomplete atomic Task Board depot input' USING ERRCODE='23514';
                END IF;
                RETURN NULL;
            END IF;
$arm$;
        WHEN 'game_character_depot_remove' THEN arm:=$arm$
            IF EXISTS(SELECT 1 FROM game_quest_callback_consumptions c JOIN game_item_instances i USING(item_instance_id)
                WHERE c.item_instance_id=OLD.item_instance_id AND c.location_kind='depot'
                  AND c.before_location=to_jsonb(OLD) AND c.created_xact_id=pg_current_xact_id()
                  AND c.transition_key='oteryn:taskboard/weekly/advance' AND c.task_board_item_id IS NOT NULL
                  AND i.quantity=0 AND i.lifecycle=2 AND game_quest_callback_after(c,i)) THEN
                UPDATE game_character_depot_box_counters SET committed=committed-1
                    WHERE character_id=OLD.character_id AND town_id=OLD.town_id AND box_number=OLD.box_number;
                RETURN OLD;
            END IF;
$arm$;
        ELSE arm:=format($arm$
            IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_quest_callback_consumptions c
                JOIN game_item_instances i USING(item_instance_id) WHERE c.item_instance_id=OLD.item_instance_id
                  AND c.location_kind=%L AND c.before_location=to_jsonb(OLD) AND c.created_xact_id=pg_current_xact_id()
                  AND c.transition_key='oteryn:taskboard/weekly/advance' AND c.task_board_item_id IS NOT NULL
                  AND i.quantity=0 AND i.lifecycle=2 AND game_quest_callback_after(c,i)) THEN
                IF TG_WHEN='BEFORE' THEN RETURN OLD; END IF; RETURN NULL;
            END IF;
$arm$,CASE n WHEN 'game_item_nested_entry_proven' THEN 'depot_nested' ELSE 'depot_container' END);
        END CASE;
        EXECUTE overlay(definition placing E'BEGIN\n'||arm from begin_at for 5);
        EXECUTE format('ALTER FUNCTION %I() SET search_path=pg_catalog,%I,pg_temp',n,current_schema());
    END LOOP;
    definition:=pg_get_functiondef('game_stash_complete(uuid)'::regprocedure);
    begin_at:=regexp_instr(definition,E'\\mBEGIN\\M',1,1,0,'i');
    IF begin_at=0 THEN RAISE EXCEPTION 'unexpected stash completion guard shape'; END IF;
    EXECUTE overlay(definition placing E'BEGIN\n IF game_task_board_stash_complete(p_transaction) THEN RETURN; END IF;\n' from begin_at for 5);
END $migration$;

CREATE FUNCTION game_task_board_stash_source_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NOT game_task_board_stash_complete(NEW.transaction_id) THEN
        RAISE EXCEPTION 'missing Task Board stash consume proof' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_task_board_stash_source_guard AFTER INSERT ON game_task_board_stash_sources
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_task_board_stash_source_guard();
CREATE CONSTRAINT TRIGGER game_task_board_stash_audit_proof_guard AFTER INSERT ON game_task_board_stash_audit_proofs
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_task_board_stash_source_guard();

CREATE FUNCTION game_task_board_stash_audit_version_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.schema_revision=2 THEN
        IF NOT game_task_board_stash_complete(NEW.transaction_id) THEN
            RAISE EXCEPTION 'schema2 stash audit requires exact Task Board consume source' USING ERRCODE='23514';
        END IF;
    ELSIF EXISTS(SELECT 1 FROM game_task_board_stash_sources WHERE transaction_id=NEW.transaction_id) THEN
        RAISE EXCEPTION 'Task Board stash consume cannot use legacy stow/withdraw schema' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_task_board_stash_audit_version_guard AFTER INSERT ON game_character_stash_audit_outbox
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_task_board_stash_audit_version_guard();

DO $$
DECLARE relation TEXT; signature TEXT;
BEGIN
    FOREACH relation IN ARRAY ARRAY['game_task_board_item_grants','game_task_board_stash_sources','game_task_board_stash_audit_proofs'] LOOP
        EXECUTE format('CREATE TRIGGER task_board_source_immutable BEFORE UPDATE OR DELETE ON %I FOR EACH ROW EXECUTE FUNCTION game_item_immutable()',relation);
        EXECUTE format('CREATE TRIGGER task_board_source_no_truncate BEFORE TRUNCATE ON %I FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate()',relation);
        EXECUTE format('REVOKE ALL ON %I FROM PUBLIC',relation);
        EXECUTE format('GRANT SELECT,INSERT ON %I TO oteryn_game_runtime',relation);
    END LOOP;
    FOREACH signature IN ARRAY ARRAY['game_task_board_depot_items(uuid,uuid,text,bigint,uuid)',
        'game_task_board_consume_instance(uuid,uuid,numeric,integer,uuid,text,jsonb,bigint)',
        'game_task_board_consume_item(uuid,uuid,uuid,numeric,integer,text,text,bigint)',
        'game_task_board_item_grant_guard()','game_task_board_stash_complete(uuid)',
        'game_task_board_custody_totals()','game_task_board_stash_source_guard()',
        'game_task_board_stash_audit_version_guard()'] LOOP
        EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',signature,current_schema());
        EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',signature);
    END LOOP;
END $$;
GRANT EXECUTE ON FUNCTION game_task_board_depot_items(UUID,UUID,TEXT,BIGINT,UUID),
    game_task_board_consume_item(UUID,UUID,UUID,NUMERIC,INTEGER,TEXT,TEXT,BIGINT)
    TO oteryn_game_runtime;

-- P05 talisman purchase joins the existing closed coin/change/bank fee owner.
CREATE TABLE game_task_board_fee_proofs (
    character_id UUID NOT NULL,
    world_id UUID NOT NULL,
    cause_id UUID NOT NULL,
    cause_ordinal NUMERIC(20,0) NOT NULL,
    fee_occurrence UUID NOT NULL UNIQUE CHECK (game_character_is_uuid_v7(fee_occurrence)),
    fee_gold_units BIGINT NOT NULL CHECK (fee_gold_units=5000),
    committed_character_revision NUMERIC(20,0) NOT NULL,
    created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
    PRIMARY KEY (character_id,cause_id,cause_ordinal),
    FOREIGN KEY (character_id,cause_id,cause_ordinal) REFERENCES game_task_board_receipts(character_id,cause_id,cause_ordinal) DEFERRABLE INITIALLY DEFERRED
);
REVOKE ALL ON game_task_board_fee_proofs FROM PUBLIC;
GRANT SELECT,INSERT ON game_task_board_fee_proofs TO oteryn_game_runtime;

CREATE FUNCTION game_task_board_fee_proof_guard() RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.created_xact_id<>pg_current_xact_id() OR NOT EXISTS (
        SELECT 1 FROM game_task_board_receipts r JOIN game_item_fee_burns f
          ON f.cause_occurrence_id=NEW.fee_occurrence AND f.cause_kind=13
        WHERE r.character_id=NEW.character_id AND r.world_id=NEW.world_id
          AND r.cause_id=NEW.cause_id AND r.cause_ordinal=NEW.cause_ordinal
          AND r.cause_kind='command' AND r.action='"BuyTalisman"'::jsonb
          AND r.refusal='null'::jsonb AND (r.effects->>'gold_cost')::bigint=NEW.fee_gold_units
          AND r.committed_character_revision=NEW.committed_character_revision
          AND f.character_id=NEW.character_id AND f.world_id=NEW.world_id
          AND f.fee_gold_units=NEW.fee_gold_units
          AND f.committed_character_revision=NEW.committed_character_revision
          AND f.created_xact_id=pg_current_xact_id() AND r.created_xact_id=pg_current_xact_id()
    ) THEN RAISE EXCEPTION 'Task Board fee requires matching purchase, burn and quest revision' USING ERRCODE='23514'; END IF;
    RETURN NULL;
END $$;
CREATE FUNCTION game_task_board_fee_receipt_guard() RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    IF COALESCE((NEW.effects->>'gold_cost')::bigint,0)>0 AND NOT EXISTS (
        SELECT 1 FROM game_task_board_fee_proofs p WHERE p.character_id=NEW.character_id
          AND p.world_id=NEW.world_id AND p.cause_id=NEW.cause_id AND p.cause_ordinal=NEW.cause_ordinal
          AND p.fee_gold_units=(NEW.effects->>'gold_cost')::bigint
          AND p.committed_character_revision=NEW.committed_character_revision
          AND p.created_xact_id=pg_current_xact_id()
    ) THEN RAISE EXCEPTION 'Task Board purchase has no gold fee proof' USING ERRCODE='23514'; END IF;
    RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_task_board_fee_proof_guard AFTER INSERT ON game_task_board_fee_proofs
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_task_board_fee_proof_guard();
CREATE CONSTRAINT TRIGGER game_task_board_fee_receipt_guard AFTER INSERT ON game_task_board_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_task_board_fee_receipt_guard();
CREATE TRIGGER game_task_board_fee_proofs_immutable BEFORE UPDATE OR DELETE ON game_task_board_fee_proofs
 FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_task_board_fee_proofs_no_truncate BEFORE TRUNCATE ON game_task_board_fee_proofs
 FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();

DO $$ DECLARE body TEXT; BEGIN
 SELECT pg_get_constraintdef(oid) INTO STRICT body FROM pg_constraint
  WHERE conrelid='game_item_fee_burns'::regclass AND conname='game_item_fee_burns_cause_kind_check';
 ALTER TABLE game_item_fee_burns DROP CONSTRAINT game_item_fee_burns_cause_kind_check;
 EXECUTE 'ALTER TABLE game_item_fee_burns ADD CONSTRAINT game_item_fee_burns_cause_kind_check CHECK (('||substr(body,7)||') OR cause_kind=13)';
 SELECT regexp_replace(pg_get_constraintdef(oid),' NOT VALID$','') INTO STRICT body FROM pg_constraint
  WHERE conrelid='game_item_fee_burns'::regclass AND conname='game_item_fee_burns_purchase_cause';
 ALTER TABLE game_item_fee_burns DROP CONSTRAINT game_item_fee_burns_purchase_cause;
 EXECUTE 'ALTER TABLE game_item_fee_burns ADD CONSTRAINT game_item_fee_burns_purchase_cause CHECK (('||substr(body,7)||') OR (cause_kind=13 AND charm_key IS NULL AND purchase_service_key=''oteryn:taskboard.talisman'' AND fee_gold_units=5000)) NOT VALID';
 SELECT pg_get_functiondef('game_item_fee_burn_consistency_guard()'::regprocedure) INTO STRICT body;
 IF position('(NEW.cause_kind=5 AND EXISTS' IN body)=0 THEN RAISE EXCEPTION 'missing fee Character proof extension anchor'; END IF;
 EXECUTE replace(body,'(NEW.cause_kind=5 AND EXISTS','(NEW.cause_kind=13 AND EXISTS(SELECT 1 FROM game_task_board_fee_proofs p WHERE p.character_id=NEW.character_id AND p.world_id=NEW.world_id AND p.fee_occurrence=NEW.cause_occurrence_id AND p.committed_character_revision=NEW.committed_character_revision AND p.fee_gold_units=NEW.fee_gold_units AND p.created_xact_id=pg_current_xact_id())) OR (NEW.cause_kind=5 AND EXISTS');
 EXECUTE format('ALTER FUNCTION game_task_board_fee_proof_guard() SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_task_board_fee_receipt_guard() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_task_board_fee_proof_guard(),game_task_board_fee_receipt_guard() FROM PUBLIC;

GRANT EXECUTE ON FUNCTION game_task_board_stash_complete(UUID) TO oteryn_game_runtime;

-- Append to 0621: Task-shop cosmetic grants extend existing appearance facts.
CREATE TABLE game_task_board_cosmetic_grants (
    grant_occurrence_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(grant_occurrence_id)),
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    cause_id UUID NOT NULL,
    cause_ordinal NUMERIC(20,0) NOT NULL,
    offer_id SMALLINT NOT NULL CHECK(offer_id BETWEEN 0 AND 9),
    created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
    FOREIGN KEY(character_id,cause_id,cause_ordinal)
        REFERENCES game_task_board_receipts(character_id,cause_id,cause_ordinal)
        DEFERRABLE INITIALLY DEFERRED,
    FOREIGN KEY(grant_occurrence_id)
        REFERENCES game_account_appearance_unlock_receipts(grant_occurrence_id)
        DEFERRABLE INITIALLY DEFERRED
);
CREATE INDEX game_task_board_cosmetic_grants_cause
    ON game_task_board_cosmetic_grants(character_id,cause_id,cause_ordinal);
CREATE TRIGGER game_task_board_cosmetic_grants_stamp BEFORE INSERT ON game_task_board_cosmetic_grants
    FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();
CREATE TRIGGER game_task_board_cosmetic_grants_immutable BEFORE UPDATE OR DELETE ON game_task_board_cosmetic_grants
    FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_task_board_cosmetic_grants_no_truncate BEFORE TRUNCATE ON game_task_board_cosmetic_grants
    FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();

CREATE FUNCTION game_task_board_cosmetic_grant_guard() RETURNS TRIGGER
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE g game_task_board_cosmetic_grants%ROWTYPE;
BEGIN
    SELECT * INTO g FROM game_task_board_cosmetic_grants
        WHERE grant_occurrence_id=NEW.grant_occurrence_id;
    IF NOT FOUND OR g.created_xact_id<>pg_current_xact_id() THEN
        RAISE EXCEPTION 'Task-shop grant lacks same-transaction proof' USING ERRCODE='23514';
    END IF;
    IF NOT EXISTS(SELECT 1 FROM game_task_board_receipts r
        WHERE r.character_id=g.character_id AND r.cause_id=g.cause_id AND r.cause_ordinal=g.cause_ordinal
        AND r.refusal='null'::jsonb AND r.created_xact_id=g.created_xact_id
        AND r.committed_character_revision IS NOT NULL
        AND r.action=jsonb_build_object('BuyShop',jsonb_build_object('offer',g.offer_id))
        AND CASE WHEN g.offer_id<6 THEN
            r.effects->'outfit'=jsonb_build_array(CASE WHEN g.offer_id<3 THEN 1907 ELSE 1282 END,g.offer_id%3)
            ELSE (r.effects->>'mount')::integer=CASE g.offer_id WHEN 6 THEN 1908 WHEN 7 THEN 1909 WHEN 8 THEN 1910 WHEN 9 THEN 1281 END END
    ) THEN
        RAISE EXCEPTION 'Task-shop cosmetic grant differs from paid offer receipt' USING ERRCODE='23514';
    END IF;
    IF NOT EXISTS(SELECT 1 FROM game_account_appearance_unlock_receipts r
        JOIN game_account_appearance_unlocks f USING(grant_occurrence_id,account_id,unlock_key)
        WHERE r.grant_occurrence_id=g.grant_occurrence_id AND r.granting_character_id=g.character_id
        AND r.source_kind='task_board' AND r.source_event_id=uuid_send(g.grant_occurrence_id)
        AND r.content_revision='task-board-shop-v1'
        AND r.unlock_key='oteryn:taskboard/shop/offer-'||g.offer_id::text
        AND r.account_id=(SELECT account_id FROM game_character_roots WHERE character_id=g.character_id)
    ) THEN
        RAISE EXCEPTION 'Paid Task-shop cosmetic lacks matching appearance entitlement' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_task_board_cosmetic_source_guard AFTER INSERT ON game_task_board_cosmetic_grants
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_task_board_cosmetic_grant_guard();
CREATE CONSTRAINT TRIGGER game_task_board_cosmetic_appearance_guard AFTER INSERT ON game_account_appearance_unlock_receipts
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW WHEN(NEW.source_kind='task_board')
    EXECUTE FUNCTION game_task_board_cosmetic_grant_guard();
REVOKE ALL ON game_task_board_cosmetic_grants FROM PUBLIC;
GRANT SELECT,INSERT ON game_task_board_cosmetic_grants TO oteryn_game_runtime;
REVOKE ALL ON FUNCTION game_task_board_cosmetic_grant_guard() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_task_board_cosmetic_grant_guard() TO oteryn_game_runtime;

DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_task_board_cosmetic_grant_guard() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;

-- Append to 0621: a paid cosmetic receipt must include its existing-owner grant.
-- The grant's reciprocal guard already checks offer/effect identity, appearance
-- receipt, admitted provenance and the same transaction.
CREATE FUNCTION game_task_board_cosmetic_receipt_guard() RETURNS TRIGGER
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE offer INTEGER;
BEGIN
    offer := (NEW.action #>> '{BuyShop,offer}')::integer;
    IF NEW.refusal='null'::jsonb AND (
        offer BETWEEN 0 AND 9
        OR COALESCE(NEW.effects->'outfit','null'::jsonb)<>'null'::jsonb
        OR COALESCE(NEW.effects->'mount','null'::jsonb)<>'null'::jsonb
    ) THEN
        IF NEW.cause_kind<>'command' OR NEW.committed_character_revision IS NULL
            OR NOT EXISTS(SELECT 1 FROM game_task_board_cosmetic_grants g
                WHERE g.character_id=NEW.character_id
                  AND g.cause_id=NEW.cause_id AND g.cause_ordinal=NEW.cause_ordinal
                  AND g.offer_id=offer AND g.created_xact_id=pg_current_xact_id()
                  AND NEW.created_xact_id=g.created_xact_id)
        THEN
            RAISE EXCEPTION 'Paid Task-shop cosmetic receipt lacks its appearance grant'
                USING ERRCODE='23514';
        END IF;
    END IF;
    RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_task_board_cosmetic_receipt_guard
    AFTER INSERT ON game_task_board_receipts DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION game_task_board_cosmetic_receipt_guard();
DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_task_board_cosmetic_receipt_guard() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_task_board_cosmetic_receipt_guard() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_task_board_cosmetic_receipt_guard() TO oteryn_game_runtime;
