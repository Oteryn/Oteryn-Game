CREATE FUNCTION game_npc_source_fee_cash_minimum(p_fee UUID,p_character UUID,p_world UUID) RETURNS BIGINT LANGUAGE sql SECURITY DEFINER AS $$
 SELECT coalesce((SELECT sum((npc_effect.value->>'gold')::bigint) FROM jsonb_array_elements(npc_policy.effects) npc_effect WHERE npc_effect.value->>'kind'='consume_money'),0)::bigint
 FROM game_npc_source_context npc_context JOIN game_npc_source_policies npc_policy ON npc_policy.program_sha256=npc_context.program_sha256 AND npc_policy.source_key=npc_context.source_key
 WHERE npc_context.backend_pid=pg_backend_pid() AND npc_context.transaction_xid=pg_current_xact_id() AND npc_context.fee_transaction_id=p_fee AND npc_context.character_id=p_character AND npc_context.world_id=p_world AND npc_policy.enabled
$$;
DO $$BEGIN EXECUTE format('ALTER FUNCTION game_npc_source_fee_cash_minimum(uuid,uuid,uuid) SET search_path=pg_catalog,%I,pg_temp',current_schema());END $$;
REVOKE ALL ON FUNCTION game_npc_source_fee_cash_minimum(uuid,uuid,uuid) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_npc_source_fee_cash_minimum(uuid,uuid,uuid) TO oteryn_game_runtime;
