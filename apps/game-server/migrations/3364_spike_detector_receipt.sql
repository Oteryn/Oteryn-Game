-- The first two surveys attest custody without an item mutation. Keep the
-- instance and the wire intent on the immutable reward occurrence for replay.
ALTER TABLE game_quest_reward_sources
    ADD COLUMN retained_detector UUID REFERENCES game_item_instances(item_instance_id),
    ADD COLUMN detector_intent BYTEA,
    ADD CHECK (
        (retained_detector IS NULL AND detector_intent IS NULL)
        OR (retained_detector IS NOT NULL AND detector_intent IS NOT NULL
            AND octet_length(detector_intent) BETWEEN 1 AND 4096
            AND transition_key IN ('oteryn:quest-reward/l12-spike-tracking-survey-1',
                                   'oteryn:quest-reward/l12-spike-tracking-survey-2'))
    );

DO $migration$
DECLARE
    body TEXT := pg_get_functiondef('game_quest_carried_use_complete()'::regprocedure);
    anchor CONSTANT TEXT := '    IF NOT (NEW.plan ? ''carried_item'') THEN';
BEGIN
    IF (length(body) - length(replace(body, anchor, ''))) / length(anchor) <> 1 THEN
        RAISE EXCEPTION 'missing carried tool guard';
    END IF;
    EXECUTE replace(body, anchor, $proof$
    IF NEW.retained_detector IS NOT NULL THEN
        IF NEW.created_xact_id <> pg_current_xact_id()
            OR NEW.plan->>'carried_item' IS DISTINCT FROM 'oteryn:item.tibia.i19205'
            OR NEW.plan->'take' IS DISTINCT FROM '[]'::jsonb
            OR NEW.plan->'give' IS DISTINCT FROM 'null'::jsonb
            OR NEW.plan->'present' IS DISTINCT FROM '[{"item":"oteryn:item.tibia.i19205","count":1}]'::jsonb
            OR NOT EXISTS (
                SELECT 1 FROM game_character_roots r
                CROSS JOIN LATERAL game_quest_callback_carried(
                    r.character_id, r.world_id, NEW.retained_detector) c
                JOIN game_item_instances i ON i.item_instance_id = c.item
                WHERE r.character_id = NEW.character_id AND c.item = NEW.retained_detector
                  AND i.lifecycle = 1 AND i.definition_family = 'Item'
                  AND i.definition_production_key = NEW.plan->>'carried_item'
            ) THEN
            RAISE EXCEPTION 'survey lacks retained detector' USING ERRCODE='23514';
        END IF;
        RETURN NULL;
    END IF;
$proof$ || anchor);
END $migration$;
