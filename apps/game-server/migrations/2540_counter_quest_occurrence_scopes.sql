-- Counter-owned quest cycles span multiple server saves. Attach scope to the
-- existing fresh Quest receipt projection; never backfill guessed legacy cycles.
CREATE TABLE game_character_counter_quest_scopes (
    character_id uuid NOT NULL,
    committed_character_revision numeric(20,0) NOT NULL,
    world_id uuid NOT NULL,
    change_key text NOT NULL CHECK (change_key IN ('hive_born','deeplings')),
    occurrence_generation numeric(20,0) NOT NULL
        CHECK (occurrence_generation BETWEEN 1 AND 18446744073709551615),
    PRIMARY KEY(character_id,committed_character_revision),
    FOREIGN KEY(character_id,committed_character_revision)
        REFERENCES game_character_event_quest_progress(character_id,committed_character_revision),
    FOREIGN KEY(world_id,change_key) REFERENCES game_world_change_counters(world_id,change_key)
);
CREATE INDEX game_counter_quest_scopes_cycle ON game_character_counter_quest_scopes
    (world_id,change_key,occurrence_generation,character_id,committed_character_revision DESC);

CREATE FUNCTION game_counter_quest_scope_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP <> 'INSERT' THEN
        RAISE EXCEPTION 'counter quest scope is immutable' USING ERRCODE='23514';
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM game_character_event_quest_progress p
        JOIN game_character_quest_receipts r USING(character_id,committed_character_revision)
        JOIN game_world_reset_active a ON a.world_id=p.world_id AND a.reset_epoch=p.reset_epoch
        JOIN game_world_change_counters c ON c.world_id=a.world_id AND c.reset_epoch=a.reset_epoch
            AND c.change_key=NEW.change_key
        WHERE p.character_id=NEW.character_id
          AND p.committed_character_revision=NEW.committed_character_revision
          AND p.world_id=NEW.world_id
          AND r.created_xact_id=pg_current_xact_id()
          AND p.quest_key=CASE NEW.change_key
              WHEN 'hive_born' THEN 'oteryn:quest.hive_native_shared'
              WHEN 'deeplings' THEN 'oteryn:quest.deeplings_native_shared' END
          AND COALESCE((c.state->>'occurrence_generation')::numeric,1)=NEW.occurrence_generation
        FOR SHARE OF c
    ) THEN
        RAISE EXCEPTION 'counter scope requires a fresh receipt in the current source cycle' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END $$;
CREATE TRIGGER game_counter_quest_scope_guard BEFORE INSERT OR UPDATE OR DELETE
    ON game_character_counter_quest_scopes FOR EACH ROW EXECUTE FUNCTION game_counter_quest_scope_guard();

CREATE FUNCTION game_event_quest_capture_counter_scope() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    counter_key text;
    generation numeric(20,0);
BEGIN
    counter_key := CASE NEW.quest_key
        WHEN 'oteryn:quest.hive_native_shared' THEN 'hive_born'
        WHEN 'oteryn:quest.deeplings_native_shared' THEN 'deeplings' END;
    IF counter_key IS NULL THEN RETURN NEW; END IF;
    SELECT COALESCE((c.state->>'occurrence_generation')::numeric,1) INTO generation
        FROM game_world_change_counters c
        WHERE c.world_id=NEW.world_id AND c.change_key=counter_key AND c.reset_epoch=NEW.reset_epoch
        FOR SHARE;
    IF generation IS NULL THEN
        RAISE EXCEPTION 'counter quest requires a current source cycle' USING ERRCODE='23514';
    END IF;
    INSERT INTO game_character_counter_quest_scopes
        VALUES(NEW.character_id,NEW.committed_character_revision,NEW.world_id,counter_key,generation);
    RETURN NEW;
END $$;
CREATE TRIGGER game_event_quest_capture_counter_scope AFTER INSERT
    ON game_character_event_quest_progress FOR EACH ROW EXECUTE FUNCTION game_event_quest_capture_counter_scope();

-- The counter must first be advanced to the active save. Progress then remains
-- current across saves until the source lifecycle increments its generation.
CREATE VIEW game_current_counter_quest_progress AS
    SELECT p.*,s.change_key,s.occurrence_generation
    FROM game_character_event_quest_progress p
    JOIN game_character_counter_quest_scopes s USING(character_id,committed_character_revision,world_id)
    JOIN game_world_change_counters c ON c.world_id=s.world_id AND c.change_key=s.change_key
        AND COALESCE((c.state->>'occurrence_generation')::numeric,1)=s.occurrence_generation
    JOIN game_world_reset_active a ON a.world_id=c.world_id AND a.reset_epoch=c.reset_epoch;
REVOKE ALL ON game_character_counter_quest_scopes,game_current_counter_quest_progress FROM PUBLIC;
GRANT SELECT,INSERT ON game_character_counter_quest_scopes TO oteryn_game_runtime;
GRANT SELECT ON game_current_counter_quest_progress TO oteryn_game_runtime;
GRANT SELECT ON game_character_counter_quest_scopes,game_current_counter_quest_progress TO oteryn_game_control;
