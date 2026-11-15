-- NPC output Text follows the same exact physical child sequence as the MINT owner.
-- Preserve every quantity, identity, text, receipt and source guard; no new grant.
DO $migration$
DECLARE before_row RECORD; after_row RECORD; definition TEXT; prior_source TEXT;
        next_source TEXT := $new_source$
DECLARE spec JSONB := p_give->'text'; entries JSONB := '[]'::jsonb;
        entry JSONB; actual JSONB; expected JSONB; index_value NUMERIC;
        last_index INTEGER := -1; child_index INTEGER; body TEXT;
        total_bytes INTEGER := 0; total_chars INTEGER := 0; outputs JSONB;
BEGIN
    -- The MINT owner places explicit backpack outputs in the same ordered
    -- contents plan; keep Text bound to the exact physical output sequence.
    IF jsonb_typeof(p_give->'contents') IS DISTINCT FROM 'array'
       OR (p_give ? 'backpack_outputs'
           AND jsonb_typeof(p_give->'backpack_outputs') IS DISTINCT FROM 'array')
    THEN RETURN FALSE; END IF;
    outputs := CASE WHEN coalesce(jsonb_array_length(p_give->'backpack_outputs'),0)>0
        THEN p_give->'backpack_outputs' ELSE p_give->'contents' END;
    IF jsonb_typeof(p_children) IS DISTINCT FROM 'array'
       OR jsonb_typeof(outputs) IS DISTINCT FROM 'array'
       OR jsonb_array_length(p_children) <> jsonb_array_length(outputs)
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
               OR (outputs->child_index->>'count')::numeric <> 1
               OR (outputs->child_index->'fluid_subtype') IS NOT NULL
                  AND (outputs->child_index->'fluid_subtype') <> 'null'::jsonb
            THEN RETURN FALSE; END IF;
        END LOOP;
    ELSE RETURN FALSE;
    END IF;
    FOR child_index IN 0..jsonb_array_length(p_children)-1 LOOP
        actual := p_children->child_index;
        expected := outputs->child_index;
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
END $new_source$;
BEGIN
    SELECT oid,proowner,prolang,provolatile,proisstrict,prosecdef,proleakproof,
           proparallel,proconfig,proacl,prosrc INTO STRICT before_row
      FROM pg_proc WHERE oid='game_quest_reward_text_matches(TEXT,JSONB,JSONB)'::regprocedure;
    IF encode(sha256(convert_to(before_row.prosrc,'UTF8')),'hex')<>'a874284f2ce950f41645f8c9ea259f55c48358cb53255568e0e5165ad34adfea' THEN
        RAISE EXCEPTION 'unexpected quest reward Text predecessor';
    END IF;
    definition := pg_get_functiondef(before_row.oid);
    prior_source := before_row.prosrc;
    IF (length(definition)-length(replace(definition,prior_source,'')))/length(prior_source)<>1 THEN
        RAISE EXCEPTION 'ambiguous quest reward Text definition';
    END IF;
    EXECUTE replace(definition,prior_source,next_source);
    SELECT oid,proowner,prolang,provolatile,proisstrict,prosecdef,proleakproof,
           proparallel,proconfig,proacl,prosrc INTO STRICT after_row
      FROM pg_proc WHERE oid='game_quest_reward_text_matches(TEXT,JSONB,JSONB)'::regprocedure;
    IF (to_jsonb(after_row)-'prosrc') IS DISTINCT FROM (to_jsonb(before_row)-'prosrc')
       OR encode(sha256(convert_to(after_row.prosrc,'UTF8')),'hex')<>'4207a533a80cc93acb59e20386c04f12d8756b564ae3b9a3cdd39d8efae2cd76' THEN
        RAISE EXCEPTION 'quest reward Text metadata or source changed unexpectedly';
    END IF;
END $migration$;
