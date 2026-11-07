-- Crystal data/libs/functions/boss_lever.lua:241; data/scripts/creaturescripts/monster/boss_lever_death.lua:1-39.
-- An isolated creature's ordinary loot remains in its real physical corpse.
-- Preserve 0042's root-owner phantom-race lock, with exact typed Instance keys
-- matching item_loop_move::read_ground_container (assignment kind 3). Channel
-- keys remain byte-for-byte unchanged. No receipt/provenance guard is relaxed.
CREATE OR REPLACE FUNCTION game_spell_corpse_entry_owner_lock() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE p UUID; w UUID; c UUID; i UUID; k SMALLINT; lock_key BYTEA;
BEGIN
 IF TG_OP='UPDATE' THEN RAISE EXCEPTION 'corpse entry relink requires source-bound delete/insert' USING ERRCODE='23514'; END IF;
 IF TG_OP='INSERT' THEN p:=NEW.parent_item_instance_id; ELSE p:=OLD.parent_item_instance_id; END IF;
 WITH RECURSIVE ancestors(id,path) AS (
   SELECT p,ARRAY[p]
   UNION ALL SELECT e.parent_item_instance_id,a.path||e.parent_item_instance_id FROM ancestors a
    JOIN game_item_corpse_container_entries e ON e.item_instance_id=a.id
    WHERE NOT e.parent_item_instance_id=ANY(a.path) AND cardinality(a.path)<500
 ) SELECT g.world_id,g.channel_id,g.instance_id,g.scope_kind INTO w,c,i,k
   FROM ancestors a JOIN game_item_ground_locations g ON g.item_instance_id=a.id;
 IF w IS NULL THEN RAISE EXCEPTION 'corpse custody mutation lacks a qualified physical Ground root' USING ERRCODE='23514'; END IF;
 IF k=1 AND c IS NOT NULL AND i IS NULL THEN
   lock_key:=uuid_send(w)||uuid_send(c);
 ELSIF k=2 AND c IS NULL AND i IS NOT NULL THEN
   lock_key:=decode('03','hex')||uuid_send(w)||uuid_send(i);
 ELSE
   RAISE EXCEPTION 'corpse custody mutation lacks an exact typed Ground root' USING ERRCODE='23514';
 END IF;
 PERFORM pg_advisory_xact_lock(hashtextextended(encode(lock_key,'hex'),33));
 IF TG_OP='DELETE' THEN RETURN OLD; END IF; RETURN NEW;
END $$;
DO $$ BEGIN EXECUTE format('ALTER FUNCTION game_spell_corpse_entry_owner_lock() SET search_path = pg_catalog, %I, pg_temp',current_schema()); END $$;
REVOKE ALL ON FUNCTION game_spell_corpse_entry_owner_lock() FROM PUBLIC;
