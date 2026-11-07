-- SRV7 W18. Generated rule values from Crystal 201189f871d23597b82caf88fd611454a2ddefb8.
-- data/items/items.xml (wrapableto/rotateto), data-crystal/scripts/actions/other/
-- construction_kits.lua:1-58, data-global/scripts/actions/other/kits.lua:1-14,
-- data/scripts/actions/items/decay_to.lua:6-511. All existing custody/ACL/session
-- and immutable-receipt proofs remain authoritative.
CREATE TABLE game_house_furniture_rules (
 action SMALLINT NOT NULL CHECK(action BETWEEN 1 AND 4),
 first_item INTEGER NOT NULL CHECK(first_item>0),
 last_item INTEGER NOT NULL CHECK(last_item>=first_item),
 result_item INTEGER NOT NULL CHECK(result_item>0),
 PRIMARY KEY(action,first_item,result_item)
);
INSERT INTO game_house_furniture_rules(action,first_item,last_item,result_item) VALUES
__RULE_ROWS__;
REVOKE ALL ON game_house_furniture_rules FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
GRANT SELECT ON game_house_furniture_rules TO oteryn_game_runtime,oteryn_game_control;

ALTER FUNCTION game_house_furniture_wrap_pair(UUID,TEXT,TEXT,SMALLINT)
 RENAME TO game_house_furniture_wrap_pair_before_srv7;
CREATE FUNCTION game_house_furniture_wrap_pair(p_item UUID,p_before TEXT,p_after TEXT,p_action SMALLINT)
 RETURNS BOOLEAN LANGUAGE sql STABLE SECURITY DEFINER STRICT AS $$
 SELECT game_house_furniture_wrap_pair_before_srv7(p_item,p_before,p_after,p_action)
  OR EXISTS(SELECT 1 FROM game_house_furniture_rules c
   WHERE c.action=p_action AND p_after='oteryn:item.tibia.i'||c.result_item::text
    AND p_before ~ '^oteryn:item[.]tibia[.]i[1-9][0-9]{0,8}$'
    AND (CASE WHEN p_before ~ '^oteryn:item[.]tibia[.]i[1-9][0-9]{0,8}$'
      THEN substring(p_before FROM 20)::integer ELSE 0 END) BETWEEN c.first_item AND c.last_item)
  OR (p_action=2 AND EXISTS(SELECT 1 FROM game_house_furniture_wrap_receipts r
   WHERE r.item_instance_id=p_item AND r.action=1 AND r.after_key=p_before
    AND r.before_item->>'definition_production_key'=p_after))
$$;

-- A real carried construction kit is itself its source provenance. A generic
-- decoration kit still requires its original NPC provenance or an exact prior
-- wrap receipt; a client-provided target can never manufacture that metadata.
DO $migration$ DECLARE body TEXT; needle TEXT; replacement TEXT; BEGIN
 SELECT pg_get_functiondef('game_house_furniture_wrap(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC,UUID,TEXT,NUMERIC,UUID,NUMERIC,NUMERIC,SMALLINT,BYTEA,BYTEA,TEXT,INTEGER,TEXT)'::regprocedure) INTO STRICT body;
 needle:=$old$AND NOT EXISTS(SELECT 1 FROM game_house_furniture_kit_origins k
      WHERE k.item_instance_id=p_item AND k.target_key=p_after AND k.target_revision=i.definition_revision_ref
       AND i.definition_production_key='oteryn:item.tibia.i23398')) THEN RAISE EXCEPTION 'WRAP_NOT_WRAPABLE'; END IF;$old$;
 replacement:=$new$AND NOT EXISTS(SELECT 1 FROM game_house_furniture_kit_origins k
      WHERE k.item_instance_id=p_item AND k.target_key=p_after AND k.target_revision=i.definition_revision_ref
       AND i.definition_production_key='oteryn:item.tibia.i23398')
    AND NOT EXISTS(SELECT 1 FROM game_house_furniture_rules c WHERE c.action=2
      AND i.definition_production_key='oteryn:item.tibia.i'||c.first_item::text
      AND p_after='oteryn:item.tibia.i'||c.result_item::text)) THEN RAISE EXCEPTION 'WRAP_NOT_WRAPABLE'; END IF;$new$;
 IF position(needle IN body)=0 THEN RAISE EXCEPTION 'SRV7 missing existing construction-kit origin proof'; END IF;
 EXECUTE replace(body,needle,replacement);
END $migration$;

ALTER FUNCTION game_item_action_house_pair(TEXT,TEXT) RENAME TO game_item_action_house_pair_before_srv7;
CREATE FUNCTION game_item_action_house_pair(p_before TEXT,p_after TEXT) RETURNS BOOLEAN
 LANGUAGE sql STABLE SECURITY DEFINER STRICT AS $$
 SELECT game_item_action_house_pair_before_srv7(p_before,p_after)
  OR EXISTS(SELECT 1 FROM game_house_furniture_rules c WHERE c.action IN(3,4)
   AND p_before='oteryn:item.tibia.i'||c.first_item::text
   AND p_after='oteryn:item.tibia.i'||c.result_item::text)
$$;
DO $$ DECLARE f regprocedure; BEGIN
 FOREACH f IN ARRAY ARRAY[
  'game_house_furniture_wrap_pair(UUID,TEXT,TEXT,SMALLINT)'::regprocedure,
  'game_item_action_house_pair(TEXT,TEXT)'::regprocedure] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',f,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC,oteryn_game_runtime,oteryn_game_control',f);
 END LOOP;
END $$;
