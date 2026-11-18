-- Crystal 201189f dreamers_challenge_quest/actions_stone_teleport.lua:4-8,44-67.
-- A poured splash has its own custody table. Keep its exact source transaction
-- beside the existing Quest receipt as the first ordered offering.
CREATE TABLE game_quest_stone_splash_consumptions (
    character_id UUID NOT NULL,
    cause_id UUID NOT NULL,
    cause_ordinal NUMERIC(20,0) NOT NULL,
    transition_key TEXT NOT NULL CHECK
        (transition_key='oteryn:quest-callback/offering-portal/dreamers-challenge-stone'),
    splash_transaction_id UUID NOT NULL UNIQUE,
    created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
    PRIMARY KEY(character_id,cause_id,cause_ordinal,transition_key),
    FOREIGN KEY(character_id,cause_id,cause_ordinal,transition_key)
        REFERENCES game_character_quest_receipts(character_id,cause_id,cause_ordinal,transition_key)
        DEFERRABLE INITIALLY DEFERRED
);
CREATE FUNCTION game_quest_stone_splash_proof() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_WHEN='BEFORE' THEN
        IF NOT EXISTS (
            SELECT 1 FROM game_item_fluid_splashes f
            JOIN game_durability_reconnect_sessions s ON s.game_session_id=NEW.cause_id
            WHERE f.transaction_id=NEW.splash_transaction_id AND f.expires_at>clock_timestamp()
              AND s.character_id=NEW.character_id AND s.session_state=2
              AND (s.runtime_scope_world_id,s.runtime_scope_channel_id)=(f.world_id,f.channel_id)
              AND octet_length(f.placement)=40
              AND substring(f.placement from 35 for 5)=int2send((32878-65536)::smallint) ||
                  int2send(32270::smallint) || decode('0e','hex')
        ) THEN
            RAISE EXCEPTION 'stone splash requires the live sacrifice tile' USING ERRCODE='23514';
        END IF;
        NEW.created_xact_id:=pg_current_xact_id();
        RETURN NEW;
    END IF;
    IF NEW.created_xact_id<>pg_current_xact_id()
        OR EXISTS(SELECT 1 FROM game_item_fluid_splashes WHERE transaction_id=NEW.splash_transaction_id)
        OR NOT EXISTS (
            SELECT 1 FROM game_character_quest_receipts r
            WHERE (r.character_id,r.cause_id,r.cause_ordinal,r.transition_key)=
                  (NEW.character_id,NEW.cause_id,NEW.cause_ordinal,NEW.transition_key)
              AND r.cause_kind='use' AND r.created_xact_id=pg_current_xact_id()
        )
    THEN
        RAISE EXCEPTION 'stone splash requires its atomic Quest receipt' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;
CREATE TRIGGER game_quest_stone_splash_admit BEFORE INSERT
    ON game_quest_stone_splash_consumptions FOR EACH ROW EXECUTE FUNCTION game_quest_stone_splash_proof();
CREATE CONSTRAINT TRIGGER game_quest_stone_splash_complete AFTER INSERT
    ON game_quest_stone_splash_consumptions DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION game_quest_stone_splash_proof();
CREATE TRIGGER game_quest_stone_splash_immutable BEFORE UPDATE OR DELETE
    ON game_quest_stone_splash_consumptions FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_quest_stone_splash_no_truncate BEFORE TRUNCATE
    ON game_quest_stone_splash_consumptions FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();
REVOKE ALL ON game_quest_stone_splash_consumptions FROM PUBLIC;
GRANT SELECT,INSERT ON game_quest_stone_splash_consumptions TO oteryn_game_runtime;
GRANT SELECT ON game_quest_stone_splash_consumptions TO oteryn_game_control;

-- The fourth offering is a live native spell field on this exact altar.
-- The existing Quest BURN owner keeps the before Item/Ground and commit proof.
CREATE FUNCTION game_dreamer_stone_field_current(i game_item_instances,g game_item_ground_locations)
RETURNS BOOLEAN LANGUAGE sql STABLE AS $$
    SELECT coalesce(i.lifecycle=1 AND i.quantity=1
       AND i.definition_family='Item' AND i.definition_production_key='oteryn:item.tibia.i2118'
       AND g.item_instance_id=i.item_instance_id AND i.world_id=g.world_id
       AND g.spatial_position=int4send(32884)||int4send(32270)||int2send((-14)::smallint)
       AND EXISTS (
           SELECT 1 FROM game_spell_item_lines l
           LEFT JOIN game_spell_field_temporal_schedules f ON f.item_instance_id=i.item_instance_id
               AND f.source_transaction_id=l.transaction_id
               AND (f.definition_production_key,f.definition_revision)=
                   (i.definition_production_key,i.definition_revision_ref)
           WHERE l.item_instance_id=i.item_instance_id AND l.transaction_id=i.minted_transaction_id
               AND l.operation_kind=1
               AND (l.world_id,l.channel_id,l.spatial_position,l.map_revision,l.content_revision,l.placement_context)=
                   (g.world_id,g.channel_id,g.spatial_position,g.map_revision,g.content_revision,g.native_room_placement_context)
               AND (f.expires_at_unix_ms>extract(epoch from statement_timestamp())*1000
                    OR (l.definition_production_key=i.definition_production_key
                        AND l.expires_at_unix_ms>extract(epoch from statement_timestamp())*1000))
       ),false)
$$;
DO $migration$
DECLARE body TEXT; needle TEXT:='(NEW.cause_ordinal>0 AND game_item_loop_drop_ref(g.corpse_ref))'; n TEXT;
BEGIN
    body:=pg_get_functiondef('game_quest_callback_admit()'::regprocedure);
    IF (length(body)-length(replace(body,needle,'')))/length(needle)<>1 THEN
        RAISE EXCEPTION 'unexpected current Quest Ground predicate';
    END IF;
    body:=replace(body,needle,'('||needle||' OR
        (NEW.cause_ordinal>0 AND NEW.quantity=1
        AND NEW.transition_key=''oteryn:quest-callback/offering-portal/dreamers-challenge-stone''
        AND game_dreamer_stone_field_current(i,g)))');
    EXECUTE body;
    FOREACH n IN ARRAY ARRAY['game_quest_stone_splash_proof','game_dreamer_stone_field_current'] LOOP
        EXECUTE format('ALTER FUNCTION %I%s SET search_path=pg_catalog,%I,pg_temp',
            n,CASE n WHEN 'game_dreamer_stone_field_current' THEN '(game_item_instances,game_item_ground_locations)' ELSE '()' END,
            current_schema());
    END LOOP;
END $migration$;
REVOKE ALL ON FUNCTION game_quest_stone_splash_proof(),
    game_dreamer_stone_field_current(game_item_instances,game_item_ground_locations) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_dreamer_stone_field_current(game_item_instances,game_item_ground_locations)
    TO oteryn_game_runtime;

-- Crystal 201189f dreamers_challenge_quest/actions_tic_tac.lua:3-12.
-- Quest supply placements share the native map Item/Ground/audit owner, with
-- provenance fenced to the exact same-transaction, authored USE receipt.
DO $migration$
DECLARE
    body TEXT;
    begin_at INTEGER;
    proof CONSTANT TEXT := $proof$
 IF left(NEW.placement_key,13)='quest-supply/' OR NEW.source_attributes ? 'quest_supply' THEN
  IF NOT EXISTS (
    SELECT 1 FROM game_character_quest_receipts q
    WHERE encode(uuid_send(q.character_id),'hex')=NEW.source_attributes #>> '{quest_supply,character}'
      AND encode(uuid_send(q.cause_id),'hex')=NEW.source_attributes #>> '{quest_supply,session}'
      AND q.cause_ordinal::text=NEW.source_attributes #>> '{quest_supply,command}'
      AND q.transition_key=NEW.source_attributes #>> '{quest_supply,transition}'
      AND q.transition_key LIKE 'oteryn:quest-callback/ground-supply/%'
      AND q.cause_kind='use' AND q.request_binding=NEW.source_binding
      AND NEW.created_xact_id=pg_current_xact_id() AND q.created_xact_id=NEW.created_xact_id
      AND NEW.placement_key='quest-supply/'||encode(uuid_send(q.cause_id),'hex')||'/'||q.cause_ordinal::text||'/'
          ||(NEW.source_attributes #>> '{quest_supply,index}')
      AND NEW.source_attributes #>> '{quest_supply,index}' IN ('0','1','2','3','4','5','6','7')
      AND NOT NEW.blocks_movement AND NOT NEW.blocks_projectile AND NOT NEW.immovable_block_solid
      AND NEW.owner_kind IS NULL
  ) THEN RAISE EXCEPTION 'Quest Ground supply needs its original USE receipt' USING ERRCODE='23514'; END IF;
 END IF;
$proof$;
BEGIN
    body:=pg_get_functiondef('game_native_map_item_proven()'::regprocedure);
    begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
    IF begin_at=0 THEN RAISE EXCEPTION 'missing native map Item proof'; END IF;
    EXECUTE overlay(body placing E'BEGIN\n'||proof from begin_at for 5);
END
$migration$;

-- Physical USE/STEP consequences can have an empty journal delta. Keep their
-- source proof separate from paid NPC grants and from synthetic quest tracks.
CREATE TABLE game_quest_world_action_proofs (
    character_id UUID NOT NULL,
    cause_id UUID NOT NULL,
    cause_ordinal NUMERIC(20,0) NOT NULL,
    transition_key TEXT NOT NULL,
    world_id UUID NOT NULL,
    channel_id UUID NOT NULL,
    scope_generation NUMERIC(20,0) NOT NULL CHECK(scope_generation>0),
    request_binding BYTEA NOT NULL CHECK(octet_length(request_binding)=33),
    callback JSONB NOT NULL CHECK(jsonb_typeof(callback)='object' AND octet_length(callback::text)<=16384),
    source_face INTEGER CHECK(source_face BETWEEN 1 AND 65535),
    created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
    PRIMARY KEY(character_id,cause_id,cause_ordinal,transition_key),
    FOREIGN KEY(character_id,cause_id,cause_ordinal,transition_key)
        REFERENCES game_character_quest_receipts(character_id,cause_id,cause_ordinal,transition_key)
        DEFERRABLE INITIALLY DEFERRED
);
CREATE TRIGGER game_quest_world_action_proof_immutable BEFORE UPDATE OR DELETE ON game_quest_world_action_proofs
    FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_quest_world_action_proof_no_truncate BEFORE TRUNCATE ON game_quest_world_action_proofs
    EXECUTE FUNCTION game_item_reject_truncate();
REVOKE ALL ON game_quest_world_action_proofs FROM PUBLIC;
GRANT SELECT,INSERT ON game_quest_world_action_proofs TO oteryn_game_runtime;
GRANT SELECT ON game_quest_world_action_proofs TO oteryn_game_control;

DO $migration$
DECLARE
    constraint_body TEXT;
    function_body TEXT;
    anchor CONSTANT TEXT:='IF cardinality(NEW.track_keys)<>0 OR NEW.completes THEN RETURN NULL; END IF;';
    proof CONSTANT TEXT := $proof$
    IF EXISTS (
        SELECT 1 FROM game_quest_world_action_proofs p
        JOIN game_durability_reconnect_sessions s ON s.game_session_id=p.cause_id
        WHERE (p.character_id,p.cause_id,p.cause_ordinal,p.transition_key)=
              (NEW.character_id,NEW.cause_id,NEW.cause_ordinal,NEW.transition_key)
          AND p.request_binding=NEW.request_binding AND p.callback->>'key'=NEW.transition_key
          AND p.callback->>'quest'=NEW.quest_key AND p.callback->>'requires_world_admission'='true'
          AND (jsonb_typeof(p.callback #> '{selector,native}')='object'
            OR jsonb_typeof(p.callback #> '{selector,position}')='object')
          AND s.character_id=p.character_id AND s.session_state=2
          AND s.runtime_scope_world_id=p.world_id AND s.runtime_scope_channel_id=p.channel_id
          AND s.scope_ownership_generation=p.scope_generation
          AND p.created_xact_id=pg_current_xact_id() AND NEW.created_xact_id=p.created_xact_id
          AND (
            (NEW.cause_kind='use' AND p.callback->>'trigger'='use' AND (
                (NEW.transition_key LIKE 'oteryn:quest-callback/ground-supply/%'
                    AND jsonb_typeof(p.callback #> '{effects,ground_supply}')='array'
                    AND (SELECT count(*) FROM game_native_map_item_receipts n
                         WHERE n.world_id=p.world_id AND n.channel_id=p.channel_id
                           AND n.created_xact_id=p.created_xact_id AND n.source_binding=NEW.request_binding
                           AND n.source_attributes #>> '{quest_supply,transition}'=p.transition_key)
                        = CASE p.source_face
                           WHEN (p.callback #>> '{lever,0}')::integer THEN
                               CASE WHEN p.callback #>> '{effects,ground_supply_on_open}'='true'
                                    THEN jsonb_array_length(p.callback #> '{effects,ground_supply}') ELSE 0 END
                           WHEN (p.callback #>> '{lever,1}')::integer THEN
                               CASE WHEN p.callback #>> '{effects,ground_supply_on_open}'='true'
                                    THEN -1 ELSE jsonb_array_length(p.callback #> '{effects,ground_supply}') END
                           ELSE -1 END
                    AND NOT EXISTS (
                        SELECT 1 FROM game_native_map_item_receipts n
                        WHERE n.world_id=p.world_id AND n.channel_id=p.channel_id
                          AND n.created_xact_id=p.created_xact_id AND n.source_binding=NEW.request_binding
                          AND n.source_attributes #>> '{quest_supply,transition}'=p.transition_key
                          AND NOT EXISTS (
                            SELECT 1 FROM jsonb_array_elements(p.callback #> '{effects,ground_supply}')
                                WITH ORDINALITY d(item,ordinal)
                            WHERE (d.ordinal-1)::text=n.source_attributes #>> '{quest_supply,index}'
                              AND d.item->>'item'=n.definition_key AND (d.item->>'count')::bigint=n.quantity
                              AND n.spatial_position=int4send((d.item #>> '{position,x}')::integer)
                                  ||int4send((d.item #>> '{position,y}')::integer)
                                  ||int2send((d.item #>> '{position,floor}')::smallint)
                          )))
                OR (NEW.transition_key LIKE 'oteryn:quest-callback/offering-portal/%'
                    AND jsonb_typeof(p.callback #> '{effects,offering_portal}')='object')))
            OR (NEW.cause_kind='step' AND p.callback->>'trigger'='step_in' AND (
                (NEW.transition_key LIKE 'oteryn:quest-callback/ground-cleanup/%'
                    AND p.callback #>> '{effects,cleanup_offerings}'='true')
                OR (NEW.transition_key LIKE 'oteryn:quest-callback/ranked-toll/%'
                    AND jsonb_typeof(p.callback #> '{effects,ranked_toll}')='object'
                    AND EXISTS(
                        SELECT 1 FROM game_quest_callback_consumptions c
                        WHERE (c.character_id,c.cause_id,c.cause_ordinal,c.transition_key)=
                              (p.character_id,p.cause_id,p.cause_ordinal,p.transition_key)
                          AND c.created_xact_id=p.created_xact_id AND c.location_kind<>'ground'
                          AND c.quantity=(p.callback #>> '{item_on_target,count}')::bigint
                          AND c.before_item->>'definition_production_key'=p.callback #>> '{item_on_target,item}'
                    ))
                OR (NEW.transition_key LIKE 'oteryn:quest-callback/solo-room/%'
                    AND jsonb_typeof(p.callback #> '{effects,encounter,solo_expiry}')='object'
                    AND EXISTS(
                        SELECT 1 FROM game_character_boss_cooldowns b
                        WHERE b.character_id=p.character_id
                          AND b.boss_key=p.callback #>> '{effects,encounter,monsters,0,creature}'
                          AND b.xmin=pg_current_xact_id()::text::xid
                    ))))
          )
    ) THEN RETURN NULL; END IF;
$proof$;
BEGIN
    SELECT pg_get_expr(conbin,conrelid) INTO STRICT constraint_body FROM pg_constraint
     WHERE conrelid='game_character_quest_receipts'::regclass
       AND conname='game_character_quest_receipt_effects_shape';
    ALTER TABLE game_character_quest_receipts DROP CONSTRAINT game_character_quest_receipt_effects_shape;
    EXECUTE 'ALTER TABLE game_character_quest_receipts ADD CONSTRAINT game_character_quest_receipt_effects_shape CHECK (('
        ||constraint_body||$shape$) OR (
        NOT completes AND cardinality(track_keys)=0 AND cardinality(values_before)=0 AND cardinality(values_after)=0
        AND level_after=level_before AND experience_after=experience_before AND (
            (cause_kind='use' AND (transition_key LIKE 'oteryn:quest-callback/ground-supply/%'
                OR transition_key LIKE 'oteryn:quest-callback/offering-portal/%'))
            OR (cause_kind='step' AND (transition_key LIKE 'oteryn:quest-callback/ground-cleanup/%'
                OR transition_key LIKE 'oteryn:quest-callback/ranked-toll/%'
                OR transition_key LIKE 'oteryn:quest-callback/solo-room/%'))
        )))$shape$;
    function_body:=pg_get_functiondef('game_quest_reward_value_only_complete()'::regprocedure);
    IF (length(function_body)-length(replace(function_body,anchor,'')))/length(anchor)<>1 THEN
        RAISE EXCEPTION 'missing or ambiguous canonical value-only Quest guard';
    END IF;
    EXECUTE replace(function_body,anchor,anchor||proof);
END
$migration$;
