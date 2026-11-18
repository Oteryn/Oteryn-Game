-- Crystal 201189f8 action_cat_basket.lua:32 creates the toy in portable Ground custody.
-- Reuse the native map Item/audit transaction and the existing Ground MOVE owner.
CREATE FUNCTION game_quest_floor_reward_before_insert() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.source_attributes->>'kind' IS DISTINCT FROM 'quest_floor_reward' THEN
        RETURN NEW;
    END IF;
    IF NEW.owner_kind IS NOT NULL
       OR NEW.blocks_movement OR NEW.blocks_projectile OR NEW.immovable_block_solid
       OR NEW.placement_key IS DISTINCT FROM (
           'quest-floor-v1/' || (NEW.source_attributes->>'session') || '/' ||
           (NEW.source_attributes->>'command')
       )
       OR NOT EXISTS (
           SELECT 1
           FROM game_character_quest_receipts q
           WHERE q.character_id = CASE
               WHEN NEW.source_attributes->>'character' ~ '^[0-9a-f]{32}$'
               THEN (NEW.source_attributes->>'character')::uuid END
             AND q.cause_id = CASE
               WHEN NEW.source_attributes->>'session' ~ '^[0-9a-f]{32}$'
               THEN (NEW.source_attributes->>'session')::uuid END
             AND q.cause_ordinal = CASE
               WHEN NEW.source_attributes->>'command' ~ '^[1-9][0-9]{0,19}$'
               THEN (NEW.source_attributes->>'command')::numeric END
             AND q.cause_kind = 'use'
             AND q.transition_key = NEW.source_attributes->>'transition'
       ) THEN
        RAISE EXCEPTION 'quest floor reward requires its exact committed USE' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END $$;

CREATE TRIGGER game_quest_floor_reward_before_insert
BEFORE INSERT ON game_native_map_item_receipts
FOR EACH ROW EXECUTE FUNCTION game_quest_floor_reward_before_insert();

CREATE FUNCTION game_quest_floor_reward_source(p_item UUID) RETURNS BOOLEAN
LANGUAGE sql STABLE AS $$
    SELECT EXISTS (
        SELECT 1
        FROM game_native_map_item_receipts r
        JOIN game_native_map_item_audit a USING (item_instance_id)
        JOIN game_item_ground_locations g USING (item_instance_id)
        JOIN game_item_instances i USING (item_instance_id)
        WHERE r.item_instance_id = p_item
          AND r.source_attributes->>'kind' = 'quest_floor_reward'
          AND a.transaction_id = r.transaction_id
          AND a.event_id = r.event_id
          AND a.envelope = r.source_intent
          AND a.created_xact_id = r.created_xact_id
          AND i.world_id = r.world_id AND i.minted_transaction_id = r.transaction_id
          AND i.lifecycle = 1 AND i.state_revision = 1 AND i.last_transaction_id IS NULL
          AND (i.definition_family, i.definition_production_key, i.definition_revision_ref, i.quantity)
            = (r.definition_family, r.definition_key, r.definition_revision, r.quantity)
          AND (g.world_id, g.channel_id, g.runtime_scope_ownership_generation, g.spatial_position,
               g.map_revision, g.content_revision, g.native_room_placement_context)
            = (r.world_id, r.channel_id, r.scope_generation, r.spatial_position,
               r.map_revision, r.content_revision, r.placement_context)
          AND g.corpse_ref = convert_to('quest-floor-v1', 'UTF8')
          AND g.stack_ordinal > 0
    )
$$;

-- Keep every admission predicate from the previous Ground source owner.
DO $$
DECLARE prior_body TEXT;
BEGIN
    SELECT prosrc INTO STRICT prior_body FROM pg_proc
      WHERE oid = 'game_item_loop_map_mint_source(uuid)'::regprocedure;
    EXECUTE format(
        'CREATE OR REPLACE FUNCTION game_item_loop_map_mint_source(p_item uuid) RETURNS boolean LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog, %I, pg_temp AS %L',
        current_schema(),
        'SELECT (' || regexp_replace(prior_body, ';[[:space:]]*$', '') ||
        ') OR game_quest_floor_reward_source(p_item)'
    );
    EXECUTE format('ALTER FUNCTION game_quest_floor_reward_before_insert() SET search_path = pg_catalog, %I, pg_temp', current_schema());
    EXECUTE format('ALTER FUNCTION game_quest_floor_reward_source(uuid) SET search_path = pg_catalog, %I, pg_temp', current_schema());
END $$;

REVOKE ALL ON FUNCTION game_quest_floor_reward_before_insert(), game_quest_floor_reward_source(uuid) FROM PUBLIC;
