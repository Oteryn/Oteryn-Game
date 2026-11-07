-- QUESTPC: Crystal201189f8 items.xml:87366-84. The approved native30
-- projection preserves the donor's source48 fact in the authoring overlay.
-- Reuse the original MINT, temporal-chain and corpse-retirement owners.
-- Preserve their current guards; extend only this closed source policy.
DO $$
DECLARE body TEXT; old TEXT; replacement TEXT;
BEGIN
 -- The DO block is atomic; an installed chain marks the complete policy.
 IF game_summer_corpse_chain('oteryn:item.tibia.i52113')=ARRAY[52113,52114,52115,52116] THEN
  RETURN;
 END IF;
 SELECT pg_get_functiondef('game_summer_corpse_chain(text)'::regprocedure) INTO STRICT body;
 old := 'WHEN ''oteryn:item.tibia.i54678'' THEN ARRAY[54678,54679,54680,54681]';
 IF position(old IN body)=0 THEN RAISE EXCEPTION 'corpse chain changed; review migration'; END IF;
 EXECUTE replace(body,old,'WHEN ''oteryn:item.tibia.i52113'' THEN ARRAY[52113,52114,52115,52116] '||old);

 SELECT pg_get_functiondef('game_summer_corpse_schedule()'::regprocedure) INTO STRICT body;
 old := 'CASE WHEN stage=1 THEN 10000 ELSE 300000 END';
 replacement := 'CASE WHEN chain[1]=52113 THEN CASE WHEN stage=3 THEN 10000 ELSE 600000 END ELSE '||old||' END';
 IF position(old IN body)=0 THEN RAISE EXCEPTION 'corpse schedule duration changed; review migration'; END IF;
 body := replace(body,old,replacement);
 old := 'CASE stage WHEN 1 THEN 10000 WHEN 2 THEN 310000 ELSE 610000 END';
 replacement := 'CASE WHEN chain[1]=52113 THEN CASE stage WHEN 1 THEN 600000 WHEN 2 THEN 1200000 ELSE 1210000 END ELSE '||old||' END';
 IF position(old IN body)=0 THEN RAISE EXCEPTION 'corpse schedule deadline changed; review migration'; END IF;
 EXECUTE replace(body,old,replacement);

 SELECT pg_get_functiondef('game_spell_field_temporal_schedule_proven()'::regprocedure) INTO STRICT body;
 old := 'CASE WHEN NEW.source_stage=1 THEN 10000 ELSE 300000 END';
 replacement := 'CASE WHEN chain[1]=52113 THEN CASE WHEN NEW.source_stage=3 THEN 10000 ELSE 600000 END ELSE '||old||' END';
 IF position(old IN body)=0 THEN RAISE EXCEPTION 'corpse proof duration changed; review migration'; END IF;
 body := replace(body,old,replacement);
 old := 'CASE NEW.source_stage WHEN 1 THEN 10000 WHEN 2 THEN 310000 ELSE 610000 END';
 replacement := 'CASE WHEN chain[1]=52113 THEN CASE NEW.source_stage WHEN 1 THEN 600000 WHEN 2 THEN 1200000 ELSE 1210000 END ELSE '||old||' END';
 IF position(old IN body)=0 THEN RAISE EXCEPTION 'corpse proof deadline changed; review migration'; END IF;
 EXECUTE replace(body,old,replacement);

 SELECT pg_get_functiondef('game_corpse_retirement_deadline(uuid)'::regprocedure) INTO STRICT body;
 old := 'ELSE 670000 END';
 replacement := 'WHEN EXISTS(SELECT 1 FROM game_spell_field_temporal_schedules p WHERE p.item_instance_id=p_item AND p.source_ordinal=0 AND p.source_stage=1 AND p.definition_production_key=''oteryn:item.tibia.i52113'') THEN 1810000 ELSE 670000 END';
 IF position(old IN body)=0 THEN RAISE EXCEPTION 'corpse retirement policy changed; review migration'; END IF;
 EXECUTE replace(body,old,replacement);
END $$;
