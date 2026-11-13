-- M49: consume the native kill-MINT tree child-first, with separate item audits.
-- Actual entry ancestry supplies an expected Ground root, never current authority:
-- completion still requires the same-XID current whole-corpse arrival for that root.
ALTER TABLE game_elemental_loot_retire_receipts
 ADD COLUMN ground_root_item_instance_id UUID REFERENCES game_item_instances,
 ADD COLUMN entry_kind SMALLINT NOT NULL DEFAULT 1 CHECK(entry_kind IN(1,2));
CREATE FUNCTION game_elemental_loot_ground_root(p_item UUID) RETURNS UUID LANGUAGE plpgsql AS $$
DECLARE roots UUID[];
BEGIN
 WITH RECURSIVE ancestry AS (
   SELECT i.item_instance_id,i.world_id,ARRAY[i.item_instance_id] AS visited,0 AS depth
     FROM game_item_instances i WHERE i.item_instance_id=p_item AND i.lifecycle=1
   UNION ALL
   SELECT i.item_instance_id,i.world_id,a.visited||i.item_instance_id,a.depth+1
     FROM ancestry a JOIN (
       SELECT item_instance_id,parent_item_instance_id,world_id FROM game_item_corpse_container_entries
       UNION ALL SELECT item_instance_id,parent_item_instance_id,world_id FROM game_item_nested_entries
     ) e ON e.item_instance_id=a.item_instance_id AND e.world_id=a.world_id
     JOIN game_item_instances i ON i.item_instance_id=e.parent_item_instance_id AND i.world_id=e.world_id
     WHERE a.depth<8 AND NOT i.item_instance_id=ANY(a.visited) AND i.lifecycle=1 AND i.quantity=1
 ) SELECT array_agg(a.item_instance_id) INTO roots FROM ancestry a
   JOIN game_item_ground_locations g ON g.item_instance_id=a.item_instance_id AND g.world_id=a.world_id
   WHERE a.depth>=1 AND g.scope_kind=1;
 IF cardinality(roots) IS DISTINCT FROM 1 THEN
   RAISE EXCEPTION 'elemental loot requires one bounded live Ground ancestry' USING ERRCODE='23514';
 END IF;
 RETURN roots[1];
END $$;
DO $migration$
DECLARE body TEXT; predecessor TEXT; at INTEGER;
BEGIN
 EXECUTE format('ALTER FUNCTION game_elemental_loot_ground_root(uuid) SET search_path=pg_catalog,%I,pg_temp',current_schema());
 body:=pg_get_functiondef('game_elemental_loot_retire_admit()'::regprocedure);
 predecessor:='SELECT * INTO e FROM game_item_corpse_container_entries WHERE item_instance_id=NEW.item_instance_id FOR UPDATE;';
 IF strpos(body,predecessor)=0 THEN RAISE EXCEPTION 'missing elemental nested admission predecessor'; END IF;
 body:=replace(body,predecessor,$arm$
   SELECT * INTO e FROM game_item_corpse_container_entries WHERE item_instance_id=NEW.item_instance_id FOR UPDATE;
   IF FOUND THEN NEW.entry_kind:=1;
   ELSE
     SELECT * INTO e FROM game_item_nested_entries WHERE item_instance_id=NEW.item_instance_id FOR UPDATE;
     NEW.entry_kind:=2;
   END IF;
   NEW.ground_root_item_instance_id:=game_elemental_loot_ground_root(NEW.item_instance_id);
 $arm$);
 predecessor:='OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE item_instance_id=i.item_instance_id OR parent_item_instance_id=i.item_instance_id)';
 IF strpos(body,predecessor)=0 THEN RAISE EXCEPTION 'missing elemental nested shape guard'; END IF;
 body:=replace(body,predecessor,'OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE parent_item_instance_id=i.item_instance_id)');
 EXECUTE body;
 body:=pg_get_functiondef('game_elemental_loot_retire_complete(uuid)'::regprocedure);
 predecessor:='a.corpse_item_instance_id<>r.parent_item_instance_id';
 IF strpos(body,predecessor)=0 THEN RAISE EXCEPTION 'missing elemental nested root completion'; END IF;
 body:=replace(body,predecessor,'a.corpse_item_instance_id IS DISTINCT FROM r.ground_root_item_instance_id');
 predecessor:='OR EXISTS(SELECT 1 FROM game_item_corpse_container_entries WHERE item_instance_id=p_item)';
 IF strpos(body,predecessor)=0 THEN RAISE EXCEPTION 'missing elemental nested location completion'; END IF;
 EXECUTE replace(body,predecessor,predecessor||' OR EXISTS(SELECT 1 FROM game_item_nested_entries WHERE item_instance_id=p_item)');
 body:=pg_get_functiondef('game_elemental_arrival_retire_loot(uuid,uuid,uuid,uuid,uuid,bigint,bytea,bigint,text)'::regprocedure);
 predecessor:=$old$SELECT to_jsonb(v),to_jsonb(c) INTO i,e FROM game_item_instances v JOIN game_item_corpse_container_entries c USING(item_instance_id)
   WHERE v.item_instance_id=p_item AND c.parent_item_instance_id=p_parent FOR UPDATE OF v,c;$old$;
 IF strpos(body,predecessor)=0 THEN RAISE EXCEPTION 'missing elemental nested writer predecessor'; END IF;
 body:=replace(body,predecessor,$arm$
   SELECT to_jsonb(v) INTO i FROM game_item_instances v WHERE v.item_instance_id=p_item FOR UPDATE;
   SELECT to_jsonb(c) INTO e FROM game_item_corpse_container_entries c
     WHERE c.item_instance_id=p_item AND c.parent_item_instance_id=p_parent FOR UPDATE;
   IF NOT FOUND THEN
     SELECT to_jsonb(c) INTO e FROM game_item_nested_entries c
       WHERE c.item_instance_id=p_item AND c.parent_item_instance_id=p_parent FOR UPDATE;
   END IF;
 $arm$);
 body:=replace(body,'IF i IS NULL THEN','IF i IS NULL OR e IS NULL THEN');
 predecessor:='DELETE FROM game_item_corpse_container_entries WHERE item_instance_id=p_item;';
 IF strpos(body,predecessor)=0 THEN RAISE EXCEPTION 'missing elemental nested removal predecessor'; END IF;
 EXECUTE replace(body,predecessor,predecessor||' DELETE FROM game_item_nested_entries WHERE item_instance_id=p_item;');
 body:=pg_get_functiondef('game_item_nested_entry_proven()'::regprocedure);
 at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
 IF at=0 THEN RAISE EXCEPTION 'missing elemental nested custody predecessor'; END IF;
 EXECUTE overlay(body placing E'BEGIN\n'||$arm$
   IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_elemental_loot_retire_receipts r
     WHERE r.created_xact_id=pg_current_xact_id() AND r.entry_kind=2 AND r.entry_before=to_jsonb(OLD)) THEN
     PERFORM game_elemental_loot_retire_complete(OLD.item_instance_id); RETURN NULL;
   END IF;
 $arm$ FROM at FOR 5);
END $migration$;
REVOKE ALL ON FUNCTION game_elemental_loot_ground_root(UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_elemental_loot_ground_root(UUID) TO oteryn_game_runtime;
