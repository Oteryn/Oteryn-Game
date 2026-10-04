-- Candidate FAMILIAR-2: offline remaining cooldown without rewriting prior receipts.
-- The V1 eight-key encoding remains byte-identical in the Rust canonical binding.
-- No absolute monotonic clock, actor reference, or periodic logout writer is introduced.
CREATE FUNCTION game_character_familiar_state_v1_valid(value JSONB) RETURNS BOOLEAN
LANGUAGE plpgsql IMMUTABLE AS $$
DECLARE
    k TEXT;
    n NUMERIC;
    elem JSONB;
    previous NUMERIC := -1;
BEGIN
    IF value IS NULL OR jsonb_typeof(value) <> 'object' THEN RETURN FALSE; END IF;
    IF octet_length(value::text) > 4096
       OR (SELECT count(*) FROM jsonb_object_keys(value)) <> 8
       OR NOT value ?& ARRAY['selected_look','granted_looks','saved_expiry_unix','last_logout_unix',
                            'lifecycle_epoch','familiar_definition','familiar_revision','profile_revision'] THEN
        RETURN FALSE;
    END IF;
    FOREACH k IN ARRAY ARRAY['selected_look','saved_expiry_unix','last_logout_unix','lifecycle_epoch'] LOOP
        IF jsonb_typeof(value->k) <> 'number' OR (value->>k) !~ '^[0-9]{1,20}$' THEN RETURN FALSE; END IF;
        n := (value->>k)::numeric;
        IF (k = 'selected_look' AND n > 4294967295)
           OR (k IN ('saved_expiry_unix','last_logout_unix') AND n > 9223372036854775807)
           OR (k = 'lifecycle_epoch' AND n > 18446744073709551615) THEN RETURN FALSE; END IF;
    END LOOP;
    IF jsonb_typeof(value->'granted_looks') <> 'array' THEN RETURN FALSE; END IF;
    IF jsonb_array_length(value->'granted_looks') > 256 THEN RETURN FALSE; END IF;
    FOR elem IN SELECT * FROM jsonb_array_elements(value->'granted_looks') LOOP
        IF jsonb_typeof(elem) <> 'number' OR elem::text !~ '^[0-9]{1,10}$' THEN RETURN FALSE; END IF;
        n := elem::text::numeric;
        IF n <= previous OR n < 1 OR n > 4294967295 THEN RETURN FALSE; END IF;
        previous := n;
    END LOOP;
    IF (jsonb_typeof(value->'familiar_definition') = 'null') <> (jsonb_typeof(value->'familiar_revision') = 'null') THEN RETURN FALSE; END IF;
    IF jsonb_typeof(value->'familiar_definition') <> 'null'
       AND (jsonb_typeof(value->'familiar_definition') <> 'string'
         OR (value->>'familiar_definition') !~ '^[A-Za-z0-9][A-Za-z0-9/._:-]{0,255}$'
         OR jsonb_typeof(value->'familiar_revision') <> 'string'
         OR (value->>'familiar_revision') !~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$') THEN RETURN FALSE; END IF;
    IF jsonb_typeof(value->'profile_revision') <> 'string'
       OR (value->>'profile_revision') !~ '^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$' THEN RETURN FALSE; END IF;
    RETURN TRUE;
END;
$$;

CREATE OR REPLACE FUNCTION game_character_familiar_state_valid(value JSONB) RETURNS BOOLEAN
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

DO $$
BEGIN
    EXECUTE format('ALTER FUNCTION game_character_familiar_state_v1_valid(jsonb) SET search_path=%I,pg_temp',current_schema());
    EXECUTE format('ALTER FUNCTION game_character_familiar_state_valid(jsonb) SET search_path=%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_character_familiar_state_v1_valid(JSONB),game_character_familiar_state_valid(JSONB) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_character_familiar_state_v1_valid(JSONB),game_character_familiar_state_valid(JSONB) TO oteryn_game_runtime,oteryn_game_control;
