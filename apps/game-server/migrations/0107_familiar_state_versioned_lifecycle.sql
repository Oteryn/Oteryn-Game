-- D1195: legacy bytes/history are immutable; only next fenced writes acquire v2 lifecycle.
CREATE FUNCTION game_character_familiar_state_legacy_valid(value JSONB) RETURNS BOOLEAN
LANGUAGE plpgsql IMMUTABLE AS $$
DECLARE cooldown JSONB; previous NUMERIC:=0;
BEGIN
    IF value IS NULL OR jsonb_typeof(value) <> 'object' OR octet_length(value::text)>4096 THEN RETURN FALSE; END IF;
    IF NOT value ? 'cooldowns' THEN RETURN game_character_familiar_state_v1_valid(value); END IF;
    IF (SELECT count(*) FROM jsonb_object_keys(value))<>9
       OR NOT game_character_familiar_state_v1_valid(value-'cooldowns') THEN RETURN FALSE; END IF;
    IF jsonb_typeof(value->'cooldowns')<>'array' OR jsonb_array_length(value->'cooldowns') NOT BETWEEN 1 AND 9 THEN RETURN FALSE; END IF;
    FOR cooldown IN SELECT * FROM jsonb_array_elements(value->'cooldowns') LOOP
        IF jsonb_typeof(cooldown)<>'object'
           OR (SELECT count(*) FROM jsonb_object_keys(cooldown))<>2
           OR NOT cooldown ?& ARRAY['reference_spell_id','remaining_micros']
           OR jsonb_typeof(cooldown->'reference_spell_id')<>'number'
           OR (cooldown->>'reference_spell_id')!~'^[0-9]{1,10}$'
           OR (cooldown->>'reference_spell_id')::numeric NOT BETWEEN 1 AND 4294967295
           OR (cooldown->>'reference_spell_id')::numeric<=previous
           OR jsonb_typeof(cooldown->'remaining_micros')<>'number'
           OR (cooldown->>'remaining_micros')!~'^[0-9]{1,20}$'
           OR (cooldown->>'remaining_micros')::numeric>18446744073709551615 THEN RETURN FALSE; END IF;
        previous:=(cooldown->>'reference_spell_id')::numeric;
    END LOOP;
    RETURN TRUE;
END;
$$;

CREATE OR REPLACE FUNCTION game_character_familiar_state_valid(value JSONB) RETURNS BOOLEAN
LANGUAGE plpgsql IMMUTABLE AS $$
DECLARE life JSONB; writer JSONB; k TEXT;
BEGIN
    IF value IS NULL OR jsonb_typeof(value)<>'object' OR octet_length(value::text)>4096 THEN RETURN FALSE; END IF;
    IF NOT value ? 'lifecycle' THEN RETURN game_character_familiar_state_legacy_valid(value); END IF;
    IF NOT game_character_familiar_state_legacy_valid(value-'lifecycle') THEN RETURN FALSE; END IF;
    life := value->'lifecycle';
    IF jsonb_typeof(life)<>'object' OR (SELECT count(*) FROM jsonb_object_keys(life))<>5
       OR NOT life ?& ARRAY['version','familiar_remaining_ms','open','writer','clean_end_game_session_id']
       OR life->'version'<>'2'::jsonb OR jsonb_typeof(life->'open')<>'boolean'
       OR jsonb_typeof(life->'familiar_remaining_ms')<>'number'
       OR (life->>'familiar_remaining_ms')!~'^[0-9]{1,20}$'
       OR (life->>'familiar_remaining_ms')::numeric>18446744073709551615 THEN RETURN FALSE; END IF;
    IF jsonb_typeof(life->'clean_end_game_session_id')<>'null' THEN
        IF jsonb_typeof(life->'clean_end_game_session_id')<>'string'
           OR (life->>'clean_end_game_session_id')!~'^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$'
           OR life->'open'='true'::jsonb THEN RETURN FALSE; END IF;
    END IF;
    writer := life->'writer';
    IF jsonb_typeof(writer)<>'object' OR (SELECT count(*) FROM jsonb_object_keys(writer))<>5
       OR NOT writer ?& ARRAY['game_session_id','connection_generation','world_id','channel_id','scope_ownership_generation'] THEN RETURN FALSE; END IF;
    FOREACH k IN ARRAY ARRAY['game_session_id','world_id','channel_id'] LOOP
        IF jsonb_typeof(writer->k)<>'string'
           OR (writer->>k)!~'^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$' THEN RETURN FALSE; END IF;
    END LOOP;
    FOREACH k IN ARRAY ARRAY['connection_generation','scope_ownership_generation'] LOOP
        IF jsonb_typeof(writer->k)<>'number' OR (writer->>k)!~'^[0-9]{1,20}$'
           OR (writer->>k)::numeric NOT BETWEEN 1 AND 18446744073709551615 THEN RETURN FALSE; END IF;
    END LOOP;
    RETURN TRUE;
END;
$$;
DO $$
BEGIN
    EXECUTE format('ALTER FUNCTION game_character_familiar_state_legacy_valid(jsonb) SET search_path=%I,pg_temp',current_schema());
    EXECUTE format('ALTER FUNCTION game_character_familiar_state_valid(jsonb) SET search_path=%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_character_familiar_state_legacy_valid(JSONB),game_character_familiar_state_valid(JSONB) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_character_familiar_state_legacy_valid(JSONB),game_character_familiar_state_valid(JSONB) TO oteryn_game_runtime,oteryn_game_control;
