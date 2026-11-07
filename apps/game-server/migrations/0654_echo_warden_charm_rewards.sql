-- W06: Crystal 201189f src/io/iobestiary.cpp:408-478, one major Charm reward per species.
-- Existing Charm state derives earnings from this immutable ledger; no second currency owner.
-- Forward-only rollback: stop routing Warden credits, retain earnings and replay receipts.
CREATE TABLE game_character_echo_warden_kill_receipts (
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    occurrence_id UUID NOT NULL CHECK (game_character_is_uuid_v7(occurrence_id)),
    species_key TEXT COLLATE "C" NOT NULL CHECK (
        species_key ~ '^oteryn:creature\.[a-z0-9_.-]+$' AND octet_length(species_key) <= 160),
    bestiary_stars SMALLINT NOT NULL CHECK (bestiary_stars BETWEEN 0 AND 255),
    first_species BOOLEAN NOT NULL,
    charm_points SMALLINT NOT NULL CHECK (charm_points BETWEEN 0 AND 30),
    created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
    PRIMARY KEY (character_id, occurrence_id),
    CHECK (charm_points = CASE WHEN NOT first_species THEN 0 ELSE
        CASE bestiary_stars WHEN 0 THEN 1 WHEN 1 THEN 2 WHEN 2 THEN 5
            WHEN 3 THEN 10 WHEN 4 THEN 15 WHEN 5 THEN 30 ELSE 0 END END)
);

CREATE TABLE game_character_echo_warden_rewards (
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
    species_key TEXT COLLATE "C" NOT NULL CHECK (
        species_key ~ '^oteryn:creature\.[a-z0-9_.-]+$' AND octet_length(species_key) <= 160),
    first_occurrence_id UUID NOT NULL,
    bestiary_stars SMALLINT NOT NULL CHECK (bestiary_stars BETWEEN 0 AND 255),
    charm_points SMALLINT NOT NULL CHECK (charm_points BETWEEN 0 AND 30),
    created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
    PRIMARY KEY (character_id, species_key),
    UNIQUE (character_id, first_occurrence_id),
    FOREIGN KEY (character_id, first_occurrence_id)
        REFERENCES game_character_echo_warden_kill_receipts(character_id, occurrence_id),
    CHECK (charm_points = CASE bestiary_stars WHEN 0 THEN 1 WHEN 1 THEN 2 WHEN 2 THEN 5
        WHEN 3 THEN 10 WHEN 4 THEN 15 WHEN 5 THEN 30 ELSE 0 END)
);

CREATE FUNCTION game_echo_warden_reward_consistent() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE receipt game_character_echo_warden_kill_receipts%ROWTYPE;
        reward game_character_echo_warden_rewards%ROWTYPE;
        occurrence UUID;
BEGIN
    IF TG_TABLE_NAME='game_character_echo_warden_rewards' THEN
        occurrence := NEW.first_occurrence_id;
    ELSE
        occurrence := NEW.occurrence_id;
    END IF;
    SELECT * INTO receipt FROM game_character_echo_warden_kill_receipts
        WHERE character_id=NEW.character_id AND occurrence_id=occurrence;
    SELECT * INTO reward FROM game_character_echo_warden_rewards
        WHERE character_id=receipt.character_id AND species_key=receipt.species_key;
    -- A later zero-point receipt for species A must never mint a new ledger for B.
    -- Check the inserted reward itself, not merely A's already-existing reward.
    IF TG_TABLE_NAME='game_character_echo_warden_rewards' THEN
        IF NOT receipt.first_species
            OR NEW.species_key IS DISTINCT FROM receipt.species_key
            OR NEW.bestiary_stars IS DISTINCT FROM receipt.bestiary_stars
            OR NEW.charm_points IS DISTINCT FROM receipt.charm_points
            OR NEW.created_xact_id IS DISTINCT FROM receipt.created_xact_id
        THEN
            RAISE EXCEPTION 'Echo Warden reward must name its exact first-species receipt' USING ERRCODE='23514';
        END IF;
    END IF;
    IF receipt.character_id IS NULL OR reward.character_id IS NULL
        OR receipt.created_xact_id<>pg_current_xact_id()
        OR receipt.first_species IS DISTINCT FROM (reward.first_occurrence_id=occurrence)
        OR (receipt.first_species AND (
            reward.created_xact_id<>pg_current_xact_id()
            OR reward.bestiary_stars<>receipt.bestiary_stars
            OR reward.charm_points<>receipt.charm_points))
    THEN
        RAISE EXCEPTION 'Echo Warden reward and first-species receipt differ' USING ERRCODE='23514';
    END IF;
    RETURN NULL;
END $$;

CREATE CONSTRAINT TRIGGER game_echo_warden_kill_consistent
    AFTER INSERT ON game_character_echo_warden_kill_receipts
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_echo_warden_reward_consistent();
CREATE CONSTRAINT TRIGGER game_echo_warden_reward_consistent
    AFTER INSERT ON game_character_echo_warden_rewards
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
    EXECUTE FUNCTION game_echo_warden_reward_consistent();

CREATE TRIGGER game_echo_warden_kills_immutable
    BEFORE UPDATE OR DELETE ON game_character_echo_warden_kill_receipts
    FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
CREATE TRIGGER game_echo_warden_kills_no_truncate
    BEFORE TRUNCATE ON game_character_echo_warden_kill_receipts
    EXECUTE FUNCTION game_character_reject_truncate();
CREATE TRIGGER game_echo_warden_rewards_immutable
    BEFORE UPDATE OR DELETE ON game_character_echo_warden_rewards
    FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
CREATE TRIGGER game_echo_warden_rewards_no_truncate
    BEFORE TRUNCATE ON game_character_echo_warden_rewards
    EXECUTE FUNCTION game_character_reject_truncate();

REVOKE ALL ON game_character_echo_warden_kill_receipts,
    game_character_echo_warden_rewards FROM PUBLIC;
GRANT SELECT, INSERT ON game_character_echo_warden_kill_receipts,
    game_character_echo_warden_rewards TO oteryn_game_runtime;
REVOKE ALL ON FUNCTION game_echo_warden_reward_consistent() FROM PUBLIC;

DO $$ BEGIN
    EXECUTE format('ALTER FUNCTION game_echo_warden_reward_consistent() SET search_path=pg_catalog,%I,pg_temp', current_schema());
END $$;
