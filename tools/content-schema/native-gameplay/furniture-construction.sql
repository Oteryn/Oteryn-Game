-- Ordinary native USE already carries an opaque inventory handle. Retain that
-- original wire intent on the same immutable wrap receipt, without exposing UUIDs.
ALTER TABLE game_house_furniture_wrap_receipts ADD COLUMN use_intent BYTEA
 CHECK(use_intent IS NULL OR octet_length(use_intent) BETWEEN 1 AND 529);

-- Qualify the constructor's source with the same Character->House->Item lock
-- order as the wrap writer, avoiding a shared-to-exclusive House lock upgrade.
CREATE FUNCTION game_house_construction_source(p_session UUID,p_character UUID,p_connection NUMERIC,
 p_lease NUMERIC,p_scope NUMERIC,p_node UUID,p_node_revision NUMERIC,p_world UUID,p_house TEXT,p_item UUID)
RETURNS TABLE(before_item JSONB,location JSONB,provenance JSONB,spatial_position BYTEA,stack_ordinal TEXT)
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE guild UUID;
BEGIN
 PERFORM game_house_podium_fence(p_session,p_character,p_connection,p_lease,p_scope,p_node,p_node_revision,p_world,p_house);
 PERFORM 1 FROM game_character_roots WHERE character_id=p_character FOR UPDATE;
 SELECT owner_guild_id INTO guild FROM game_house_properties WHERE world_id=p_world AND house_key=p_house;
 IF guild IS NOT NULL THEN PERFORM 1 FROM game_guilds WHERE guild_id=guild AND world_id=p_world FOR SHARE; END IF;
 PERFORM 1 FROM game_house_properties WHERE world_id=p_world AND house_key=p_house FOR UPDATE;
 IF (SELECT owner_guild_id FROM game_house_properties WHERE world_id=p_world AND house_key=p_house) IS DISTINCT FROM guild
  THEN RAISE EXCEPTION 'WRAP_STALE'; END IF;
 RETURN QUERY SELECT * FROM game_item_action_house_source(p_session,p_character,p_connection,p_lease,p_scope,
  p_node,p_node_revision,p_world,p_house,p_item);
END $$;

-- Reuse the existing complete writer body and its authority/lineage checks. The
-- only additional input is the bounded original native USE binding. Existing
-- command402 retains its exact45-byte contract and NULL use_intent.
DO $migration$ DECLARE body TEXT; needle TEXT; BEGIN
 body:=pg_get_functiondef('game_house_furniture_wrap_before_srv7_timers(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC,UUID,TEXT,NUMERIC,UUID,NUMERIC,NUMERIC,SMALLINT,BYTEA,BYTEA,TEXT,INTEGER,TEXT)'::regprocedure);
 needle:='game_house_furniture_wrap_before_srv7_timers(';
 IF position(needle IN body)=0 THEN RAISE EXCEPTION 'SRV7 missing constructor writer name'; END IF;
 body:=replace(body,needle,'game_house_furniture_construct(');
 needle:='p_backpack text)';
 IF position(needle IN body)=0 THEN RAISE EXCEPTION 'SRV7 missing constructor parameter signature'; END IF;
 body:=replace(body,needle,'p_backpack text, p_use_intent bytea)');
 needle:='IF r.intent IS DISTINCT FROM p_intent OR r.character_id<>p_character';
 IF position(needle IN body)=0 THEN RAISE EXCEPTION 'SRV7 missing constructor replay binding'; END IF;
 body:=replace(body,needle,'IF r.use_intent IS DISTINCT FROM p_use_intent OR r.intent IS DISTINCT FROM p_intent OR r.character_id<>p_character');
 needle:='world_id,house_key,intent,action,before_item,after_key,house_location,inventory_location,before_timer)';
 IF position(needle IN body)=0 THEN RAISE EXCEPTION 'SRV7 missing constructor original-intent columns'; END IF;
 body:=replace(body,needle,'world_id,house_key,intent,action,before_item,after_key,house_location,inventory_location,before_timer,use_intent)');
 needle:='(SELECT to_jsonb(t) FROM game_item_timed_states t WHERE t.item_instance_id=p_item));';
 IF position(needle IN body)=0 THEN RAISE EXCEPTION 'SRV7 missing constructor original-intent values'; END IF;
 body:=replace(body,needle,'(SELECT to_jsonb(t) FROM game_item_timed_states t WHERE t.item_instance_id=p_item),p_use_intent);');
 needle:='BEGIN';
 body:=overlay(body placing E'BEGIN\n IF p_action<>2 OR p_use_intent IS NULL OR octet_length(p_use_intent) NOT BETWEEN 1 AND 529 THEN RAISE EXCEPTION ''WRAP_STALE''; END IF;\n'
  from regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i') for 5);
 EXECUTE body;
 -- A command402 retry cannot masquerade as a nativeUSE retry of the same id.
 body:=pg_get_functiondef('game_house_furniture_wrap_before_srv7_timers(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC,UUID,TEXT,NUMERIC,UUID,NUMERIC,NUMERIC,SMALLINT,BYTEA,BYTEA,TEXT,INTEGER,TEXT)'::regprocedure);
 needle:='IF r.intent IS DISTINCT FROM p_intent OR r.character_id<>p_character';
 IF position(needle IN body)=0 THEN RAISE EXCEPTION 'SRV7 missing legacy replay binding'; END IF;
 EXECUTE replace(body,needle,'IF r.use_intent IS NOT NULL OR r.intent IS DISTINCT FROM p_intent OR r.character_id<>p_character');
END $migration$;
DO $$ DECLARE f regprocedure; BEGIN
 FOREACH f IN ARRAY ARRAY[
  'game_house_construction_source(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC,UUID,TEXT,UUID)'::regprocedure,
  'game_house_furniture_construct(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC,UUID,TEXT,NUMERIC,UUID,NUMERIC,NUMERIC,SMALLINT,BYTEA,BYTEA,TEXT,INTEGER,TEXT,BYTEA)'::regprocedure] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',f,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC,oteryn_game_runtime,oteryn_game_control',f);
  EXECUTE format('GRANT EXECUTE ON FUNCTION %s TO oteryn_game_runtime',f);
 END LOOP;
END $$;
