-- A carried tool that summons a temporary creature stays in the player's custody.
CREATE TABLE game_quest_carried_presence (
    character_id UUID NOT NULL,
    cause_id UUID NOT NULL,
    cause_ordinal NUMERIC(20,0) NOT NULL,
    transition_key TEXT NOT NULL,
    item_instance_id UUID NOT NULL REFERENCES game_item_instances(item_instance_id),
    before_item JSONB NOT NULL,
    before_location JSONB NOT NULL,
    location_kind TEXT NOT NULL CHECK(location_kind IN ('backpack','container','nested','equipment')),
    ingress BYTEA NOT NULL CHECK(octet_length(ingress) BETWEEN 1 AND 4096),
    created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
    PRIMARY KEY(character_id,cause_id,cause_ordinal,transition_key),
    FOREIGN KEY(character_id,cause_id,cause_ordinal,transition_key)
        REFERENCES game_quest_reward_sources(character_id,cause_id,cause_ordinal,transition_key)
        DEFERRABLE INITIALLY DEFERRED
);
CREATE TRIGGER game_quest_carried_presence_immutable
    BEFORE UPDATE OR DELETE ON game_quest_carried_presence
    FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_quest_carried_presence_no_truncate
    BEFORE TRUNCATE ON game_quest_carried_presence
    EXECUTE FUNCTION game_item_reject_truncate();
REVOKE ALL ON game_quest_carried_presence FROM PUBLIC;
GRANT SELECT,INSERT ON game_quest_carried_presence TO oteryn_game_runtime;
GRANT SELECT ON game_quest_carried_presence TO oteryn_game_control;

DO $migration$
DECLARE body TEXT; anchor TEXT := $anchor$    IF jsonb_typeof(NEW.plan->'take') IS DISTINCT FROM 'array'$anchor$;
BEGIN
    body := pg_get_functiondef('game_quest_carried_use_complete()'::regprocedure);
    IF (length(body)-length(replace(body,anchor,'')))/length(anchor)<>1 THEN
        RAISE EXCEPTION 'missing carried quest use guard';
    END IF;
    EXECUTE replace(body,anchor,$arm$    IF NEW.plan ? 'temporary_creature' THEN
        IF NEW.created_xact_id<>pg_current_xact_id()
            OR jsonb_typeof(NEW.plan->'temporary_creature') IS DISTINCT FROM 'object'
            OR NEW.plan->'take' IS DISTINCT FROM '[]'::jsonb
            OR NOT game_quest_carried_output_valid(NEW.plan)
            OR (SELECT count(*) FROM jsonb_array_elements(NEW.plan->'present') p
                WHERE p->>'item'=NEW.plan->>'carried_item' AND p->>'count'='1'
                    AND p->>'fluid_subtype' IS NULL)<>1
            OR NOT EXISTS (
                SELECT 1
                FROM game_quest_carried_presence p
                JOIN game_item_instances i USING(item_instance_id)
                CROSS JOIN LATERAL game_quest_callback_carried(p.character_id,i.world_id) c
                WHERE (p.character_id,p.cause_id,p.cause_ordinal,p.transition_key)
                    =(NEW.character_id,NEW.cause_id,NEW.cause_ordinal,NEW.transition_key)
                  AND p.created_xact_id=pg_current_xact_id()
                  AND i.lifecycle=1 AND i.quantity>=1 AND i.definition_family='Item'
                  AND i.definition_production_key=NEW.plan->>'carried_item'
                  AND p.before_item=to_jsonb(i) AND c.item=i.item_instance_id
                  AND (c.location,c.kind)=(p.before_location,p.location_kind)) THEN
            RAISE EXCEPTION 'non-consuming carried quest use lacks exact selected-instance proof'
                USING ERRCODE='23514';
        END IF;
        RETURN NULL;
    END IF;
$arm$ || anchor);
END $migration$;
