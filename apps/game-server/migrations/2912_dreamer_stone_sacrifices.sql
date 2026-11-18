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
