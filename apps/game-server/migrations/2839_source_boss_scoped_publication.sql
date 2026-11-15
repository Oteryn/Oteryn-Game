-- Replace only the four reviewed source bodies using their ACTUAL installed declarations.
-- Preserve complete current pg_proc metadata, including every GUC/config and ACL.
-- Accepted historical migrations remain immutable.

-- qualified_source_function: game_primal_boss_catalogue_activate
DO $migration$
DECLARE signature REGPROCEDURE := 'game_primal_boss_catalogue_activate(UUID,UUID,NUMERIC,UUID,NUMERIC,BYTEA,NUMERIC,BYTEA,BYTEA,TEXT)'::regprocedure;
        before_row JSONB; after_row JSONB; old_source TEXT; definition TEXT;
        next_source TEXT := $source_game_primal_boss_catalogue_activate$
DECLARE current_sequence NUMERIC; current_server BYTEA; current_client BYTEA; current_frame BYTEA; scope BYTEA;
BEGIN
    IF NOT game_runtime_scope_assignment_history_valid()
        OR NOT game_node_prove_current_incarnation(p_node,p_registration,p_secret)
        OR octet_length(p_server)<>32 OR octet_length(p_client)<>32
        OR octet_length(p_content_revision) NOT BETWEEN 1 AND 128 THEN RETURN false; END IF;
    scope:=decode('01','hex')||uuid_send(p_world)||uuid_send(p_channel);
    PERFORM 1 FROM game_runtime_scope_assignments a JOIN game_durability_admission_runtime_guards g ON g.scope_key=scope
        WHERE a.scope_key=scope AND a.scope_kind=1 AND a.world_id=p_world AND a.channel_id=p_channel
            AND a.activity_instance_id IS NULL AND a.house_key IS NULL AND a.state=1
            AND a.ownership_generation=p_generation AND a.holder_node_id=p_node AND a.holder_registration_revision=p_registration
            AND g.ready AND g.ownership_generation=p_generation FOR SHARE OF a,g;
    IF NOT FOUND THEN RETURN false; END IF;
    SELECT activation_sequence,server_artifact_digest,client_artifact_digest,frame_binding_digest INTO current_sequence,current_server,current_client,current_frame
        FROM game_content_activations WHERE world_id=p_world AND channel_id=p_channel
        ORDER BY activation_sequence DESC LIMIT 1 FOR SHARE;
    IF NOT FOUND OR current_sequence<>p_activation OR current_server<>p_server OR current_client<>p_client THEN RETURN false; END IF;
    PERFORM pg_advisory_xact_lock(hashtextextended('oteryn:boss.the_primal_menace',985));
    -- Preserve the live interpretation fence without consulting old global metadata.
    IF EXISTS(SELECT 1 FROM game_activity_handoff_groups
        WHERE world_id=p_world AND activity_key='oteryn:boss.the_primal_menace' AND state=2
            AND content_revision<>p_content_revision)
    THEN RETURN false; END IF;
    -- This is validation of the existing qualified publisher's exact immutable
    -- interpretation, not permission to write/replace a legacy global recipe.
    RETURN EXISTS(SELECT 1 FROM game_boss_room_publications p
        WHERE p.world_id=p_world AND p.boss_key='oteryn:boss.the_primal_menace'
            AND p.content_revision=p_content_revision AND p.enabled
            AND p.server_artifact_digest=p_server
            AND p.activation_frame_digest=current_frame);
END $source_game_primal_boss_catalogue_activate$;
BEGIN
    SELECT to_jsonb(p),p.prosrc,pg_get_functiondef(p.oid)
      INTO STRICT before_row,old_source,definition FROM pg_proc p WHERE p.oid=signature;
    IF encode(sha256(convert_to(old_source,'UTF8')),'hex')<>'5e04e46934a15b4bfb53548721f6de56c5884cb095dd63ff3154565cb39d2943' THEN
        RAISE EXCEPTION 'unexpected game_primal_boss_catalogue_activate source predecessor';
    END IF;
    IF (length(definition)-length(replace(definition,old_source,'')))/length(old_source)<>1 THEN
        RAISE EXCEPTION 'ambiguous game_primal_boss_catalogue_activate definition';
    END IF;
    EXECUTE replace(definition,old_source,next_source);
    SELECT to_jsonb(p) INTO STRICT after_row FROM pg_proc p WHERE p.oid=signature;
    IF (before_row-'prosrc') IS DISTINCT FROM (after_row-'prosrc')
       OR after_row->>'prosrc' IS DISTINCT FROM next_source THEN
        RAISE EXCEPTION 'game_primal_boss_catalogue_activate metadata or reviewed source changed unexpectedly';
    END IF;
END $migration$;

-- qualified_source_function: game_soul_war_boss_catalogue_activate
DO $migration$
DECLARE signature REGPROCEDURE := 'game_soul_war_boss_catalogue_activate(UUID,UUID,NUMERIC,UUID,NUMERIC,BYTEA,NUMERIC,BYTEA,BYTEA,TEXT,TEXT)'::regprocedure;
        before_row JSONB; after_row JSONB; old_source TEXT; definition TEXT;
        next_source TEXT := $source_game_soul_war_boss_catalogue_activate$
DECLARE current_sequence NUMERIC; current_server BYTEA; current_client BYTEA; current_frame BYTEA; scope BYTEA; recipe JSONB;
BEGIN
    recipe:=CASE p_boss_key
        WHEN 'oteryn:boss.goshnar_s_spite' THEN '{"creature":"oteryn:creature.goshnar_s_spite","cooldown":72000,"levers":[{"x":33774,"y":31634,"floor":14},{"x":33775,"y":31634,"floor":14},{"x":33776,"y":31634,"floor":14},{"x":33777,"y":31634,"floor":14},{"x":33778,"y":31634,"floor":14}],"entries":[{"x":33742,"y":31639,"floor":14},{"x":33742,"y":31639,"floor":14},{"x":33742,"y":31639,"floor":14},{"x":33742,"y":31639,"floor":14},{"x":33742,"y":31639,"floor":14}],"exit":{"x":33621,"y":31427,"floor":10},"boss":{"x":33743,"y":31632,"floor":14},"template":"oteryn:room.soul-war-spite-source"}'::jsonb
        WHEN 'oteryn:boss.goshnar_s_malice' THEN '{"creature":"oteryn:creature.goshnar_s_malice","cooldown":72000,"levers":[{"x":33679,"y":31599,"floor":14},{"x":33680,"y":31599,"floor":14},{"x":33681,"y":31599,"floor":14},{"x":33682,"y":31599,"floor":14},{"x":33683,"y":31599,"floor":14}],"entries":[{"x":33710,"y":31605,"floor":14},{"x":33710,"y":31605,"floor":14},{"x":33710,"y":31605,"floor":14},{"x":33710,"y":31605,"floor":14},{"x":33710,"y":31605,"floor":14}],"exit":{"x":33621,"y":31427,"floor":10},"boss":{"x":33709,"y":31599,"floor":14},"template":"oteryn:room.soul-war-malice-source"}'::jsonb
        WHEN 'oteryn:boss.goshnar_s_greed' THEN '{"creature":"oteryn:creature.goshnar_s_greed","cooldown":72000,"levers":[{"x":33776,"y":31665,"floor":14},{"x":33777,"y":31665,"floor":14},{"x":33778,"y":31665,"floor":14},{"x":33779,"y":31665,"floor":14},{"x":33780,"y":31665,"floor":14}],"entries":[{"x":33747,"y":31671,"floor":14},{"x":33747,"y":31671,"floor":14},{"x":33747,"y":31671,"floor":14},{"x":33747,"y":31671,"floor":14},{"x":33747,"y":31671,"floor":14}],"exit":{"x":33621,"y":31427,"floor":10},"boss":{"x":33746,"y":31666,"floor":14},"template":"oteryn:room.soul-war-greed-source"}'::jsonb
        WHEN 'oteryn:boss.goshnar_s_hatred' THEN '{"creature":"oteryn:creature.goshnar_s_hatred","cooldown":72000,"levers":[{"x":33773,"y":31601,"floor":14},{"x":33774,"y":31601,"floor":14},{"x":33775,"y":31601,"floor":14},{"x":33776,"y":31601,"floor":14},{"x":33777,"y":31601,"floor":14}],"entries":[{"x":33743,"y":31604,"floor":14},{"x":33743,"y":31604,"floor":14},{"x":33743,"y":31604,"floor":14},{"x":33743,"y":31604,"floor":14},{"x":33743,"y":31604,"floor":14}],"exit":{"x":33621,"y":31427,"floor":10},"boss":{"x":33744,"y":31599,"floor":14},"template":"oteryn:room.soul-war-hatred-source"}'::jsonb
        WHEN 'oteryn:boss.goshnar_s_cruelty' THEN '{"creature":"oteryn:creature.goshnar_s_cruelty","cooldown":72000,"levers":[{"x":33854,"y":31854,"floor":6},{"x":33855,"y":31854,"floor":6},{"x":33856,"y":31854,"floor":6},{"x":33857,"y":31854,"floor":6},{"x":33858,"y":31854,"floor":6}],"entries":[{"x":33856,"y":31872,"floor":7},{"x":33856,"y":31872,"floor":7},{"x":33856,"y":31872,"floor":7},{"x":33856,"y":31872,"floor":7},{"x":33856,"y":31872,"floor":7}],"exit":{"x":33621,"y":31427,"floor":10},"boss":{"x":33856,"y":31866,"floor":7},"template":"oteryn:room.soul-war-cruelty-source"}'::jsonb
        WHEN 'oteryn:boss.goshnar_s_megalomania' THEN '{"creature":"oteryn:creature.goshnar_s_megalomania_purple","cooldown":244800,"levers":[{"x":33676,"y":31634,"floor":14},{"x":33677,"y":31634,"floor":14},{"x":33678,"y":31634,"floor":14},{"x":33679,"y":31634,"floor":14},{"x":33680,"y":31634,"floor":14}],"entries":[{"x":33710,"y":31639,"floor":14},{"x":33710,"y":31639,"floor":14},{"x":33710,"y":31639,"floor":14},{"x":33710,"y":31639,"floor":14},{"x":33710,"y":31639,"floor":14}],"exit":{"x":33621,"y":31427,"floor":10},"boss":{"x":33710,"y":31634,"floor":14},"template":"oteryn:room.soul-war-megalomania-source"}'::jsonb
        ELSE NULL END;
    IF recipe IS NULL THEN RETURN false; END IF;
    IF p_world IS NULL OR p_channel IS NULL OR p_generation IS NULL OR p_generation<=0
        OR p_node IS NULL OR p_registration IS NULL OR p_registration<=0 OR p_secret IS NULL
        OR p_activation IS NULL OR p_activation<=0 OR p_server IS NULL OR p_client IS NULL
        OR p_content_revision IS NULL
        OR NOT game_runtime_scope_assignment_history_valid()
        OR NOT game_node_prove_current_incarnation(p_node,p_registration,p_secret)
        OR octet_length(p_server)<>32 OR octet_length(p_client)<>32
        OR octet_length(p_content_revision) NOT BETWEEN 1 AND 128 THEN RETURN false; END IF;
    scope:=decode('01','hex')||uuid_send(p_world)||uuid_send(p_channel);
    PERFORM 1 FROM game_runtime_scope_assignments a JOIN game_durability_admission_runtime_guards g ON g.scope_key=scope
        WHERE a.scope_key=scope AND a.scope_kind=1 AND a.world_id=p_world AND a.channel_id=p_channel
            AND a.activity_instance_id IS NULL AND a.house_key IS NULL AND a.state=1
            AND a.ownership_generation=p_generation AND a.holder_node_id=p_node AND a.holder_registration_revision=p_registration
            AND g.ready AND g.ownership_generation=p_generation FOR SHARE OF a,g;
    IF NOT FOUND THEN RETURN false; END IF;
    SELECT activation_sequence,server_artifact_digest,client_artifact_digest,frame_binding_digest INTO current_sequence,current_server,current_client,current_frame
        FROM game_content_activations WHERE world_id=p_world AND channel_id=p_channel
        ORDER BY activation_sequence DESC LIMIT 1 FOR SHARE;
    IF NOT FOUND OR current_sequence<>p_activation OR current_server<>p_server OR current_client<>p_client THEN RETURN false; END IF;
    PERFORM pg_advisory_xact_lock(hashtextextended(p_boss_key,985));
    -- Preserve the live interpretation fence without consulting old global metadata.
    IF EXISTS(SELECT 1 FROM game_activity_handoff_groups
        WHERE world_id=p_world AND activity_key=p_boss_key AND state=2
            AND content_revision<>p_content_revision)
    THEN RETURN false; END IF;
    -- This is validation of the existing qualified publisher's exact immutable
    -- interpretation, not permission to write/replace a legacy global recipe.
    RETURN EXISTS(SELECT 1 FROM game_boss_room_publications p
        WHERE p.world_id=p_world AND p.boss_key=p_boss_key
            AND p.content_revision=p_content_revision AND p.enabled
            AND p.server_artifact_digest=p_server
            AND p.activation_frame_digest=current_frame);
END $source_game_soul_war_boss_catalogue_activate$;
BEGIN
    SELECT to_jsonb(p),p.prosrc,pg_get_functiondef(p.oid)
      INTO STRICT before_row,old_source,definition FROM pg_proc p WHERE p.oid=signature;
    IF encode(sha256(convert_to(old_source,'UTF8')),'hex')<>'7ff094e020580947a432731687cec638226e5b1ee9e0939f024675df398d9efa' THEN
        RAISE EXCEPTION 'unexpected game_soul_war_boss_catalogue_activate source predecessor';
    END IF;
    IF (length(definition)-length(replace(definition,old_source,'')))/length(old_source)<>1 THEN
        RAISE EXCEPTION 'ambiguous game_soul_war_boss_catalogue_activate definition';
    END IF;
    EXECUTE replace(definition,old_source,next_source);
    SELECT to_jsonb(p) INTO STRICT after_row FROM pg_proc p WHERE p.oid=signature;
    IF (before_row-'prosrc') IS DISTINCT FROM (after_row-'prosrc')
       OR after_row->>'prosrc' IS DISTINCT FROM next_source THEN
        RAISE EXCEPTION 'game_soul_war_boss_catalogue_activate metadata or reviewed source changed unexpectedly';
    END IF;
END $migration$;

-- qualified_source_function: game_rotten_blood_boss_catalogue_activate
DO $migration$
DECLARE signature REGPROCEDURE := 'game_rotten_blood_boss_catalogue_activate(TEXT,UUID,UUID,NUMERIC,UUID,NUMERIC,BYTEA,NUMERIC,BYTEA,BYTEA,TEXT)'::regprocedure;
        before_row JSONB; after_row JSONB; old_source TEXT; definition TEXT;
        next_source TEXT := $source_game_rotten_blood_boss_catalogue_activate$
DECLARE current_sequence NUMERIC; current_server BYTEA; current_client BYTEA; current_frame BYTEA; scope BYTEA; recipe RECORD;
BEGIN
    IF NOT game_runtime_scope_assignment_history_valid()
        OR NOT game_node_prove_current_incarnation(p_node,p_registration,p_secret)
        OR octet_length(p_server)<>32 OR octet_length(p_client)<>32
        OR octet_length(p_content_revision) NOT BETWEEN 1 AND 128 THEN RETURN false; END IF;
    IF p_boss NOT IN ('oteryn:boss.ichgahal','oteryn:boss.murcion','oteryn:boss.vemiath','oteryn:boss.chagorz','oteryn:boss.bakragore') THEN RETURN false; END IF;
    scope:=decode('01','hex')||uuid_send(p_world)||uuid_send(p_channel);
    PERFORM 1 FROM game_runtime_scope_assignments a JOIN game_durability_admission_runtime_guards g ON g.scope_key=scope
        WHERE a.scope_key=scope AND a.scope_kind=1 AND a.world_id=p_world AND a.channel_id=p_channel
            AND a.activity_instance_id IS NULL AND a.house_key IS NULL AND a.state=1
            AND a.ownership_generation=p_generation AND a.holder_node_id=p_node AND a.holder_registration_revision=p_registration
            AND g.ready AND g.ownership_generation=p_generation FOR SHARE OF a,g;
    IF NOT FOUND THEN RETURN false; END IF;
    SELECT activation_sequence,server_artifact_digest,client_artifact_digest,frame_binding_digest INTO current_sequence,current_server,current_client,current_frame
        FROM game_content_activations WHERE world_id=p_world AND channel_id=p_channel
        ORDER BY activation_sequence DESC LIMIT 1 FOR SHARE;
    IF NOT FOUND OR current_sequence<>p_activation OR current_server<>p_server OR current_client<>p_client THEN RETURN false; END IF;
    PERFORM pg_advisory_xact_lock(hashtextextended(p_boss,985));
    -- Preserve the live interpretation fence without consulting old global metadata.
    IF EXISTS(SELECT 1 FROM game_activity_handoff_groups
        WHERE world_id=p_world AND activity_key=p_boss AND state=2
            AND content_revision<>p_content_revision)
    THEN RETURN false; END IF;
    -- This is validation of the existing qualified publisher's exact immutable
    -- interpretation, not permission to write/replace a legacy global recipe.
    RETURN EXISTS(SELECT 1 FROM game_boss_room_publications p
        WHERE p.world_id=p_world AND p.boss_key=p_boss
            AND p.content_revision=p_content_revision AND p.enabled
            AND p.server_artifact_digest=p_server
            AND p.activation_frame_digest=current_frame);
END $source_game_rotten_blood_boss_catalogue_activate$;
BEGIN
    SELECT to_jsonb(p),p.prosrc,pg_get_functiondef(p.oid)
      INTO STRICT before_row,old_source,definition FROM pg_proc p WHERE p.oid=signature;
    IF encode(sha256(convert_to(old_source,'UTF8')),'hex')<>'c42e8cd4e07577f1d479ec85e8a683d3f4803df84d8156d324c62533e46abaa5' THEN
        RAISE EXCEPTION 'unexpected game_rotten_blood_boss_catalogue_activate source predecessor';
    END IF;
    IF (length(definition)-length(replace(definition,old_source,'')))/length(old_source)<>1 THEN
        RAISE EXCEPTION 'ambiguous game_rotten_blood_boss_catalogue_activate definition';
    END IF;
    EXECUTE replace(definition,old_source,next_source);
    SELECT to_jsonb(p) INTO STRICT after_row FROM pg_proc p WHERE p.oid=signature;
    IF (before_row-'prosrc') IS DISTINCT FROM (after_row-'prosrc')
       OR after_row->>'prosrc' IS DISTINCT FROM next_source THEN
        RAISE EXCEPTION 'game_rotten_blood_boss_catalogue_activate metadata or reviewed source changed unexpectedly';
    END IF;
END $migration$;

-- qualified_source_function: game_outburst_boss_catalogue_activate
DO $migration$
DECLARE signature REGPROCEDURE := 'game_outburst_boss_catalogue_activate(TEXT,UUID,UUID,NUMERIC,UUID,NUMERIC,BYTEA,NUMERIC,BYTEA,BYTEA,TEXT)'::regprocedure;
        before_row JSONB; after_row JSONB; old_source TEXT; definition TEXT;
        next_source TEXT := $source_game_outburst_boss_catalogue_activate$
DECLARE current_sequence NUMERIC; current_server BYTEA; current_client BYTEA; current_frame BYTEA; scope BYTEA; recipe RECORD;
BEGIN
    IF NOT game_runtime_scope_assignment_history_valid()
        OR NOT game_node_prove_current_incarnation(p_node,p_registration,p_secret)
        OR octet_length(p_server)<>32 OR octet_length(p_client)<>32
        OR octet_length(p_content_revision) NOT BETWEEN 1 AND 128 THEN RETURN false; END IF;
    IF p_boss<>'oteryn:boss.outburst' THEN RETURN false; END IF;
    scope:=decode('01','hex')||uuid_send(p_world)||uuid_send(p_channel);
    PERFORM 1 FROM game_runtime_scope_assignments a JOIN game_durability_admission_runtime_guards g ON g.scope_key=scope
        WHERE a.scope_key=scope AND a.scope_kind=1 AND a.world_id=p_world AND a.channel_id=p_channel
            AND a.activity_instance_id IS NULL AND a.house_key IS NULL AND a.state=1
            AND a.ownership_generation=p_generation AND a.holder_node_id=p_node AND a.holder_registration_revision=p_registration
            AND g.ready AND g.ownership_generation=p_generation FOR SHARE OF a,g;
    IF NOT FOUND THEN RETURN false; END IF;
    SELECT activation_sequence,server_artifact_digest,client_artifact_digest,frame_binding_digest INTO current_sequence,current_server,current_client,current_frame
        FROM game_content_activations WHERE world_id=p_world AND channel_id=p_channel
        ORDER BY activation_sequence DESC LIMIT 1 FOR SHARE;
    IF NOT FOUND OR current_sequence<>p_activation OR current_server<>p_server OR current_client<>p_client THEN RETURN false; END IF;
    PERFORM pg_advisory_xact_lock(hashtextextended(p_boss,985));
    -- Preserve the live interpretation fence without consulting old global metadata.
    IF EXISTS(SELECT 1 FROM game_activity_handoff_groups
        WHERE world_id=p_world AND activity_key=p_boss AND state=2
            AND content_revision<>p_content_revision)
    THEN RETURN false; END IF;
    -- This is validation of the existing qualified publisher's exact immutable
    -- interpretation, not permission to write/replace a legacy global recipe.
    RETURN EXISTS(SELECT 1 FROM game_boss_room_publications p
        WHERE p.world_id=p_world AND p.boss_key=p_boss
            AND p.content_revision=p_content_revision AND p.enabled
            AND p.server_artifact_digest=p_server
            AND p.activation_frame_digest=current_frame);
END $source_game_outburst_boss_catalogue_activate$;
BEGIN
    SELECT to_jsonb(p),p.prosrc,pg_get_functiondef(p.oid)
      INTO STRICT before_row,old_source,definition FROM pg_proc p WHERE p.oid=signature;
    IF encode(sha256(convert_to(old_source,'UTF8')),'hex')<>'b8989c64e28768770ac0ebd23da086fac0584d49361a5816439a47e5b68868e2' THEN
        RAISE EXCEPTION 'unexpected game_outburst_boss_catalogue_activate source predecessor';
    END IF;
    IF (length(definition)-length(replace(definition,old_source,'')))/length(old_source)<>1 THEN
        RAISE EXCEPTION 'ambiguous game_outburst_boss_catalogue_activate definition';
    END IF;
    EXECUTE replace(definition,old_source,next_source);
    SELECT to_jsonb(p) INTO STRICT after_row FROM pg_proc p WHERE p.oid=signature;
    IF (before_row-'prosrc') IS DISTINCT FROM (after_row-'prosrc')
       OR after_row->>'prosrc' IS DISTINCT FROM next_source THEN
        RAISE EXCEPTION 'game_outburst_boss_catalogue_activate metadata or reviewed source changed unexpectedly';
    END IF;
END $migration$;
