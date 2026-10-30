-- PKG3 C14: advance the low-level experience declaration without changing gameplay values.
-- The bonus already follows Global (TibiaWiki Experience Point r1161628). This corrects
-- descriptive metadata only: no XP, CharacterRevision, policy, or historical receipt changes.
-- Preserve the latest-receipt proof for the exact legacy/current declaration pair. Rewriting
-- immutable receipts would change the historical interpretation of replayed occurrences.
DO $$
DECLARE
    definition text;
    previous text := 'AND c.declaration_revision = v_state.declaration_revision';
    corrected text := $comparison$AND (c.declaration_revision = v_state.declaration_revision
                   OR (c.declaration_revision = 'character-progression-differences-v1-c6d0829911772c01b4a229a5bb0aa8f9'
                       AND v_state.declaration_revision = 'character-progression-differences-v1-aa9d260834741d235ba9bd77e14888bf'))$comparison$;
BEGIN
    -- Extend the installed guard, including every receipt union added by earlier migrations.
    SELECT pg_get_functiondef('game_character_progression_consistency_guard()'::regprocedure)
      INTO STRICT definition;
    IF strpos(definition, corrected) = 0 THEN
        IF strpos(definition, previous) = 0 THEN
            RAISE EXCEPTION 'progression declaration comparison is missing';
        END IF;
        EXECUTE replace(definition, previous, corrected);
    END IF;
END $$;

-- The existing guard deliberately disallows metadata-only updates. Suspend only that guard
-- under the migration's exclusive table lock. The state transition trigger also requires
-- a new receipt for every UPDATE; this metadata correction creates no gameplay transition.
-- Root/receipt consistency triggers stay installed, using the exact pair above.
ALTER TABLE game_character_progression_state DISABLE TRIGGER game_character_progression_state_guard;
ALTER TABLE game_character_progression_state DISABLE TRIGGER game_character_state_progression_consistent;
UPDATE game_character_progression_state
   SET declaration_revision = 'character-progression-differences-v1-aa9d260834741d235ba9bd77e14888bf'
 WHERE declaration_revision = 'character-progression-differences-v1-c6d0829911772c01b4a229a5bb0aa8f9';
-- Flush deferred events before ALTER TABLE (PostgreSQL rejects ALTER with pending events).
SET CONSTRAINTS ALL IMMEDIATE;
ALTER TABLE game_character_progression_state ENABLE TRIGGER game_character_progression_state_guard;
ALTER TABLE game_character_progression_state ENABLE TRIGGER game_character_state_progression_consistent;
SET CONSTRAINTS ALL DEFERRED;
