-- W10/W11 descendants retain the same disjoint death-key namespaces as PARTY-XP-1.
-- Keep all same-transaction, creature/member, revision-chain and replay guards.
DO $migration$
DECLARE
    target REGCLASS;
    constraint_row RECORD;
    definition TEXT;
    changed INTEGER;
BEGIN
    FOREACH target IN ARRAY ARRAY[
        'game_character_soul_war_receipts'::regclass,
        'game_character_rotten_blood_receipts'::regclass
    ] LOOP
        changed := 0;
        FOR constraint_row IN
            SELECT conname, pg_get_constraintdef(oid) AS definition
            FROM pg_constraint
            WHERE conrelid=target AND contype='c'
              AND position('octet_length(source_death_key) = 52' IN pg_get_constraintdef(oid))>0
        LOOP
            IF position('get_byte(source_death_key, 0)' IN constraint_row.definition)>0 THEN
                changed := changed + 1;
                CONTINUE;
            END IF;
            -- Do not drop the surrounding Rotten Blood kind/source truth table.
            definition := replace(constraint_row.definition,
                'octet_length(source_death_key) = 52',
                '(octet_length(source_death_key) = 52 OR (octet_length(source_death_key) = 53 AND get_byte(source_death_key,0) = 2))');
            EXECUTE format('ALTER TABLE %s DROP CONSTRAINT %I', target, constraint_row.conname);
            EXECUTE format('ALTER TABLE %s ADD CONSTRAINT %I %s', target, constraint_row.conname, definition);
            changed := changed + 1;
        END LOOP;
        -- On reapplication the expanded constraint still contains the old
        -- alternative. Recognize the tagged branch instead of nesting it again.
        IF changed <> 1 THEN
            RAISE EXCEPTION 'Expected one native encounter source-death constraint on %', target;
        END IF;
    END LOOP;
END
$migration$;
