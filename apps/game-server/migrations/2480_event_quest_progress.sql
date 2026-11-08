-- M55: personal event history belongs to a World occurrence, not a reward MINT epoch.
-- Each entry is a projection of a fresh, fenced Quest receipt in the same transaction.
CREATE TABLE game_character_event_quest_progress (
    character_id uuid NOT NULL,
    world_id uuid NOT NULL,
    reset_epoch numeric(20,0) NOT NULL,
    committed_character_revision numeric(20,0) NOT NULL,
    quest_key text NOT NULL,
    transition_key text NOT NULL,
    completes boolean NOT NULL,
    track_keys text[] NOT NULL,
    values_after bigint[] NOT NULL,
    PRIMARY KEY(character_id,committed_character_revision),
    FOREIGN KEY(character_id,committed_character_revision)
        REFERENCES game_character_quest_receipts(character_id,committed_character_revision),
    FOREIGN KEY(world_id,reset_epoch) REFERENCES game_world_reset_epochs(world_id,reset_epoch)
);
CREATE INDEX game_event_quest_progress_occurrence ON game_character_event_quest_progress
    (world_id,reset_epoch,character_id,quest_key,committed_character_revision DESC);
CREATE FUNCTION game_event_quest_progress_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP <> 'INSERT' THEN
        RAISE EXCEPTION 'event quest history is immutable' USING ERRCODE='23514';
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM game_character_quest_receipts r
        JOIN game_character_roots c ON c.character_id=r.character_id AND c.world_id=NEW.world_id
        JOIN game_world_reset_active a ON a.world_id=NEW.world_id AND a.reset_epoch=NEW.reset_epoch
        WHERE r.character_id=NEW.character_id
          AND r.committed_character_revision=NEW.committed_character_revision
          AND r.created_xact_id=pg_current_xact_id()
          AND (r.quest_key,r.transition_key,r.completes,r.track_keys,r.values_after)
              IS NOT DISTINCT FROM
              (NEW.quest_key,NEW.transition_key,NEW.completes,NEW.track_keys,NEW.values_after)
    ) THEN
        RAISE EXCEPTION 'event progress requires a fresh receipt and current occurrence' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END $$;
CREATE TRIGGER game_event_quest_progress_guard BEFORE INSERT OR UPDATE OR DELETE
    ON game_character_event_quest_progress FOR EACH ROW EXECUTE FUNCTION game_event_quest_progress_guard();
-- No row in a new occurrence means initial progress; history never leaks into the new save.
CREATE VIEW game_current_event_quest_progress AS
    SELECT p.* FROM game_character_event_quest_progress p
    JOIN game_world_reset_active a USING(world_id,reset_epoch);
REVOKE ALL ON game_character_event_quest_progress,game_current_event_quest_progress FROM PUBLIC;
GRANT SELECT,INSERT ON game_character_event_quest_progress TO oteryn_game_runtime;
GRANT SELECT ON game_current_event_quest_progress TO oteryn_game_runtime;
GRANT SELECT ON game_character_event_quest_progress,game_current_event_quest_progress TO oteryn_game_control;
