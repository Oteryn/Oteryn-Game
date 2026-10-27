-- QUEST-CALLBACKS-3: extend the existing quest consumption receipt with carried custody.
ALTER TABLE game_quest_callback_consumptions
 ALTER COLUMN before_ground DROP NOT NULL,
 ADD COLUMN before_location JSONB,
 ADD COLUMN location_kind TEXT NOT NULL DEFAULT 'ground'
   CHECK(location_kind IN ('ground','container','nested','backpack','equipment')),
 ADD COLUMN replacement TEXT CHECK(octet_length(replacement) BETWEEN 1 AND 256),
 ADD CONSTRAINT game_quest_callback_location CHECK(
   (location_kind='ground' AND before_ground IS NOT NULL AND before_location IS NULL AND replacement IS NULL)
   OR (location_kind<>'ground' AND before_ground IS NULL AND before_location IS NOT NULL)),
 ADD CONSTRAINT game_quest_callback_replacement CHECK(replacement IS NULL OR quantity=1);

-- Same carried families and bounds as read_item_use_inventory, including closed bags.
CREATE FUNCTION game_quest_callback_carried(actor UUID, world UUID)
RETURNS TABLE(item UUID, location JSONB, kind TEXT) LANGUAGE sql STABLE AS $$
 WITH RECURSIVE tree(item,world,location,kind,depth,path) AS (
   SELECT s.item_instance_id,s.world_id,to_jsonb(s),'backpack',0,ARRAY[s.item_instance_id]
     FROM game_item_container_slots s WHERE s.character_id=actor
   UNION ALL
   SELECT e.item,e.world,e.location,e.kind,t.depth+1,t.path||e.item FROM tree t JOIN (
     SELECT e.item_instance_id AS item,e.world_id AS world,e.parent_item_instance_id AS parent,
            to_jsonb(e) AS location,'container' AS kind
       FROM game_item_container_entries e WHERE e.character_id=actor
     UNION ALL
     SELECT e.item_instance_id,e.world_id,e.parent_item_instance_id,to_jsonb(e),'nested'
       FROM game_item_nested_entries e
   ) e ON e.parent=t.item AND e.world=t.world
   WHERE t.depth<=8 AND NOT e.item=ANY(t.path)
 ), bounded AS (SELECT * FROM tree LIMIT 501), carried AS (
   SELECT item,world,location,kind FROM bounded
    WHERE NOT EXISTS(SELECT 1 FROM bounded WHERE depth>8)
      AND (SELECT count(*) FROM bounded)<=500
   UNION ALL
   SELECT e.item_instance_id,e.world_id,to_jsonb(e),'equipment'
     FROM game_character_equipment_slots e WHERE e.character_id=actor
 ) SELECT item,location,kind FROM carried WHERE carried.world=$2
$$;

CREATE FUNCTION game_quest_callback_after(r game_quest_callback_consumptions, i game_item_instances)
RETURNS BOOLEAN LANGUAGE sql IMMUTABLE AS $$
 SELECT i.last_transaction_id=r.transaction_id AND
   CASE WHEN r.replacement IS NULL THEN
     i.quantity=(r.before_item->>'quantity')::bigint-r.quantity
     AND i.lifecycle=CASE WHEN i.quantity=0 THEN 2 ELSE 1 END
     AND (to_jsonb(i)-ARRAY['quantity','lifecycle','last_transaction_id'])=
         (r.before_item-ARRAY['quantity','lifecycle','last_transaction_id'])
   ELSE
     r.quantity=1 AND (r.before_item->>'quantity')::bigint=1 AND i.quantity=1 AND i.lifecycle=1
     AND i.definition_production_key=r.replacement
     AND (to_jsonb(i)-ARRAY['definition_production_key','last_transaction_id'])=
         (r.before_item-ARRAY['definition_production_key','last_transaction_id'])
   END
$$;

-- Prepend bounded carried arms to the existing guards; all original proof arms remain intact.
DO $migration$
DECLARE n TEXT; body TEXT; arm TEXT; begin_at INT;
BEGIN
 FOREACH n IN ARRAY ARRAY['game_quest_callback_admit','game_quest_callback_complete',
   'game_item_instance_guard','game_item_instance_change_proven',
   'game_item_container_entry_removal_proven','game_item_nested_entry_proven',
   'game_equipment_slot_proven','game_item_immutable'] LOOP
  -- The backpack slot is named separately from its container-entry guard.
  IF to_regprocedure(n||'()') IS NULL THEN
    RAISE EXCEPTION 'missing quest carried guard: %',n;
  END IF;
  body:=pg_get_functiondef((n||'()')::regprocedure);
  begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
  IF begin_at=0 THEN RAISE EXCEPTION 'unexpected quest carried guard body: %',n; END IF;
  IF n='game_quest_callback_admit' THEN
   arm:=$arm$
    IF NEW.location_kind<>'ground' THEN
      SELECT * INTO i FROM game_item_instances WHERE item_instance_id=NEW.item_instance_id FOR UPDATE;
      IF i.item_instance_id IS NULL OR to_jsonb(i) IS DISTINCT FROM NEW.before_item
        OR i.lifecycle<>1 OR i.quantity<NEW.quantity OR i.definition_family<>'Item'
        OR (NEW.replacement IS NOT NULL AND (NEW.quantity<>1 OR i.quantity<>1))
        OR NOT EXISTS(SELECT 1 FROM game_quest_callback_carried(NEW.character_id,i.world_id) c
          WHERE c.item=i.item_instance_id AND c.location=NEW.before_location AND c.kind=NEW.location_kind)
        OR EXISTS(SELECT 1 FROM game_item_keys WHERE item_instance_id=i.item_instance_id)
        OR EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=i.item_instance_id)
        OR EXISTS(SELECT 1 FROM game_item_texts WHERE item_instance_id=i.item_instance_id)
        OR EXISTS(SELECT 1 FROM game_item_imbuements WHERE item_instance_id=i.item_instance_id)
        OR EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=i.item_instance_id)
        OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=i.item_instance_id)
      THEN RAISE EXCEPTION 'quest carried input lacks plain owned custody' USING ERRCODE='23514'; END IF;
      NEW.created_xact_id:=pg_current_xact_id();
      RETURN NEW;
    END IF;
$arm$;
  ELSIF n='game_quest_callback_complete' THEN
   arm:=$arm$
    IF NEW.location_kind<>'ground' THEN
      SELECT * INTO i FROM game_item_instances WHERE item_instance_id=NEW.item_instance_id;
      IF NEW.created_xact_id<>pg_current_xact_id()
        OR NOT coalesce(game_quest_callback_after(NEW,i),false)
        OR (i.quantity=0 AND EXISTS(SELECT 1 FROM game_quest_callback_carried(NEW.character_id,i.world_id) c WHERE c.item=i.item_instance_id))
        OR (i.quantity>0 AND NOT EXISTS(SELECT 1 FROM game_quest_callback_carried(NEW.character_id,i.world_id) c
          WHERE c.item=i.item_instance_id AND c.location=NEW.before_location AND c.kind=NEW.location_kind))
        OR NOT EXISTS(SELECT 1 FROM game_character_quest_receipts r WHERE r.character_id=NEW.character_id
          AND r.cause_id=NEW.cause_id AND r.cause_ordinal=NEW.cause_ordinal AND r.transition_key=NEW.transition_key
          AND r.cause_kind='use' AND r.created_xact_id=pg_current_xact_id())
      THEN RAISE EXCEPTION 'incomplete atomic carried quest input' USING ERRCODE='23514'; END IF;
      RETURN NULL;
    END IF;
$arm$;
  ELSIF n='game_item_instance_guard' THEN
   arm:=$arm$
    IF TG_OP='UPDATE' AND EXISTS(SELECT 1 FROM game_quest_callback_consumptions r
      WHERE r.location_kind<>'ground' AND r.item_instance_id=OLD.item_instance_id
        AND r.before_item=to_jsonb(OLD) AND r.created_xact_id=pg_current_xact_id()
        AND game_quest_callback_after(r,NEW)) THEN RETURN NEW; END IF;
$arm$;
  ELSIF n='game_item_instance_change_proven' THEN
   arm:=$arm$
    IF EXISTS(SELECT 1 FROM game_quest_callback_consumptions r
      WHERE r.location_kind<>'ground' AND r.item_instance_id=NEW.item_instance_id
        AND r.created_xact_id=pg_current_xact_id() AND game_quest_callback_after(r,NEW)) THEN RETURN NULL; END IF;
$arm$;
  ELSE
   arm:=format($arm$
    IF TG_OP='DELETE' AND TG_TABLE_NAME=%L THEN
    IF EXISTS(SELECT 1 FROM game_quest_callback_consumptions r
      JOIN game_item_instances i USING(item_instance_id)
      WHERE r.location_kind=%L AND r.before_location=to_jsonb(OLD)
        AND r.item_instance_id=OLD.item_instance_id AND r.created_xact_id=pg_current_xact_id()
        AND i.quantity=0 AND i.lifecycle=2 AND game_quest_callback_after(r,i)) THEN
      IF TG_WHEN='BEFORE' THEN RETURN OLD; END IF;
      RETURN NULL;
    END IF;
    END IF;
$arm$,CASE n WHEN 'game_item_container_entry_removal_proven' THEN 'game_item_container_entries'
 WHEN 'game_item_nested_entry_proven' THEN 'game_item_nested_entries'
 WHEN 'game_equipment_slot_proven' THEN 'game_character_equipment_slots' ELSE 'game_item_container_slots' END,
 CASE n WHEN 'game_item_container_entry_removal_proven' THEN 'container'
 WHEN 'game_item_nested_entry_proven' THEN 'nested'
 WHEN 'game_equipment_slot_proven' THEN 'equipment' ELSE 'backpack' END);
  END IF;
  EXECUTE overlay(body placing E'BEGIN\n'||arm from begin_at for 5);
  EXECUTE format('ALTER FUNCTION %I() SET search_path=pg_catalog,%I,pg_temp',n,current_schema());
 END LOOP;
END $migration$;

DO $$ DECLARE n TEXT; BEGIN
 FOREACH n IN ARRAY ARRAY['game_quest_callback_carried(uuid,uuid)',
   'game_quest_callback_after(game_quest_callback_consumptions,game_item_instances)'] LOOP
   EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',n,current_schema());
   EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',n);
   EXECUTE format('GRANT EXECUTE ON FUNCTION %s TO oteryn_game_runtime',n);
 END LOOP;
END $$;

GRANT DELETE ON game_item_container_slots TO oteryn_game_runtime;
