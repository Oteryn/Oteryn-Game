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
