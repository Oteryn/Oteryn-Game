-- M16: personal NPC marks compose the existing fenced Character/Quest receipt.
CREATE TABLE game_character_quest_map_marks (
    character_id UUID NOT NULL,
    source_key TEXT NOT NULL CHECK (octet_length(source_key) BETWEEN 1 AND 128),
    quest_key TEXT NOT NULL CHECK (octet_length(quest_key) BETWEEN 1 AND 128),
    world_id UUID NOT NULL CHECK (game_node_is_uuid_v7(world_id)),
    x INTEGER NOT NULL CHECK (x BETWEEN 0 AND 65535),
    y INTEGER NOT NULL CHECK (y BETWEEN 0 AND 65535),
    floor SMALLINT NOT NULL CHECK (floor BETWEEN -15 AND 0),
    kind TEXT NOT NULL CHECK (kind = 'cross'),
    label TEXT NOT NULL CHECK (char_length(label) BETWEEN 1 AND 128
        AND octet_length(label) <= 256 AND label !~ '[[:cntrl:]]' AND btrim(label) <> ''),
    committed_character_revision NUMERIC(20,0) NOT NULL,
    created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
    PRIMARY KEY (character_id, source_key),
    FOREIGN KEY (character_id, quest_key)
        REFERENCES game_character_quest_states(character_id, quest_key),
    FOREIGN KEY (character_id, committed_character_revision)
        REFERENCES game_character_quest_receipts(character_id, committed_character_revision)
);
CREATE INDEX game_character_quest_map_marks_quest
    ON game_character_quest_map_marks(character_id, quest_key);
CREATE INDEX game_character_quest_map_marks_receipt
    ON game_character_quest_map_marks(character_id, committed_character_revision);

CREATE FUNCTION game_quest_map_mark_payload(mark game_character_quest_map_marks)
RETURNS JSONB LANGUAGE SQL IMMUTABLE SET search_path=pg_catalog,public AS $$
    SELECT jsonb_build_object('position',jsonb_build_object('x',(mark).x,'y',(mark).y,'floor',(mark).floor),
        'kind',(mark).kind,'text',(mark).label)
$$;

CREATE FUNCTION game_quest_map_mark_row_guard() RETURNS trigger
LANGUAGE plpgsql SET search_path=pg_catalog,public AS $$
BEGIN
    IF TG_OP <> 'INSERT' THEN
        RAISE EXCEPTION 'quest map marks are immutable source facts' USING ERRCODE='23514';
    END IF;
    IF NEW.created_xact_id <> pg_current_xact_id() OR NOT EXISTS (
        SELECT 1 FROM game_character_quest_receipts r
        JOIN game_durability_reconnect_sessions s ON s.game_session_id=r.cause_id
        WHERE r.character_id=NEW.character_id AND r.transition_key=NEW.source_key
          AND r.quest_key=NEW.quest_key AND r.cause_kind='command'
          AND r.committed_character_revision=NEW.committed_character_revision
          AND r.created_xact_id=pg_current_xact_id()
          AND s.character_id=NEW.character_id AND s.world_id=NEW.world_id
          AND s.session_state IN (1,2)) THEN
        RAISE EXCEPTION 'quest map mark lacks same-transaction receipt and live session World' USING ERRCODE='23514';
    END IF;
    -- Every producer takes the existing Character root lock; a source cannot
    -- reserve markers by bypassing the normal Character/Quest transaction.
    PERFORM 1 FROM game_character_roots WHERE character_id=NEW.character_id FOR UPDATE;
    IF (SELECT count(*) FROM game_character_quest_map_marks WHERE character_id=NEW.character_id) >= 256 THEN
        RAISE EXCEPTION 'personal quest map mark capacity exceeded' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END $$;
CREATE TRIGGER game_quest_map_mark_row_guard BEFORE INSERT OR UPDATE OR DELETE
    ON game_character_quest_map_marks FOR EACH ROW EXECUTE FUNCTION game_quest_map_mark_row_guard();

CREATE FUNCTION game_quest_map_mark_complete() RETURNS trigger
LANGUAGE plpgsql SET search_path=pg_catalog,public AS $$
DECLARE expected JSONB;
BEGIN
    IF TG_TABLE_NAME='game_quest_reward_sources' THEN
        expected := NEW.plan->'map_mark';
        IF expected IS NULL OR expected='null'::jsonb THEN RETURN NEW; END IF;
        IF NOT EXISTS (SELECT 1 FROM game_character_quest_map_marks m
            JOIN game_character_quest_receipts r ON r.character_id=m.character_id
              AND r.cause_id=NEW.cause_id AND r.cause_ordinal=NEW.cause_ordinal
              AND r.transition_key=NEW.transition_key
            JOIN game_durability_reconnect_sessions session ON session.game_session_id=NEW.cause_id
            WHERE m.character_id=NEW.character_id AND m.source_key=NEW.transition_key
              AND m.quest_key=NEW.plan->>'quest' AND m.world_id=session.world_id
              AND m.committed_character_revision<=r.committed_character_revision
              AND game_quest_map_mark_payload(m)=expected
              AND r.created_xact_id=pg_current_xact_id()) THEN
            RAISE EXCEPTION 'quest source lacks exact atomic personal map mark' USING ERRCODE='23514';
        END IF;
    ELSE
        IF NOT EXISTS (SELECT 1 FROM game_quest_reward_sources s
            JOIN game_character_quest_receipts r
              USING(character_id,cause_id,cause_ordinal,transition_key)
            WHERE s.character_id=NEW.character_id AND s.transition_key=NEW.source_key
              AND r.committed_character_revision=NEW.committed_character_revision
              AND s.plan->>'quest'=NEW.quest_key
              AND s.plan->'map_mark'=game_quest_map_mark_payload(NEW)
              AND s.created_xact_id=pg_current_xact_id()) THEN
            RAISE EXCEPTION 'personal map mark lacks exact native dialogue source plan' USING ERRCODE='23514';
        END IF;
    END IF;
    RETURN NEW;
END $$;
CREATE CONSTRAINT TRIGGER game_quest_map_mark_complete
    AFTER INSERT ON game_character_quest_map_marks DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION game_quest_map_mark_complete();
CREATE CONSTRAINT TRIGGER game_quest_source_map_mark_complete
    AFTER INSERT ON game_quest_reward_sources DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION game_quest_map_mark_complete();

REVOKE ALL ON game_character_quest_map_marks FROM PUBLIC;
REVOKE ALL ON FUNCTION game_quest_map_mark_payload(game_character_quest_map_marks),
    game_quest_map_mark_row_guard(),game_quest_map_mark_complete() FROM PUBLIC;
GRANT SELECT,INSERT ON game_character_quest_map_marks TO oteryn_game_runtime;
GRANT SELECT ON game_character_quest_map_marks TO oteryn_game_control;

GRANT EXECUTE ON FUNCTION game_quest_map_mark_payload(game_character_quest_map_marks)
    TO oteryn_game_runtime,oteryn_game_control;
