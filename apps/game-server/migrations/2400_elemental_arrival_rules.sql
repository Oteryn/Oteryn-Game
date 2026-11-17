-- M49: canonical Elemental Spheres witnesses. Current map revisions are proved
-- by the Rust reservation; this predicate alone never grants mutation authority.
CREATE TABLE game_elemental_arrival_rules (
 altar_id INTEGER PRIMARY KEY CHECK (altar_id BETWEEN 1 AND 65535),
 corpse_key TEXT NOT NULL,
 charged_appearance INTEGER NOT NULL CHECK (charged_appearance BETWEEN 1 AND 65535),
 creature_key TEXT NOT NULL,
 summon_position JSONB NOT NULL CHECK (jsonb_typeof(summon_position)='array' AND jsonb_array_length(summon_position)=3),
 positions JSONB NOT NULL CHECK (jsonb_typeof(positions)='array' AND jsonb_array_length(positions)=8)
);
-- Crystal 9f5a72c movements_summon_overlord.lua: all eight positions, not seven.
INSERT INTO game_elemental_arrival_rules VALUES
 (7738,'oteryn:item.tibia.i8139',2126,'oteryn:creature.energy_overlord','[33095,32194,-13]','[[33094,32189,-13],[33097,32189,-13],[33099,32191,-13],[33099,32194,-13],[33097,32196,-13],[33094,32196,-13],[33092,32194,-13],[33092,32191,-13]]'),
 (7739,'oteryn:item.tibia.i8141',7745,'oteryn:creature.fire_overlord','[33199,32103,-13]','[[33198,32102,-13],[33201,32102,-13],[33203,32104,-13],[33203,32107,-13],[33201,32109,-13],[33198,32109,-13],[33196,32107,-13],[33196,32104,-13]]'),
 (7740,'oteryn:item.tibia.i8140',7744,'oteryn:creature.ice_overlord','[33286,32102,-13]','[[33285,32097,-13],[33288,32097,-13],[33290,32099,-13],[33290,32102,-13],[33288,32104,-13],[33285,32104,-13],[33283,32102,-13],[33283,32099,-13]]'),
 (7748,'oteryn:item.tibia.i8106',7746,'oteryn:creature.earth_overlord','[33347,32208,-13]','[[33346,32203,-13],[33349,32203,-13],[33351,32205,-13],[33351,32208,-13],[33349,32210,-13],[33346,32210,-13],[33344,32208,-13],[33344,32205,-13]]');
CREATE FUNCTION game_elemental_arrival_rules_immutable() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN RAISE EXCEPTION 'Elemental Spheres recipes are immutable' USING ERRCODE='23514'; END $$;
CREATE TRIGGER game_elemental_arrival_rules_immutable BEFORE INSERT OR UPDATE OR DELETE ON game_elemental_arrival_rules
 FOR EACH ROW EXECUTE FUNCTION game_elemental_arrival_rules_immutable();
CREATE TRIGGER game_elemental_arrival_rules_no_truncate BEFORE TRUNCATE ON game_elemental_arrival_rules
 EXECUTE FUNCTION game_elemental_arrival_rules_immutable();

CREATE FUNCTION game_elemental_arrival_readset_valid(p_altar INTEGER,p_target_key NUMERIC,p_readset JSONB)
RETURNS BOOLEAN LANGUAGE plpgsql STABLE PARALLEL SAFE AS $$
DECLARE rule game_elemental_arrival_rules%ROWTYPE; cell JSONB; n INTEGER:=0; seen NUMERIC[]:='{}'; k NUMERIC; revision NUMERIC;
BEGIN
 IF p_target_key IS NULL OR p_target_key<1 OR p_target_key>18446744073709551615 OR p_target_key<>trunc(p_target_key)
   OR p_readset IS NULL OR jsonb_typeof(p_readset)<>'array' OR octet_length(p_readset::text)>8192
 THEN RETURN FALSE; END IF;
 IF jsonb_array_length(p_readset)<>8 THEN RETURN FALSE; END IF;
 SELECT * INTO rule FROM game_elemental_arrival_rules WHERE altar_id=p_altar;
 IF NOT FOUND THEN RETURN FALSE; END IF;
 FOR cell IN SELECT value FROM jsonb_array_elements(p_readset) LOOP
   IF jsonb_typeof(cell)<>'object' THEN RETURN FALSE; END IF;
   IF (SELECT count(*) FROM jsonb_object_keys(cell))<>4
     OR jsonb_typeof(cell->'key') IS DISTINCT FROM 'number'
     OR jsonb_typeof(cell->'revision') IS DISTINCT FROM 'number'
     OR cell->'position' IS DISTINCT FROM rule.positions->n
     OR cell->'appearance' IS DISTINCT FROM to_jsonb(rule.charged_appearance)
   THEN RETURN FALSE; END IF;
   k:=(cell->>'key')::numeric; revision:=(cell->>'revision')::numeric;
   IF k<1 OR k>18446744073709551615 OR k<>trunc(k) OR k=p_target_key OR k=ANY(seen)
     OR revision<0 OR revision>18446744073709551615 OR revision<>trunc(revision)
   THEN RETURN FALSE; END IF;
   seen:=array_append(seen,k); n:=n+1;
 END LOOP;
 RETURN TRUE;
END $$;
DO $migration$
BEGIN
 EXECUTE format('ALTER FUNCTION game_elemental_arrival_rules_immutable() SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_elemental_arrival_readset_valid(integer,numeric,jsonb) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;
REVOKE ALL ON game_elemental_arrival_rules FROM PUBLIC;
REVOKE ALL ON FUNCTION game_elemental_arrival_rules_immutable(),game_elemental_arrival_readset_valid(INTEGER,NUMERIC,JSONB) FROM PUBLIC;
GRANT SELECT ON game_elemental_arrival_rules TO oteryn_game_runtime,oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_elemental_arrival_readset_valid(INTEGER,NUMERIC,JSONB) TO oteryn_game_runtime,oteryn_game_control;
