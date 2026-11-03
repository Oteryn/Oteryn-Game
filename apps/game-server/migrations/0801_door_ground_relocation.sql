-- M06-CLOSE: relocating a Ground root uses the existing item-loop receipt/lines,
-- current-session fence and insertion/removal proofs. No intermediate carried custody.
-- Both source and destination remain exactly proven by the existing trigger chain.
DO $$
DECLARE c TEXT;
BEGIN
    SELECT conname INTO STRICT c FROM pg_constraint
     WHERE conrelid = 'game_item_loop_move_lines'::regclass AND contype = 'c'
       AND pg_get_constraintdef(oid) = 'CHECK (((from_kind <> 3) OR (to_kind = ANY (ARRAY[0, 1, 2]))))';
    EXECUTE format('ALTER TABLE game_item_loop_move_lines DROP CONSTRAINT %I', c);
    SELECT conname INTO STRICT c FROM pg_constraint
     WHERE conrelid = 'game_item_loop_move_lines'::regclass AND contype = 'c'
       AND pg_get_constraintdef(oid) = 'CHECK (((to_kind <> 3) OR (from_kind = ANY (ARRAY[0, 1, 2]))))';
    EXECUTE format('ALTER TABLE game_item_loop_move_lines DROP CONSTRAINT %I', c);
END $$;
ALTER TABLE game_item_loop_move_lines
    ADD CONSTRAINT game_item_loop_ground_source_destination
        CHECK (from_kind <> 3 OR to_kind IN (0,1,2,3)),
    ADD CONSTRAINT game_item_loop_ground_destination_source
        CHECK (to_kind <> 3 OR from_kind IN (0,1,2,3));
