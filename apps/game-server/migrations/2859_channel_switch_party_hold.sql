-- chsw (owner decision 2026-11-17): a Party and its chat survive a voluntary Channel switch.
-- The switch's safe exit is the ordinary terminal release, which removes the member from its
-- Party; a switch release holds the membership instead, for the time the fresh admission into
-- the target Channel needs. The admission into the same World takes the hold back; an expired
-- hold without a current session removes the member as the ordinary logout would.
CREATE TABLE game_channel_switch_party_holds (
    character_id UUID PRIMARY KEY REFERENCES game_party_members(character_id) ON DELETE CASCADE,
    held_until_ms BIGINT NOT NULL CHECK (held_until_ms > 0)
);
CREATE INDEX game_channel_switch_party_hold_deadlines
    ON game_channel_switch_party_holds (held_until_ms, character_id);
REVOKE ALL ON game_channel_switch_party_holds FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE, DELETE ON game_channel_switch_party_holds TO oteryn_game_runtime;
GRANT SELECT ON game_channel_switch_party_holds TO oteryn_game_control;

-- Two minutes from the committed safe exit to the fresh admission into the target Channel.
CREATE FUNCTION game_channel_switch_party_hold_ms() RETURNS BIGINT LANGUAGE sql IMMUTABLE AS $$
    SELECT 120000::BIGINT
$$;
REVOKE ALL ON FUNCTION game_channel_switch_party_hold_ms() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_channel_switch_party_hold_ms()
    TO oteryn_game_runtime, oteryn_game_control;

-- A committed admission ends the hold in its own transaction; a refused admission rolls back
-- and leaves the hold to run out.
CREATE FUNCTION game_channel_switch_party_return() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog, public AS $$
BEGIN
    DELETE FROM game_channel_switch_party_holds WHERE character_id = NEW.character_id;
    RETURN NULL;
END
$$;
REVOKE ALL ON FUNCTION game_channel_switch_party_return() FROM PUBLIC;
CREATE TRIGGER game_channel_switch_party_return
    AFTER INSERT ON game_durability_fresh_admission_receipts
    FOR EACH ROW EXECUTE FUNCTION game_channel_switch_party_return();
