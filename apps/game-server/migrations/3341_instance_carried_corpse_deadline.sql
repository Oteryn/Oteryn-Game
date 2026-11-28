-- Carried corpse stages follow the current private activity session. The
-- original MINT qualifies the stage, while current rows prove its timer owner.
CREATE FUNCTION game_instance_carried_corpse_timer_current(
    p_item UUID, p_stage INTEGER, p_world UUID, p_instance UUID, p_generation NUMERIC,
    p_character UUID, p_session UUID, p_lease NUMERIC, p_connection NUMERIC
) RETURNS BOOLEAN LANGUAGE sql STABLE STRICT AS $$
    SELECT EXISTS (
        SELECT 1
        FROM game_durability_reconnect_sessions s
        JOIN game_durability_admission_character_guards cg USING(character_id)
        JOIN game_durability_admission_account_guards ag ON ag.account_id=s.account_id
        JOIN game_character_roots c ON c.character_id=s.character_id
        JOIN game_runtime_scope_assignments a ON a.scope_key=game_activity_scope_key(p_world,p_instance)
        -- Activity assignments use tag 3. Instance runtime guards use tag 2.
        JOIN game_durability_admission_runtime_guards rg ON rg.scope_key=set_byte(a.scope_key,0,2)
        WHERE s.game_session_id=p_session AND s.character_id=p_character AND s.world_id=p_world
          AND s.session_state IN (1,2) AND s.current_generation=p_connection AND s.character_lease_generation=p_lease
          AND s.runtime_scope_kind=2 AND s.runtime_scope_world_id=p_world AND s.runtime_scope_instance_id=p_instance
          AND s.runtime_scope_channel_id IS NULL AND s.runtime_scope_house_key IS NULL
          AND s.scope_ownership_generation=p_generation
          AND a.state=1 AND a.ownership_generation=p_generation
          AND rg.ready AND rg.ownership_generation=p_generation
          AND cg.world_id=p_world AND cg.account_id=s.account_id AND cg.eligible
          AND cg.lease_generation=p_lease AND cg.holder_game_session_id=p_session
          AND ag.presence_character_id=p_character AND ag.holder_game_session_id=p_session
          AND c.world_id=p_world AND c.account_id=s.account_id AND c.lifecycle=1
          AND s.activity_handoff_id IS NOT NULL AND game_activity_session_origin_valid(p_session,p_character)
          AND NOT EXISTS (SELECT 1 FROM game_activity_runtime_returns r
              WHERE r.handoff_id=s.activity_handoff_id AND r.character_id=p_character)
          AND EXISTS (
              SELECT 1 FROM game_spell_field_temporal_schedules p
              JOIN game_item_mint_receipts m ON m.item_instance_id=p.item_instance_id
                 AND m.transaction_id=p.source_transaction_id
              WHERE p.item_instance_id=p_item AND p.source_stage=p_stage AND p.source_ordinal=0
                AND m.death_world_id=p_world AND m.loot_purpose_key='CORPSE_MATERIALIZATION'
                AND m.draw_ordinal=0 AND m.materialized_at IS NOT NULL
          )
    )
$$;
REVOKE ALL ON FUNCTION game_instance_carried_corpse_timer_current(UUID,INTEGER,UUID,UUID,NUMERIC,UUID,UUID,NUMERIC,NUMERIC) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_instance_carried_corpse_timer_current(UUID,INTEGER,UUID,UUID,NUMERIC,UUID,UUID,NUMERIC,NUMERIC) TO oteryn_game_runtime;

ALTER TABLE game_spell_field_temporal_receipts
    DROP CONSTRAINT game_field_timer_exact_scope,
    ADD CONSTRAINT game_field_timer_exact_scope CHECK (
        (channel_id IS NOT NULL AND instance_id IS NULL)
        OR (channel_id IS NULL AND instance_id IS NOT NULL AND game_character_is_uuid_v7(instance_id))
    );

-- The existing capture and outcome guards prove the exact carried custody.
-- Check activity ownership separately so both Ground and Channel proofs stay intact.
DO $carried_instance_timer$
DECLARE
    body TEXT := pg_get_functiondef('game_spell_field_temporal_receipt_proven()'::regprocedure);
    anchor CONSTANT TEXT := 'IF NEW.custody_kind<>1 AND NOT EXISTS(';
BEGIN
    IF (length(body)-length(replace(body,anchor,'')))/length(anchor) <> 1 THEN
        RAISE EXCEPTION 'Carried corpse timer session proof changed';
    END IF;
    EXECUTE replace(body,anchor,$prove$
    IF NEW.custody_kind<>1 AND NEW.instance_id IS NOT NULL
      AND game_instance_carried_corpse_timer_current(NEW.item_instance_id,NEW.source_stage,NEW.world_id,
          NEW.instance_id,NEW.ownership_generation,NEW.character_id,NEW.game_session_id,
          NEW.character_lease_generation,NEW.connection_generation) IS NOT TRUE THEN
      RAISE EXCEPTION 'carried Instance corpse timer requires its current Character/session/scope' USING ERRCODE='23514';
    END IF;
    IF NEW.custody_kind<>1 AND NEW.instance_id IS NULL AND NOT EXISTS($prove$);
    EXECUTE format('ALTER FUNCTION game_instance_carried_corpse_timer_current(UUID,INTEGER,UUID,UUID,NUMERIC,UUID,UUID,NUMERIC,NUMERIC) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $carried_instance_timer$;
