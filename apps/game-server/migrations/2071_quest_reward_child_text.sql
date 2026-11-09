-- M15: bind every initial child Text to its ordered authored Quest reward.
CREATE FUNCTION game_quest_reward_text_matches(p_root TEXT, p_children JSONB, p_give JSONB)
RETURNS BOOLEAN LANGUAGE plpgsql IMMUTABLE AS $$
DECLARE spec JSONB := p_give->'text'; entries JSONB := '[]'::jsonb;
        entry JSONB; actual JSONB; expected JSONB; index_value NUMERIC;
        last_index INTEGER := -1; child_index INTEGER; body TEXT;
        total_bytes INTEGER := 0; total_chars INTEGER := 0;
BEGIN
    IF jsonb_typeof(p_children) IS DISTINCT FROM 'array'
       OR jsonb_typeof(p_give->'contents') IS DISTINCT FROM 'array'
       OR jsonb_array_length(p_children) <> jsonb_array_length(p_give->'contents')
    THEN RETURN FALSE; END IF;
    IF spec IS NULL OR spec = 'null'::jsonb THEN
        IF p_root IS NOT NULL THEN RETURN FALSE; END IF;
    ELSIF jsonb_typeof(spec) = 'string' THEN
        IF p_root IS DISTINCT FROM (p_give->>'text') OR jsonb_array_length(p_children) <> 0
        THEN RETURN FALSE; END IF;
    ELSIF jsonb_typeof(spec) = 'object' THEN
        IF p_root IS NOT NULL OR NOT spec ? 'contents'
           OR (SELECT count(*) FROM jsonb_object_keys(spec)) <> 1
           OR jsonb_typeof(spec->'contents') IS DISTINCT FROM 'array'
           OR jsonb_array_length(spec->'contents') NOT BETWEEN 1 AND 20
        THEN RETURN FALSE; END IF;
        entries := spec->'contents';
        FOR entry IN SELECT value FROM jsonb_array_elements(entries) LOOP
            IF jsonb_typeof(entry) IS DISTINCT FROM 'object'
               OR NOT entry ?& ARRAY['index','text']
               OR (SELECT count(*) FROM jsonb_object_keys(entry)) <> 2
               OR jsonb_typeof(entry->'index') IS DISTINCT FROM 'number'
               OR jsonb_typeof(entry->'text') IS DISTINCT FROM 'string'
            THEN RETURN FALSE; END IF;
            index_value := (entry->>'index')::numeric;
            IF index_value <> trunc(index_value) OR index_value <= last_index
               OR index_value < 0 OR index_value >= jsonb_array_length(p_children)
            THEN RETURN FALSE; END IF;
            child_index := index_value::integer;
            last_index := child_index;
            body := entry->>'text';
            total_bytes := total_bytes + octet_length(body);
            total_chars := total_chars + char_length(body);
            IF octet_length(body) NOT BETWEEN 1 AND 7996 OR char_length(body) > 4000
               OR total_bytes > 7996 OR total_chars > 4000
               OR (p_give->'contents'->child_index->>'count')::numeric <> 1
               OR (p_give->'contents'->child_index->'fluid_subtype') IS NOT NULL
                  AND (p_give->'contents'->child_index->'fluid_subtype') <> 'null'::jsonb
            THEN RETURN FALSE; END IF;
        END LOOP;
    ELSE RETURN FALSE;
    END IF;
    FOR child_index IN 0..jsonb_array_length(p_children)-1 LOOP
        actual := p_children->child_index;
        expected := p_give->'contents'->child_index;
        SELECT value->>'text' INTO body FROM jsonb_array_elements(entries)
            WHERE (value->>'index')::integer = child_index;
        IF (actual->>'ordinal')::numeric IS DISTINCT FROM child_index+1
           OR actual->>'production_key' IS DISTINCT FROM expected->>'item'
           OR (actual->>'quantity')::numeric IS DISTINCT FROM (expected->>'count')::numeric
           OR actual->'text' IS DISTINCT FROM COALESCE(to_jsonb(body),'null'::jsonb)
        THEN RETURN FALSE; END IF;
    END LOOP;
    RETURN TRUE;
EXCEPTION WHEN invalid_text_representation OR numeric_value_out_of_range THEN RETURN FALSE;
END $$;

DO $migration$
DECLARE definition TEXT;
    anchor TEXT := $anchor$m.root_text IS NOT DISTINCT FROM (NEW.plan #>> '{give,text}')$anchor$;
BEGIN
    definition := pg_get_functiondef('game_quest_reward_complete()'::regprocedure);
    IF (length(definition)-length(replace(definition,anchor,'')))/length(anchor) <> 1 THEN
        RAISE EXCEPTION 'child quest reward text requires exact literal MINT guard anchor';
    END IF;
    EXECUTE replace(definition, anchor,
        $proof$game_quest_reward_text_matches(m.root_text,m.contents_plan,NEW.plan->'give')$proof$);
    EXECUTE format('ALTER FUNCTION game_quest_reward_text_matches(TEXT,JSONB,JSONB) SET search_path=pg_catalog,%I,pg_temp', current_schema());
END $migration$;
REVOKE ALL ON FUNCTION game_quest_reward_text_matches(TEXT,JSONB,JSONB) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_quest_reward_text_matches(TEXT,JSONB,JSONB)
    TO oteryn_game_runtime,oteryn_game_control;
