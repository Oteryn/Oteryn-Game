-- Endgame consumers reuse the native party-kill owner's existing tagged Instance
-- key (53 bytes, tag 2). Preserve every other receipt constraint verbatim.
DO $$
DECLARE c record; definition text; updated integer := 0;
BEGIN
    FOR c IN SELECT oid,conrelid,conname FROM pg_constraint
        WHERE contype='c' AND conrelid IN (
            'game_character_soul_war_receipts'::regclass,
            'game_character_rotten_blood_receipts'::regclass)
        ORDER BY conrelid,conname
    LOOP
        definition := pg_get_constraintdef(c.oid);
        IF position('octet_length(source_death_key) = 52' IN definition) > 0 THEN
            definition := replace(definition, 'octet_length(source_death_key) = 52',
                '(octet_length(source_death_key) = 52 OR CASE WHEN octet_length(source_death_key) = 53 THEN get_byte(source_death_key,0) = 2 ELSE false END)');
            EXECUTE format('ALTER TABLE %s DROP CONSTRAINT %I',c.conrelid::regclass,c.conname);
            EXECUTE format('ALTER TABLE %s ADD CONSTRAINT %I %s',c.conrelid::regclass,c.conname,definition);
            updated := updated + 1;
        END IF;
    END LOOP;
    IF updated <> 2 THEN
        RAISE EXCEPTION 'expected two endgame native death receipt constraints, found %',updated;
    END IF;
END $$;
