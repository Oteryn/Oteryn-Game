-- A paid ground STEP can preserve all progress while keeping its exact
-- Character revision, item consumption and replay receipt.
ALTER TABLE game_character_quest_receipts
    ADD COLUMN guard_only BOOLEAN NOT NULL DEFAULT FALSE;

CREATE OR REPLACE FUNCTION game_character_quest_receipt_guard() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
    v_root_revision NUMERIC(20,0);
    v_state game_character_quest_states%ROWTYPE;
    v_index INTEGER;
BEGIN
    SELECT character_revision INTO v_root_revision
      FROM game_character_roots WHERE character_id = NEW.character_id;
    IF NOT FOUND OR NEW.committed_character_revision <> v_root_revision THEN
        RAISE EXCEPTION 'quest receipt is not at the current Character root revision'
            USING ERRCODE = '23514';
    END IF;
    IF (SELECT count(DISTINCT k) FROM unnest(NEW.track_keys) AS k)
           <> cardinality(NEW.track_keys)
       OR EXISTS (SELECT 1 FROM unnest(NEW.track_keys) AS k
                   WHERE NOT (k ~ '^oteryn:[A-Za-z0-9._:/-]+$' AND octet_length(k) <= 128)) THEN
        RAISE EXCEPTION 'quest receipt effects must name distinct track keys'
            USING ERRCODE = '23514';
    END IF;
    -- The ground debit is still proved by the existing deferred item-custody
    -- guards. These values are admission evidence and are never progress writes.
    IF NEW.guard_only THEN
        IF NEW.cause_kind <> 'step'
           OR NEW.completes
           OR NEW.values_before <> NEW.values_after
           OR (SELECT count(*) FROM game_quest_callback_consumptions c
               WHERE c.character_id = NEW.character_id
                 AND c.cause_id = NEW.cause_id
                 AND c.cause_ordinal = NEW.cause_ordinal
                 AND c.transition_key = NEW.transition_key
                 AND c.location_kind = 'ground') < 2 THEN
            RAISE EXCEPTION 'guard-only quest receipt requires a ground-funded step'
                USING ERRCODE = '23514';
        END IF;
        RETURN NULL;
    END IF;
    FOR v_index IN 1 .. cardinality(NEW.track_keys) LOOP
        IF NOT EXISTS (
                SELECT 1 FROM game_character_quest_tracks t
                 WHERE t.character_id = NEW.character_id
                   AND t.track_key = NEW.track_keys[v_index]
                   AND t.quest_key = NEW.quest_key
                   AND t.value = NEW.values_after[v_index]
                   AND t.committed_character_revision = NEW.committed_character_revision) THEN
            RAISE EXCEPTION 'quest receipt effect has no track row of its quest written by it'
                USING ERRCODE = '23514';
        END IF;
    END LOOP;
    SELECT * INTO v_state FROM game_character_quest_states
     WHERE character_id = NEW.character_id AND quest_key = NEW.quest_key;
    IF NOT FOUND
       OR v_state.committed_character_revision <> NEW.committed_character_revision
       OR v_state.pinned_content_revision <> NEW.pinned_content_revision
       OR v_state.definition_hash <> NEW.definition_hash
       OR (NEW.completes AND v_state.completed_character_revision IS NULL) THEN
        RAISE EXCEPTION 'quest receipt has no quest state of its pin written by it'
            USING ERRCODE = '23514';
    END IF;
    IF NEW.cause_kind = 'claim_obligation' AND (
           NOT EXISTS (
               SELECT 1 FROM game_reward_claim_mint_receipts r
                WHERE r.game_session_id = NEW.cause_id
                  AND r.command_id = NEW.cause_ordinal
                  AND r.character_id = NEW.character_id)
           OR NOT EXISTS (
               SELECT 1 FROM game_character_quest_obligations o
                WHERE o.claim_game_session_id = NEW.cause_id
                  AND o.claim_command_id = NEW.cause_ordinal
                  AND o.character_id = NEW.character_id
                  AND o.transition_key = NEW.transition_key
                  AND o.state = 'CONSUMED')) THEN
        RAISE EXCEPTION 'quest obligation receipt must consume the obligation of its claim'
            USING ERRCODE = '23514';
    END IF;
    RETURN NULL;
END;
$$;
