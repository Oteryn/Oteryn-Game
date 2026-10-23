-- BED-REGEN-1: extend the ordered runtime projection, not CharacterRevision.
-- NULL is a legacy snapshot or respawn: no durable food/Soul exists to debit.
ALTER TABLE game_character_last_positions ADD COLUMN rest INTEGER[],
    ADD CONSTRAINT game_character_last_positions_rest CHECK (
        rest IS NULL OR (cardinality(rest)=4 AND array_ndims(rest)=1 AND array_lower(rest,1)=1
            AND array_length(rest,1)=4 AND array_position(rest,NULL) IS NULL
            AND rest[1] BETWEEN 0 AND 16383 AND rest[2] BETWEEN 0 AND 1200000
            AND rest[3] BETWEEN 0 AND 12000 AND rest[4] BETWEEN 0 AND 6000));

-- Stamp fresh acquisition in the DB; settlement retries cannot grow the award.
CREATE TABLE game_bed_admission_times (
    game_session_id UUID PRIMARY KEY REFERENCES game_durability_fresh_admission_receipts(game_session_id),
    admitted_at BIGINT NOT NULL CHECK (admitted_at >= 0)
);
REVOKE ALL ON game_bed_admission_times FROM PUBLIC, oteryn_game_runtime, oteryn_game_control;
CREATE TRIGGER game_bed_admission_times_immutable BEFORE UPDATE OR DELETE OR TRUNCATE
    ON game_bed_admission_times FOR EACH STATEMENT
    EXECUTE FUNCTION game_house_bed_sleep_receipt_immutable();
CREATE FUNCTION game_bed_admission_clock() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
    INSERT INTO game_bed_admission_times VALUES(NEW.game_session_id,
        floor(extract(epoch FROM clock_timestamp())*1000)::BIGINT);
    RETURN NULL;
END $$;
CREATE TRIGGER game_bed_admission_clock AFTER INSERT ON game_durability_fresh_admission_receipts
    FOR EACH ROW EXECUTE FUNCTION game_bed_admission_clock();

CREATE TABLE game_bed_regen_receipts (
    game_session_id UUID PRIMARY KEY REFERENCES game_durability_reconnect_sessions,
    character_id UUID NOT NULL REFERENCES game_character_roots,
    slept_at BIGINT NOT NULL CHECK (slept_at>=0),
    settled_at BIGINT NOT NULL CHECK (settled_at>=0),
    health INTEGER,
    mana INTEGER,
    rest INTEGER[]
);
REVOKE ALL ON game_bed_regen_receipts FROM PUBLIC, oteryn_game_runtime, oteryn_game_control;
GRANT SELECT ON game_bed_regen_receipts TO oteryn_game_runtime;
CREATE TRIGGER game_bed_regen_receipts_immutable BEFORE UPDATE OR DELETE OR TRUNCATE
    ON game_bed_regen_receipts FOR EACH STATEMENT
    EXECUTE FUNCTION game_house_bed_sleep_receipt_immutable();

-- Read and lock only this Character's sleeper. Native fence checks precede this
-- helper in the caller transaction. No direct sleeper/revision writes are granted.
CREATE FUNCTION game_bed_regen_lock(p_session UUID, p_character UUID, p_generation NUMERIC, p_lease NUMERIC)
RETURNS TABLE(slept_at BIGINT, freed_at BIGINT, lease TEXT, wake_at BIGINT, eligible BOOLEAN)
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE bed game_house_bed_sleepers; previous UUID; previous_lease NUMERIC; acquired BIGINT;
BEGIN
    PERFORM 1 FROM game_durability_reconnect_sessions
        WHERE game_session_id=p_session AND character_id=p_character AND current_generation=p_generation
          AND character_lease_generation=p_lease AND session_state IN (1,2) FOR SHARE;
    IF NOT FOUND THEN RAISE EXCEPTION 'BED_REGEN_STALE' USING ERRCODE='23514'; END IF;
    IF EXISTS(SELECT 1 FROM game_bed_regen_receipts WHERE game_session_id=p_session) THEN RETURN; END IF;
    SELECT * INTO bed FROM game_house_bed_sleepers WHERE character_id=p_character;
    IF NOT FOUND OR bed.lease_generation=p_lease THEN RETURN; END IF;
    PERFORM 1 FROM game_house_ownership WHERE world_id=bed.world_id AND house_key=bed.house_key FOR SHARE;
    PERFORM 1 FROM game_house_bed_revisions WHERE world_id=bed.world_id AND house_key=bed.house_key FOR UPDATE;
    IF NOT FOUND THEN RAISE EXCEPTION 'BED_REGEN_REVISION_MISSING' USING ERRCODE='23514'; END IF;
    SELECT * INTO STRICT bed FROM game_house_bed_sleepers WHERE character_id=p_character FOR UPDATE;
    SELECT s.game_session_id,s.character_lease_generation INTO previous,previous_lease
        FROM game_durability_reconnect_sessions s WHERE s.character_id=p_character
          AND s.character_lease_generation<p_lease
        ORDER BY s.character_lease_generation DESC LIMIT 1;
    SELECT t.admitted_at INTO acquired FROM game_bed_admission_times t WHERE t.game_session_id=p_session;
    RETURN QUERY SELECT bed.slept_at,bed.freed_at,bed.lease_generation::TEXT,acquired,
        acquired IS NOT NULL AND acquired>=bed.slept_at AND previous_lease=bed.lease_generation
        AND EXISTS(SELECT 1 FROM game_house_bed_sleep_receipts r
            JOIN game_durability_reconnect_sessions s USING(game_session_id)
            WHERE r.game_session_id=previous AND s.session_state=3 AND s.world_id=bed.world_id
              AND r.slept_at=bed.slept_at AND r.binding->>0=p_character::TEXT
              AND r.binding->>1=bed.world_id::TEXT AND r.binding->>2=bed.house_key
              AND (r.binding->>4)::NUMERIC=bed.lease_generation
              AND (r.binding->>7)::INTEGER=bed.bed_x AND (r.binding->>8)::INTEGER=bed.bed_y
              AND (r.binding->>9)::INTEGER=bed.bed_z AND r.binding->>10=bed.occupied_variant);
END $$;

CREATE FUNCTION game_bed_regen_finish(p_session UUID, p_character UUID, p_generation NUMERIC, p_lease NUMERIC)
RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE locked RECORD;
BEGIN
    SELECT * INTO locked FROM game_bed_regen_lock(p_session,p_character,p_generation,p_lease);
    IF NOT FOUND THEN RETURN; END IF;
    INSERT INTO game_bed_regen_receipts
        SELECT p_session,p_character,locked.slept_at,
            coalesce(locked.wake_at,floor(extract(epoch FROM clock_timestamp())*1000)::BIGINT),
            p.health,p.mana,p.rest
        FROM (SELECT 1) seed LEFT JOIN game_character_last_positions p ON p.character_id=p_character;
    DELETE FROM game_house_bed_sleepers WHERE character_id=p_character;
END $$;
REVOKE ALL ON FUNCTION game_bed_admission_clock(), game_bed_regen_lock(UUID,UUID,NUMERIC,NUMERIC),
    game_bed_regen_finish(UUID,UUID,NUMERIC,NUMERIC) FROM PUBLIC, oteryn_game_runtime, oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_bed_regen_lock(UUID,UUID,NUMERIC,NUMERIC),
    game_bed_regen_finish(UUID,UUID,NUMERIC,NUMERIC) TO oteryn_game_runtime;
DO $$
BEGIN
    EXECUTE format('ALTER FUNCTION game_bed_admission_clock() SET search_path=pg_catalog,%I,pg_temp',current_schema());
    EXECUTE format('ALTER FUNCTION game_bed_regen_lock(UUID,UUID,NUMERIC,NUMERIC) SET search_path=pg_catalog,%I,pg_temp',current_schema());
    EXECUTE format('ALTER FUNCTION game_bed_regen_finish(UUID,UUID,NUMERIC,NUMERIC) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
