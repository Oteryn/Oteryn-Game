-- Gift of Life remaining online cooldown. Crystal 201189f player_wheel.cpp:3952-4002.
-- The existing Character root/session fence serializes each owner snapshot.
CREATE TABLE game_character_wheel_gift_receipts (
    character_id uuid NOT NULL REFERENCES game_character_roots(character_id),
    occurrence uuid NOT NULL,
    request_binding bytea NOT NULL CHECK (octet_length(request_binding) = 32),
    before_revision bigint NOT NULL CHECK (before_revision >= 0),
    after_revision bigint NOT NULL CHECK (after_revision = before_revision + 1),
    remaining_micros bigint NOT NULL CHECK (remaining_micros BETWEEN 0 AND 108000000000),
    PRIMARY KEY (character_id, occurrence),
    UNIQUE (character_id, after_revision)
);
CREATE TABLE game_character_wheel_gift_state (
    character_id uuid PRIMARY KEY REFERENCES game_character_roots(character_id),
    revision bigint NOT NULL CHECK (revision > 0),
    remaining_micros bigint NOT NULL CHECK (remaining_micros BETWEEN 0 AND 108000000000),
    occurrence uuid NOT NULL,
    FOREIGN KEY (character_id, occurrence)
        REFERENCES game_character_wheel_gift_receipts(character_id, occurrence)
);
REVOKE ALL ON game_character_wheel_gift_receipts, game_character_wheel_gift_state FROM PUBLIC;
GRANT SELECT, INSERT ON game_character_wheel_gift_receipts TO oteryn_game_runtime;
GRANT SELECT, INSERT, UPDATE ON game_character_wheel_gift_state TO oteryn_game_runtime;

CREATE FUNCTION game_guard_wheel_gift_state() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path=pg_catalog,public,pg_temp AS $$
BEGIN
    IF (TG_OP = 'INSERT' AND NEW.revision <> 1)
       OR (TG_OP = 'UPDATE' AND (NEW.character_id <> OLD.character_id
                               OR NEW.revision <> OLD.revision + 1)) THEN
        RAISE EXCEPTION 'wheel gift revision mismatch';
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM public.game_character_wheel_gift_receipts r
        WHERE r.character_id=NEW.character_id AND r.occurrence=NEW.occurrence
          AND r.after_revision=NEW.revision AND r.remaining_micros=NEW.remaining_micros
    ) THEN
        RAISE EXCEPTION 'wheel gift receipt missing';
    END IF;
    RETURN NEW;
END;
$$;
REVOKE ALL ON FUNCTION game_guard_wheel_gift_state() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_guard_wheel_gift_state() TO oteryn_game_runtime;
CREATE TRIGGER game_character_wheel_gift_state_guard
BEFORE INSERT OR UPDATE ON game_character_wheel_gift_state
FOR EACH ROW EXECUTE FUNCTION game_guard_wheel_gift_state();
