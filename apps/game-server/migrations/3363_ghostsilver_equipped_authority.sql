-- Lock the live Channel authority through the equipped USE and its deferred proof.
CREATE FUNCTION game_ghostsilver_equipped_current(r game_item_consume_transform_records)
RETURNS boolean LANGUAGE sql VOLATILE SECURITY DEFINER AS $$
 SELECT EXISTS (
      SELECT 1 FROM game_durability_reconnect_sessions s
      JOIN game_durability_admission_character_guards c ON c.character_id=s.character_id
        AND c.account_id=s.account_id AND c.world_id=s.world_id
      JOIN game_durability_admission_account_guards a ON a.account_id=c.account_id
      JOIN game_runtime_scope_assignments scope ON scope.world_id=s.world_id
        AND scope.channel_id=s.runtime_scope_channel_id AND scope.scope_kind=1
      JOIN game_durability_admission_runtime_guards ready ON ready.scope_key=scope.scope_key
      WHERE s.world_id=r.world_id AND s.character_id=r.character_id
        AND s.game_session_id=r.game_session_id AND s.session_state IN (1,2)
        AND s.runtime_scope_kind=1 AND s.runtime_scope_world_id=r.world_id
        AND s.current_generation=(r.audit_context->>'connection_generation')::numeric
        AND s.character_lease_generation=(r.audit_context->>'lease_generation')::numeric
        AND s.scope_ownership_generation=(r.audit_context->>'scope_generation')::numeric
        AND c.eligible AND c.holder_game_session_id=s.game_session_id
        AND c.lease_generation=s.character_lease_generation
        AND a.presence_character_id=c.character_id AND a.holder_game_session_id=s.game_session_id
        AND scope.state=1 AND scope.ownership_generation=s.scope_ownership_generation
        AND ready.ready AND ready.ownership_generation=scope.ownership_generation
      FOR SHARE OF s,c,a,scope,ready)
$$;

DO $fence$
DECLARE
 signature text;
 body text;
 anchor CONSTANT text := 'IF NOT game_ghostsilver_equipped(r)';
BEGIN
 EXECUTE format('ALTER FUNCTION game_ghostsilver_equipped_current(game_item_consume_transform_records) SET search_path=pg_catalog,%I,pg_temp',current_schema());
 REVOKE ALL ON FUNCTION game_ghostsilver_equipped_current(game_item_consume_transform_records) FROM PUBLIC;
 GRANT EXECUTE ON FUNCTION game_ghostsilver_equipped_current(game_item_consume_transform_records) TO oteryn_game_runtime;

 -- Keep the custody predicate independent so a stale lantern still routes to
 -- lantern completion, where a failed fence aborts the transaction.
 FOREACH signature IN ARRAY ARRAY[
     'game_ghostsilver_equipped_apply(uuid)',
     'game_ghostsilver_equipped_complete(game_item_consume_transform_records)'] LOOP
   body := pg_get_functiondef(signature::regprocedure);
   IF (length(body)-length(replace(body,anchor,'')))/length(anchor) <> 1 THEN
     RAISE EXCEPTION 'Equipped lantern guard layout changed in %',signature;
   END IF;
   EXECUTE replace(body,anchor,anchor || ' OR NOT game_ghostsilver_equipped_current(r)');
 END LOOP;
END $fence$;
