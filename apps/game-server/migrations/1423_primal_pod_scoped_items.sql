-- SRX-3 W10: extend the existing physical Item owner with exact tagged scope.
-- Player Ground and custody relocation remain Channel-only; the exact carried
-- ranged-consumption sibling below retains its actual Instance tag. No origin
-- Channel stands in for an Instance; the original custody/audit chain remains.
ALTER TABLE game_spell_item_lines
    ADD COLUMN runtime_scope_kind SMALLINT NOT NULL DEFAULT 1,
    ADD COLUMN instance_id UUID,
    ALTER COLUMN channel_id DROP NOT NULL;
ALTER TABLE game_spell_item_lines ADD CONSTRAINT game_spell_item_line_exact_scope CHECK ((
    (runtime_scope_kind=1 AND channel_id IS NOT NULL AND instance_id IS NULL)
    OR
    (runtime_scope_kind=2 AND channel_id IS NULL AND instance_id IS NOT NULL
        AND game_node_is_uuid_v7(instance_id)
        AND operation_kind IN (1,2)
        AND destination_parent_item_instance_id IS NULL
        AND source_parent_item_instance_id IS NULL
        AND source_equipment_slot IS NULL)
) IS TRUE);

-- A Pod is an Encounter callback, never a fabricated spell/Ability or a
-- creature-death loot cause. The source class admits one scoped Ground Item.
ALTER TABLE game_spell_item_receipts
    DROP CONSTRAINT game_spell_item_receipts_cause_kind_check,
    ADD CONSTRAINT game_spell_item_receipts_cause_kind_check CHECK (cause_kind IN(0,1,2));
DO $$
DECLARE typed TEXT; scoped TEXT; arm TEXT;
BEGIN
    SELECT pg_get_constraintdef(oid) INTO STRICT typed FROM pg_constraint
        WHERE conrelid='game_spell_item_receipts'::regclass AND conname='game_spell_item_typed_cause';
    SELECT pg_get_constraintdef(oid) INTO STRICT scoped FROM pg_constraint
        WHERE conrelid='game_spell_item_receipts'::regclass AND conname='game_spell_item_exact_runtime_scope';
    arm:=$predicate$(cause_kind=2 AND spell_family='Encounter'
        AND spell_production_key='oteryn:encounter.the_primal_menace' AND spell_revision='definition-r1'
        AND creature_definition_key='oteryn:creature.the_primal_menace' AND creature_definition_revision='definition-r1'
        AND runtime_scope_kind=2 AND channel_id IS NULL AND instance_id IS NOT NULL
        AND game_node_is_uuid_v7(instance_id) AND operation_count=1 AND swing_ordinal IS NULL
        AND game_session_id IS NULL AND command_id IS NULL AND character_id IS NULL
        AND cost IS NULL AND cost_binding IS NULL AND caster_lease_generation IS NULL AND caster_placement_digest IS NULL
        AND octet_length(creature_placement)=16 AND creature_placement<>decode(repeat('00',16),'hex')
        AND creature_actor_generation BETWEEN 1 AND 18446744073709551615
        AND octet_length(creature_occurrence) BETWEEN 1 AND 1024
        AND octet_length(creature_map_digest)=32 AND octet_length(creature_frame_digest)=32
        AND octet_length(creature_source_body_digest)=32 AND octet_length(creature_cast_digest)=32
        AND convert_from(intent,'UTF8')::jsonb->>'schema'='OTERYN_ENCOUNTER_GROUND_CAUSE/v2'
        AND convert_from(intent,'UTF8')::jsonb->>'runtime_scope_kind'='2'
        AND convert_from(intent,'UTF8')::jsonb->'channel'='null'::jsonb
        AND convert_from(intent,'UTF8')::jsonb->>'instance'=replace(instance_id::text,'-',''))$predicate$;
    IF left(typed,6)<>'CHECK ' OR left(scoped,6)<>'CHECK ' THEN
        RAISE EXCEPTION 'unexpected source receipt constraint form';
    END IF;
    ALTER TABLE game_spell_item_receipts DROP CONSTRAINT game_spell_item_typed_cause;
    ALTER TABLE game_spell_item_receipts DROP CONSTRAINT game_spell_item_exact_runtime_scope;
    EXECUTE 'ALTER TABLE game_spell_item_receipts ADD CONSTRAINT game_spell_item_typed_cause CHECK (('
        ||substr(typed,7)||') OR ('||arm||') IS TRUE)';
    EXECUTE 'ALTER TABLE game_spell_item_receipts ADD CONSTRAINT game_spell_item_exact_runtime_scope CHECK (('
        ||substr(scoped,7)||') OR ('||arm||') IS TRUE)';
END $$;
CREATE UNIQUE INDEX game_spell_item_encounter_once ON game_spell_item_receipts
    (world_id,instance_id,ownership_generation,creature_placement,creature_actor_generation,
     spell_production_key,spell_revision,creature_occurrence) WHERE cause_kind=2;

-- Extend the source discriminator in place. Every existing source-body,
-- occurrence, map, frame, current-XID, operation and audit predicate still runs.
DO $$
DECLARE body TEXT; amended TEXT;
BEGIN
    body:=pg_get_functiondef('game_spell_creature_cause_proven()'::regprocedure);
    amended:=replace(body,'IF NEW.cause_kind<>1 THEN RETURN NULL; END IF;',
        'IF NEW.cause_kind NOT IN(1,2) THEN RETURN NULL; END IF;');
    amended:=replace(amended,
        'p->>''schema'' IS DISTINCT FROM ''OTERYN_CREATURE_GROUND_CAUSE/v1''',
        'p->>''schema'' IS DISTINCT FROM (CASE NEW.cause_kind WHEN 1 THEN ''OTERYN_CREATURE_GROUND_CAUSE/v1'' ELSE ''OTERYN_ENCOUNTER_GROUND_CAUSE/v2'' END)');
    IF amended=body OR strpos(amended,'NEW.cause_kind NOT IN(1,2)')=0 THEN
        RAISE EXCEPTION 'source cause trigger form changed';
    END IF;
    EXECUTE amended;
END $$;

CREATE FUNCTION game_primal_pod_source_proven() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.cause_kind<>2 THEN RETURN NULL; END IF;
    IF EXISTS(SELECT 1 FROM game_spell_item_lines l WHERE l.transaction_id=NEW.transaction_id
        AND (l.runtime_scope_kind<>2 OR l.channel_id IS NOT NULL OR l.instance_id IS DISTINCT FROM NEW.instance_id
            OR l.world_id<>NEW.world_id OR l.created_xact_id<>pg_current_xact_id()
            OR l.operation_kind NOT IN(1,2) OR l.definition_family<>'Item'
            OR l.definition_production_key<>'oteryn:item.tibia.i39176' OR l.definition_revision<>'definition-r1'
            OR l.destination_parent_item_instance_id IS NOT NULL OR l.source_parent_item_instance_id IS NOT NULL
            OR l.blocks_movement OR l.blocks_projectile OR l.immovable_block_solid OR l.expires_at_unix_ms IS NOT NULL
            OR (l.operation_kind=1 AND (l.quantity_before<>0 OR l.quantity_after<>1))
            OR (l.operation_kind=2 AND (l.quantity_before<>1 OR l.quantity_after<>0))))
       OR (SELECT count(*) FROM game_spell_item_lines l WHERE l.transaction_id=NEW.transaction_id)<>1
    THEN RAISE EXCEPTION 'Primal Pod Encounter source requires its exact scoped physical Item' USING ERRCODE='23514'; END IF;
    RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_primal_pod_source_proven AFTER INSERT ON game_spell_item_receipts
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_primal_pod_source_proven();

-- Reuse the original line/custody successor proofs, replacing only their scope
-- equality operands. coalesce is safe here because both tagged-scope CHECKs
-- independently require exactly one non-null scope identity.
DO $$
DECLARE body TEXT; amended TEXT;
BEGIN
    body:=pg_get_functiondef('game_spell_item_line_proven()'::regprocedure);
    amended:=replace(body,'(NEW.world_id,NEW.channel_id)',
        '(NEW.world_id,NEW.runtime_scope_kind,coalesce(NEW.channel_id,NEW.instance_id))');
    amended:=replace(amended,'(r.world_id,r.channel_id)',
        '(r.world_id,r.runtime_scope_kind,coalesce(r.channel_id,r.instance_id))');
    amended:=replace(amended,'(g.world_id,g.channel_id,g.spatial_position',
        '(g.world_id,g.scope_kind,coalesce(g.channel_id,g.instance_id),g.spatial_position');
    amended:=replace(amended,'(NEW.world_id,NEW.channel_id,NEW.spatial_position',
        '(NEW.world_id,NEW.runtime_scope_kind,coalesce(NEW.channel_id,NEW.instance_id),NEW.spatial_position');
    amended:=replace(amended,'(root.world_id,root.channel_id,root.spatial_position',
        '(root.world_id,root.runtime_scope_kind,coalesce(root.channel_id,root.instance_id),root.spatial_position');
    IF amended=body OR strpos(amended,'(NEW.world_id,NEW.channel_id)')>0
        OR strpos(amended,'(g.world_id,g.channel_id,g.spatial_position')>0 THEN
        RAISE EXCEPTION 'spell physical line scope proof form changed';
    END IF;
    EXECUTE amended;

    body:=pg_get_functiondef('game_item_ground_removal_proven()'::regprocedure);
    amended:=replace(body,'(l.world_id,l.channel_id,l.spatial_position',
        '(l.world_id,l.runtime_scope_kind,coalesce(l.channel_id,l.instance_id),l.spatial_position');
    amended:=replace(amended,'(OLD.world_id,OLD.channel_id,OLD.spatial_position',
        '(OLD.world_id,OLD.scope_kind,coalesce(OLD.channel_id,OLD.instance_id),OLD.spatial_position');
    IF amended=body THEN RAISE EXCEPTION 'source Ground removal scope proof form changed'; END IF;
    EXECUTE amended;
END $$;

-- Join both receipt and line before passing the closed Instance gate, then
-- continue through the full existing physical Item MINT/custody proof chain.
DO $$
DECLARE body TEXT; amended TEXT; old TEXT;
BEGIN
    body:=pg_get_functiondef('game_item_ground_insertion_guard()'::regprocedure);
    old:='RAISE EXCEPTION ''Instance Ground requires its exact accepted placement cause'' USING ERRCODE=''23514'';';
    IF (length(body)-length(replace(body,old,'')))/length(old)<>1 THEN
        RAISE EXCEPTION 'Instance Ground source gate form changed';
    END IF;
    amended:=replace(body,old,$arm$
    IF NOT EXISTS(SELECT 1 FROM game_spell_item_receipts r
        JOIN game_spell_item_lines l USING(transaction_id)
        WHERE r.cause_kind=2 AND r.runtime_scope_kind=2 AND r.channel_id IS NULL
            AND r.world_id=NEW.world_id AND r.instance_id=NEW.instance_id
            AND r.ownership_generation=NEW.runtime_scope_ownership_generation
            AND l.runtime_scope_kind=2 AND l.channel_id IS NULL AND l.instance_id=NEW.instance_id
            AND l.item_instance_id=NEW.item_instance_id AND l.operation_kind=1
            AND r.created_xact_id=pg_current_xact_id() AND l.created_xact_id=pg_current_xact_id()) THEN
        RAISE EXCEPTION 'Instance Ground requires its exact accepted placement cause' USING ERRCODE='23514';
    END IF;
$arm$);
    EXECUTE amended;
END $$;

-- Pod custom attributes and addEvent deadlines belong to the Encounter, not to
-- ItemType decay. Each source callback retains its position until it executes.
CREATE TABLE game_primal_pod_callbacks (
    source_transaction_id UUID PRIMARY KEY REFERENCES game_spell_item_receipts(transaction_id),
    item_instance_id UUID NOT NULL UNIQUE REFERENCES game_item_instances(item_instance_id),
    world_id UUID NOT NULL,
    instance_id UUID NOT NULL CHECK(game_node_is_uuid_v7(instance_id)),
    ownership_generation NUMERIC(20,0) NOT NULL CHECK(ownership_generation BETWEEN 1 AND 18446744073709551615),
    spatial_position BYTEA NOT NULL CHECK(octet_length(spatial_position)=10),
    content_digest BYTEA NOT NULL CHECK(octet_length(content_digest)=32),
    map_digest BYTEA NOT NULL CHECK(octet_length(map_digest)=32),
    frame_digest BYTEA NOT NULL CHECK(octet_length(frame_digest)=32),
    source_timer_unix_ms BIGINT NOT NULL,
    due_unix_ms BIGINT NOT NULL,
    completed_transaction_id UUID REFERENCES game_spell_item_receipts(transaction_id),
    completion_kind SMALLINT NOT NULL DEFAULT 0 CHECK(completion_kind IN(0,2,3)),
    effect_acknowledged BOOLEAN NOT NULL DEFAULT false,
    created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
    CHECK((completion_kind=0 AND completed_transaction_id IS NULL) OR (completion_kind=2 AND completed_transaction_id IS NOT NULL) OR (completion_kind=3 AND completed_transaction_id IS NULL)),
    CHECK(source_timer_unix_ms>=0 AND source_timer_unix_ms%1000=0 AND due_unix_ms>source_timer_unix_ms)
);
CREATE INDEX game_primal_pod_callbacks_due ON game_primal_pod_callbacks(world_id,instance_id,due_unix_ms)
    WHERE completion_kind=0 OR (completion_kind=2 AND NOT effect_acknowledged);
REVOKE ALL ON game_primal_pod_callbacks FROM PUBLIC;
GRANT SELECT,INSERT,UPDATE ON game_primal_pod_callbacks TO oteryn_game_runtime;
CREATE FUNCTION game_primal_pod_callback_proven() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP='UPDATE' THEN
        IF OLD.completion_kind=2 AND NEW.completion_kind=2 AND NOT OLD.effect_acknowledged AND NEW.effect_acknowledged
            AND (to_jsonb(NEW)-'effect_acknowledged')=(to_jsonb(OLD)-'effect_acknowledged') THEN
            RETURN NULL;
        END IF;
        IF OLD.completion_kind<>0 OR NEW.completion_kind=0
            OR (to_jsonb(NEW)-ARRAY['completed_transaction_id','completion_kind','effect_acknowledged']) IS DISTINCT FROM (to_jsonb(OLD)-ARRAY['completed_transaction_id','completion_kind','effect_acknowledged'])
        THEN RAISE EXCEPTION 'Pod callback only completes once' USING ERRCODE='23514'; END IF;
        IF NEW.completion_kind=3 THEN
            IF NOT NEW.effect_acknowledged OR floor(extract(epoch FROM statement_timestamp())*1000)::bigint<NEW.due_unix_ms
                OR EXISTS(SELECT 1 FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id)
                    WHERE g.world_id=NEW.world_id AND g.scope_kind=2 AND g.channel_id IS NULL AND g.instance_id=NEW.instance_id
                        AND g.spatial_position=NEW.spatial_position AND g.native_room_placement_context=NEW.frame_digest
                        AND i.lifecycle=1 AND i.definition_production_key='oteryn:item.tibia.i39176')
            THEN RAISE EXCEPTION 'empty Pod callback requires due actual empty position' USING ERRCODE='23514'; END IF;
            RETURN NULL;
        END IF;
        IF NOT EXISTS(SELECT 1 FROM game_spell_item_receipts r JOIN game_spell_item_lines l USING(transaction_id)
            WHERE r.transaction_id=NEW.completed_transaction_id AND r.cause_kind=2
                AND r.world_id=NEW.world_id AND r.runtime_scope_kind=2 AND r.channel_id IS NULL
                AND r.instance_id=NEW.instance_id AND r.ownership_generation=NEW.ownership_generation
                AND r.catalog_digest=NEW.content_digest AND r.creature_map_digest=NEW.map_digest AND r.creature_frame_digest=NEW.frame_digest
                AND l.operation_kind=2 AND l.spatial_position=NEW.spatial_position
                AND r.created_xact_id=pg_current_xact_id() AND l.created_xact_id=pg_current_xact_id()
                AND r.occurred_at_unix_ms>=NEW.due_unix_ms)
        THEN RAISE EXCEPTION 'Pod callback completion requires its due current scoped source removal' USING ERRCODE='23514'; END IF;
        RETURN NULL;
    END IF;
    IF NOT EXISTS(SELECT 1 FROM game_spell_item_receipts r JOIN game_spell_item_lines l USING(transaction_id)
        WHERE r.transaction_id=NEW.source_transaction_id AND r.cause_kind=2
            AND r.world_id=NEW.world_id AND r.runtime_scope_kind=2 AND r.channel_id IS NULL
            AND r.instance_id=NEW.instance_id AND r.ownership_generation=NEW.ownership_generation
            AND r.catalog_digest=NEW.content_digest AND r.creature_map_digest=NEW.map_digest AND r.creature_frame_digest=NEW.frame_digest
            AND l.item_instance_id=NEW.item_instance_id AND l.operation_kind=1 AND l.spatial_position=NEW.spatial_position
            AND NEW.source_timer_unix_ms=(r.occurred_at_unix_ms/1000)*1000 AND NEW.due_unix_ms=r.occurred_at_unix_ms+4000
            AND r.created_xact_id=pg_current_xact_id() AND l.created_xact_id=pg_current_xact_id()
            AND NEW.created_xact_id=pg_current_xact_id())
    THEN RAISE EXCEPTION 'Pod custom timer requires same-XID actual source Item MINT' USING ERRCODE='23514'; END IF;
    RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_primal_pod_callback_proven AFTER INSERT OR UPDATE ON game_primal_pod_callbacks
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_primal_pod_callback_proven();
CREATE UNIQUE INDEX game_primal_pod_player_step_once ON game_spell_item_receipts
    (world_id,instance_id,ownership_generation,creature_cast_digest)
    WHERE cause_kind=2 AND creature_occurrence LIKE 'primal-pod-step:%';

-- Source-qualified Primal catalogue composition. The runtime still has no
-- generic INSERT/UPDATE right on the catalogue. Only this literal pinned recipe
-- can be activated, by the actual current source-Channel Node and content pair.
CREATE FUNCTION game_primal_boss_catalogue_activate(
    p_world UUID,p_channel UUID,p_generation NUMERIC,p_node UUID,p_registration NUMERIC,p_secret BYTEA,
    p_activation NUMERIC,p_server BYTEA,p_client BYTEA,p_content_revision TEXT
) RETURNS BOOLEAN LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
DECLARE current_sequence NUMERIC; current_server BYTEA; current_client BYTEA; scope BYTEA;
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
    SELECT activation_sequence,server_artifact_digest,client_artifact_digest INTO current_sequence,current_server,current_client
        FROM game_content_activations WHERE world_id=p_world AND channel_id=p_channel
        ORDER BY activation_sequence DESC LIMIT 1 FOR SHARE;
    IF NOT FOUND OR current_sequence<>p_activation OR current_server<>p_server OR current_client<>p_client THEN RETURN false; END IF;
    PERFORM pg_advisory_xact_lock(hashtextextended('oteryn:boss.the_primal_menace',985));
    IF EXISTS(SELECT 1 FROM game_boss_room_catalogue WHERE boss_key='oteryn:boss.the_primal_menace'
        AND content_revision<>p_content_revision)
        AND EXISTS(SELECT 1 FROM game_activity_handoff_groups WHERE activity_key='oteryn:boss.the_primal_menace' AND state=2)
    THEN RETURN false; END IF;
    INSERT INTO game_boss_room_catalogue(boss_key,content_revision,boss_creature_key,minimum_players,maximum_players,minimum_level,
        cooldown_seconds,time_limit_seconds,lever_positions,room_template_key,reward_key,enabled,template_revision,entry_positions,
        exit_position,boss_position,after_kill_seconds)
    VALUES('oteryn:boss.the_primal_menace',p_content_revision,'oteryn:creature.the_primal_menace',1,5,500,72000,1200,
        '[{"x":33548,"y":32752,"floor":14},{"x":33549,"y":32752,"floor":14},{"x":33550,"y":32752,"floor":14},{"x":33551,"y":32752,"floor":14},{"x":33552,"y":32752,"floor":14}]'::jsonb,
        'oteryn:room.primal-source','oteryn:boss.the_primal_menace',true,'crystal-201189f',
        '[{"x":33565,"y":32758,"floor":15},{"x":33565,"y":32758,"floor":15},{"x":33565,"y":32758,"floor":15},{"x":33565,"y":32758,"floor":15},{"x":33565,"y":32758,"floor":15}]'::jsonb,
        '{"x":33520,"y":32871,"floor":15}'::jsonb,'{"x":33558,"y":32759,"floor":15}'::jsonb,60)
    ON CONFLICT(boss_key) DO UPDATE SET content_revision=excluded.content_revision,boss_creature_key=excluded.boss_creature_key,
        minimum_players=excluded.minimum_players,maximum_players=excluded.maximum_players,minimum_level=excluded.minimum_level,
        cooldown_seconds=excluded.cooldown_seconds,time_limit_seconds=excluded.time_limit_seconds,lever_positions=excluded.lever_positions,
        room_template_key=excluded.room_template_key,reward_key=excluded.reward_key,enabled=excluded.enabled,
        template_revision=excluded.template_revision,entry_positions=excluded.entry_positions,exit_position=excluded.exit_position,
        boss_position=excluded.boss_position,after_kill_seconds=excluded.after_kill_seconds;
    RETURN true;
END $$;
REVOKE ALL ON FUNCTION game_primal_boss_catalogue_activate(UUID,UUID,NUMERIC,UUID,NUMERIC,BYTEA,NUMERIC,BYTEA,BYTEA,TEXT) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_primal_boss_catalogue_activate(UUID,UUID,NUMERIC,UUID,NUMERIC,BYTEA,NUMERIC,BYTEA,BYTEA,TEXT) TO oteryn_game_runtime;

-- SRX-3 W11: actual scoped native deaths keep Channel52 and Instance(tag2+52)
-- disjoint. All existing matching source/member/current-xact/Character guards
-- remain installed; this only admits the already-canonical tagged DATA encoding.
ALTER TABLE game_character_soul_war_receipts
    DROP CONSTRAINT game_character_soul_war_receipts_source_death_key_check,
    ADD CONSTRAINT game_character_soul_war_receipts_source_death_key_check CHECK (
        CASE
            WHEN source_death_key IS NULL THEN true
            WHEN octet_length(source_death_key)=52 THEN true
            WHEN octet_length(source_death_key)=53 THEN get_byte(source_death_key,0)=2
            ELSE false
        END
    );

-- SRX-3 W17: persist the complete bounded native arena projection before
-- returning it. Existing immutable receipts and all command fences stay intact.
ALTER TABLE game_tibiadrome_receipts
    DROP CONSTRAINT game_tibiadrome_receipts_reply_check,
    ADD CONSTRAINT game_tibiadrome_receipts_reply_check
        CHECK (octet_length(reply) BETWEEN 1 AND 8192);

-- Soulpit staging is owned by merged main1141: runtime/LastPosition floor
-- stays exactly +8; only native TilePos projection negates it (ADR-0021).
-- The older duplicate dual-floor rewrite is superseded by that shared owner.

-- Source fixed recipes: Crystal201189f lib/quests/soul_war.lua:296-477; BossLever defaults20min/60s.
CREATE FUNCTION game_soul_war_boss_catalogue_activate(
    p_world UUID,p_channel UUID,p_generation NUMERIC,p_node UUID,p_registration NUMERIC,p_secret BYTEA,
    p_activation NUMERIC,p_server BYTEA,p_client BYTEA,p_content_revision TEXT,p_boss_key TEXT
) RETURNS BOOLEAN LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public AS $$
DECLARE current_sequence NUMERIC; current_server BYTEA; current_client BYTEA; scope BYTEA; recipe JSONB;
BEGIN
    recipe:=CASE p_boss_key
        WHEN 'oteryn:boss.goshnar_s_spite' THEN '{"creature":"oteryn:creature.goshnar_s_spite","cooldown":72000,"levers":[{"x":33774,"y":31634,"floor":14},{"x":33775,"y":31634,"floor":14},{"x":33776,"y":31634,"floor":14},{"x":33777,"y":31634,"floor":14},{"x":33778,"y":31634,"floor":14}],"entries":[{"x":33742,"y":31639,"floor":14},{"x":33742,"y":31639,"floor":14},{"x":33742,"y":31639,"floor":14},{"x":33742,"y":31639,"floor":14},{"x":33742,"y":31639,"floor":14}],"exit":{"x":33621,"y":31427,"floor":10},"boss":{"x":33743,"y":31632,"floor":14},"template":"oteryn:room.soul-war-spite-source"}'::jsonb
        WHEN 'oteryn:boss.goshnar_s_malice' THEN '{"creature":"oteryn:creature.goshnar_s_malice","cooldown":72000,"levers":[{"x":33679,"y":31599,"floor":14},{"x":33680,"y":31599,"floor":14},{"x":33681,"y":31599,"floor":14},{"x":33682,"y":31599,"floor":14},{"x":33683,"y":31599,"floor":14}],"entries":[{"x":33710,"y":31605,"floor":14},{"x":33710,"y":31605,"floor":14},{"x":33710,"y":31605,"floor":14},{"x":33710,"y":31605,"floor":14},{"x":33710,"y":31605,"floor":14}],"exit":{"x":33621,"y":31427,"floor":10},"boss":{"x":33709,"y":31599,"floor":14},"template":"oteryn:room.soul-war-malice-source"}'::jsonb
        WHEN 'oteryn:boss.goshnar_s_greed' THEN '{"creature":"oteryn:creature.goshnar_s_greed","cooldown":72000,"levers":[{"x":33776,"y":31665,"floor":14},{"x":33777,"y":31665,"floor":14},{"x":33778,"y":31665,"floor":14},{"x":33779,"y":31665,"floor":14},{"x":33780,"y":31665,"floor":14}],"entries":[{"x":33747,"y":31671,"floor":14},{"x":33747,"y":31671,"floor":14},{"x":33747,"y":31671,"floor":14},{"x":33747,"y":31671,"floor":14},{"x":33747,"y":31671,"floor":14}],"exit":{"x":33621,"y":31427,"floor":10},"boss":{"x":33746,"y":31666,"floor":14},"template":"oteryn:room.soul-war-greed-source"}'::jsonb
        WHEN 'oteryn:boss.goshnar_s_hatred' THEN '{"creature":"oteryn:creature.goshnar_s_hatred","cooldown":72000,"levers":[{"x":33773,"y":31601,"floor":14},{"x":33774,"y":31601,"floor":14},{"x":33775,"y":31601,"floor":14},{"x":33776,"y":31601,"floor":14},{"x":33777,"y":31601,"floor":14}],"entries":[{"x":33743,"y":31604,"floor":14},{"x":33743,"y":31604,"floor":14},{"x":33743,"y":31604,"floor":14},{"x":33743,"y":31604,"floor":14},{"x":33743,"y":31604,"floor":14}],"exit":{"x":33621,"y":31427,"floor":10},"boss":{"x":33744,"y":31599,"floor":14},"template":"oteryn:room.soul-war-hatred-source"}'::jsonb
        WHEN 'oteryn:boss.goshnar_s_cruelty' THEN '{"creature":"oteryn:creature.goshnar_s_cruelty","cooldown":72000,"levers":[{"x":33854,"y":31854,"floor":6},{"x":33855,"y":31854,"floor":6},{"x":33856,"y":31854,"floor":6},{"x":33857,"y":31854,"floor":6},{"x":33858,"y":31854,"floor":6}],"entries":[{"x":33856,"y":31872,"floor":7},{"x":33856,"y":31872,"floor":7},{"x":33856,"y":31872,"floor":7},{"x":33856,"y":31872,"floor":7},{"x":33856,"y":31872,"floor":7}],"exit":{"x":33621,"y":31427,"floor":10},"boss":{"x":33856,"y":31866,"floor":7},"template":"oteryn:room.soul-war-cruelty-source"}'::jsonb
        WHEN 'oteryn:boss.goshnar_s_megalomania' THEN '{"creature":"oteryn:creature.goshnar_s_megalomania_purple","cooldown":259200,"levers":[{"x":33676,"y":31634,"floor":14},{"x":33677,"y":31634,"floor":14},{"x":33678,"y":31634,"floor":14},{"x":33679,"y":31634,"floor":14},{"x":33680,"y":31634,"floor":14}],"entries":[{"x":33710,"y":31639,"floor":14},{"x":33710,"y":31639,"floor":14},{"x":33710,"y":31639,"floor":14},{"x":33710,"y":31639,"floor":14},{"x":33710,"y":31639,"floor":14}],"exit":{"x":33621,"y":31427,"floor":10},"boss":{"x":33710,"y":31634,"floor":14},"template":"oteryn:room.soul-war-megalomania-source"}'::jsonb
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
    SELECT activation_sequence,server_artifact_digest,client_artifact_digest INTO current_sequence,current_server,current_client
        FROM game_content_activations WHERE world_id=p_world AND channel_id=p_channel
        ORDER BY activation_sequence DESC LIMIT 1 FOR SHARE;
    IF NOT FOUND OR current_sequence<>p_activation OR current_server<>p_server OR current_client<>p_client THEN RETURN false; END IF;
    PERFORM pg_advisory_xact_lock(hashtextextended(p_boss_key,985));
    IF EXISTS(SELECT 1 FROM game_boss_room_catalogue WHERE boss_key=p_boss_key
        AND content_revision<>p_content_revision)
        AND EXISTS(SELECT 1 FROM game_activity_handoff_groups WHERE activity_key=p_boss_key AND state=2)
    THEN RETURN false; END IF;
    INSERT INTO game_boss_room_catalogue(boss_key,content_revision,boss_creature_key,minimum_players,maximum_players,minimum_level,
        cooldown_seconds,time_limit_seconds,lever_positions,room_template_key,reward_key,enabled,template_revision,entry_positions,
        exit_position,boss_position,after_kill_seconds)
    VALUES(p_boss_key,p_content_revision,recipe->>'creature',1,5,250,
        (recipe->>'cooldown')::BIGINT,1200,recipe->'levers',recipe->>'template',p_boss_key,true,'crystal-201189f',
        recipe->'entries',recipe->'exit',recipe->'boss',60)
    ON CONFLICT(boss_key) DO UPDATE SET content_revision=excluded.content_revision,boss_creature_key=excluded.boss_creature_key,
        minimum_players=excluded.minimum_players,maximum_players=excluded.maximum_players,minimum_level=excluded.minimum_level,
        cooldown_seconds=excluded.cooldown_seconds,time_limit_seconds=excluded.time_limit_seconds,lever_positions=excluded.lever_positions,
        room_template_key=excluded.room_template_key,reward_key=excluded.reward_key,enabled=excluded.enabled,
        template_revision=excluded.template_revision,entry_positions=excluded.entry_positions,exit_position=excluded.exit_position,
        boss_position=excluded.boss_position,after_kill_seconds=excluded.after_kill_seconds;
    RETURN true;
END $$;
REVOKE ALL ON FUNCTION game_soul_war_boss_catalogue_activate(UUID,UUID,NUMERIC,UUID,NUMERIC,BYTEA,NUMERIC,BYTEA,BYTEA,TEXT,TEXT) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_soul_war_boss_catalogue_activate(UUID,UUID,NUMERIC,UUID,NUMERIC,BYTEA,NUMERIC,BYTEA,BYTEA,TEXT,TEXT) TO oteryn_game_runtime;

-- SRX-3: an actual held native ranged swing may consume one already-owned
-- projectile inside its committed activity Instance. No Ground placement,
-- spell/rune consumption, container retirement or other custody route opens.
DO $$
DECLARE prior TEXT; arm TEXT;
BEGIN
    SELECT pg_get_constraintdef(oid) INTO STRICT prior FROM pg_constraint
        WHERE conrelid='game_spell_item_lines'::regclass AND conname='game_spell_item_line_exact_scope';
    IF left(prior,6)<>'CHECK ' THEN RAISE EXCEPTION 'unexpected physical line scope constraint'; END IF;
    arm:=$line$(runtime_scope_kind=2 AND channel_id IS NULL
        AND instance_id IS NOT NULL AND game_node_is_uuid_v7(instance_id)
        AND operation_kind=5 AND quantity_before=quantity_after+1
        AND source_corpse_root IS NULL AND source_entry_is_corpse IS NULL
        AND destination_parent_item_instance_id IS NULL AND destination_ordinal IS NULL
        AND ((source_equipment_slot IN(5,6,10) AND source_equipment_revision BETWEEN 1 AND 18446744073709551614
                AND source_parent_item_instance_id IS NULL AND source_placement_ordinal IS NULL)
            OR (source_equipment_slot IS NULL AND source_equipment_revision IS NULL
                AND source_parent_item_instance_id IS NOT NULL AND game_node_is_uuid_v7(source_parent_item_instance_id)
                AND source_placement_ordinal BETWEEN 1 AND 18446744073709551615)))$line$;
    ALTER TABLE game_spell_item_lines DROP CONSTRAINT game_spell_item_line_exact_scope;
    EXECUTE 'ALTER TABLE game_spell_item_lines ADD CONSTRAINT game_spell_item_line_exact_scope CHECK (('
        ||substr(prior,7)||') OR ('||arm||') IS TRUE)';
    SELECT pg_get_constraintdef(oid) INTO STRICT prior FROM pg_constraint
        WHERE conrelid='game_spell_item_receipts'::regclass AND conname='game_spell_item_exact_runtime_scope';
    IF left(prior,6)<>'CHECK ' THEN RAISE EXCEPTION 'unexpected source receipt scope constraint'; END IF;
    arm:=$receipt$(runtime_scope_kind=2 AND channel_id IS NULL
        AND instance_id IS NOT NULL AND game_node_is_uuid_v7(instance_id)
        AND cause_kind=0 AND spell_family='Item' AND swing_ordinal BETWEEN 0 AND 65535
        AND operation_count=1 AND game_session_id IS NOT NULL AND character_id IS NOT NULL AND command_id IS NOT NULL
        AND caster_lease_generation IS NOT NULL AND caster_placement_digest IS NOT NULL
        AND cost IS NOT NULL AND cost_binding IS NOT NULL AND game_spell_cost_binding_valid(cost)
        AND convert_from(intent,'UTF8')::jsonb->>'version'='2'
        AND convert_from(intent,'UTF8')::jsonb->>'runtime_scope_kind'='2'
        AND convert_from(intent,'UTF8')::jsonb->'channel'='null'::jsonb
        AND convert_from(intent,'UTF8')::jsonb->>'instance'=replace(instance_id::text,'-','')
        AND jsonb_array_length(convert_from(intent,'UTF8')::jsonb->'operations')=1
        AND convert_from(intent,'UTF8')::jsonb->'operations'->0->>'kind'='consume_inventory'
        AND convert_from(intent,'UTF8')::jsonb->'companion'='null'::jsonb
        AND COALESCE(convert_from(intent,'UTF8')::jsonb->'direct_companion','null'::jsonb)='null'::jsonb)$receipt$;
    ALTER TABLE game_spell_item_receipts DROP CONSTRAINT game_spell_item_exact_runtime_scope;
    EXECUTE 'ALTER TABLE game_spell_item_receipts ADD CONSTRAINT game_spell_item_exact_runtime_scope CHECK (('
        ||substr(prior,7)||') OR ('||arm||') IS TRUE)';
END $$;
-- Existing ranged-source proof, current-XID OLD custody capture, exact Item
-- successor, equipment epoch, line census and immutable audit all still run.
CREATE FUNCTION game_instance_ranged_scope_proven() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.runtime_scope_kind<>2 OR NEW.cause_kind<>0 OR NEW.spell_family<>'Item' THEN RETURN NULL; END IF;
    IF NOT COALESCE(game_activity_session_origin_valid(NEW.game_session_id,NEW.character_id),false)
        OR NOT EXISTS(SELECT 1 FROM game_durability_reconnect_sessions s
            WHERE s.game_session_id=NEW.game_session_id AND s.character_id=NEW.character_id
                AND s.world_id=NEW.world_id AND s.runtime_scope_kind=2 AND s.runtime_scope_world_id=NEW.world_id
                AND s.runtime_scope_channel_id IS NULL AND s.runtime_scope_house_key IS NULL
                AND s.runtime_scope_instance_id=NEW.instance_id AND s.session_state IN(1,2)
                AND s.scope_ownership_generation=NEW.ownership_generation
                AND s.character_lease_generation=NEW.caster_lease_generation)
        OR EXISTS(SELECT 1 FROM game_spell_item_lines l WHERE l.transaction_id=NEW.transaction_id
            AND (l.runtime_scope_kind<>2 OR l.channel_id IS NOT NULL OR l.instance_id IS DISTINCT FROM NEW.instance_id
                OR l.world_id<>NEW.world_id OR l.operation_kind<>5 OR l.quantity_before<>l.quantity_after+1))
    THEN RAISE EXCEPTION 'Instance ranged consumption requires its exact current private session and carried projectile' USING ERRCODE='23514'; END IF;
    RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_instance_ranged_scope_proven AFTER INSERT ON game_spell_item_receipts
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_instance_ranged_scope_proven();

-- SRX-3 W17: canonical job keeps its main grants. Retire any obsolete
-- Character-projection grant here, before later migration consumers run.
REVOKE SELECT ON game_character_tibiadrome FROM oteryn_game_highscore_job;
-- No lifetime counter backfill: a missing legacy JSON field remains explicitly unknown.

-- SRX-3 W11: fixed STEP-triggered Soul War Encounters are not XML chance,
-- adaptive Lua chance or calendar callbacks. The existing Raid owner keeps
-- their firing/run/placement receipts; only these literal source keys use4.
ALTER TABLE public.game_world_raid_schedule_sources
    DROP CONSTRAINT game_world_raid_schedule_sources_source_kind_check;
ALTER TABLE public.game_world_raid_schedule_sources
    ADD CONSTRAINT game_world_raid_schedule_sources_source_kind_check CHECK (
        source_kind IN (1, 2, 3)
        OR (source_kind = 4 AND raid_key IN (
            'raid:soul-war.claustrophobic-inferno-1',
            'raid:soul-war.claustrophobic-inferno-2',
            'raid:soul-war.claustrophobic-inferno-3'
        ))
    );

-- Source class 3 is a real Rotten Blood CreatureEvent, not a spell or death.
-- Crystal vemiath/chagorz/bakragore onThink: current 37000/action50001.
ALTER TABLE game_spell_item_receipts DROP CONSTRAINT game_spell_item_receipts_cause_kind_check,
 ADD CONSTRAINT game_spell_item_receipts_cause_kind_check CHECK(cause_kind IN(0,1,2,3));
DO $$
DECLARE n TEXT; prior TEXT; arm TEXT;
BEGIN
 arm:=$p$(cause_kind=3 AND spell_family='Creature'
  AND spell_production_key=creature_definition_key AND spell_revision='definition-r1'
  AND creature_definition_revision='definition-r1'
  AND creature_definition_key IN('oteryn:creature.vemiath','oteryn:creature.chagorz','oteryn:creature.bakragore')
  AND runtime_scope_kind=2 AND channel_id IS NULL AND instance_id IS NOT NULL AND game_node_is_uuid_v7(instance_id)
  AND operation_count BETWEEN 0 AND 1 AND swing_ordinal IS NULL
  AND game_session_id IS NULL AND command_id IS NULL AND character_id IS NULL
  AND cost IS NULL AND cost_binding IS NULL AND caster_lease_generation IS NULL AND caster_placement_digest IS NULL
  AND octet_length(creature_placement)=16 AND creature_placement<>decode(repeat('00',16),'hex')
  AND creature_actor_generation BETWEEN 1 AND 18446744073709551615
  AND octet_length(creature_occurrence) BETWEEN 1 AND 1024
  AND octet_length(creature_map_digest)=32 AND octet_length(creature_frame_digest)=32
  AND octet_length(creature_source_body_digest)=32 AND octet_length(creature_cast_digest)=32
  AND convert_from(intent,'UTF8')::jsonb->>'schema'='OTERYN_CREATURE_EVENT_GROUND_CAUSE/v1'
  AND convert_from(intent,'UTF8')::jsonb->>'runtime_scope_kind'='2'
  AND convert_from(intent,'UTF8')::jsonb->'channel'='null'::jsonb
  AND convert_from(intent,'UTF8')::jsonb->>'instance'=replace(instance_id::text,'-','')
  AND convert_from(intent,'UTF8')::jsonb->'source_event'->>'action_id'='50001'
  AND convert_from(intent,'UTF8')::jsonb->'source_event'->>'rule' IN
    ('rotten-blood.portal.create','rotten-blood.portal.expire','rotten-blood.portal.step'))$p$;
 FOREACH n IN ARRAY ARRAY['game_spell_item_typed_cause','game_spell_item_exact_runtime_scope'] LOOP
  SELECT pg_get_constraintdef(oid) INTO STRICT prior FROM pg_constraint
   WHERE conrelid='game_spell_item_receipts'::regclass AND conname=n;
  IF left(prior,6)<>'CHECK ' THEN RAISE EXCEPTION 'unexpected source constraint'; END IF;
  EXECUTE format('ALTER TABLE game_spell_item_receipts DROP CONSTRAINT %I',n);
  EXECUTE format('ALTER TABLE game_spell_item_receipts ADD CONSTRAINT %I CHECK ((%s OR %s) IS TRUE)',n,substr(prior,7),arm);
 END LOOP;
END $$;
CREATE UNIQUE INDEX game_spell_item_creature_event_once ON game_spell_item_receipts
 (world_id,instance_id,ownership_generation,creature_placement,creature_actor_generation,
  spell_production_key,spell_revision,creature_occurrence) WHERE cause_kind=3;
DO $$
DECLARE body TEXT; amended TEXT;
BEGIN
 body:=pg_get_functiondef('game_spell_creature_cause_proven()'::regprocedure);
 amended:=replace(body,'NEW.cause_kind NOT IN(1,2)','NEW.cause_kind NOT IN(1,2,3)');
 amended:=replace(amended,'ELSE ''OTERYN_ENCOUNTER_GROUND_CAUSE/v2'' END',
  'WHEN 2 THEN ''OTERYN_ENCOUNTER_GROUND_CAUSE/v2'' ELSE ''OTERYN_CREATURE_EVENT_GROUND_CAUSE/v1'' END');
 IF amended=body OR strpos(amended,'NEW.cause_kind NOT IN(1,2,3)')=0 THEN
  RAISE EXCEPTION 'CreatureEvent source proof changed'; END IF;
 EXECUTE amended;
 body:=pg_get_functiondef('game_item_ground_insertion_guard()'::regprocedure);
 amended:=replace(body,'r.cause_kind=2 AND r.runtime_scope_kind=2','r.cause_kind IN(2,3) AND r.runtime_scope_kind=2');
 IF amended=body THEN RAISE EXCEPTION 'CreatureEvent ground gate changed'; END IF;
 EXECUTE amended;
END $$;
CREATE FUNCTION game_rotten_portal_source_proven() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE rule TEXT;
BEGIN
 IF NEW.cause_kind<>3 THEN RETURN NULL; END IF;
 rule:=convert_from(NEW.intent,'UTF8')::jsonb->'source_event'->>'rule';
 IF (rule IN('rotten-blood.portal.create','rotten-blood.portal.step') AND NEW.operation_count<>1)
  OR EXISTS(SELECT 1 FROM game_spell_item_lines l WHERE l.transaction_id=NEW.transaction_id
   AND (l.runtime_scope_kind<>2 OR l.channel_id IS NOT NULL OR l.instance_id IS DISTINCT FROM NEW.instance_id
    OR l.world_id<>NEW.world_id OR l.created_xact_id<>pg_current_xact_id()
    OR l.operation_kind<>CASE WHEN rule='rotten-blood.portal.create' THEN 1 ELSE 2 END
    OR l.definition_family<>'Item' OR l.definition_production_key<>'oteryn:item.tibia.i37000'
    OR l.definition_revision<>'definition-r1' OR l.destination_parent_item_instance_id IS NOT NULL
    OR l.source_parent_item_instance_id IS NOT NULL OR l.blocks_movement OR l.blocks_projectile
    OR l.immovable_block_solid OR l.expires_at_unix_ms IS NOT NULL
    OR (l.operation_kind=1 AND (l.quantity_before<>0 OR l.quantity_after<>1))
    OR (l.operation_kind=2 AND (l.quantity_before<>1 OR l.quantity_after<>0))))
  OR (SELECT count(*) FROM game_spell_item_lines l WHERE l.transaction_id=NEW.transaction_id)<>NEW.operation_count
 THEN RAISE EXCEPTION 'Rotten CreatureEvent requires exact scoped portal operation' USING ERRCODE='23514'; END IF;
 IF NOT EXISTS(SELECT 1 FROM game_activity_handoff_groups g
  JOIN game_activity_runtime_entries e USING(handoff_id)
  JOIN game_boss_room_activities a ON a.admission=g.handoff_id
  WHERE g.world_id=NEW.world_id AND g.instance_id=NEW.instance_id AND g.state=2
   AND g.ownership_generation=NEW.ownership_generation
   AND e.destination_server_artifact_digest=NEW.catalog_digest
   AND e.destination_map_revision_digest=NEW.creature_map_digest
   AND e.destination_coordinate_frame_digest=NEW.creature_frame_digest
   AND a.boss_creature_key=NEW.creature_definition_key)
 THEN RAISE EXCEPTION 'Rotten CreatureEvent requires current accepted source activity' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_rotten_portal_source_proven AFTER INSERT ON game_spell_item_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_rotten_portal_source_proven();
-- SRX-3 fixed Soul War source cues reuse the raid effect/cursor owner.
ALTER TABLE game_world_raid_channel_runs ADD COLUMN source_state JSONB;
ALTER TABLE game_world_raid_channel_runs ADD CONSTRAINT raid_source_state_bounded
 CHECK(source_state IS NULL OR (jsonb_typeof(source_state)='object' AND octet_length(source_state::text)<=32768));
ALTER TABLE game_world_raid_effects ADD COLUMN source_cue JSONB;
ALTER TABLE game_world_raid_effects DROP CONSTRAINT game_world_raid_effects_action_kind_check;
ALTER TABLE game_world_raid_effects ADD CONSTRAINT game_world_raid_effects_action_kind_check CHECK(action_kind IN(1,2,3,4,5));
ALTER TABLE game_world_raid_effects ADD CONSTRAINT raid_source_cue_bounded
 CHECK((action_kind=5)=(source_cue IS NOT NULL) AND (source_cue IS NULL OR (jsonb_typeof(source_cue)='object' AND octet_length(source_cue::text)<=2048)));
ALTER TABLE game_world_raid_step_receipts DROP CONSTRAINT game_world_raid_step_receipts_action_kind_check;
ALTER TABLE game_world_raid_step_receipts ADD CONSTRAINT game_world_raid_step_receipts_action_kind_check CHECK(action_kind IN(1,2,3,5));
CREATE FUNCTION game_claustrophobic_raid_cue_guard() RETURNS trigger
LANGUAGE plpgsql SET search_path=pg_catalog,public AS $$
BEGIN
 IF (TG_TABLE_NAME='game_world_raid_effects' AND (to_jsonb(NEW)->>'action_kind')::integer=5)
 OR (TG_TABLE_NAME='game_world_raid_channel_runs' AND to_jsonb(NEW)->>'source_state' IS NOT NULL)
 OR (TG_TABLE_NAME='game_world_raid_step_receipts' AND (to_jsonb(NEW)->>'action_kind')::integer=5) THEN
  IF NOT EXISTS(SELECT 1 FROM game_world_raid_firings f JOIN game_world_raid_schedule_sources s USING(world_id,raid_key)
    WHERE f.firing_id=NEW.firing_id AND s.source_kind=4
    AND f.raid_key IN('raid:soul-war.claustrophobic-inferno-1','raid:soul-war.claustrophobic-inferno-2','raid:soul-war.claustrophobic-inferno-3')) THEN
   RAISE EXCEPTION 'fixed source raid cue owner mismatch' USING ERRCODE='23514';
  END IF;
 END IF;
 IF TG_TABLE_NAME='game_world_raid_effects' AND TG_OP='UPDATE' AND to_jsonb(NEW)->'source_cue' IS DISTINCT FROM to_jsonb(OLD)->'source_cue' THEN
  RAISE EXCEPTION 'fixed source raid cue is immutable' USING ERRCODE='23514';
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER claustrophobic_source_cue BEFORE INSERT OR UPDATE ON game_world_raid_effects FOR EACH ROW EXECUTE FUNCTION game_claustrophobic_raid_cue_guard();
CREATE TRIGGER claustrophobic_source_cursor BEFORE INSERT OR UPDATE ON game_world_raid_channel_runs FOR EACH ROW EXECUTE FUNCTION game_claustrophobic_raid_cue_guard();
CREATE TRIGGER claustrophobic_source_receipt BEFORE INSERT ON game_world_raid_step_receipts FOR EACH ROW EXECUTE FUNCTION game_claustrophobic_raid_cue_guard();
REVOKE ALL ON FUNCTION game_claustrophobic_raid_cue_guard() FROM PUBLIC;
GRANT UPDATE(source_state) ON game_world_raid_channel_runs TO oteryn_game_runtime;
-- Existing terminal raid cursor cannot acquire a new fixed-source successor.
CREATE FUNCTION game_claustrophobic_cursor_successor_guard() RETURNS trigger
LANGUAGE plpgsql SET search_path=pg_catalog,public AS $$
BEGIN
 IF NEW.source_state IS DISTINCT FROM OLD.source_state
 AND (OLD.state<>1 OR NEW.committed_steps<>OLD.committed_steps+1) THEN
  RAISE EXCEPTION 'fixed source cursor requires one current raid receipt successor' USING ERRCODE='23514';
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER claustrophobic_cursor_successor BEFORE UPDATE ON game_world_raid_channel_runs
 FOR EACH ROW EXECUTE FUNCTION game_claustrophobic_cursor_successor_guard();
REVOKE ALL ON FUNCTION game_claustrophobic_cursor_successor_guard() FROM PUBLIC;
-- SRX-3 source onMove uses the existing scoped Item writer. Internal op7 is
-- an in-place transform: identity, quantity, Ground custody and stack stay fixed.
ALTER TABLE game_spell_item_lines DROP CONSTRAINT spell_item_operation_kind,
 ADD CONSTRAINT spell_item_operation_kind CHECK(operation_kind IN(1,2,3,4,5,6,7));
DO $$
DECLARE name TEXT; prior TEXT; arm TEXT;
BEGIN
 FOREACH name IN ARRAY ARRAY['spell_item_operation_shape','game_spell_item_line_exact_scope'] LOOP
  SELECT pg_get_constraintdef(oid) INTO STRICT prior FROM pg_constraint
   WHERE conrelid='game_spell_item_lines'::regclass AND conname=name;
  arm:=$p$(operation_kind=7 AND runtime_scope_kind=2 AND channel_id IS NULL
   AND instance_id IS NOT NULL AND game_node_is_uuid_v7(instance_id)
   AND quantity_before=1 AND quantity_after=1 AND state_revision_before>0 AND source_stack_ordinal>0
   AND destination_parent_item_instance_id IS NULL AND destination_ordinal IS NULL
   AND source_parent_item_instance_id IS NULL AND source_equipment_slot IS NULL
   AND source_corpse_root IS NULL AND expires_at_unix_ms IS NULL
   AND definition_family='Item' AND definition_revision='definition-r1')$p$;
  IF left(prior,6)<>'CHECK ' THEN RAISE EXCEPTION 'unexpected source transform line constraint'; END IF;
  EXECUTE format('ALTER TABLE game_spell_item_lines DROP CONSTRAINT %I',name);
  EXECUTE format('ALTER TABLE game_spell_item_lines ADD CONSTRAINT %I CHECK ((%s OR %s) IS TRUE)',name,substr(prior,7),arm);
 END LOOP;
END $$;
DO $$
DECLARE name TEXT; prior TEXT; arm TEXT;
BEGIN
 arm:=$p$(cause_kind=3 AND spell_family='Creature'
  AND spell_production_key=creature_definition_key AND spell_revision='definition-r1'
  AND creature_definition_revision='definition-r1'
  AND creature_definition_key IN('oteryn:creature.vemiath','oteryn:creature.chagorz','oteryn:creature.bakragore')
  AND runtime_scope_kind=2 AND channel_id IS NULL AND instance_id IS NOT NULL AND game_node_is_uuid_v7(instance_id)
  AND operation_count BETWEEN 0 AND 1 AND swing_ordinal IS NULL
  AND game_session_id IS NULL AND command_id IS NULL AND character_id IS NULL
  AND cost IS NULL AND cost_binding IS NULL AND caster_lease_generation IS NULL AND caster_placement_digest IS NULL
  AND octet_length(creature_placement)=16 AND creature_placement<>decode(repeat('00',16),'hex')
  AND creature_actor_generation BETWEEN 1 AND 18446744073709551615
  AND octet_length(creature_occurrence) BETWEEN 1 AND 1024
  AND octet_length(creature_map_digest)=32 AND octet_length(creature_frame_digest)=32
  AND octet_length(creature_source_body_digest)=32 AND octet_length(creature_cast_digest)=32
  AND convert_from(intent,'UTF8')::jsonb->>'schema'='OTERYN_CREATURE_EVENT_GROUND_CAUSE/v1'
  AND convert_from(intent,'UTF8')::jsonb->>'runtime_scope_kind'='2'
  AND convert_from(intent,'UTF8')::jsonb->'channel'='null'::jsonb
  AND convert_from(intent,'UTF8')::jsonb->>'instance'=replace(instance_id::text,'-','')
  AND convert_from(intent,'UTF8')::jsonb->'source_event'->'action_id'='null'::jsonb
  AND convert_from(intent,'UTF8')::jsonb->'source_event'->>'rule' IN
   ('rotten-blood.footprint.create','rotten-blood.footprint.transform','rotten-blood.footprint.remove'))$p$;
 FOREACH name IN ARRAY ARRAY['game_spell_item_typed_cause','game_spell_item_exact_runtime_scope'] LOOP
  SELECT pg_get_constraintdef(oid) INTO STRICT prior FROM pg_constraint
   WHERE conrelid='game_spell_item_receipts'::regclass AND conname=name;
  IF left(prior,6)<>'CHECK ' THEN RAISE EXCEPTION 'unexpected source transform receipt constraint'; END IF;
  EXECUTE format('ALTER TABLE game_spell_item_receipts DROP CONSTRAINT %I',name);
  EXECUTE format('ALTER TABLE game_spell_item_receipts ADD CONSTRAINT %I CHECK ((%s OR %s) IS TRUE)',name,substr(prior,7),arm);
 END LOOP;
END $$;
CREATE FUNCTION game_rotten_move_transform_edge(creature TEXT,before_key TEXT,after_key TEXT) RETURNS BOOLEAN
LANGUAGE sql IMMUTABLE AS $$
 SELECT COALESCE(creature IN('oteryn:creature.vemiath','oteryn:creature.chagorz','oteryn:creature.bakragore') AND (
  (before_key='oteryn:item.tibia.i43589' AND after_key='oteryn:item.tibia.i43590')
  OR (before_key='oteryn:item.tibia.i43590' AND after_key=CASE WHEN creature='oteryn:creature.chagorz' THEN 'oteryn:item.tibia.i43625' ELSE 'oteryn:item.tibia.i43297' END)
  OR (creature='oteryn:creature.bakragore' AND before_key='oteryn:item.tibia.i43294' AND after_key='oteryn:item.tibia.i43295')),false)
$$;
CREATE FUNCTION game_rotten_move_transform_line_complete(l game_spell_item_lines) RETURNS BOOLEAN
LANGUAGE sql SECURITY DEFINER AS $$
 SELECT COALESCE(l.operation_kind=7 AND l.created_xact_id=pg_current_xact_id() AND EXISTS(
  SELECT 1 FROM game_spell_item_receipts r JOIN game_item_instances i ON i.item_instance_id=l.item_instance_id
  JOIN game_item_ground_locations g ON g.item_instance_id=i.item_instance_id
  WHERE r.transaction_id=l.transaction_id AND r.created_xact_id=pg_current_xact_id()
   AND r.cause_kind=3 AND r.runtime_scope_kind=2 AND r.channel_id IS NULL
   AND (r.world_id,r.instance_id,r.ownership_generation)=(l.world_id,l.instance_id,g.runtime_scope_ownership_generation)
   AND g.world_id=r.world_id AND g.scope_kind=2 AND g.channel_id IS NULL AND g.instance_id=r.instance_id
   AND (g.spatial_position,g.map_revision,g.content_revision,g.native_room_placement_context,g.stack_ordinal)
    =(l.spatial_position,l.map_revision,l.content_revision,l.placement_context,l.source_stack_ordinal)
   AND i.lifecycle=1 AND i.quantity=1 AND i.state_revision=l.state_revision_before+1
   AND i.xmin=pg_current_xact_id()::xid
   AND EXISTS(SELECT 1 FROM game_item_transfer_quantity_evidence q WHERE q.item_instance_id=i.item_instance_id
    AND q.transaction_id=l.transaction_id AND q.quantity_before=1 AND q.xmin=pg_current_xact_id()::xid)
   AND i.last_transaction_id=l.transaction_id AND i.world_id=r.world_id
   AND (i.definition_family,i.definition_production_key,i.definition_revision_ref)
    =(l.definition_family,l.definition_production_key,l.definition_revision)
   AND convert_from(r.intent,'UTF8')::jsonb->'source_event'->>'rule'='rotten-blood.footprint.transform'
   AND NOT EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.item_instance_id=i.item_instance_id)
   AND NOT EXISTS(SELECT 1 FROM game_character_equipment_slots e WHERE e.item_instance_id=i.item_instance_id)
   AND NOT EXISTS(SELECT 1 FROM game_item_corpse_container_entries e WHERE e.parent_item_instance_id=i.item_instance_id)
 ),false)
$$;
CREATE FUNCTION game_rotten_move_transform_before(b game_item_instances,a game_item_instances) RETURNS BOOLEAN
LANGUAGE sql SECURITY DEFINER AS $$
 SELECT COALESCE(b.lifecycle=1 AND a.lifecycle=1 AND b.quantity=1 AND a.quantity=1
  AND b.definition_family='Item' AND b.definition_revision_ref='definition-r1'
  AND a.definition_family='Item' AND a.definition_revision_ref='definition-r1'
  AND a.state_revision=b.state_revision+1 AND a.last_transaction_id IS DISTINCT FROM b.last_transaction_id
  AND (to_jsonb(a)-ARRAY['definition_family','definition_production_key','definition_revision_ref','state_revision','last_transaction_id'])
     =(to_jsonb(b)-ARRAY['definition_family','definition_production_key','definition_revision_ref','state_revision','last_transaction_id'])
  AND EXISTS(SELECT 1 FROM game_spell_item_lines l JOIN game_spell_item_receipts r USING(transaction_id)
   JOIN game_item_ground_locations g ON g.item_instance_id=l.item_instance_id
   WHERE l.operation_kind=7 AND l.transaction_id=a.last_transaction_id AND l.item_instance_id=b.item_instance_id
    AND l.created_xact_id=pg_current_xact_id() AND r.created_xact_id=pg_current_xact_id()
    AND r.cause_kind=3 AND r.runtime_scope_kind=2 AND r.channel_id IS NULL AND l.runtime_scope_kind=2 AND l.channel_id IS NULL
    AND (r.world_id,r.instance_id,r.ownership_generation)=(b.world_id,g.instance_id,g.runtime_scope_ownership_generation)
    AND g.world_id=r.world_id AND g.scope_kind=2 AND g.channel_id IS NULL AND l.instance_id=r.instance_id AND l.world_id=r.world_id
    AND (g.spatial_position,g.map_revision,g.content_revision,g.native_room_placement_context,g.stack_ordinal)
       =(l.spatial_position,l.map_revision,l.content_revision,l.placement_context,l.source_stack_ordinal)
    AND l.state_revision_before=b.state_revision AND l.quantity_before=1 AND l.quantity_after=1
    AND (l.definition_family,l.definition_production_key,l.definition_revision)=(a.definition_family,a.definition_production_key,a.definition_revision_ref)
    AND convert_from(r.intent,'UTF8')::jsonb->'source_event'->>'rule'='rotten-blood.footprint.transform'
    AND convert_from(r.intent,'UTF8')::jsonb->'operations'->(l.ordinal-1)->>'kind'='transform_ground'
    AND convert_from(r.intent,'UTF8')::jsonb->'operations'->(l.ordinal-1)->'before'
        =jsonb_build_object('family',b.definition_family,'key',b.definition_production_key,'revision',b.definition_revision_ref)
    AND game_rotten_move_transform_edge(r.creature_definition_key,b.definition_production_key,a.definition_production_key)
  ),false)
$$;
DO $$
DECLARE body TEXT; amended TEXT;
BEGIN
 -- Item instance transform admission is composed after accepted audit guards by migration 2815.
 body:=pg_get_functiondef('game_spell_item_line_proven()'::regprocedure);
 amended:=replace(body,E'BEGIN\n',E'BEGIN\n IF NEW.operation_kind=7 THEN\n  IF NOT game_rotten_move_transform_line_complete(NEW) THEN RAISE EXCEPTION ''source transform exact physical successor missing'' USING ERRCODE=''23514''; END IF;\n  RETURN NULL;\n END IF;\n');
 IF amended=body THEN RAISE EXCEPTION 'source transform line guard form changed'; END IF;
 EXECUTE amended;
 body:=pg_get_functiondef('game_spell_creature_cause_proven()'::regprocedure);
 amended:=replace(body,'l.operation_kind NOT IN(1,2)',
  '(l.operation_kind NOT IN(1,2) AND NOT(l.operation_kind=7 AND NEW.cause_kind=3 AND p->''source_event''->>''rule''=''rotten-blood.footprint.transform''))');
 IF amended=body THEN RAISE EXCEPTION 'source transform cause guard form changed'; END IF;
 EXECUTE amended;
 body:=pg_get_functiondef('game_rotten_portal_source_proven()'::regprocedure);
 amended:=replace(body,E'BEGIN\n',E'BEGIN\n IF convert_from(NEW.intent,''UTF8'')::jsonb->''source_event''->>''rule'' LIKE ''rotten-blood.footprint.%'' THEN RETURN NULL; END IF;\n');
 IF amended=body THEN RAISE EXCEPTION 'source transform portal guard form changed'; END IF;
 EXECUTE amended;
END $$;
CREATE FUNCTION game_rotten_footprint_source_proven() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE rule TEXT; expected_item TEXT;
BEGIN
 IF NEW.cause_kind<>3 THEN RETURN NULL; END IF;
 rule:=convert_from(NEW.intent,'UTF8')::jsonb->'source_event'->>'rule';
 IF rule NOT LIKE 'rotten-blood.footprint.%' THEN RETURN NULL; END IF;
 IF rule='rotten-blood.footprint.create' THEN expected_item:='oteryn:item.tibia.i43589';
 ELSIF NEW.creature_definition_key='oteryn:creature.chagorz' THEN expected_item:='oteryn:item.tibia.i43625';
 ELSE expected_item:='oteryn:item.tibia.i43297'; END IF;
 IF NEW.created_xact_id<>pg_current_xact_id() OR NEW.operation_count NOT BETWEEN 0 AND 1
  OR (SELECT count(*) FROM game_spell_item_lines l WHERE l.transaction_id=NEW.transaction_id)<>NEW.operation_count
  OR EXISTS(SELECT 1 FROM game_spell_item_lines l WHERE l.transaction_id=NEW.transaction_id AND (
   l.created_xact_id<>pg_current_xact_id() OR l.world_id<>NEW.world_id OR l.runtime_scope_kind<>2
   OR l.channel_id IS NOT NULL OR l.instance_id IS DISTINCT FROM NEW.instance_id
   OR l.destination_parent_item_instance_id IS NOT NULL OR l.source_parent_item_instance_id IS NOT NULL
   OR l.blocks_movement OR l.blocks_projectile OR l.immovable_block_solid OR l.expires_at_unix_ms IS NOT NULL
   OR l.definition_family<>'Item' OR l.definition_revision<>'definition-r1'
   OR (rule='rotten-blood.footprint.create' AND (l.operation_kind<>1 OR l.quantity_before<>0 OR l.quantity_after<>1
      OR NOT(l.definition_production_key=expected_item OR (NEW.creature_definition_key='oteryn:creature.bakragore' AND l.definition_production_key='oteryn:item.tibia.i43294'))))
   OR (rule='rotten-blood.footprint.transform' AND NOT game_rotten_move_transform_line_complete(l))
   OR (rule='rotten-blood.footprint.remove' AND (l.operation_kind<>2 OR l.quantity_before<>1 OR l.quantity_after<>0 OR l.definition_production_key<>expected_item))))
  OR NOT EXISTS(SELECT 1 FROM game_activity_handoff_groups g JOIN game_activity_runtime_entries e USING(handoff_id)
   JOIN game_boss_room_activities a ON a.admission=g.handoff_id WHERE g.world_id=NEW.world_id AND g.instance_id=NEW.instance_id
    AND g.state=2 AND g.ownership_generation=NEW.ownership_generation AND a.boss_creature_key=NEW.creature_definition_key
    AND e.destination_server_artifact_digest=NEW.catalog_digest AND e.destination_map_revision_digest=NEW.creature_map_digest
    AND e.destination_coordinate_frame_digest=NEW.creature_frame_digest)
 THEN RAISE EXCEPTION 'source onMove footprint exact Item/activity proof missing' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_rotten_footprint_source_proven AFTER INSERT ON game_spell_item_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_rotten_footprint_source_proven();
DO $$
DECLARE signature REGPROCEDURE;
BEGIN
 FOREACH signature IN ARRAY ARRAY['game_rotten_move_transform_before(game_item_instances,game_item_instances)'::regprocedure,
  'game_rotten_move_transform_line_complete(game_spell_item_lines)'::regprocedure,'game_rotten_footprint_source_proven()'::regprocedure] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',signature,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',signature);
 END LOOP;
END $$;

REVOKE ALL ON FUNCTION game_rotten_move_transform_edge(TEXT,TEXT,TEXT) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_rotten_move_transform_edge(TEXT,TEXT,TEXT),
 game_rotten_move_transform_before(game_item_instances,game_item_instances),
 game_rotten_move_transform_line_complete(game_spell_item_lines) TO oteryn_game_runtime,oteryn_game_control;

-- Internal source cause4 is a registered GlobalEvent, never a fictitious actor.
ALTER TABLE game_spell_item_receipts DROP CONSTRAINT game_spell_item_receipts_cause_kind_check,
 ADD CONSTRAINT game_spell_item_receipts_cause_kind_check CHECK(cause_kind IN(0,1,2,3,4));
DO $$
DECLARE name TEXT; prior TEXT; arm TEXT;
BEGIN
 arm:=$p$(cause_kind=4 AND spell_family='Encounter'
  AND spell_production_key='oteryn:encounter.soul_war_taint_zones' AND spell_revision='definition-r1'
  AND ((runtime_scope_kind=1 AND channel_id IS NOT NULL AND instance_id IS NULL)
    OR (runtime_scope_kind=2 AND channel_id IS NULL AND instance_id IS NOT NULL AND game_node_is_uuid_v7(instance_id)))
  AND operation_count BETWEEN 0 AND 1 AND swing_ordinal IS NULL
  AND game_session_id IS NULL AND command_id IS NULL AND character_id IS NULL
  AND cost IS NULL AND cost_binding IS NULL AND caster_lease_generation IS NULL AND caster_placement_digest IS NULL
  AND creature_placement IS NULL AND creature_actor_generation IS NULL
  AND creature_definition_key IS NULL AND creature_definition_revision IS NULL
  AND creature_occurrence ~ '^registered-global:CreateGoshnarSpiteFire:[1-9][0-9]{0,19}:[0-3]:(create|expire)$'
  AND octet_length(creature_map_digest)=32 AND creature_map_digest<>decode(repeat('00',32),'hex')
  AND octet_length(creature_frame_digest)=32 AND creature_frame_digest<>decode(repeat('00',32),'hex')
  AND creature_source_body_digest=decode('9e1107dc41abc16b4f56e795180c1c17c917d8e73754547c443f070ed3167916','hex')
  AND octet_length(creature_cast_digest)=32 AND creature_cast_digest<>decode(repeat('00',32),'hex')
  AND convert_from(intent,'UTF8')::jsonb->>'schema'='OTERYN_REGISTERED_GLOBAL_GROUND_CAUSE/v1'
  AND convert_from(intent,'UTF8')::jsonb->'player'='null'::jsonb
  AND convert_from(intent,'UTF8')::jsonb->'cost'='null'::jsonb
  AND convert_from(intent,'UTF8')::jsonb->'caster_origin'='null'::jsonb
  AND convert_from(intent,'UTF8')::jsonb->'creature'='null'::jsonb
  AND convert_from(intent,'UTF8')::jsonb->'global_event'->>'name'='CreateGoshnarSpiteFire'
  AND convert_from(intent,'UTF8')::jsonb->'global_event'->>'action' IN('create','expire')
  AND convert_from(intent,'UTF8')::jsonb->'global_event'->>'node' ~ '^[0-9a-f]{32}$'
  AND convert_from(intent,'UTF8')::jsonb->'global_event'->>'node_registration_revision' ~ '^[1-9][0-9]{0,19}$'
  AND convert_from(intent,'UTF8')::jsonb->'global_event'->>'assignment_revision' ~ '^[1-9][0-9]{0,19}$'
  AND convert_from(intent,'UTF8')::jsonb->'global_event'->>'work_ordinal' ~ '^[1-9][0-9]{0,19}$'
  AND convert_from(intent,'UTF8')::jsonb->>'activation_sequence' ~ '^[1-9][0-9]{0,19}$'
  AND convert_from(intent,'UTF8')::jsonb->>'client_content' ~ '^[0-9a-f]{64}$'
  AND convert_from(intent,'UTF8')::jsonb->>'client_content'<>repeat('0',64))$p$;
 FOREACH name IN ARRAY ARRAY['game_spell_item_typed_cause','game_spell_item_exact_runtime_scope'] LOOP
  SELECT pg_get_constraintdef(oid) INTO STRICT prior FROM pg_constraint
   WHERE conrelid='game_spell_item_receipts'::regclass AND conname=name;
  IF left(prior,6)<>'CHECK ' THEN RAISE EXCEPTION 'unexpected registered Global source constraint'; END IF;
  EXECUTE format('ALTER TABLE game_spell_item_receipts DROP CONSTRAINT %I',name);
  EXECUTE format('ALTER TABLE game_spell_item_receipts ADD CONSTRAINT %I CHECK ((%s OR %s) IS TRUE)',name,substr(prior,7),arm);
 END LOOP;
END $$;
CREATE UNIQUE INDEX game_spell_registered_global_once ON game_spell_item_receipts
 (world_id,runtime_scope_kind,coalesce(channel_id,instance_id),ownership_generation,spell_production_key,spell_revision,creature_occurrence)
 WHERE cause_kind=4;
CREATE FUNCTION game_spell_registered_global_proven() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public,pg_temp AS $$
DECLARE p JSONB; identity JSONB; occurrence TEXT[]; action TEXT; point BYTEA; x INTEGER; y INTEGER;
BEGIN
 IF NEW.cause_kind<>4 THEN RETURN NULL; END IF;
 p:=convert_from(NEW.intent,'UTF8')::jsonb; identity:=p->'global_event'; action:=identity->>'action';
 IF jsonb_typeof(p->'operations') IS DISTINCT FROM 'array' THEN RAISE EXCEPTION 'Global operations must be an array' USING ERRCODE='23514'; END IF;
 occurrence:=regexp_match(NEW.creature_occurrence,'^registered-global:CreateGoshnarSpiteFire:([1-9][0-9]{0,19}):([0-3]):(create|expire)$');
 IF occurrence IS NULL OR occurrence[3] IS DISTINCT FROM action
  OR (occurrence[1])::numeric NOT BETWEEN 1 AND 18446744073709551615
  OR jsonb_array_length(p->'operations')<>NEW.operation_count
  OR NEW.created_xact_id<>pg_current_xact_id() OR NEW.binding<>sha256(NEW.intent)
  OR p->>'world' IS DISTINCT FROM replace(NEW.world_id::text,'-','')
  OR p->>'runtime_scope_kind' IS DISTINCT FROM NEW.runtime_scope_kind::text
  OR p->'channel' IS DISTINCT FROM coalesce(to_jsonb(replace(NEW.channel_id::text,'-','')),'null'::jsonb)
  OR p->'instance' IS DISTINCT FROM coalesce(to_jsonb(replace(NEW.instance_id::text,'-','')),'null'::jsonb)
  OR p->>'ownership_generation' IS DISTINCT FROM NEW.ownership_generation::text
  OR p->'ability' IS DISTINCT FROM jsonb_build_object('family',NEW.spell_family,'key',NEW.spell_production_key,'revision',NEW.spell_revision)
  OR p->>'occurrence' IS DISTINCT FROM NEW.creature_occurrence
  OR p->>'content' IS DISTINCT FROM encode(NEW.catalog_digest,'hex')
  OR p->>'map' IS DISTINCT FROM encode(NEW.creature_map_digest,'hex')
  OR p->>'frame' IS DISTINCT FROM encode(NEW.creature_frame_digest,'hex')
  OR p->>'source_body' IS DISTINCT FROM encode(NEW.creature_source_body_digest,'hex')
  OR p->>'source_cast' IS DISTINCT FROM encode(NEW.creature_cast_digest,'hex')
  OR identity->>'registration_scope' IS DISTINCT FROM replace(coalesce(NEW.channel_id,NEW.instance_id)::text,'-','')
  OR (identity->>'node_registration_revision')::numeric NOT BETWEEN 1 AND 18446744073709551615
  OR (identity->>'assignment_revision')::numeric NOT BETWEEN 1 AND 18446744073709551615
  OR (identity->>'work_ordinal')::numeric NOT BETWEEN 1 AND 18446744073709551615
  OR (p->>'activation_sequence')::numeric NOT BETWEEN 1 AND 18446744073709551615
  OR NOT game_node_is_uuid_v7((identity->>'node')::uuid)
  OR NOT EXISTS(SELECT 1 FROM game_runtime_scope_assignments a JOIN game_node_registrations n ON n.node_id=a.holder_node_id
   WHERE a.world_id=NEW.world_id AND a.state=1 AND a.ownership_generation=NEW.ownership_generation
    AND n.state=1 AND n.registration_revision=a.holder_registration_revision
    AND a.holder_node_id=(identity->>'node')::uuid
    AND a.holder_registration_revision=(identity->>'node_registration_revision')::numeric
    AND a.source_revision=(identity->>'assignment_revision')::numeric
    AND ((NEW.runtime_scope_kind=1 AND a.scope_kind=1 AND a.channel_id=NEW.channel_id)
      OR (NEW.runtime_scope_kind=2 AND a.scope_kind=3 AND a.activity_instance_id=NEW.instance_id)))
  OR NOT EXISTS(SELECT 1 FROM game_content_activations c WHERE c.world_id=NEW.world_id
   AND c.activation_sequence=(p->>'activation_sequence')::numeric AND c.server_artifact_digest=NEW.catalog_digest
   AND c.client_artifact_digest=decode(p->>'client_content','hex')
   AND (NEW.runtime_scope_kind=2 OR (c.channel_id=NEW.channel_id AND c.frame_binding_digest=NEW.creature_frame_digest))
   AND NOT EXISTS(SELECT 1 FROM game_content_activations later WHERE later.world_id=c.world_id AND later.channel_id=c.channel_id AND later.activation_sequence>c.activation_sequence))
  OR (SELECT count(*) FROM game_spell_item_lines l WHERE l.transaction_id=NEW.transaction_id)<>NEW.operation_count
  OR (action='create' AND NEW.operation_count<>1)
  OR (action='expire' AND NOT EXISTS(SELECT 1 FROM game_spell_item_receipts parent
   WHERE parent.cause_kind=4 AND parent.creature_occurrence=regexp_replace(NEW.creature_occurrence,':expire$',':create')
    AND (parent.world_id,parent.runtime_scope_kind,parent.channel_id,parent.instance_id,parent.ownership_generation,parent.catalog_digest,parent.creature_map_digest,parent.creature_frame_digest,parent.creature_source_body_digest)
     IS NOT DISTINCT FROM (NEW.world_id,NEW.runtime_scope_kind,NEW.channel_id,NEW.instance_id,NEW.ownership_generation,NEW.catalog_digest,NEW.creature_map_digest,NEW.creature_frame_digest,NEW.creature_source_body_digest)
    AND parent.operation_count=1 AND parent.binding=sha256(parent.intent)
    AND convert_from(parent.intent,'UTF8')::jsonb->'global_event'->>'action'='create'))
 THEN RAISE EXCEPTION 'registered Global source requires its exact current scope and immutable occurrence' USING ERRCODE='23514'; END IF;
 x:=(ARRAY[33743,33736,33750,33742])[occurrence[2]::integer+1];
 y:=(ARRAY[31628,31632,31632,31637])[occurrence[2]::integer+1];
 point:=int4send(x)||int4send(y)||int2send(14::smallint);
 IF EXISTS(SELECT 1 FROM game_spell_item_lines l WHERE l.transaction_id=NEW.transaction_id AND (
  l.created_xact_id<>pg_current_xact_id() OR l.created_xact_id<>NEW.created_xact_id
  OR (l.world_id,l.runtime_scope_kind,l.channel_id,l.instance_id) IS DISTINCT FROM (NEW.world_id,NEW.runtime_scope_kind,NEW.channel_id,NEW.instance_id)
  OR l.ordinal<>1 OR l.operation_kind<>CASE action WHEN 'create' THEN 1 ELSE 2 END
  OR l.definition_family<>'Item' OR l.definition_production_key<>'oteryn:item.tibia.i33877' OR l.definition_revision<>'definition-r1'
  OR l.quantity_before<>CASE action WHEN 'create' THEN 0 ELSE 1 END
  OR l.quantity_after<>CASE action WHEN 'create' THEN 1 ELSE 0 END
  OR p->'operations'->0->>'kind' IS DISTINCT FROM CASE action WHEN 'create' THEN 'mint_ground' ELSE 'remove_ground' END
  OR p->'operations'->0->'id' IS DISTINCT FROM to_jsonb(ARRAY(SELECT get_byte(uuid_send(l.item_instance_id),n) FROM generate_series(0,15)n))
  OR p->'operations'->0->'definition' IS DISTINCT FROM jsonb_build_object('family',l.definition_family,'key',l.definition_production_key,'revision',l.definition_revision)
  OR p->'operations'->0->>'quantity' IS DISTINCT FROM '1'
  OR p->'operations'->0->'placement'->'position' IS DISTINCT FROM to_jsonb(ARRAY(SELECT get_byte(l.spatial_position,n) FROM generate_series(0,octet_length(l.spatial_position)-1)n))
  OR p->'operations'->0->'placement'->>'map_revision' IS DISTINCT FROM l.map_revision
  OR p->'operations'->0->'placement'->>'content_revision' IS DISTINCT FROM l.content_revision
  OR p->'operations'->0->'placement'->'context' IS DISTINCT FROM to_jsonb(ARRAY(SELECT get_byte(l.placement_context,n) FROM generate_series(0,octet_length(l.placement_context)-1)n))
  OR (action='create' AND (p->'operations'->0->'lifetime' IS DISTINCT FROM 'null'::jsonb
    OR p->'operations'->0->'decay_chain' IS DISTINCT FROM '[]'::jsonb
    OR p->'operations'->0->'blocks_movement' IS DISTINCT FROM 'false'::jsonb
    OR p->'operations'->0->'blocks_projectile' IS DISTINCT FROM 'false'::jsonb
    OR p->'operations'->0->'placement'->'corpse_ref' IS DISTINCT FROM to_jsonb(ARRAY(SELECT get_byte(convert_to(NEW.creature_occurrence,'UTF8'),n) FROM generate_series(0,octet_length(NEW.creature_occurrence)-1)n))))
  OR (action='expire' AND (p->'operations'->0->>'revision' IS DISTINCT FROM l.state_revision_before::text
    OR p->'operations'->0->>'ordinal' IS DISTINCT FROM l.source_stack_ordinal::text))
  OR l.spatial_position<>point OR l.map_revision<>'sha256:'||encode(NEW.creature_map_digest,'hex')
  OR l.content_revision<>'sha256:'||encode(NEW.catalog_digest,'hex') OR l.content_generation_digest<>NEW.catalog_digest
  OR l.placement_context<>NEW.creature_frame_digest
  OR l.blocks_movement OR l.blocks_projectile OR l.immovable_block_solid OR l.expires_at_unix_ms IS NOT NULL
  OR l.destination_parent_item_instance_id IS NOT NULL OR l.source_parent_item_instance_id IS NOT NULL
  OR l.source_equipment_slot IS NOT NULL OR l.source_corpse_root IS NOT NULL
 )) THEN RAISE EXCEPTION 'registered Spite GlobalEvent admits only its literal current fire operation' USING ERRCODE='23514'; END IF;
 IF NOT EXISTS(SELECT 1 FROM game_spell_item_audit_outbox a WHERE a.transaction_id=NEW.transaction_id
  AND a.created_xact_id=NEW.created_xact_id AND a.event_id=NEW.event_id AND a.envelope=NEW.intent
  AND a.envelope_sha256=NEW.binding AND a.occurred_at_unix_ms=NEW.occurred_at_unix_ms)
 THEN RAISE EXCEPTION 'registered Global source requires its immutable same-xact audit' USING ERRCODE='23514'; END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_spell_registered_global_proven AFTER INSERT ON game_spell_item_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_spell_registered_global_proven();
REVOKE ALL ON FUNCTION game_spell_registered_global_proven() FROM PUBLIC;

-- New class4 may pass the closed Instance entry gate only with its same-XID
-- receipt+physical line; all original MINT/custody proofs still follow.
DO $$
DECLARE body TEXT; amended TEXT;
BEGIN
 body:=pg_get_functiondef('game_spell_creature_cause_proven()'::regprocedure);
 IF strpos(body,'NEW.cause_kind NOT IN(1,2,3)')=0 THEN RAISE EXCEPTION 'Creature proof changed before Global sibling'; END IF;
 body:=pg_get_functiondef('game_item_ground_insertion_guard()'::regprocedure);
 amended:=replace(body,'r.cause_kind IN(2,3) AND r.runtime_scope_kind=2','r.cause_kind IN(2,3,4) AND r.runtime_scope_kind=2');
 IF amended=body THEN RAISE EXCEPTION 'Global scoped Ground gate form changed'; END IF;
 EXECUTE amended;
END $$;
-- Copied native W01 death events remain actual Creature sources in the same Item owner.
CREATE FUNCTION game_w01_death_ground_shape(r game_spell_item_receipts) RETURNS BOOLEAN
LANGUAGE sql STABLE AS $$
 SELECT COALESCE(r.cause_kind=3 AND r.spell_family='Creature'
  AND r.spell_production_key=r.creature_definition_key AND r.spell_revision=r.creature_definition_revision
  AND r.creature_definition_revision='definition-r1'
  AND r.game_session_id IS NULL AND r.command_id IS NULL AND r.character_id IS NULL
  AND r.cost IS NULL AND r.cost_binding IS NULL AND r.caster_lease_generation IS NULL AND r.caster_placement_digest IS NULL
  AND r.swing_ordinal IS NULL AND octet_length(r.creature_placement)=16
  AND r.creature_placement<>decode(repeat('00',16),'hex') AND r.creature_actor_generation BETWEEN 1 AND 18446744073709551615
  AND r.ownership_generation BETWEEN 1 AND 18446744073709551615
  AND octet_length(r.creature_occurrence) BETWEEN 1 AND 1024
  AND octet_length(r.catalog_digest)=32 AND octet_length(r.creature_map_digest)=32
  AND octet_length(r.creature_frame_digest)=32 AND octet_length(r.creature_cast_digest)=32
  AND r.catalog_digest<>decode(repeat('00',32),'hex') AND r.creature_map_digest<>decode(repeat('00',32),'hex')
  AND r.creature_frame_digest<>decode(repeat('00',32),'hex') AND r.creature_cast_digest<>decode(repeat('00',32),'hex')
  AND ((r.runtime_scope_kind=1 AND r.channel_id IS NOT NULL AND r.instance_id IS NULL)
    OR (r.runtime_scope_kind=2 AND r.channel_id IS NULL AND r.instance_id IS NOT NULL))
  AND convert_from(r.intent,'UTF8')::jsonb->>'schema'='OTERYN_CREATURE_EVENT_GROUND_CAUSE/v1'
  AND convert_from(r.intent,'UTF8')::jsonb->>'runtime_scope_kind'=r.runtime_scope_kind::text
  AND convert_from(r.intent,'UTF8')::jsonb->'channel'=coalesce(to_jsonb(replace(r.channel_id::text,'-','')),'null'::jsonb)
  AND convert_from(r.intent,'UTF8')::jsonb->'instance'=coalesce(to_jsonb(replace(r.instance_id::text,'-','')),'null'::jsonb)
  AND ((r.creature_definition_key='oteryn:creature.minion_of_gaz_haragoth'
    AND r.creature_source_body_digest=decode('140e7db53054ac626d49e0b411395fe7b5e0a3d4bdb951de42be0853c77d6159','hex')
    AND ((convert_from(r.intent,'UTF8')::jsonb->'source_event'->>'rule'='w01.gaz-minion.create'
       AND convert_from(r.intent,'UTF8')::jsonb->'source_event'->'action_id'='33542'::jsonb)
     OR (convert_from(r.intent,'UTF8')::jsonb->'source_event'->>'rule'='w01.gaz-minion.expire'
       AND convert_from(r.intent,'UTF8')::jsonb->'source_event'->'action_id'='null'::jsonb)))
   OR (r.creature_definition_key='oteryn:creature.corrupted_soul'
    AND r.creature_source_body_digest=decode('1e03253a5fdbe294e57b0a7066b7ec232dfac8191a7b62665be633d71421c0fa','hex')
    AND convert_from(r.intent,'UTF8')::jsonb->'source_event'->>'rule' IN
      ('w01.corrupted-soul.create','w01.corrupted-soul.transform','w01.corrupted-soul.expire')
    AND convert_from(r.intent,'UTF8')::jsonb->'source_event'->'action_id'='null'::jsonb)),false)
$$;
DO $$ DECLARE n TEXT; prior TEXT; BEGIN
 FOREACH n IN ARRAY ARRAY['game_spell_item_typed_cause','game_spell_item_exact_runtime_scope'] LOOP
  SELECT pg_get_constraintdef(oid) INTO STRICT prior FROM pg_constraint
   WHERE conrelid='game_spell_item_receipts'::regclass AND conname=n;
  IF left(prior,6)<>'CHECK ' THEN RAISE EXCEPTION 'W01 source receipt guard form changed'; END IF;
  EXECUTE format('ALTER TABLE game_spell_item_receipts DROP CONSTRAINT %I',n);
  EXECUTE format('ALTER TABLE game_spell_item_receipts ADD CONSTRAINT %I CHECK ((%s OR game_w01_death_ground_shape(game_spell_item_receipts)) IS TRUE)',n,substr(prior,7));
 END LOOP;
END $$;
CREATE UNIQUE INDEX game_spell_item_channel_creature_event_once ON game_spell_item_receipts
 (world_id,channel_id,ownership_generation,creature_placement,creature_actor_generation,
  spell_production_key,spell_revision,creature_occurrence) WHERE cause_kind=3 AND runtime_scope_kind=1;
CREATE FUNCTION game_w01_death_transform_edge(creature TEXT,before_key TEXT,after_key TEXT) RETURNS BOOLEAN
LANGUAGE sql IMMUTABLE AS $$
 SELECT COALESCE(creature='oteryn:creature.corrupted_soul' AND
  ((before_key='oteryn:item.tibia.i23726' AND after_key='oteryn:item.tibia.i23727')
   OR (before_key='oteryn:item.tibia.i23727' AND after_key='oteryn:item.tibia.i23728')),false)
$$;
CREATE FUNCTION game_w01_death_transform_line_complete(l game_spell_item_lines) RETURNS BOOLEAN
LANGUAGE sql SECURITY DEFINER AS $$
 SELECT COALESCE(l.operation_kind=7 AND l.quantity_before=1 AND l.quantity_after=1 AND l.created_xact_id=pg_current_xact_id() AND EXISTS(
  SELECT 1 FROM game_spell_item_receipts r JOIN game_item_instances i ON i.item_instance_id=l.item_instance_id
  JOIN game_item_ground_locations g ON g.item_instance_id=i.item_instance_id
  WHERE r.transaction_id=l.transaction_id AND r.created_xact_id=pg_current_xact_id()
   AND game_w01_death_ground_shape(r)
   AND r.world_id=l.world_id AND l.runtime_scope_kind=r.runtime_scope_kind AND l.channel_id IS NOT DISTINCT FROM r.channel_id AND l.instance_id IS NOT DISTINCT FROM r.instance_id AND r.ownership_generation=g.runtime_scope_ownership_generation
   AND g.world_id=r.world_id AND g.scope_kind=r.runtime_scope_kind AND g.channel_id IS NOT DISTINCT FROM r.channel_id AND g.instance_id IS NOT DISTINCT FROM r.instance_id
   AND (g.spatial_position,g.map_revision,g.content_revision,g.native_room_placement_context,g.stack_ordinal)
    =(l.spatial_position,l.map_revision,l.content_revision,l.placement_context,l.source_stack_ordinal)
   AND i.lifecycle=1 AND i.quantity=1 AND i.state_revision=l.state_revision_before+1
   AND i.xmin=pg_current_xact_id()::xid
   AND EXISTS(SELECT 1 FROM game_item_transfer_quantity_evidence q WHERE q.item_instance_id=i.item_instance_id
    AND q.transaction_id=l.transaction_id AND q.quantity_before=1 AND q.xmin=pg_current_xact_id()::xid)
   AND i.last_transaction_id=l.transaction_id AND i.world_id=r.world_id
   AND (i.definition_family,i.definition_production_key,i.definition_revision_ref)
    =(l.definition_family,l.definition_production_key,l.definition_revision)
   AND convert_from(r.intent,'UTF8')::jsonb->'source_event'->>'rule'='w01.corrupted-soul.transform'
   AND NOT EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.item_instance_id=i.item_instance_id)
   AND NOT EXISTS(SELECT 1 FROM game_character_equipment_slots e WHERE e.item_instance_id=i.item_instance_id)
   AND NOT EXISTS(SELECT 1 FROM game_item_corpse_container_entries e WHERE e.parent_item_instance_id=i.item_instance_id)
 ),false)
$$;
CREATE FUNCTION game_w01_death_transform_before(b game_item_instances,a game_item_instances) RETURNS BOOLEAN
LANGUAGE sql SECURITY DEFINER AS $$
 SELECT COALESCE(b.lifecycle=1 AND a.lifecycle=1 AND b.quantity=1 AND a.quantity=1
  AND b.definition_family='Item' AND b.definition_revision_ref='definition-r1'
  AND a.definition_family='Item' AND a.definition_revision_ref='definition-r1'
  AND a.state_revision=b.state_revision+1 AND a.last_transaction_id IS DISTINCT FROM b.last_transaction_id
  AND (to_jsonb(a)-ARRAY['definition_family','definition_production_key','definition_revision_ref','state_revision','last_transaction_id'])
     =(to_jsonb(b)-ARRAY['definition_family','definition_production_key','definition_revision_ref','state_revision','last_transaction_id'])
  AND EXISTS(SELECT 1 FROM game_spell_item_lines l JOIN game_spell_item_receipts r USING(transaction_id)
   JOIN game_item_ground_locations g ON g.item_instance_id=l.item_instance_id
   WHERE l.operation_kind=7 AND l.transaction_id=a.last_transaction_id AND l.item_instance_id=b.item_instance_id
    AND l.created_xact_id=pg_current_xact_id() AND r.created_xact_id=pg_current_xact_id()
    AND game_w01_death_ground_shape(r) AND l.runtime_scope_kind=r.runtime_scope_kind AND l.channel_id IS NOT DISTINCT FROM r.channel_id AND l.instance_id IS NOT DISTINCT FROM r.instance_id
    AND r.world_id=b.world_id AND g.scope_kind=r.runtime_scope_kind AND g.channel_id IS NOT DISTINCT FROM r.channel_id AND g.instance_id IS NOT DISTINCT FROM r.instance_id AND r.ownership_generation=g.runtime_scope_ownership_generation
    AND g.world_id=r.world_id AND l.runtime_scope_kind=r.runtime_scope_kind AND l.channel_id IS NOT DISTINCT FROM r.channel_id AND l.instance_id IS NOT DISTINCT FROM r.instance_id AND l.world_id=r.world_id
    AND (g.spatial_position,g.map_revision,g.content_revision,g.native_room_placement_context,g.stack_ordinal)
       =(l.spatial_position,l.map_revision,l.content_revision,l.placement_context,l.source_stack_ordinal)
    AND l.state_revision_before=b.state_revision AND l.quantity_before=1 AND l.quantity_after=1
    AND (l.definition_family,l.definition_production_key,l.definition_revision)=(a.definition_family,a.definition_production_key,a.definition_revision_ref)
    AND convert_from(r.intent,'UTF8')::jsonb->'source_event'->>'rule'='w01.corrupted-soul.transform'
    AND convert_from(r.intent,'UTF8')::jsonb->'operations'->(l.ordinal-1)->>'kind'='transform_ground'
    AND convert_from(r.intent,'UTF8')::jsonb->'operations'->(l.ordinal-1)->'before'
        =jsonb_build_object('family',b.definition_family,'key',b.definition_production_key,'revision',b.definition_revision_ref)
    AND game_w01_death_transform_edge(r.creature_definition_key,b.definition_production_key,a.definition_production_key)
  ),false)
$$;
DO $$ DECLARE body TEXT; amended TEXT; BEGIN
 -- Item instance transform admission is composed after accepted audit guards by migration 2815.
 body:=pg_get_functiondef('game_spell_item_line_proven()'::regprocedure);
 amended:=replace(body,'IF NOT game_rotten_move_transform_line_complete(NEW) THEN',
  'IF NOT (game_rotten_move_transform_line_complete(NEW) OR game_w01_death_transform_line_complete(NEW)) THEN');
 IF amended=body THEN RAISE EXCEPTION 'W01 transform line guard changed'; END IF;
 EXECUTE amended;
 body:=pg_get_functiondef('game_spell_creature_cause_proven()'::regprocedure);
 amended:=replace(body,'p->''source_event''->>''rule''=''rotten-blood.footprint.transform''',
  'p->''source_event''->>''rule'' IN(''rotten-blood.footprint.transform'',''w01.corrupted-soul.transform'')');
 IF amended=body THEN RAISE EXCEPTION 'W01 transform cause guard changed'; END IF;
 EXECUTE amended;
 body:=pg_get_functiondef('game_rotten_portal_source_proven()'::regprocedure);
 amended:=replace(body,E'BEGIN\n',E'BEGIN\n IF NEW.cause_kind=3 AND convert_from(NEW.intent,''UTF8'')::jsonb->''source_event''->>''rule'' LIKE ''w01.%'' THEN RETURN NULL; END IF;\n');
 IF amended=body THEN RAISE EXCEPTION 'W01 independent source guard changed'; END IF;
 EXECUTE amended;
END $$;
CREATE FUNCTION game_w01_death_ground_proven() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE p JSONB; rule TEXT; l game_spell_item_lines; op JSONB;
BEGIN
 IF NEW.cause_kind<>3 THEN RETURN NULL; END IF;
 p:=convert_from(NEW.intent,'UTF8')::jsonb;
 rule:=p->'source_event'->>'rule';
 IF rule NOT LIKE 'w01.%' THEN RETURN NULL; END IF;
 IF NOT game_w01_death_ground_shape(NEW) OR NEW.created_xact_id<>pg_current_xact_id()
  OR NEW.binding<>sha256(NEW.intent) OR NEW.operation_count NOT BETWEEN 0 AND 1
  OR jsonb_typeof(p->'operations') IS DISTINCT FROM 'array'
  OR jsonb_array_length(p->'operations')<>NEW.operation_count
  OR (SELECT count(*) FROM game_spell_item_lines x WHERE x.transaction_id=NEW.transaction_id)<>NEW.operation_count
  OR NOT EXISTS(SELECT 1 FROM game_runtime_scope_assignments a JOIN game_node_registrations n ON n.node_id=a.holder_node_id
   WHERE a.world_id=NEW.world_id AND a.state=1 AND a.ownership_generation=NEW.ownership_generation
    AND n.state=1 AND n.registration_revision=a.holder_registration_revision
    AND ((NEW.runtime_scope_kind=1 AND a.scope_kind=1 AND a.channel_id=NEW.channel_id)
     OR (NEW.runtime_scope_kind=2 AND a.scope_kind=3 AND a.activity_instance_id=NEW.instance_id)))
  OR NOT EXISTS(SELECT 1 FROM game_content_activations c WHERE c.world_id=NEW.world_id
   AND c.server_artifact_digest=NEW.catalog_digest
   AND (NEW.runtime_scope_kind=2 OR c.channel_id=NEW.channel_id)
   AND NOT EXISTS(SELECT 1 FROM game_content_activations later WHERE later.world_id=c.world_id AND later.channel_id=c.channel_id AND later.activation_sequence>c.activation_sequence))
  OR NOT EXISTS(SELECT 1 FROM game_spell_item_audit_outbox a WHERE a.transaction_id=NEW.transaction_id
   AND a.event_id=NEW.event_id AND a.created_xact_id=NEW.created_xact_id
   AND a.envelope=NEW.intent AND a.envelope_sha256=NEW.binding)
 THEN RAISE EXCEPTION 'W01 copied Creature event requires current scope and immutable physical receipt' USING ERRCODE='23514'; END IF;
 FOR l IN SELECT * FROM game_spell_item_lines WHERE transaction_id=NEW.transaction_id LOOP
  op:=p->'operations'->(l.ordinal-1);
  IF l.ordinal<>1 OR l.created_xact_id<>pg_current_xact_id() OR l.world_id<>NEW.world_id
   OR l.runtime_scope_kind<>NEW.runtime_scope_kind OR l.channel_id IS DISTINCT FROM NEW.channel_id OR l.instance_id IS DISTINCT FROM NEW.instance_id
   OR l.definition_family<>'Item' OR l.definition_revision<>'definition-r1'
   OR l.destination_parent_item_instance_id IS NOT NULL OR l.source_parent_item_instance_id IS NOT NULL
   OR l.source_equipment_slot IS NOT NULL OR l.source_corpse_root IS NOT NULL OR l.expires_at_unix_ms IS NOT NULL
   OR l.map_revision<>'sha256:'||encode(NEW.creature_map_digest,'hex') OR l.content_revision<>'sha256:'||encode(NEW.catalog_digest,'hex')
   OR l.content_generation_digest<>NEW.catalog_digest OR l.placement_context<>NEW.creature_frame_digest
   OR op->'id' IS DISTINCT FROM to_jsonb(ARRAY(SELECT get_byte(uuid_send(l.item_instance_id),n) FROM generate_series(0,15)n))
   OR op->'definition' IS DISTINCT FROM jsonb_build_object('family',l.definition_family,'key',l.definition_production_key,'revision',l.definition_revision)
   OR op->'placement'->>'map_revision' IS DISTINCT FROM l.map_revision
   OR op->'placement'->>'content_revision' IS DISTINCT FROM l.content_revision
   OR op->'placement'->'position' IS DISTINCT FROM to_jsonb(ARRAY(SELECT get_byte(l.spatial_position,n) FROM generate_series(0,octet_length(l.spatial_position)-1)n))
   OR op->'placement'->'context' IS DISTINCT FROM to_jsonb(ARRAY(SELECT get_byte(l.placement_context,n) FROM generate_series(0,octet_length(l.placement_context)-1)n))
   OR (rule IN('w01.gaz-minion.create','w01.corrupted-soul.create') AND
    (l.operation_kind<>1 OR l.quantity_before<>0 OR l.quantity_after<>1 OR op->>'kind' IS DISTINCT FROM 'mint_ground'
     OR op->'quantity' IS DISTINCT FROM '1'::jsonb OR op->'lifetime' IS DISTINCT FROM 'null'::jsonb
     OR op->'decay_chain' IS DISTINCT FROM '[]'::jsonb
     OR l.definition_production_key<>CASE WHEN rule='w01.gaz-minion.create' THEN 'oteryn:item.tibia.i20121' ELSE 'oteryn:item.tibia.i23726' END))
   OR (rule='w01.corrupted-soul.transform' AND
    (NOT game_w01_death_transform_line_complete(l) OR op->>'kind' IS DISTINCT FROM 'transform_ground' OR op->'quantity' IS DISTINCT FROM '1'::jsonb))
   OR (rule IN('w01.gaz-minion.expire','w01.corrupted-soul.expire') AND
    (l.operation_kind<>2 OR l.quantity_before<>1 OR l.quantity_after<>0 OR op->>'kind' IS DISTINCT FROM 'remove_ground'
     OR op->'quantity' IS DISTINCT FROM '1'::jsonb OR op->>'revision' IS DISTINCT FROM l.state_revision_before::text
     OR op->>'ordinal' IS DISTINCT FROM l.source_stack_ordinal::text
     OR (rule='w01.gaz-minion.expire' AND l.definition_production_key<>'oteryn:item.tibia.i20121')
     OR (rule='w01.corrupted-soul.expire' AND l.definition_production_key NOT IN('oteryn:item.tibia.i23726','oteryn:item.tibia.i23727','oteryn:item.tibia.i23728'))))
  THEN RAISE EXCEPTION 'W01 copied event admits only its exact current Ground operation' USING ERRCODE='23514'; END IF;
 END LOOP;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_w01_death_ground_proven AFTER INSERT ON game_spell_item_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_w01_death_ground_proven();
DO $$ DECLARE signature TEXT; BEGIN
 FOREACH signature IN ARRAY ARRAY['game_w01_death_ground_shape(game_spell_item_receipts)',
  'game_w01_death_transform_edge(text,text,text)','game_w01_death_transform_line_complete(game_spell_item_lines)',
  'game_w01_death_transform_before(game_item_instances,game_item_instances)','game_w01_death_ground_proven()'] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',signature,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',signature);
  EXECUTE format('GRANT EXECUTE ON FUNCTION %s TO oteryn_game_runtime',signature);
 END LOOP;
END $$;
-- A real reserved native onThink has an opaque UUIDv7 occurrence, not a command.
-- Preserve every old cause predicate verbatim; only this sealed source family
-- gains ordinal zero. Bare SourceThink remains refused by the sole Rust writer.
DO $$
DECLARE name TEXT; prior TEXT; arm TEXT;
BEGIN
 arm:=$p$(cause_kind='source_think' AND cause_ordinal=0
  AND game_character_is_uuid_v7(cause_id)
  AND quest_key='oteryn:quest/rotten-blood/area-icons'
  AND transition_key ~ '^oteryn:quest-transition/rotten-blood/area-icons/[1-7]$'
  AND completes=false AND octet_length(request_binding)=33 AND get_byte(request_binding,0)=1
  AND track_keys=CASE right(transition_key,1)
   WHEN '1' THEN ARRAY['oteryn:quest-progress/rotten-blood/area-icons/last-hazard-update']
   WHEN '2' THEN ARRAY['oteryn:quest-progress/rotten-blood/area-icons/last-upper-left-update']
   WHEN '3' THEN ARRAY['oteryn:quest-progress/rotten-blood/area-icons/last-hazard-update','oteryn:quest-progress/rotten-blood/area-icons/last-upper-left-update']
   WHEN '4' THEN ARRAY['oteryn:quest-progress/rotten-blood/area-icons/last-lower-left-update']
   WHEN '5' THEN ARRAY['oteryn:quest-progress/rotten-blood/area-icons/last-hazard-update','oteryn:quest-progress/rotten-blood/area-icons/last-lower-left-update']
   WHEN '6' THEN ARRAY['oteryn:quest-progress/rotten-blood/area-icons/last-upper-left-update','oteryn:quest-progress/rotten-blood/area-icons/last-lower-left-update']
   WHEN '7' THEN ARRAY['oteryn:quest-progress/rotten-blood/area-icons/last-hazard-update','oteryn:quest-progress/rotten-blood/area-icons/last-upper-left-update','oteryn:quest-progress/rotten-blood/area-icons/last-lower-left-update']
   ELSE NULL::text[] END
  AND cardinality(values_before)=cardinality(track_keys)
  AND cardinality(values_after)=cardinality(track_keys)
  AND 0<=ALL(values_before) AND 0<ALL(values_after)
  AND values_before[1]::numeric<=values_after[1]::numeric-CASE track_keys[1]
   WHEN 'oteryn:quest-progress/rotten-blood/area-icons/last-hazard-update' THEN 8
   WHEN 'oteryn:quest-progress/rotten-blood/area-icons/last-upper-left-update' THEN 30
   WHEN 'oteryn:quest-progress/rotten-blood/area-icons/last-lower-left-update' THEN 1 END
  AND (cardinality(track_keys)<2 OR values_before[2]::numeric<=values_after[2]::numeric-CASE track_keys[2]
   WHEN 'oteryn:quest-progress/rotten-blood/area-icons/last-hazard-update' THEN 8
   WHEN 'oteryn:quest-progress/rotten-blood/area-icons/last-upper-left-update' THEN 30
   WHEN 'oteryn:quest-progress/rotten-blood/area-icons/last-lower-left-update' THEN 1 END)
  AND (cardinality(track_keys)<3 OR values_before[3]::numeric<=values_after[3]::numeric-1)
  AND values_after[1]=values_after[cardinality(values_after)]
  AND (cardinality(values_after)<2 OR values_after[1]=values_after[2]))$p$;
 FOREACH name IN ARRAY ARRAY['game_character_quest_receipts_cause_kind_check','game_character_quest_receipt_cause_shape'] LOOP
  SELECT pg_get_constraintdef(oid) INTO STRICT prior FROM pg_constraint
   WHERE conrelid='game_character_quest_receipts'::regclass AND conname=name;
  IF left(prior,6)<>'CHECK ' OR strpos(prior,'source_think')<>0 THEN
   RAISE EXCEPTION 'unexpected prior native source-think cause constraint';
  END IF;
  EXECUTE format('ALTER TABLE game_character_quest_receipts DROP CONSTRAINT %I',name);
  EXECUTE format('ALTER TABLE game_character_quest_receipts ADD CONSTRAINT %I CHECK (((cause_kind<>''source_think'' AND %s) OR %s) IS TRUE)',name,substr(prior,7),arm);
 END LOOP;
END $$;
