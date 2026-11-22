ALTER TABLE game_npc_trades ADD COLUMN include_equipped BOOLEAN NOT NULL DEFAULT FALSE
  CHECK (NOT include_equipped OR (side=2 AND sell_all AND house_key IS NULL)),
  ADD COLUMN equipment_revision_before NUMERIC(20,0)
  CHECK (equipment_revision_before IS NULL OR (include_equipped AND equipment_revision_before>0));

-- Only the equipment deletion trigger can record a slot that actually existed.
CREATE TABLE game_npc_trade_equipment_burns (
  transaction_id UUID NOT NULL REFERENCES game_npc_trades(transaction_id),
  slot SMALLINT NOT NULL CHECK (slot BETWEEN 1 AND 10 AND slot<>9),
  item_instance_id UUID NOT NULL REFERENCES game_item_instances(item_instance_id),
  created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
  PRIMARY KEY(transaction_id,slot),
  UNIQUE(transaction_id,item_instance_id)
);
REVOKE ALL ON game_npc_trade_equipment_burns FROM PUBLIC;
GRANT SELECT ON game_npc_trade_equipment_burns TO oteryn_game_runtime,oteryn_game_control;

CREATE FUNCTION game_npc_trade_capture_equipment_burn() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $capture$
BEGIN
 INSERT INTO game_npc_trade_equipment_burns(transaction_id,slot,item_instance_id)
 SELECT t.transaction_id,OLD.slot,OLD.item_instance_id
 FROM game_item_instances i JOIN game_npc_trades t ON t.transaction_id=i.last_transaction_id
 WHERE i.item_instance_id=OLD.item_instance_id AND i.lifecycle=2 AND i.quantity=0
   AND i.world_id=OLD.world_id AND t.world_id=OLD.world_id AND t.character_id=OLD.character_id
   AND t.include_equipped AND t.equipment_revision_before IS NOT NULL
   AND t.created_xact_id=pg_current_xact_id();
 RETURN OLD;
END $capture$;

CREATE TRIGGER game_npc_trade_capture_equipment_burn
 BEFORE DELETE ON game_character_equipment_slots FOR EACH ROW
 EXECUTE FUNCTION game_npc_trade_capture_equipment_burn();

DO $migration$
DECLARE body TEXT; name TEXT; arm TEXT;
BEGIN
 EXECUTE format('ALTER FUNCTION game_npc_trade_capture_equipment_burn() SET search_path=pg_catalog,%I,pg_temp',current_schema());
 FOREACH name IN ARRAY ARRAY['game_npc_trade_consistency_guard','game_equipment_slot_proven','game_equipment_state_proven'] LOOP
  SELECT pg_get_functiondef(format('%I()',name)::regprocedure) INTO STRICT body;
  IF position('BEGIN' IN body)=0 THEN RAISE EXCEPTION 'missing equipment sale guard entry %',name; END IF;
  CASE name
    WHEN 'game_npc_trade_consistency_guard' THEN
      arm:=$arm$IF NEW.equipment_revision_before IS NOT NULL THEN
        IF NOT EXISTS (SELECT 1 FROM game_character_equipment_state s WHERE s.character_id=NEW.character_id
           AND s.revision=NEW.equipment_revision_before+1 AND s.last_transaction_id=NEW.transaction_id)
          OR NOT EXISTS (SELECT 1 FROM game_npc_trade_equipment_burns e WHERE e.transaction_id=NEW.transaction_id)
          OR EXISTS (
            SELECT 1 FROM game_npc_trade_equipment_burns e
            WHERE e.transaction_id=NEW.transaction_id AND NOT EXISTS (
              SELECT 1 FROM game_npc_trade_lines l WHERE l.transaction_id=e.transaction_id
                AND l.item_instance_id=e.item_instance_id AND l.line_kind=3 AND l.quantity_after=0
            )
          )
        THEN RAISE EXCEPTION 'NPC equipment sale lacks custody burn or revision proof' USING ERRCODE='23514'; END IF;
      END IF;
      IF EXISTS (SELECT 1 FROM game_npc_trade_lines l JOIN game_character_equipment_slots s USING(item_instance_id)
        WHERE l.transaction_id=NEW.transaction_id AND l.line_kind=3)
      THEN RAISE EXCEPTION 'NPC sale left a burned item equipped' USING ERRCODE='23514'; END IF;$arm$;
    WHEN 'game_equipment_slot_proven' THEN
      arm:=$arm$IF TG_OP='DELETE' THEN
        IF EXISTS (SELECT 1 FROM game_npc_trade_equipment_burns e JOIN game_npc_trades t USING(transaction_id)
          JOIN game_npc_trade_lines l USING(transaction_id,item_instance_id)
          WHERE e.created_xact_id=pg_current_xact_id() AND t.created_xact_id=e.created_xact_id
          AND e.item_instance_id=OLD.item_instance_id AND e.slot=OLD.slot
          AND t.character_id=OLD.character_id AND t.world_id=OLD.world_id
          AND l.line_kind=3 AND l.quantity_after=0) THEN RETURN NULL; END IF;
      END IF;$arm$;
    WHEN 'game_equipment_state_proven' THEN
      arm:=$arm$IF NEW.combat_mode IS NOT DISTINCT FROM OLD.combat_mode AND EXISTS (
        SELECT 1 FROM game_npc_trades t WHERE t.transaction_id=NEW.last_transaction_id
          AND t.created_xact_id=pg_current_xact_id() AND t.character_id=NEW.character_id
          AND t.equipment_revision_before=OLD.revision AND NEW.revision=OLD.revision+1
          AND EXISTS (SELECT 1 FROM game_npc_trade_equipment_burns e WHERE e.transaction_id=t.transaction_id)
      ) THEN RETURN NULL; END IF;$arm$;
  END CASE;
  EXECUTE regexp_replace(body,'BEGIN','BEGIN '||arm);
 END LOOP;
END $migration$;
REVOKE ALL ON FUNCTION game_npc_trade_capture_equipment_burn() FROM PUBLIC;
