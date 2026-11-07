-- Programme receipts are admitted only by the private same-XID Node observation.
ALTER TABLE game_npc_cosmetic_receipts DROP CONSTRAINT game_npc_cosmetic_receipts_npc_key_check;
ALTER TABLE game_npc_cosmetic_receipts ADD CONSTRAINT game_npc_cosmetic_receipts_npc_key_check CHECK(
 npc_key IN('oteryn:npc.amber','oteryn:npc.norma','oteryn:npc.tom','oteryn:npc.cledwyn','oteryn:npc.kesar_the_younger_day','oteryn:npc.hairycles') OR program_sha256 IS NOT NULL);
DO $migration$ DECLARE constraint_name TEXT;BEGIN
 SELECT conname INTO STRICT constraint_name FROM pg_constraint WHERE conrelid='game_npc_cosmetic_receipts'::regclass
  AND contype='c' AND pg_get_expr(conbin,conrelid) LIKE '%cardinality(grant_keys)%';
 EXECUTE format('ALTER TABLE game_npc_cosmetic_receipts DROP CONSTRAINT %I',constraint_name);
END $migration$;
ALTER TABLE game_npc_cosmetic_receipts ADD CHECK(cardinality(grant_keys)<=2 AND (program_sha256 IS NOT NULL OR cardinality(grant_keys)<=1));
CREATE FUNCTION game_npc_program_receipt_admit() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE npc_ctx game_npc_source_context%ROWTYPE;BEGIN
 IF NEW.program_sha256 IS NULL THEN
  IF NEW.plan_sha256 IS NOT NULL OR NEW.fee_transaction_id IS NOT NULL THEN RAISE EXCEPTION 'legacy NPC receipt claims a programme child' USING ERRCODE='23514';END IF;
  RETURN NEW;
 END IF;
 SELECT * INTO npc_ctx FROM game_npc_source_context WHERE backend_pid=pg_backend_pid() AND transaction_xid=pg_current_xact_id() FOR UPDATE;
 IF npc_ctx.backend_pid IS NULL OR npc_ctx.captured OR npc_ctx.program_sha256 IS DISTINCT FROM NEW.program_sha256
  OR npc_ctx.plan_sha256 IS DISTINCT FROM NEW.plan_sha256 OR npc_ctx.npc_transaction_id IS DISTINCT FROM NEW.transaction_id
  OR npc_ctx.character_id IS DISTINCT FROM NEW.character_id OR npc_ctx.account_id IS DISTINCT FROM NEW.account_id
  OR npc_ctx.game_session_id IS DISTINCT FROM NEW.game_session_id OR npc_ctx.command_id IS DISTINCT FROM NEW.command_id
  OR npc_ctx.source_key IS DISTINCT FROM NEW.source_key OR npc_ctx.npc_key IS DISTINCT FROM NEW.npc_key
  OR npc_ctx.request_binding IS DISTINCT FROM NEW.request_binding OR npc_ctx.original_revision IS DISTINCT FROM NEW.original_character_revision
  OR NEW.committed_character_revision IS DISTINCT FROM NEW.original_character_revision+1
  OR npc_ctx.cost_transaction_id IS DISTINCT FROM NEW.cost_transaction_id OR npc_ctx.fee_transaction_id IS DISTINCT FROM NEW.fee_transaction_id
  OR npc_ctx.expected_plan->'changes' IS DISTINCT FROM NEW.track_changes
  OR npc_ctx.expected_plan->'grant_keys' IS DISTINCT FROM to_jsonb(NEW.grant_keys)
  OR npc_ctx.expected_plan->'outcome' IS DISTINCT FROM NEW.outcome THEN
  RAISE EXCEPTION 'NPC programme receipt lacks its sealed source plan' USING ERRCODE='23514';END IF;
 UPDATE game_npc_source_context SET captured=TRUE WHERE backend_pid=pg_backend_pid() AND transaction_xid=pg_current_xact_id();
 NEW.created_xact_id:=pg_current_xact_id();RETURN NEW;
END $$;
CREATE TRIGGER game_npc_program_receipt_admit BEFORE INSERT ON game_npc_cosmetic_receipts FOR EACH ROW EXECUTE FUNCTION game_npc_program_receipt_admit();
CREATE FUNCTION game_npc_program_receipt_complete(npc_receipt game_npc_cosmetic_receipts) RETURNS VOID LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE npc_ctx game_npc_source_context%ROWTYPE;npc_change JSONB;npc_grant TEXT;npc_effect JSONB;npc_achievement TEXT;npc_progress BIGINT;BEGIN
 SELECT * INTO npc_ctx FROM game_npc_source_context WHERE backend_pid=pg_backend_pid() AND transaction_xid=pg_current_xact_id();
 IF npc_ctx.npc_transaction_id IS DISTINCT FROM npc_receipt.transaction_id OR npc_ctx.captured IS DISTINCT FROM TRUE
  OR npc_receipt.created_xact_id IS DISTINCT FROM pg_current_xact_id()
  OR NOT EXISTS(SELECT 1 FROM game_character_roots npc_root WHERE npc_root.character_id=npc_receipt.character_id
   AND npc_root.account_id=npc_receipt.account_id AND npc_root.character_revision=npc_receipt.committed_character_revision)
  OR NOT EXISTS(SELECT 1 FROM game_character_progression_state npc_progress WHERE npc_progress.character_id=npc_receipt.character_id
   AND npc_progress.character_revision=npc_receipt.committed_character_revision)
  OR (npc_ctx.expected_plan->'costs'='[]'::jsonb) IS DISTINCT FROM (npc_receipt.cost_transaction_id IS NULL) THEN
  RAISE EXCEPTION 'NPC programme did not finish its single Character edge' USING ERRCODE='23514';END IF;
 IF npc_receipt.cost_transaction_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM game_srv7_item_costs npc_cost
  WHERE npc_cost.transaction_id=npc_receipt.cost_transaction_id AND npc_cost.character_id=npc_receipt.character_id
   AND npc_cost.source_key=npc_receipt.source_key AND npc_cost.game_session_id=npc_receipt.game_session_id
   AND npc_cost.command_id=npc_receipt.command_id AND npc_cost.requirements=npc_ctx.expected_plan->'costs'
   AND npc_cost.created_xact_id=pg_current_xact_id()) THEN
  RAISE EXCEPTION 'NPC programme did not finish its exact item cost child' USING ERRCODE='23514';END IF;
 FOR npc_change IN SELECT value FROM jsonb_array_elements(npc_receipt.track_changes) LOOP
  IF NOT EXISTS(SELECT 1 FROM game_character_quest_tracks npc_track WHERE npc_track.character_id=npc_receipt.character_id
   AND npc_track.track_key=npc_change->>'key' AND npc_track.quest_key=npc_change->>'quest'
   AND npc_track.value=(npc_change->>'after')::bigint AND npc_track.committed_character_revision=npc_receipt.committed_character_revision) THEN
   RAISE EXCEPTION 'NPC programme lost a source track successor' USING ERRCODE='23514';END IF;
 END LOOP;
 FOR npc_effect IN SELECT value FROM jsonb_array_elements(npc_ctx.expected_plan->'effects') LOOP
  IF npc_effect->>'kind' IN('achievement','achievement_progress') THEN
   npc_achievement:='oteryn:achievement/'||trim(both '_' from regexp_replace(lower(npc_effect->>'name'),'[^a-z0-9]+','_','g'));
   IF npc_effect->>'kind'='achievement_progress' THEN
    IF NOT (npc_ctx.before_state->'achievement_progress' ? npc_achievement) THEN
     RAISE EXCEPTION 'NPC source progress lacks actual predecessor observation' USING ERRCODE='23514';END IF;
    npc_progress:=greatest((npc_ctx.before_state->'achievement_progress'->>npc_achievement)::bigint,least((npc_effect->>'target')::bigint,(npc_ctx.before_state->'achievement_progress'->>npc_achievement)::bigint+1));
    IF NOT EXISTS(SELECT 1 FROM game_character_achievement_progress npc_progress_row WHERE npc_progress_row.character_id=npc_receipt.character_id AND npc_progress_row.achievement_key=npc_achievement AND npc_progress_row.progress=npc_progress) THEN
     RAISE EXCEPTION 'NPC source did not finish achievement progress' USING ERRCODE='23514';END IF;
   END IF;
   IF (npc_effect->>'kind'='achievement' OR npc_progress=(npc_effect->>'target')::bigint) AND NOT EXISTS(SELECT 1 FROM game_account_achievements npc_achievement_row WHERE npc_achievement_row.account_id=npc_receipt.account_id AND npc_achievement_row.achievement_key=npc_achievement) THEN
    RAISE EXCEPTION 'NPC source did not finish its achievement successor' USING ERRCODE='23514';END IF;
  END IF;
 END LOOP;
 FOREACH npc_grant IN ARRAY npc_receipt.grant_keys LOOP
  IF NOT EXISTS(SELECT 1 FROM game_account_appearance_unlock_receipts npc_unlock
   JOIN game_account_appearance_unlocks npc_owned USING(account_id,unlock_key,grant_occurrence_id)
   WHERE npc_unlock.account_id=npc_receipt.account_id AND npc_unlock.granting_character_id=npc_receipt.character_id
    AND npc_unlock.unlock_key=npc_grant AND npc_unlock.source_kind='npc'
    AND npc_unlock.content_revision='crystal-201189f-npc-cosmetics-v1' AND npc_unlock.source_event_id=uuid_send(npc_receipt.transaction_id)) THEN
   RAISE EXCEPTION 'NPC programme lost an exact account grant successor' USING ERRCODE='23514';END IF;
 END LOOP;
END $$;
DO $migration$ DECLARE npc_definition TEXT;npc_begin INTEGER;BEGIN
 npc_definition:=pg_get_functiondef('game_npc_cosmetic_receipt_guard()'::regprocedure);
 npc_begin:=regexp_instr(npc_definition,E'\\mBEGIN\\M',1,1,0,'i');
 IF npc_begin=0 THEN RAISE EXCEPTION 'NPC receipt guard boundary changed';END IF;
 EXECUTE overlay(npc_definition placing E'BEGIN\n IF NEW.program_sha256 IS NOT NULL THEN PERFORM game_npc_program_receipt_complete(NEW); RETURN NULL;END IF;\n' from npc_begin for 5);
END $migration$;
DO $migration$ DECLARE npc_function TEXT;BEGIN
 FOREACH npc_function IN ARRAY ARRAY['game_npc_program_receipt_admit()','game_npc_program_receipt_complete(game_npc_cosmetic_receipts)'] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',npc_function,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',npc_function);
 END LOOP;
END $migration$;
