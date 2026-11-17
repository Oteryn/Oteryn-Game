-- Quest hand-ins verify every stack independently. Keep the complete bounded
-- custody walk, but defer wide location JSON until after membership filtering;
-- otherwise a 275-stack Royal Costume recipe repeatedly serializes its entire
-- inventory inside every admission trigger and exceeds the semantic deadline.
CREATE OR REPLACE FUNCTION game_quest_callback_carried(actor UUID, world UUID, selected_item UUID)
RETURNS TABLE(item UUID, location JSONB, kind TEXT) LANGUAGE sql STABLE AS $$
 WITH RECURSIVE tree(item,world,kind,depth,path) AS (
   SELECT s.item_instance_id,s.world_id,'backpack',0,ARRAY[s.item_instance_id]
     FROM game_item_container_slots s WHERE s.character_id=actor
   UNION ALL
   SELECT e.item,e.world,e.kind,t.depth+1,t.path||e.item FROM tree t JOIN (
     SELECT e.item_instance_id AS item,e.world_id AS world,e.parent_item_instance_id AS parent,
            'container' AS kind
       FROM game_item_container_entries e WHERE e.character_id=actor
     UNION ALL
     SELECT e.item_instance_id,e.world_id,e.parent_item_instance_id,'nested'
       FROM game_item_nested_entries e
   ) e ON e.parent=t.item AND e.world=t.world
   WHERE t.depth<=8 AND NOT e.item=ANY(t.path)
 ), bounded AS (SELECT * FROM tree LIMIT 501), carried AS (
   SELECT item,world,kind FROM bounded
    WHERE NOT EXISTS(SELECT 1 FROM bounded WHERE depth>8)
      AND (SELECT count(*) FROM bounded)<=500
   UNION ALL
   SELECT e.item_instance_id,e.world_id,'equipment'
     FROM game_character_equipment_slots e WHERE e.character_id=actor
 ) SELECT c.item,CASE c.kind
       WHEN 'backpack' THEN to_jsonb(s)
       WHEN 'container' THEN to_jsonb(e)
       WHEN 'nested' THEN to_jsonb(n)
       WHEN 'equipment' THEN to_jsonb(q)
     END,c.kind
   FROM carried c
   LEFT JOIN game_item_container_slots s ON c.kind='backpack' AND s.item_instance_id=c.item
   LEFT JOIN game_item_container_entries e ON c.kind='container' AND e.item_instance_id=c.item
   LEFT JOIN game_item_nested_entries n ON c.kind='nested' AND n.item_instance_id=c.item
   LEFT JOIN game_character_equipment_slots q ON c.kind='equipment' AND q.item_instance_id=c.item
   WHERE c.world=$2 AND ($3 IS NULL OR c.item=$3)
$$;
CREATE OR REPLACE FUNCTION game_quest_callback_carried(actor UUID, world UUID)
RETURNS TABLE(item UUID, location JSONB, kind TEXT) LANGUAGE sql STABLE AS $$
 SELECT * FROM game_quest_callback_carried($1,$2,NULL)
$$;
DO $migration$
DECLARE body TEXT; target TEXT;
 original TEXT := 'game_quest_callback_carried(NEW.character_id,i.world_id)';
 replacement TEXT := 'game_quest_callback_carried(NEW.character_id,i.world_id,i.item_instance_id)';
BEGIN
 FOREACH target IN ARRAY ARRAY['game_quest_callback_admit()','game_quest_callback_complete()'] LOOP
  SELECT pg_get_functiondef(target::regprocedure) INTO STRICT body;
  IF position(original IN body)>0 THEN
   EXECUTE replace(body,original,replacement);
  ELSIF position(replacement IN body)=0 THEN
   RAISE EXCEPTION 'Quest carried membership proof anchor changed: %',target;
  END IF;
 END LOOP;
END $migration$;
DO $$ DECLARE target TEXT; BEGIN
 FOREACH target IN ARRAY ARRAY['game_quest_callback_carried(uuid,uuid)','game_quest_callback_carried(uuid,uuid,uuid)'] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',target,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',target);
  EXECUTE format('GRANT EXECUTE ON FUNCTION %s TO oteryn_game_runtime',target);
 END LOOP;
END $$;
