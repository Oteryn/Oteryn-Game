-- CORPSE48 (owner decision 2026-11-17): every corpse container admits up to 48
-- entries, as in Tibia Global (Canary items.xml gives 119 corpses containersize=48;
-- Crystal gives The Corruptor's corpses 52113-52115 containersize 48).
-- Migrations 0242, 0533 and 2805 stay as history. Rewrite the installed trigger
-- body rather than copying an older CREATE FUNCTION, so every later proof arm stays.
DO $migration$
DECLARE body TEXT; needle CONSTANT TEXT := 'NEW.parent_item_instance_id) > 30 THEN';
BEGIN
    body := pg_get_functiondef('game_item_corpse_container_entry_proven()'::regprocedure);
    -- One ceiling from 0242 and one in 0533's nested-loot arm.
    IF (length(body) - length(replace(body, needle, ''))) / length(needle) <> 2 THEN
        RAISE EXCEPTION 'unexpected installed corpse ceiling body';
    END IF;
    EXECUTE replace(body, needle, 'NEW.parent_item_instance_id) > 48 THEN');
END
$migration$;

-- Corpse-decay retirement ordinals follow the corpse capacity; other causes keep
-- 2805's u64 ordinal range.
DO $migration$
DECLARE t TEXT;
BEGIN
    FOREACH t IN ARRAY ARRAY['game_item_decay_retire_reservations','game_item_decay_retire_receipts'] LOOP
        EXECUTE format('ALTER TABLE %I DROP CONSTRAINT game_retire_placement_ordinal', t);
        EXECUTE format('ALTER TABLE %I ADD CONSTRAINT game_retire_placement_ordinal CHECK (
            placement_ordinal BETWEEN 1 AND 18446744073709551615
            AND (cause_kind<>''CORPSE_DECAY'' OR placement_ordinal<=48))', t);
    END LOOP;
END
$migration$;
