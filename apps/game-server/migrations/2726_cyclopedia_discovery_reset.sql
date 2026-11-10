-- T187: only the same-transaction fenced reset receipt may remove subarea completion.
ALTER TABLE game_character_cyclopedia_receipts ADD COLUMN reset_subarea BIGINT;
ALTER TABLE game_character_cyclopedia_receipts ADD CONSTRAINT game_cyclopedia_reset_receipt CHECK (
 reset_subarea IS NULL OR (reset_subarea BETWEEN 1 AND 1024 AND donation=0
  AND committed_character_revision IS NOT NULL AND get_byte(result,0)=0
  AND get_byte(result,9)=0 AND get_byte(result,10)=0));
DO $$ DECLARE body TEXT; old_clause TEXT; BEGIN
 SELECT pg_get_functiondef('game_cyclopedia_state_guard()'::regprocedure) INTO body;
 old_clause := 'IF (entry.value->>''discovered'')::boolean AND NOT COALESCE((NEW.progress->''subareas''->entry.key->>''discovered'')::boolean,false) THEN';
 IF position(old_clause IN body)=0 THEN RAISE EXCEPTION 'expected canonical discovery guard'; END IF;
 body := replace(body,old_clause,
  'IF (entry.value->>''discovered'')::boolean AND NOT COALESCE((NEW.progress->''subareas''->entry.key->>''discovered'')::boolean,false)
   AND NOT EXISTS(SELECT 1 FROM game_character_cyclopedia_receipts r WHERE r.occurrence=NEW.occurrence
    AND r.character_id=NEW.character_id AND r.reset_subarea=entry.key::bigint
    AND r.committed_character_revision IS NOT NULL AND r.created_xact_id=pg_current_xact_id()) THEN');
 EXECUTE body;
END $$;
