-- SUPPLY-STASH-0. Character + World value, converted by the existing item owner.
-- No custody family for stash units: a stow retires the input; a withdrawal
-- creates fresh default-state items. Existing item guards gain narrow proof arms.
CREATE TABLE game_character_stash_operations (
 transaction_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(transaction_id)),
 game_session_id UUID NOT NULL,
 command_id NUMERIC(20,0) NOT NULL CHECK(command_id BETWEEN 1 AND 18446744073709551615),
 character_id UUID NOT NULL REFERENCES game_character_roots ON DELETE RESTRICT,
 world_id UUID NOT NULL,
 intent BYTEA NOT NULL CHECK(octet_length(intent) BETWEEN 1 AND 48),
 action SMALLINT NOT NULL CHECK(action IN (2,3)),
 outcome SMALLINT NOT NULL CHECK(outcome BETWEEN 2 AND 12),
 event_id UUID UNIQUE CHECK(game_character_is_uuid_v7(event_id)),
 occurred_at BIGINT NOT NULL CHECK(occurred_at>0),
 envelope_sha256 BYTEA CHECK(octet_length(envelope_sha256)=32),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 CHECK ((outcome=2)=(event_id IS NOT NULL)),
 CHECK ((outcome=2)=(envelope_sha256 IS NOT NULL)),
 UNIQUE(game_session_id,command_id)
);
CREATE TABLE game_character_stash_balances (
 character_id UUID NOT NULL REFERENCES game_character_roots ON DELETE RESTRICT,
 world_id UUID NOT NULL,
 definition_key TEXT NOT NULL CHECK(octet_length(definition_key) BETWEEN 1 AND 512),
 definition_revision TEXT NOT NULL CHECK(octet_length(definition_revision) BETWEEN 1 AND 512),
 quantity BIGINT NOT NULL CHECK(quantity BETWEEN 0 AND 1000000000),
 last_entry_id UUID NOT NULL UNIQUE,
 PRIMARY KEY(character_id,world_id,definition_key,definition_revision)
);
CREATE TABLE game_character_stash_entries (
 entry_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(entry_id)),
 transaction_id UUID NOT NULL UNIQUE REFERENCES game_character_stash_operations,
 character_id UUID NOT NULL REFERENCES game_character_roots,
 world_id UUID NOT NULL,
 definition_key TEXT NOT NULL,
 definition_revision TEXT NOT NULL,
 amount BIGINT NOT NULL CHECK(amount>0 AND amount<=1000000000),
 quantity_before BIGINT NOT NULL CHECK(quantity_before BETWEEN 0 AND 1000000000),
 quantity_after BIGINT NOT NULL CHECK(quantity_after BETWEEN 0 AND 1000000000),
 previous_entry_id UUID REFERENCES game_character_stash_entries,
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 FOREIGN KEY(character_id,world_id,definition_key,definition_revision)
 REFERENCES game_character_stash_balances DEFERRABLE INITIALLY DEFERRED
);
ALTER TABLE game_character_stash_balances ADD FOREIGN KEY(last_entry_id)
 REFERENCES game_character_stash_entries DEFERRABLE INITIALLY DEFERRED;
CREATE TABLE game_character_stash_item_lines (
 transaction_id UUID NOT NULL REFERENCES game_character_stash_operations,
 ordinal SMALLINT NOT NULL CHECK(ordinal BETWEEN 1 AND 20),
 item_instance_id UUID NOT NULL CHECK(game_character_is_uuid_v7(item_instance_id)),
 definition_key TEXT NOT NULL,
 definition_revision TEXT NOT NULL,
 quantity BIGINT NOT NULL CHECK(quantity BETWEEN 1 AND 100),
 direction SMALLINT NOT NULL CHECK(direction IN (1,2)),
 parent_item_instance_id UUID NOT NULL REFERENCES game_item_instances,
 placement_ordinal NUMERIC(20,0) NOT NULL CHECK(placement_ordinal>0),
 before_item JSONB,
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 PRIMARY KEY(transaction_id,ordinal),
 UNIQUE(transaction_id,item_instance_id),
 UNIQUE(transaction_id,placement_ordinal),
 CHECK((direction=1)=(before_item IS NOT NULL))
);
CREATE TABLE game_character_stash_audit_outbox (
 event_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(event_id)),
 transaction_id UUID NOT NULL UNIQUE REFERENCES game_character_stash_operations,
 transaction_ordinal INTEGER NOT NULL CHECK(transaction_ordinal=1),
 transaction_count INTEGER NOT NULL CHECK(transaction_count=1),
 event_type_id BIGINT NOT NULL CHECK(event_type_id=9),
 schema_revision BIGINT NOT NULL CHECK(schema_revision=1),
 retention_profile_id TEXT NOT NULL CHECK(retention_profile_id='ECONOMY_LEDGER_RETENTION_V1'),
 world_id UUID NOT NULL CHECK(game_character_is_uuid_v7(world_id)),
 occurred_at BIGINT NOT NULL CHECK(occurred_at>0),
 expires_at BIGINT NOT NULL CHECK(expires_at=occurred_at+2592000000),
 envelope BYTEA NOT NULL CHECK(octet_length(envelope) BETWEEN 1 AND 36000),
 envelope_sha256 BYTEA NOT NULL CHECK(envelope_sha256=sha256(envelope)),
 publication_state SMALLINT NOT NULL CHECK(publication_state IN (1,2)),
 published_at BIGINT CHECK(published_at IS NULL OR published_at>=occurred_at),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 CHECK((publication_state=2)=(published_at IS NOT NULL))
);
CREATE INDEX game_stash_page ON game_character_stash_balances
 (character_id,world_id,definition_key COLLATE "C",definition_revision COLLATE "C") WHERE quantity>0;

CREATE FUNCTION game_stash_admit() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE r game_character_stash_operations%ROWTYPE; i game_item_instances%ROWTYPE;
BEGIN
 IF TG_TABLE_NAME='game_character_stash_operations' THEN
  PERFORM 1 FROM game_character_roots WHERE character_id=NEW.character_id AND world_id=NEW.world_id AND lifecycle=1 FOR UPDATE;
  IF NOT FOUND THEN RAISE EXCEPTION 'stash requires Character in World' USING ERRCODE='23514'; END IF;
 ELSE
  SELECT * INTO r FROM game_character_stash_operations WHERE transaction_id=NEW.transaction_id;
  IF NOT FOUND OR r.created_xact_id<>pg_current_xact_id() OR r.outcome<>2 THEN
   RAISE EXCEPTION 'stash proof requires current successful operation' USING ERRCODE='23514'; END IF;
  IF TG_TABLE_NAME='game_character_stash_item_lines' THEN
   IF (r.action=2)<>(NEW.direction=1) THEN RAISE EXCEPTION 'stash direction mismatch' USING ERRCODE='23514'; END IF;
   IF NEW.direction=1 THEN
    SELECT * INTO i FROM game_item_instances WHERE item_instance_id=NEW.item_instance_id FOR UPDATE;
    IF i.item_instance_id IS NULL OR to_jsonb(i) IS DISTINCT FROM NEW.before_item OR i.lifecycle<>1
       OR i.world_id<>r.world_id OR i.definition_family<>'Item' OR i.quantity<>NEW.quantity
       OR (i.definition_production_key,i.definition_revision_ref)<>(NEW.definition_key,NEW.definition_revision)
       OR EXISTS(SELECT 1 FROM game_item_keys WHERE item_instance_id=i.item_instance_id)
       OR EXISTS(SELECT 1 FROM game_item_tiers WHERE item_instance_id=i.item_instance_id)
       OR EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=i.item_instance_id)
       OR EXISTS(SELECT 1 FROM game_item_texts WHERE item_instance_id=i.item_instance_id)
       OR EXISTS(SELECT 1 FROM game_item_imbuements WHERE item_instance_id=i.item_instance_id)
       OR EXISTS(SELECT 1 FROM game_spell_item_temporal_schedules WHERE item_instance_id=i.item_instance_id)
       OR EXISTS(SELECT 1 FROM game_spell_field_temporal_schedules WHERE item_instance_id=i.item_instance_id)
       OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=i.item_instance_id)
       OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=i.item_instance_id)
       OR NOT EXISTS(SELECT 1 FROM game_item_container_entries e JOIN game_item_container_slots b
            ON b.character_id=e.character_id AND b.item_instance_id=e.parent_item_instance_id
            WHERE e.item_instance_id=i.item_instance_id AND e.character_id=r.character_id
              AND e.world_id=r.world_id AND b.world_id=r.world_id
              AND e.parent_item_instance_id=NEW.parent_item_instance_id AND e.placement_ordinal=NEW.placement_ordinal)
    THEN RAISE EXCEPTION 'stash requires exact plain main-backpack source' USING ERRCODE='23514'; END IF;
   ELSIF EXISTS(SELECT 1 FROM game_item_instances WHERE item_instance_id=NEW.item_instance_id) THEN
    RAISE EXCEPTION 'stash withdrawal requires fresh identity' USING ERRCODE='23514';
   END IF;
  END IF;
 END IF;
 NEW.created_xact_id:=pg_current_xact_id();
 RETURN NEW;
END $$;

CREATE FUNCTION game_stash_balance_guard() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE e game_character_stash_entries%ROWTYPE; r game_character_stash_operations%ROWTYPE;
BEGIN
 IF TG_OP='DELETE' THEN RAISE EXCEPTION 'stash balances cannot be deleted' USING ERRCODE='23514'; END IF;
 SELECT * INTO e FROM game_character_stash_entries WHERE entry_id=NEW.last_entry_id;
 SELECT * INTO r FROM game_character_stash_operations WHERE transaction_id=e.transaction_id;
 IF e.entry_id IS NULL OR e.created_xact_id<>pg_current_xact_id() OR r.outcome<>2
    OR r.created_xact_id<>pg_current_xact_id()
    OR (e.character_id,e.world_id,e.definition_key,e.definition_revision,e.quantity_after)
       IS DISTINCT FROM (NEW.character_id,NEW.world_id,NEW.definition_key,NEW.definition_revision,NEW.quantity)
 THEN RAISE EXCEPTION 'stash balance requires current matching entry' USING ERRCODE='23514'; END IF;
 IF TG_OP='INSERT' THEN
  IF r.action<>2 OR e.quantity_before<>0 OR e.previous_entry_id IS NOT NULL THEN
   RAISE EXCEPTION 'only stow creates a balance' USING ERRCODE='23514'; END IF;
 ELSE
  IF (NEW.character_id,NEW.world_id,NEW.definition_key,NEW.definition_revision)
       IS DISTINCT FROM (OLD.character_id,OLD.world_id,OLD.definition_key,OLD.definition_revision)
     OR e.quantity_before<>OLD.quantity OR e.previous_entry_id IS DISTINCT FROM OLD.last_entry_id THEN
   RAISE EXCEPTION 'stash ledger chain mismatch' USING ERRCODE='23514'; END IF;
 END IF;
 RETURN NEW;
END $$;

CREATE FUNCTION game_stash_complete(p_transaction UUID) RETURNS void LANGUAGE plpgsql AS $$
DECLARE r game_character_stash_operations%ROWTYPE; e game_character_stash_entries%ROWTYPE;
 l game_character_stash_item_lines%ROWTYPE; i game_item_instances%ROWTYPE; b game_character_stash_balances%ROWTYPE;
BEGIN
 SELECT * INTO STRICT r FROM game_character_stash_operations WHERE transaction_id=p_transaction;
 IF r.created_xact_id<>pg_current_xact_id() THEN RAISE EXCEPTION 'historical stash proof' USING ERRCODE='23514'; END IF;
 IF r.outcome<>2 THEN
  IF EXISTS(SELECT 1 FROM game_character_stash_entries WHERE transaction_id=p_transaction)
    OR EXISTS(SELECT 1 FROM game_character_stash_item_lines WHERE transaction_id=p_transaction)
    OR EXISTS(SELECT 1 FROM game_item_instances WHERE minted_transaction_id=p_transaction OR last_transaction_id=p_transaction)
    OR EXISTS(SELECT 1 FROM game_character_stash_audit_outbox WHERE transaction_id=p_transaction)
  THEN RAISE EXCEPTION 'refused stash operation changed value' USING ERRCODE='23514'; END IF;
  RETURN;
 END IF;
 SELECT * INTO e FROM game_character_stash_entries WHERE transaction_id=p_transaction;
 SELECT * INTO b FROM game_character_stash_balances WHERE character_id=e.character_id AND world_id=e.world_id
   AND definition_key=e.definition_key AND definition_revision=e.definition_revision;
 IF e.entry_id IS NULL OR e.created_xact_id<>pg_current_xact_id() OR b.last_entry_id IS DISTINCT FROM e.entry_id
    OR b.quantity<>e.quantity_after OR (e.character_id,e.world_id)<>(r.character_id,r.world_id)
    OR e.quantity_after<>e.quantity_before+(CASE WHEN r.action=2 THEN e.amount ELSE -e.amount END)
    OR e.amount IS DISTINCT FROM (SELECT sum(quantity) FROM game_character_stash_item_lines WHERE transaction_id=p_transaction)
    OR (r.action=2 AND (SELECT count(*) FROM game_character_stash_item_lines WHERE transaction_id=p_transaction)<>1)
    OR (SELECT count(*) FROM game_character_stash_item_lines WHERE transaction_id=p_transaction) NOT BETWEEN 1 AND 20
    OR (SELECT count(*) FROM game_item_instances WHERE minted_transaction_id=p_transaction)<>
        (CASE WHEN r.action=3 THEN (SELECT count(*) FROM game_character_stash_item_lines WHERE transaction_id=p_transaction) ELSE 0 END)
    OR (SELECT count(*) FROM game_item_instances WHERE last_transaction_id=p_transaction)<>(CASE WHEN r.action=2 THEN 1 ELSE 0 END)
    OR (SELECT count(*) FROM game_character_stash_audit_outbox WHERE transaction_id=p_transaction)<>1
    OR NOT EXISTS(SELECT 1 FROM game_character_stash_audit_outbox a WHERE a.event_id=r.event_id AND a.transaction_id=p_transaction
       AND a.envelope_sha256=r.envelope_sha256 AND a.occurred_at=r.occurred_at AND a.transaction_ordinal=1
       AND a.transaction_count=1 AND a.publication_state=1 AND a.published_at IS NULL AND a.created_xact_id=pg_current_xact_id()
       AND a.world_id=r.world_id)
 THEN RAISE EXCEPTION 'stash conversion does not conserve units' USING ERRCODE='23514'; END IF;
 FOR l IN SELECT * FROM game_character_stash_item_lines WHERE transaction_id=p_transaction ORDER BY ordinal LOOP
  SELECT * INTO i FROM game_item_instances WHERE item_instance_id=l.item_instance_id;
  IF i.item_instance_id IS NULL OR l.created_xact_id<>pg_current_xact_id()
     OR (l.definition_key,l.definition_revision)<>(e.definition_key,e.definition_revision)
     OR (i.definition_family,i.definition_production_key,i.definition_revision_ref)<>('Item',e.definition_key,e.definition_revision)
     OR i.world_id<>r.world_id OR (r.action=2)<>(l.direction=1)
  THEN RAISE EXCEPTION 'stash cross-asset conversion' USING ERRCODE='23514'; END IF;
  IF l.direction=1 THEN
   IF i.quantity<>0 OR i.lifecycle<>2 OR i.last_transaction_id IS DISTINCT FROM p_transaction
     OR (to_jsonb(i)-ARRAY['quantity','lifecycle','last_transaction_id']) IS DISTINCT FROM
        (l.before_item-ARRAY['quantity','lifecycle','last_transaction_id'])
     OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=i.item_instance_id)
   THEN RAISE EXCEPTION 'stash input not retired' USING ERRCODE='23514'; END IF;
  ELSE
   IF i.quantity<>l.quantity OR i.lifecycle<>1 OR i.minted_transaction_id<>p_transaction OR i.last_transaction_id IS NOT NULL
     OR NOT EXISTS(SELECT 1 FROM game_item_container_entries c JOIN game_item_container_slots s
          ON s.character_id=c.character_id AND s.item_instance_id=c.parent_item_instance_id
          WHERE c.item_instance_id=i.item_instance_id AND c.character_id=r.character_id AND c.world_id=r.world_id AND s.world_id=r.world_id
            AND c.parent_item_instance_id=l.parent_item_instance_id AND c.placement_ordinal=l.placement_ordinal
            AND c.placed_transaction_id=p_transaction)
     OR (SELECT count(*) FROM game_item_container_entries WHERE parent_item_instance_id=l.parent_item_instance_id)>20
   THEN RAISE EXCEPTION 'stash output not placed' USING ERRCODE='23514'; END IF;
  END IF;
 END LOOP;
END $$;
CREATE FUNCTION game_stash_consistent() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN PERFORM game_stash_complete(NEW.transaction_id); RETURN NULL; END $$;

-- Keep every earlier proof arm; only admit exact current stash conversion lines.
DO $migration$
DECLARE n TEXT; body TEXT; arm TEXT; begin_at INT;
BEGIN
 FOREACH n IN ARRAY ARRAY['game_item_instance_guard','game_item_instance_change_proven',
     'game_item_container_entry_removal_proven','game_item_placement_proven','game_item_mint_consistency_guard'] LOOP
  body:=pg_get_functiondef((n||'()')::regprocedure);
  begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
  IF begin_at=0 THEN RAISE EXCEPTION 'unexpected proof body: %',n; END IF;
  IF n='game_item_instance_guard' THEN
   arm:=$arm$
    IF TG_OP='UPDATE' AND EXISTS(SELECT 1 FROM game_character_stash_item_lines l
      WHERE l.direction=1 AND l.item_instance_id=OLD.item_instance_id AND l.transaction_id=NEW.last_transaction_id
        AND l.before_item=to_jsonb(OLD) AND l.created_xact_id=pg_current_xact_id()) THEN
     IF NEW.quantity<>0 OR NEW.lifecycle<>2 OR (to_jsonb(NEW)-ARRAY['quantity','lifecycle','last_transaction_id']) IS DISTINCT FROM
        (to_jsonb(OLD)-ARRAY['quantity','lifecycle','last_transaction_id']) THEN
      RAISE EXCEPTION 'stash retires exactly the input' USING ERRCODE='23514'; END IF;
     RETURN NEW;
    END IF;
$arm$;
  ELSIF n='game_item_instance_change_proven' THEN
   arm:=$arm$
    IF EXISTS(SELECT 1 FROM game_character_stash_item_lines l WHERE l.direction=1 AND l.item_instance_id=NEW.item_instance_id
      AND l.transaction_id=NEW.last_transaction_id AND l.created_xact_id=pg_current_xact_id()) THEN
     PERFORM game_stash_complete(NEW.last_transaction_id); RETURN NULL;
    END IF;
$arm$;
  ELSIF n='game_item_container_entry_removal_proven' THEN
   arm:=$arm$
    IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_character_stash_item_lines l JOIN game_character_stash_operations r USING(transaction_id)
       JOIN game_item_instances i ON i.item_instance_id=l.item_instance_id
       WHERE l.direction=1 AND l.item_instance_id=OLD.item_instance_id AND r.character_id=OLD.character_id
       AND r.world_id=OLD.world_id AND l.parent_item_instance_id=OLD.parent_item_instance_id
       AND l.placement_ordinal=OLD.placement_ordinal AND i.lifecycle=2 AND i.quantity=0
       AND i.last_transaction_id=l.transaction_id AND l.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;
$arm$;
  ELSIF n='game_item_placement_proven' THEN
   arm:=$arm$
    IF TG_TABLE_NAME='game_item_container_entries' THEN
     IF EXISTS(SELECT 1 FROM game_character_stash_item_lines l JOIN game_character_stash_operations r USING(transaction_id)
       WHERE l.direction=2 AND l.item_instance_id=NEW.item_instance_id AND l.transaction_id=NEW.placed_transaction_id
       AND l.parent_item_instance_id=NEW.parent_item_instance_id AND l.placement_ordinal=NEW.placement_ordinal
       AND r.character_id=NEW.character_id AND r.world_id=NEW.world_id AND l.created_xact_id=pg_current_xact_id())
     THEN RETURN NULL; END IF;
    END IF;
$arm$;
  ELSE
   arm:=$arm$
    IF EXISTS(SELECT 1 FROM game_character_stash_item_lines l WHERE l.direction=2 AND l.item_instance_id=NEW.item_instance_id
      AND l.transaction_id=NEW.minted_transaction_id AND l.created_xact_id=pg_current_xact_id()) THEN
     PERFORM game_stash_complete(NEW.minted_transaction_id); RETURN NULL;
    END IF;
$arm$;
  END IF;
  BEGIN
   EXECUTE overlay(body placing E'BEGIN\n'||arm from begin_at for 5);
  EXCEPTION WHEN OTHERS THEN RAISE EXCEPTION 'stash proof %: %', n, SQLERRM;
  END;
 END LOOP;
END $migration$;

DO $$ DECLARE n TEXT; BEGIN
 FOREACH n IN ARRAY ARRAY['operations','entries','item_lines'] LOOP
  EXECUTE format('CREATE TRIGGER game_stash_admit BEFORE INSERT ON game_character_stash_%I FOR EACH ROW EXECUTE FUNCTION game_stash_admit()',n);
  EXECUTE format('CREATE CONSTRAINT TRIGGER game_stash_consistent AFTER INSERT ON game_character_stash_%I DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_stash_consistent()',n);
  EXECUTE format('CREATE TRIGGER game_stash_immutable BEFORE UPDATE OR DELETE ON game_character_stash_%I FOR EACH ROW EXECUTE FUNCTION game_item_immutable()',n);
 END LOOP;
 FOREACH n IN ARRAY ARRAY['operations','entries','item_lines','balances','audit_outbox'] LOOP
  EXECUTE format('CREATE TRIGGER game_stash_no_truncate BEFORE TRUNCATE ON game_character_stash_%I EXECUTE FUNCTION game_item_reject_truncate()',n);
  EXECUTE format('REVOKE ALL ON game_character_stash_%I FROM PUBLIC',n);
  EXECUTE format('GRANT SELECT,INSERT ON game_character_stash_%I TO oteryn_game_runtime',n);
  EXECUTE format('GRANT SELECT ON game_character_stash_%I TO oteryn_game_control',n);
 END LOOP;
 FOREACH n IN ARRAY ARRAY['game_stash_admit','game_stash_balance_guard','game_stash_consistent'] LOOP
  EXECUTE format('ALTER FUNCTION %I() SET search_path=pg_catalog,%I,pg_temp',n,current_schema());
 END LOOP;
 EXECUTE format('ALTER FUNCTION game_stash_complete(uuid) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
CREATE TRIGGER game_stash_balance_guard BEFORE INSERT OR UPDATE OR DELETE ON game_character_stash_balances
 FOR EACH ROW EXECUTE FUNCTION game_stash_balance_guard();
GRANT UPDATE(quantity,last_entry_id) ON game_character_stash_balances TO oteryn_game_runtime;
REVOKE ALL ON FUNCTION game_stash_admit(),game_stash_balance_guard(),game_stash_consistent(),game_stash_complete(UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_stash_complete(UUID) TO oteryn_game_runtime;

-- The existing town-specific depot capacity owner counts this World-wide asset.
-- Older revisions are incompatible unless active Content explicitly admits them,
-- so this slice counts each revision independently (never merges value assets).
DO $migration$
DECLARE body TEXT;
BEGIN
 body:=pg_get_functiondef('game_character_depot_insert()'::regprocedure);
 IF position('IF v_count > 15000 THEN' IN body)=0 THEN RAISE EXCEPTION 'unexpected depot capacity owner'; END IF;
 body:=replace(body,'IF v_count > 15000 THEN',
   'IF v_count + (SELECT COALESCE(sum((quantity+99)/100),0) FROM game_character_stash_balances WHERE character_id=NEW.character_id AND world_id=NEW.world_id AND quantity>0) > 15000 THEN');
 EXECUTE body;
END $migration$;
CREATE FUNCTION game_stash_capacity() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE used BIGINT; depot BIGINT;
BEGIN
 PERFORM 1 FROM game_character_roots WHERE character_id=NEW.character_id AND world_id=NEW.world_id AND lifecycle=1 FOR UPDATE;
 SELECT COALESCE(sum((quantity+99)/100),0) INTO used FROM game_character_stash_balances WHERE character_id=NEW.character_id AND world_id=NEW.world_id AND quantity>0;
 SELECT COALESCE(max(n),0) INTO depot FROM (SELECT sum(committed) AS n FROM game_character_depot_box_counters WHERE character_id=NEW.character_id GROUP BY town_id) towns;
 IF used+depot>15000 THEN RAISE EXCEPTION 'stash exceeds counted depot capacity' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
DO $$ BEGIN EXECUTE format('ALTER FUNCTION game_stash_capacity() SET search_path=pg_catalog,%I,pg_temp',current_schema()); END $$;
REVOKE ALL ON FUNCTION game_stash_capacity() FROM PUBLIC;
CREATE CONSTRAINT TRIGGER game_stash_capacity AFTER INSERT OR UPDATE ON game_character_stash_balances
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_stash_capacity();

CREATE CONSTRAINT TRIGGER game_stash_audit_consistent AFTER INSERT ON game_character_stash_audit_outbox
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_stash_consistent();
CREATE TRIGGER game_stash_audit_immutable BEFORE UPDATE OR DELETE ON game_character_stash_audit_outbox
 FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
