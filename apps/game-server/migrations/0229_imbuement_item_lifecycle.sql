-- IMBUEITEMS-1: filled-scroll and etcher USE-with outcomes.
-- The immutable receipt is both the replay record and the only authority allowed to consume
-- the source and alter the target's imbuements.  Both items must still be direct entries of the
-- admitted character's current main backpack when the receipt is inserted.
CREATE TABLE game_item_imbuement_item_records (
 transaction_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(transaction_id)),
 event_id UUID NOT NULL UNIQUE CHECK(game_character_is_uuid_v7(event_id)),
 game_session_id UUID NOT NULL,
 command_id NUMERIC(20,0) NOT NULL CHECK(command_id BETWEEN 1 AND 18446744073709551615),
 character_id UUID NOT NULL REFERENCES game_character_roots,
 world_id UUID NOT NULL,
 request_binding BYTEA NOT NULL CHECK(octet_length(request_binding)=32),
 action SMALLINT NOT NULL CHECK(action IN (1,2)), -- 1 scroll, 2 etcher
 source_item_instance_id UUID NOT NULL REFERENCES game_item_instances,
 source_production_key TEXT NOT NULL CHECK(octet_length(source_production_key) BETWEEN 1 AND 512),
 target_item_instance_id UUID NOT NULL REFERENCES game_item_instances,
 parent_item_instance_id UUID NOT NULL REFERENCES game_item_instances,
 source_ordinal NUMERIC(20,0) NOT NULL CHECK(source_ordinal>0),
 target_ordinal NUMERIC(20,0) NOT NULL CHECK(target_ordinal>0),
 slot SMALLINT CHECK(slot BETWEEN 0 AND 2),
 category SMALLINT CHECK(category BETWEEN 0 AND 19),
 imbuement_key TEXT CHECK(imbuement_key IS NULL OR octet_length(imbuement_key) BETWEEN 1 AND 512),
 imbuement_revision TEXT CHECK(imbuement_revision IS NULL OR octet_length(imbuement_revision) BETWEEN 1 AND 512),
 tier SMALLINT CHECK(tier BETWEEN 1 AND 3),
 cleared_slots SMALLINT[] NOT NULL DEFAULT '{}',
 occurred_at BIGINT NOT NULL CHECK(occurred_at>0),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 UNIQUE(game_session_id,command_id),
 CHECK(source_item_instance_id<>target_item_instance_id),
 CHECK((action=1 AND slot IS NOT NULL AND category IS NOT NULL AND imbuement_key IS NOT NULL
        AND imbuement_revision IS NOT NULL AND tier IN (2,3) AND cardinality(cleared_slots)=0)
    OR (action=2 AND slot IS NULL AND category IS NULL AND imbuement_key IS NULL
        AND imbuement_revision IS NULL AND tier IS NULL AND cardinality(cleared_slots) BETWEEN 1 AND 3))
);

CREATE FUNCTION game_item_imbuement_item_apply() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE source game_item_instances%ROWTYPE; target game_item_instances%ROWTYPE;
        actual_slots SMALLINT[];
BEGIN
 IF EXISTS(SELECT 1 FROM game_item_imbuement_item_records WHERE created_xact_id=pg_current_xact_id()
              AND transaction_id<>NEW.transaction_id) THEN
  RAISE EXCEPTION 'one imbuement item command per transaction' USING ERRCODE='23514'; END IF;
 SELECT * INTO source FROM game_item_instances WHERE item_instance_id=NEW.source_item_instance_id FOR UPDATE;
 SELECT * INTO target FROM game_item_instances WHERE item_instance_id=NEW.target_item_instance_id FOR UPDATE;
 IF source.item_instance_id IS NULL OR target.item_instance_id IS NULL OR source.lifecycle<>1
    OR target.lifecycle<>1 OR source.world_id<>NEW.world_id OR target.world_id<>NEW.world_id
    OR source.definition_family<>'Item' OR source.definition_production_key<>NEW.source_production_key
    OR source.quantity<1 OR EXISTS(SELECT 1 FROM game_item_keys WHERE item_instance_id=source.item_instance_id)
    OR EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=source.item_instance_id)
    OR EXISTS(SELECT 1 FROM game_item_texts WHERE item_instance_id=source.item_instance_id)
    OR EXISTS(SELECT 1 FROM game_item_imbuements WHERE item_instance_id=source.item_instance_id)
    OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=source.item_instance_id)
    OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=source.item_instance_id)
    OR NOT EXISTS(SELECT 1 FROM game_item_container_slots s JOIN game_item_container_entries e
          ON e.character_id=s.character_id AND e.parent_item_instance_id=s.item_instance_id
         WHERE s.character_id=NEW.character_id AND e.world_id=NEW.world_id
           AND e.item_instance_id=source.item_instance_id AND e.placement_ordinal=NEW.source_ordinal
           AND e.parent_item_instance_id=NEW.parent_item_instance_id)
    OR NOT EXISTS(SELECT 1 FROM game_item_container_slots s JOIN game_item_container_entries e
          ON e.character_id=s.character_id AND e.parent_item_instance_id=s.item_instance_id
         WHERE s.character_id=NEW.character_id AND e.world_id=NEW.world_id
           AND e.item_instance_id=target.item_instance_id AND e.placement_ordinal=NEW.target_ordinal
           AND e.parent_item_instance_id=NEW.parent_item_instance_id)
 THEN RAISE EXCEPTION 'imbuement source and target must be unchanged direct backpack entries' USING ERRCODE='23514'; END IF;
 SELECT coalesce(array_agg(slot ORDER BY slot),'{}') INTO actual_slots
   FROM game_item_imbuements WHERE item_instance_id=target.item_instance_id;
 IF NEW.action=1 THEN
  IF EXISTS(SELECT 1 FROM game_item_imbuements WHERE item_instance_id=target.item_instance_id
       AND (slot=NEW.slot OR category=NEW.category)) THEN
   RAISE EXCEPTION 'scroll slot or category occupied' USING ERRCODE='23514'; END IF;
 ELSE
  IF actual_slots<>NEW.cleared_slots THEN
   RAISE EXCEPTION 'etcher must bind every occupied slot' USING ERRCODE='23514'; END IF;
 END IF;
 UPDATE game_item_instances SET quantity=quantity-1,
   lifecycle=CASE WHEN quantity=1 THEN 2 ELSE 1 END,last_transaction_id=NEW.transaction_id
   WHERE item_instance_id=source.item_instance_id;
 IF source.quantity=1 THEN DELETE FROM game_item_container_entries WHERE item_instance_id=source.item_instance_id; END IF;
 IF NEW.action=1 THEN
  INSERT INTO game_item_imbuements(item_instance_id,slot,category,imbuement_key,imbuement_revision,tier,
    remaining_ms,applying_occurrence_id,applying_transaction_id)
  VALUES(target.item_instance_id,NEW.slot,NEW.category,NEW.imbuement_key,NEW.imbuement_revision,NEW.tier,
    72000000,NEW.transaction_id,NEW.transaction_id);
 ELSE
  DELETE FROM game_item_imbuements WHERE item_instance_id=target.item_instance_id;
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_item_imbuement_item_records_apply AFTER INSERT ON game_item_imbuement_item_records
 FOR EACH ROW EXECUTE FUNCTION game_item_imbuement_item_apply();
CREATE TRIGGER game_item_imbuement_item_records_immutable BEFORE UPDATE OR DELETE ON game_item_imbuement_item_records
 FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_item_imbuement_item_records_no_truncate BEFORE TRUNCATE ON game_item_imbuement_item_records
 EXECUTE FUNCTION game_item_reject_truncate();
REVOKE ALL ON game_item_imbuement_item_records FROM PUBLIC;
GRANT SELECT,INSERT ON game_item_imbuement_item_records TO oteryn_game_runtime;
GRANT SELECT ON game_item_imbuement_item_records TO oteryn_game_control;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_item_imbuement_item_apply() SET search_path=%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_item_imbuement_item_apply() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_item_imbuement_item_apply() TO oteryn_game_runtime;

-- Extend the existing mutation guards without granting direct table mutation to the runtime.
DO $migration$
DECLARE body TEXT; begin_at INT;
BEGIN
 body:=pg_get_functiondef('game_item_instance_change_proven()'::regprocedure);
 begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 EXECUTE overlay(body placing E'BEGIN\n'||$arm$
  IF EXISTS(SELECT 1 FROM game_item_imbuement_item_records r WHERE
      r.source_item_instance_id=NEW.item_instance_id AND r.transaction_id=NEW.last_transaction_id
      AND r.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;
$arm$ from begin_at for 5);
 body:=pg_get_functiondef('game_item_container_entry_removal_proven()'::regprocedure);
 begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 EXECUTE overlay(body placing E'BEGIN\n'||$arm$
  IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_item_imbuement_item_records r
      JOIN game_item_instances i ON i.item_instance_id=r.source_item_instance_id
      WHERE r.source_item_instance_id=OLD.item_instance_id AND r.character_id=OLD.character_id
      AND r.parent_item_instance_id=OLD.parent_item_instance_id AND r.source_ordinal=OLD.placement_ordinal
      AND r.world_id=OLD.world_id AND i.quantity=0 AND i.lifecycle=2
      AND i.last_transaction_id=r.transaction_id AND r.created_xact_id=pg_current_xact_id())
    THEN RETURN NULL; END IF;
$arm$ from begin_at for 5);
 body:=pg_get_functiondef('game_item_imbuement_row_guard()'::regprocedure);
 begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 EXECUTE overlay(body placing E'BEGIN\n'||$arm$
  IF TG_OP='INSERT' AND EXISTS(SELECT 1 FROM game_item_imbuement_item_records r WHERE r.action=1
      AND r.target_item_instance_id=NEW.item_instance_id AND r.slot=NEW.slot AND r.category=NEW.category
      AND r.imbuement_key=NEW.imbuement_key AND r.imbuement_revision=NEW.imbuement_revision
      AND r.tier=NEW.tier AND r.transaction_id=NEW.applying_transaction_id
      AND r.created_xact_id=pg_current_xact_id()) THEN RETURN NEW; END IF;
  IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_item_imbuement_item_records r WHERE r.action=2
      AND r.target_item_instance_id=OLD.item_instance_id AND OLD.slot=ANY(r.cleared_slots)
      AND r.created_xact_id=pg_current_xact_id()) THEN RETURN OLD; END IF;
$arm$ from begin_at for 5);
END $migration$;

-- Blank-scroll inscription extends the shrine receipt rather than creating a second item owner.
-- Action 3 consumes the same recipe/fee as apply and transforms the blank instance in place,
-- preserving custody and making the filled scroll immediately tradeable.
ALTER TABLE game_item_imbuement_records
  ADD COLUMN output_definition_family TEXT,
  ADD COLUMN output_production_key TEXT,
  ADD COLUMN output_revision_ref TEXT;
ALTER TABLE game_item_imbuement_records DROP CONSTRAINT game_item_imbuement_records_action_check;
ALTER TABLE game_item_imbuement_records ADD CONSTRAINT game_item_imbuement_records_action_check
  CHECK(action IN (1,2,3));
ALTER TABLE game_item_imbuement_records DROP CONSTRAINT game_item_imbuement_records_check;
ALTER TABLE game_item_imbuement_records ADD CONSTRAINT game_item_imbuement_records_check CHECK(
 (action=1 AND jsonb_array_length(materials) BETWEEN 1 AND 3
   AND fee_gold_units=(CASE tier WHEN 1 THEN 5000 WHEN 2 THEN 30000 ELSE 200000 END)
   AND output_definition_family IS NULL AND output_production_key IS NULL AND output_revision_ref IS NULL)
 OR (action=2 AND jsonb_array_length(materials)=0 AND fee_gold_units=15000
   AND output_definition_family IS NULL AND output_production_key IS NULL AND output_revision_ref IS NULL)
 OR (action=3 AND tier IN (2,3) AND jsonb_array_length(materials) BETWEEN 1 AND 3
   AND fee_gold_units=(CASE tier WHEN 2 THEN 30000 ELSE 200000 END)
   AND output_definition_family='Item' AND output_production_key IS NOT NULL
   AND output_revision_ref IS NOT NULL));

CREATE OR REPLACE FUNCTION game_item_imbuement_admit() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE i game_item_instances%ROWTYPE; m JSONB; b game_item_imbuements%ROWTYPE;
BEGIN
 IF EXISTS(SELECT 1 FROM game_item_imbuement_records WHERE created_xact_id=pg_current_xact_id()) THEN
  RAISE EXCEPTION 'one imbuement command per transaction' USING ERRCODE='23514'; END IF;
 SELECT * INTO i FROM game_item_instances WHERE item_instance_id=NEW.item_instance_id FOR UPDATE;
 IF i.item_instance_id IS NULL OR i.lifecycle<>1 OR i.world_id<>NEW.world_id OR i.definition_family<>'Item'
    OR (NEW.action=3 AND (i.definition_production_key<>'oteryn:item.tibia.i51442' OR i.quantity<>1))
    OR NOT EXISTS(SELECT 1 FROM game_item_container_entries e JOIN game_item_container_slots s
                    ON s.character_id=e.character_id AND s.item_instance_id=e.parent_item_instance_id
                   WHERE e.item_instance_id=i.item_instance_id AND e.character_id=NEW.character_id
                     AND e.world_id=NEW.world_id AND e.parent_item_instance_id=NEW.parent_item_instance_id
                     AND e.placement_ordinal=NEW.item_ordinal)
 THEN RAISE EXCEPTION 'imbuement target must be an eligible carried entry' USING ERRCODE='23514'; END IF;
 SELECT * INTO b FROM game_item_imbuements WHERE item_instance_id=NEW.item_instance_id AND slot=NEW.slot;
 IF NEW.action=1 AND (b.item_instance_id IS NOT NULL OR EXISTS(SELECT 1 FROM game_item_imbuements
      WHERE item_instance_id=NEW.item_instance_id AND category=NEW.category))
 THEN RAISE EXCEPTION 'imbuement slot or category is occupied' USING ERRCODE='23514'; END IF;
 IF NEW.action=2 AND (b.item_instance_id IS NULL OR (b.category,b.imbuement_key,b.imbuement_revision,b.tier)
      IS DISTINCT FROM (NEW.category,NEW.imbuement_key,NEW.imbuement_revision,NEW.tier))
 THEN RAISE EXCEPTION 'imbuement clear must name the stored row' USING ERRCODE='23514'; END IF;
 IF NEW.action=3 AND EXISTS(SELECT 1 FROM game_item_imbuements WHERE item_instance_id=NEW.item_instance_id)
 THEN RAISE EXCEPTION 'blank scroll has item state' USING ERRCODE='23514'; END IF;
 IF (SELECT count(DISTINCT value->>'item') FROM jsonb_array_elements(NEW.materials))<>jsonb_array_length(NEW.materials)
 THEN RAISE EXCEPTION 'duplicate imbuement material' USING ERRCODE='23514'; END IF;
 FOR m IN SELECT value FROM jsonb_array_elements(NEW.materials) LOOP
  SELECT * INTO i FROM game_item_instances WHERE item_instance_id=(m->>'item')::uuid FOR UPDATE;
  IF i.item_instance_id IS NULL OR i.item_instance_id=NEW.item_instance_id OR to_jsonb(i)<>m->'before'
     OR i.lifecycle<>1 OR i.world_id<>NEW.world_id OR i.definition_family<>'Item'
     OR (m->>'count')::bigint NOT BETWEEN 1 AND i.quantity
     OR EXISTS(SELECT 1 FROM game_item_keys WHERE item_instance_id=i.item_instance_id)
     OR EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=i.item_instance_id)
     OR EXISTS(SELECT 1 FROM game_item_texts WHERE item_instance_id=i.item_instance_id)
     OR EXISTS(SELECT 1 FROM game_item_imbuements WHERE item_instance_id=i.item_instance_id)
     OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=i.item_instance_id)
     OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=i.item_instance_id)
     OR NOT EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.item_instance_id=i.item_instance_id
                     AND e.character_id=NEW.character_id AND e.world_id=NEW.world_id
                     AND e.parent_item_instance_id=NEW.parent_item_instance_id
                     AND e.placement_ordinal=(m->>'ordinal')::numeric)
  THEN RAISE EXCEPTION 'imbuement requires unchanged plain carried materials' USING ERRCODE='23514'; END IF;
 END LOOP;
 NEW.created_xact_id:=pg_current_xact_id(); RETURN NEW;
END $$;

CREATE OR REPLACE FUNCTION game_item_imbuement_write() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF NEW.action=1 THEN
  INSERT INTO game_item_imbuements(item_instance_id,slot,category,imbuement_key,imbuement_revision,tier,
    remaining_ms,applying_occurrence_id,applying_transaction_id)
  VALUES(NEW.item_instance_id,NEW.slot,NEW.category,NEW.imbuement_key,NEW.imbuement_revision,NEW.tier,
    72000000,NEW.transaction_id,NEW.transaction_id);
 ELSIF NEW.action=2 THEN
  DELETE FROM game_item_imbuements WHERE item_instance_id=NEW.item_instance_id AND slot=NEW.slot;
  IF NOT FOUND THEN RAISE EXCEPTION 'imbuement clear found no row' USING ERRCODE='23514'; END IF;
 ELSIF NEW.action=3 THEN
  UPDATE game_item_instances SET definition_family=NEW.output_definition_family,
    definition_production_key=NEW.output_production_key,definition_revision_ref=NEW.output_revision_ref,
    last_transaction_id=NEW.transaction_id WHERE item_instance_id=NEW.item_instance_id
      AND definition_family='Item' AND definition_production_key='oteryn:item.tibia.i51442'
      AND quantity=1 AND lifecycle=1;
  IF NOT FOUND THEN RAISE EXCEPTION 'blank scroll disappeared' USING ERRCODE='23514'; END IF;
 ELSE
  RAISE EXCEPTION 'unknown imbuement action' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END $$;

-- The existing deferred consistency function remains authoritative for materials and fee. Teach
-- its item-count and target predicates about the one additional transformed target row.
CREATE OR REPLACE FUNCTION game_item_imbuement_complete(p_transaction UUID) RETURNS void
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE r game_item_imbuement_records%ROWTYPE; i game_item_instances%ROWTYPE; m JSONB; lines INTEGER;
BEGIN
 SELECT * INTO r FROM game_item_imbuement_records WHERE transaction_id=p_transaction;
 SELECT f.line_count INTO lines FROM game_item_fee_burns f
  WHERE f.transaction_id=r.transaction_id AND f.cause_kind=7 AND f.cause_occurrence_id=r.transaction_id
    AND f.event_id=r.event_id AND f.character_id=r.character_id AND f.world_id=r.world_id
    AND f.fee_gold_units=r.fee_gold_units AND f.created_xact_id=pg_current_xact_id();
 IF r.transaction_id IS NULL OR r.created_xact_id<>pg_current_xact_id() OR lines IS NULL
    OR (SELECT count(*) FROM game_item_instances WHERE last_transaction_id=r.transaction_id)
       <>jsonb_array_length(r.materials)+lines+(CASE WHEN r.action=3 THEN 1 ELSE 0 END)
    OR (r.action=1 AND NOT EXISTS(SELECT 1 FROM game_item_imbuements b WHERE b.item_instance_id=r.item_instance_id
         AND b.slot=r.slot AND b.category=r.category AND b.imbuement_key=r.imbuement_key
         AND b.imbuement_revision=r.imbuement_revision AND b.tier=r.tier AND b.remaining_ms=72000000
         AND b.applying_transaction_id=r.transaction_id AND b.applying_occurrence_id=r.transaction_id))
    OR (r.action=2 AND EXISTS(SELECT 1 FROM game_item_imbuements b WHERE b.item_instance_id=r.item_instance_id AND b.slot=r.slot))
    OR (r.action=3 AND NOT EXISTS(SELECT 1 FROM game_item_instances target WHERE target.item_instance_id=r.item_instance_id
         AND target.definition_family=r.output_definition_family AND target.definition_production_key=r.output_production_key
         AND target.definition_revision_ref=r.output_revision_ref AND target.lifecycle=1 AND target.quantity=1
         AND target.last_transaction_id=r.transaction_id))
    OR NOT EXISTS(SELECT 1 FROM game_item_container_entries e JOIN game_item_container_slots s
                    ON s.character_id=e.character_id AND s.item_instance_id=e.parent_item_instance_id
                   WHERE e.item_instance_id=r.item_instance_id AND e.character_id=r.character_id
                     AND e.parent_item_instance_id=r.parent_item_instance_id)
 THEN RAISE EXCEPTION 'incomplete atomic imbuement' USING ERRCODE='23514'; END IF;
 FOR m IN SELECT value FROM jsonb_array_elements(r.materials) LOOP
  SELECT * INTO i FROM game_item_instances WHERE item_instance_id=(m->>'item')::uuid;
  IF i.quantity<>(m->'before'->>'quantity')::bigint-(m->>'count')::bigint
     OR i.lifecycle<>(CASE WHEN i.quantity=0 THEN 2 ELSE 1 END)
     OR i.last_transaction_id IS DISTINCT FROM r.transaction_id
     OR (to_jsonb(i)-ARRAY['quantity','lifecycle','last_transaction_id','state_revision']) IS DISTINCT FROM
        ((m->'before')-ARRAY['quantity','lifecycle','last_transaction_id','state_revision'])
     OR (i.quantity=0 AND EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=i.item_instance_id))
     OR (i.quantity>0 AND NOT EXISTS(SELECT 1 FROM game_item_container_entries WHERE item_instance_id=i.item_instance_id
          AND character_id=r.character_id AND parent_item_instance_id=r.parent_item_instance_id
          AND placement_ordinal=(m->>'ordinal')::numeric))
  THEN RAISE EXCEPTION 'imbuement material consumption mismatch' USING ERRCODE='23514'; END IF;
 END LOOP;
END $$;

DO $migration$
DECLARE body TEXT; begin_at INT;
BEGIN
 body:=pg_get_functiondef('game_item_instance_change_proven()'::regprocedure);
 begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 EXECUTE overlay(body placing E'BEGIN\n'||$arm$
  IF EXISTS(SELECT 1 FROM game_item_imbuement_records r WHERE r.action=3
      AND r.item_instance_id=NEW.item_instance_id AND r.transaction_id=NEW.last_transaction_id
      AND r.created_xact_id=pg_current_xact_id()) THEN
    PERFORM game_item_imbuement_complete(NEW.last_transaction_id); RETURN NULL; END IF;
$arm$ from begin_at for 5);
END $migration$;
