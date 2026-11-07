-- Preserve the complete unsigned native death tuple when matching MINT causes
-- to event kill receipts. No narrowing of u64 generations to signed BIGINT.
CREATE FUNCTION game_world_event_death_key(
 world UUID, channel UUID, scope_generation NUMERIC, actor_id BIGINT, actor_generation NUMERIC
) RETURNS BYTEA LANGUAGE SQL IMMUTABLE STRICT PARALLEL SAFE AS $$
 SELECT CASE WHEN scope_generation BETWEEN 1 AND 18446744073709551615
  AND scope_generation=trunc(scope_generation)
  AND actor_id BETWEEN 0 AND 4294967295
  AND actor_generation BETWEEN 1 AND 18446744073709551615
  AND actor_generation=trunc(actor_generation)
 THEN uuid_send(world)||uuid_send(channel)||decode(
  lpad(to_hex(div(scope_generation,4294967296)::bigint),8,'0')||
  lpad(to_hex(mod(scope_generation,4294967296)::bigint),8,'0')||
  lpad(to_hex(actor_id),8,'0')||
  lpad(to_hex(div(actor_generation,4294967296)::bigint),8,'0')||
  lpad(to_hex(mod(actor_generation,4294967296)::bigint),8,'0'),'hex') END
$$;
REVOKE ALL ON FUNCTION game_world_event_death_key(UUID,UUID,NUMERIC,BIGINT,NUMERIC) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_world_event_death_key(UUID,UUID,NUMERIC,BIGINT,NUMERIC) TO oteryn_game_runtime,oteryn_game_control;


-- Native selector rewards retain the NPC owner's exact same-transaction
-- quest receipt, carried debit, backpack MINT and earned-outfit checks.
DO $migration$
DECLARE body TEXT; original TEXT:='r.cause_kind=''command''';
BEGIN
    body:=pg_get_functiondef('game_quest_reward_complete()'::regprocedure);
    IF (length(body)-length(replace(body,original,'')))/length(original)<>1 THEN
        RAISE EXCEPTION 'unexpected quest reward cause proof body';
    END IF;
    EXECUTE replace(body,original,'r.cause_kind IN (''command'',''use'',''step'')');
    EXECUTE format('ALTER FUNCTION game_quest_reward_complete() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $migration$;
