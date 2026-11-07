-- Quest MINT joins the existing absolute ItemType schedule and expiry owner.
-- Preserve original cast/action/event source proofs; add an exact same-XID Quest source.
CREATE FUNCTION game_quest_reward_decay_proven(p game_spell_item_temporal_schedules)
RETURNS BOOLEAN LANGUAGE sql SECURITY DEFINER AS $$
 SELECT EXISTS(
  SELECT 1 FROM game_reward_claim_mint_receipts r
  JOIN game_reward_claim_mint_reservations v USING(game_session_id,command_id)
  JOIN game_quest_reward_sources q ON q.mint_transaction_id=r.transaction_id
  JOIN game_item_instances i ON i.item_instance_id=r.item_instance_id
  WHERE r.transaction_id=p.source_transaction_id AND p.source_ordinal=1
   AND p.item_instance_id=r.item_instance_id AND r.transaction_id=v.transaction_id
   AND r.item_instance_id=v.item_instance_id AND r.intent_binding=v.intent_binding
   AND r.character_id=v.character_id AND q.character_id=r.character_id AND i.world_id=v.world_id
   AND r.quantity=1 AND i.quantity=1 AND i.lifecycle=1 AND i.state_revision=1
   AND i.last_transaction_id IS NULL AND i.minted_transaction_id=r.transaction_id
   AND (i.definition_family,i.definition_production_key,i.definition_revision_ref)=
       (p.definition_family,p.definition_production_key,p.definition_revision)
   AND r.claim_revision_ref='sha256:'||encode(p.content_digest,'hex')
   AND jsonb_array_length(r.contents_plan)=0
   AND q.plan->'give'->>'item'=p.definition_production_key
   AND (q.plan->'give'->>'count')::bigint=1
   AND (q.plan->'give'->'decay'->>'duration_millis')::bigint=p.duration_millis
   AND q.plan->'give'->'decay'->'target'=CASE WHEN p.target_family IS NULL THEN 'null'::jsonb
       ELSE jsonb_build_object('key',p.target_production_key,'revision',p.target_revision) END
   AND (p.target_family IS NULL OR p.target_family='Item')
   AND p.expires_at_unix_ms=r.occurred_at+p.duration_millis
   AND p.created_xact_id=pg_current_xact_id() AND r.created_xact_id=p.created_xact_id
   AND q.created_xact_id=p.created_xact_id AND game_reward_claim_container_complete(r.transaction_id)
 );
$$;
DO $schedule$ DECLARE body TEXT; insert_at INTEGER; BEGIN
 SELECT pg_get_functiondef('game_spell_item_temporal_schedule_proven()'::regprocedure) INTO STRICT body;
 insert_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,1,'i');
 IF insert_at=0 THEN RAISE EXCEPTION 'Quest deadline requires original temporal source owner'; END IF;
 EXECUTE overlay(body placing E'\n IF game_quest_reward_decay_proven(NEW::game_spell_item_temporal_schedules) THEN RETURN NULL; END IF;\n' from insert_at for 0);
 SELECT pg_get_functiondef('game_quest_reward_complete()'::regprocedure) INTO STRICT body;
 IF position('RETURN NULL;' IN body)=0 THEN RAISE EXCEPTION 'Quest deadline requires complete reward proof'; END IF;
 EXECUTE replace(body,'RETURN NULL;', $proof$
 IF NEW.plan->'give' ? 'decay' AND NOT EXISTS(
  SELECT 1 FROM game_spell_item_temporal_schedules p
   WHERE p.source_transaction_id=NEW.mint_transaction_id AND game_quest_reward_decay_proven(p)
 ) THEN RAISE EXCEPTION 'Quest reward lacks its exact atomic source deadline' USING ERRCODE='23514'; END IF;
 RETURN NULL;
 $proof$);
END $schedule$;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_quest_reward_decay_proven(game_spell_item_temporal_schedules) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_quest_reward_decay_proven(game_spell_item_temporal_schedules) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_quest_reward_decay_proven(game_spell_item_temporal_schedules) TO oteryn_game_runtime,oteryn_game_control;
