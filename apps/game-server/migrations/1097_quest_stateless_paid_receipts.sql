-- Reuse QENGINE's bounded zero-effect receipt arm (1003 target sets), with the
-- paid-item proof supplied by questpc6's 1096 fee participant. General limits,
-- completion-only receipts and Father's separate atomic programme stay intact.
ALTER TABLE game_character_quest_receipts DROP CONSTRAINT game_character_quest_receipt_effects_shape;
ALTER TABLE game_character_quest_receipts ADD CONSTRAINT game_character_quest_receipt_effects_shape CHECK (
    cardinality(track_keys) BETWEEN 0 AND 8
    AND (cardinality(track_keys)>0 OR completes OR transition_key LIKE 'oteryn:quest-reward/%')
    AND (cardinality(track_keys)=0 OR (array_ndims(track_keys)=1 AND array_lower(track_keys,1)=1
        AND array_ndims(values_before)=1 AND array_lower(values_before,1)=1
        AND array_ndims(values_after)=1 AND array_lower(values_after,1)=1))
    AND cardinality(values_before)=cardinality(track_keys) AND cardinality(values_after)=cardinality(track_keys)
    AND array_position(track_keys,NULL) IS NULL AND array_position(values_before,NULL) IS NULL
    AND array_position(values_after,NULL) IS NULL);

CREATE FUNCTION game_quest_stateless_paid_complete() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE source game_quest_reward_sources%ROWTYPE;
BEGIN
    IF cardinality(NEW.track_keys)<>0 OR NEW.completes THEN RETURN NULL; END IF;
    SELECT * INTO source FROM game_quest_reward_sources
        WHERE character_id=NEW.character_id AND cause_id=NEW.cause_id
            AND cause_ordinal=NEW.cause_ordinal AND transition_key=NEW.transition_key;
    IF NOT FOUND OR NEW.cause_kind<>'command'
        OR NEW.created_xact_id<>pg_current_xact_id()
        OR source.created_xact_id<>pg_current_xact_id()
        OR source.plan->>'key' IS DISTINCT FROM NEW.transition_key
        OR source.plan->>'quest' IS DISTINCT FROM NEW.quest_key
        OR source.plan->'storage' IS DISTINCT FROM '[]'::jsonb
        OR source.plan->'repeatable' IS DISTINCT FROM 'true'::jsonb
        OR jsonb_typeof(source.plan->'fee') IS DISTINCT FROM 'object'
        OR jsonb_typeof(source.plan->'give') IS DISTINCT FROM 'object'
        OR source.mint_transaction_id IS NULL
        OR NOT EXISTS(SELECT 1 FROM game_quest_reward_fees fee
            WHERE fee.character_id=NEW.character_id AND fee.cause_id=NEW.cause_id
                AND fee.cause_ordinal=NEW.cause_ordinal AND fee.transition_key=NEW.transition_key
                AND fee.created_xact_id=pg_current_xact_id()
                AND fee.committed_character_revision=NEW.committed_character_revision
                AND source.plan->'fee'->>'gold'=fee.fee_gold_units::text
                AND source.plan->'fee'->>'include_bank'=fee.include_bank::text)
    THEN RAISE EXCEPTION 'stateless paid quest lacks exact same-transaction source, fee and MINT'
        USING ERRCODE='23514'; END IF;
    RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_quest_stateless_paid_complete AFTER INSERT ON game_character_quest_receipts
    DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_quest_stateless_paid_complete();
DO $migration$
BEGIN
    EXECUTE format('ALTER FUNCTION game_quest_stateless_paid_complete() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;
REVOKE ALL ON FUNCTION game_quest_stateless_paid_complete() FROM PUBLIC;
