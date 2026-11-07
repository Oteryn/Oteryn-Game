-- Actual Ground UID rotation reuses the portable action receipt and clock owner.
DO $migration$
DECLARE body TEXT;old TEXT;n TEXT;
BEGIN
 body:=pg_get_functiondef('game_item_action_furniture_custody(game_item_consume_transform_records)'::regprocedure);
 old:=' ELSE FALSE END';
 IF position(old IN body)=0 THEN RAISE EXCEPTION 'Ground Rotate custody boundary missing';END IF;
 EXECUTE replace(body,old,$arm$
 WHEN 3 THEN r.parent_item_instance_id IS NULL AND game_item_action_custody_current(r)
  AND EXISTS(SELECT 1 FROM game_durability_reconnect_sessions s JOIN game_item_ground_locations g ON g.item_instance_id=r.source_item_instance_id
   WHERE s.game_session_id=r.game_session_id AND s.character_id=r.character_id AND s.runtime_scope_kind=1
    AND g.scope_kind=1 AND g.instance_id IS NULL AND g.world_id=r.world_id AND g.channel_id=s.runtime_scope_channel_id
    AND g.runtime_scope_ownership_generation=s.scope_ownership_generation AND to_jsonb(g)=r.source_location-'qualified_house'
    AND g.stack_ordinal=r.source_ordinal)
 ELSE FALSE END$arm$);
 FOREACH n IN ARRAY ARRAY['game_item_action_furniture_admit(game_item_consume_transform_records)','game_item_action_furniture_complete(uuid)'] LOOP
  body:=pg_get_functiondef(n::regprocedure);old:='r.source_kind NOT IN(1,4)';
  IF position(old IN body)=0 THEN RAISE EXCEPTION 'Ground Rotate source boundary missing: %',n;END IF;
  EXECUTE replace(body,old,'r.source_kind NOT IN(1,3,4)');
 END LOOP;
 body:=pg_get_functiondef('game_item_instance_guard()'::regprocedure);
 old:='(srv7_furniture_receipt.effects->>''kind''=''furniture_rotation'' OR srv7_furniture_receipt.effects->>''portable_use''=''true'') AND srv7_furniture_receipt.source_kind IN(1,4)';
 IF position(old IN body)=0 THEN RAISE EXCEPTION 'Ground Rotate instance proof missing';END IF;
 EXECUTE replace(body,old,'(srv7_furniture_receipt.effects->>''kind''=''furniture_rotation'' OR srv7_furniture_receipt.effects->>''portable_use''=''true'') AND srv7_furniture_receipt.source_kind IN(1,3,4)');
 body:=pg_get_functiondef('game_house_furniture_absolute_guard()'::regprocedure);
 old:='source_kind IN(1,4) AND effects->>''kind''=''furniture_rotation''';
 IF position(old IN body)=0 THEN RAISE EXCEPTION 'Ground Rotate absolute proof missing';END IF;
 EXECUTE replace(body,old,'source_kind IN(1,3,4) AND effects->>''kind''=''furniture_rotation''');
 body:=pg_get_functiondef('game_house_furniture_timer_record(uuid)'::regprocedure);
 old:='source_kind IN(1,4) AND (effects->>''kind''=''furniture_rotation'' OR effects->>''portable_use''=''true'')';
 IF position(old IN body)=0 THEN RAISE EXCEPTION 'Ground Rotate timed proof missing';END IF;
 EXECUTE replace(body,old,'source_kind IN(1,3,4) AND (effects->>''kind''=''furniture_rotation'' OR effects->>''portable_use''=''true'')');
 body:=pg_get_functiondef('game_item_action_furniture_admit(game_item_consume_transform_records)'::regprocedure);
 old:='OR r.consume_source';
 IF position(old IN body)=0 THEN RAISE EXCEPTION 'Ground Rotate closed source boundary missing';END IF;
 EXECUTE replace(body,old,'OR (r.source_kind=3 AND r.effects->>''kind'' IS DISTINCT FROM ''furniture_rotation'' AND r.effects->>''portable_use'' IS DISTINCT FROM ''true'') OR r.consume_source');
 SELECT pg_get_constraintdef(oid) INTO body FROM pg_constraint WHERE conrelid='game_item_consume_transform_records'::regclass AND conname='game_item_action_source_custody';
 IF body IS NULL OR left(body,7)<>'CHECK (' THEN RAISE EXCEPTION 'Ground Rotate source shape missing';END IF;
 ALTER TABLE game_item_consume_transform_records DROP CONSTRAINT game_item_action_source_custody;
 EXECUTE 'ALTER TABLE game_item_consume_transform_records ADD CONSTRAINT game_item_action_source_custody CHECK ('||substring(body FROM 8 FOR length(body)-8)
  ||' OR (source_kind=3 AND (effects->>''kind''=''furniture_rotation'' OR effects->>''portable_use''=''true'') AND NOT consume_source AND source_count=0 AND parent_item_instance_id IS NULL AND secondary_item_instance_id=source_item_instance_id AND outputs=''[]''::jsonb AND extra_inputs=''[]''::jsonb))';
END $migration$;
