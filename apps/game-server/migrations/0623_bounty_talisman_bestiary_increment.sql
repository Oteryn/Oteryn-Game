-- P05: Crystal player.cpp:6555-6561 doubles the Bestiary increment of one
-- credited death. Preserve one immutable receipt and one Character revision.
-- Existing receipts keep their original increment of one.
ALTER TABLE game_character_bestiary_kill_receipts
    ADD COLUMN kill_count_increment SMALLINT NOT NULL DEFAULT 1
    CONSTRAINT game_bestiary_kill_increment_bound CHECK (kill_count_increment IN (1, 2));

-- Migration 0019 used an automatically named CHECK. Resolve that exact
-- three-column count constraint from the catalogue, rather than depending on
-- PostgreSQL's generated identifier spelling or replacing the receipt guard.
DO $$
DECLARE
    v_constraint TEXT;
    v_after SMALLINT;
    v_before SMALLINT;
    v_threshold SMALLINT;
BEGIN
    SELECT attnum INTO STRICT v_after FROM pg_catalog.pg_attribute
     WHERE attrelid = 'game_character_bestiary_kill_receipts'::regclass AND attname = 'kill_count_after';
    SELECT attnum INTO STRICT v_before FROM pg_catalog.pg_attribute
     WHERE attrelid = 'game_character_bestiary_kill_receipts'::regclass AND attname = 'kill_count_before';
    SELECT attnum INTO STRICT v_threshold FROM pg_catalog.pg_attribute
     WHERE attrelid = 'game_character_bestiary_kill_receipts'::regclass AND attname = 'final_kill_threshold';
    SELECT conname INTO STRICT v_constraint FROM pg_catalog.pg_constraint
     WHERE conrelid = 'game_character_bestiary_kill_receipts'::regclass
       AND contype = 'c' AND cardinality(conkey) = 3
       AND conkey @> ARRAY[v_after, v_before, v_threshold]::smallint[];
    EXECUTE format('ALTER TABLE game_character_bestiary_kill_receipts DROP CONSTRAINT %I', v_constraint);
END;
$$;

ALTER TABLE game_character_bestiary_kill_receipts
    ADD CONSTRAINT game_bestiary_kill_count_increment_exact CHECK (
        kill_count_before < final_kill_threshold
        AND kill_count_after = LEAST(kill_count_before + kill_count_increment, final_kill_threshold)
    );
