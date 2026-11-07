-- Owned0993 source-event routing: later ordinary/participant function revisions
-- remain authoritative for ordinary events; no shared core function is replaced.
CREATE FUNCTION game_srv7_npc_track_event(p_character UUID,p_key TEXT,p_quest TEXT,p_revision NUMERIC)
 RETURNS BOOLEAN LANGUAGE sql SECURITY DEFINER AS $$
 SELECT EXISTS(SELECT 1 FROM game_npc_cosmetic_receipts parent
 CROSS JOIN LATERAL jsonb_array_elements(parent.track_changes) delta
 WHERE parent.character_id=p_character AND parent.committed_character_revision=p_revision
 AND parent.created_xact_id=pg_current_xact_id()
 AND delta->>'key'=p_key AND delta->>'quest'=p_quest)
 OR EXISTS(SELECT 1 FROM game_npc_quest_children child
 JOIN game_npc_cosmetic_receipts parent ON parent.transaction_id=child.parent_transaction_id
 CROSS JOIN LATERAL jsonb_array_elements(child.track_changes) delta
 WHERE child.character_id=p_character AND child.quest_key=p_quest AND child.committed_revision=p_revision
 AND child.created_xact_id=pg_current_xact_id() AND parent.created_xact_id=pg_current_xact_id()
 AND parent.character_id=child.character_id AND parent.committed_character_revision=child.committed_revision
 AND delta->>'key'=p_key AND delta->>'quest'=p_quest)
$$;
CREATE FUNCTION game_srv7_npc_state_event(p_character UUID,p_quest TEXT,p_revision NUMERIC)
 RETURNS BOOLEAN LANGUAGE sql SECURITY DEFINER AS $$
 SELECT EXISTS(SELECT 1 FROM game_npc_quest_children child
 JOIN game_npc_cosmetic_receipts parent ON parent.transaction_id=child.parent_transaction_id
 WHERE child.character_id=p_character AND child.quest_key=p_quest AND child.committed_revision=p_revision
 AND child.created_xact_id=pg_current_xact_id() AND parent.created_xact_id=pg_current_xact_id()
 AND parent.character_id=child.character_id AND parent.committed_character_revision=child.committed_revision)
$$;

-- These are strict source alternatives only. No ordinary receipt fallback.
CREATE FUNCTION game_srv7_npc_track_guard() RETURNS TRIGGER LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF EXISTS(SELECT 1 FROM game_npc_cosmetic_receipts parent,
 LATERAL jsonb_array_elements(parent.track_changes) delta
 WHERE parent.character_id=NEW.character_id AND parent.committed_character_revision=NEW.committed_character_revision
 AND parent.created_xact_id=pg_current_xact_id() AND delta->>'key'=NEW.track_key
 AND delta->>'quest'=NEW.quest_key AND (delta->>'after')::bigint=NEW.value
 AND (TG_OP='INSERT' OR (delta->>'before')::bigint=OLD.value)) THEN RETURN NULL;END IF;
 IF TG_OP='INSERT' THEN
  IF game_npc_quest_track_proven(NEW.character_id,NEW.track_key,NEW.quest_key,
    NEW.committed_character_revision,0,NEW.value,TRUE) IS TRUE THEN RETURN NULL;END IF;
 ELSIF TG_OP='UPDATE' THEN
  IF game_npc_quest_track_proven(NEW.character_id,NEW.track_key,NEW.quest_key,
    NEW.committed_character_revision,OLD.value,NEW.value,FALSE) IS TRUE THEN RETURN NULL;END IF;
 END IF;
 RAISE EXCEPTION 'SRV7 source Quest track lacks exact current-XID parent/child effect' USING ERRCODE='23514';
END $$;
CREATE FUNCTION game_srv7_npc_state_guard() RETURNS TRIGGER LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF TG_OP='INSERT' THEN
  IF game_npc_quest_state_proven(NEW.character_id,NEW.quest_key,
    NEW.committed_character_revision,NEW.pinned_content_revision,NEW.definition_hash,
    NEW.started_character_revision,NEW.completed_character_revision,TRUE,NULL) IS TRUE THEN RETURN NULL;END IF;
 ELSIF TG_OP='UPDATE' THEN
  IF game_npc_quest_state_proven(NEW.character_id,NEW.quest_key,
    NEW.committed_character_revision,NEW.pinned_content_revision,NEW.definition_hash,
    NEW.started_character_revision,NEW.completed_character_revision,FALSE,
    OLD.completed_character_revision) IS TRUE THEN RETURN NULL;END IF;
 END IF;
 RAISE EXCEPTION 'SRV7 source Quest state lacks exact current-XID admitted child' USING ERRCODE='23514';
END $$;
DO $routing_paths$
DECLARE signature TEXT;
BEGIN
 FOREACH signature IN ARRAY ARRAY[
 'game_srv7_npc_track_event(uuid,text,text,numeric)',
 'game_srv7_npc_state_event(uuid,text,numeric)',
 'game_srv7_npc_track_guard()','game_srv7_npc_state_guard()'] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',signature,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC,oteryn_game_runtime,oteryn_game_control',signature);
 END LOOP;
END $routing_paths$;
-- WHEN expressions execute as the row writer. Read-only classifiers carry no
-- mutation authority and disclose only already SELECT-visible receipt identity.
GRANT EXECUTE ON FUNCTION game_srv7_npc_track_event(UUID,TEXT,TEXT,NUMERIC),
 game_srv7_npc_state_event(UUID,TEXT,NUMERIC) TO oteryn_game_runtime;

-- Verify the baseline trigger owner/event/deferred contract before replacing
-- only its routing expression. BEFORE row/bounds and truncate guards untouched.
DO $routing_baseline$
BEGIN
 IF (SELECT count(*) FROM pg_trigger WHERE tgrelid='game_character_quest_tracks'::regclass
  AND tgname='game_character_quest_track_consistent' AND NOT tgisinternal
  AND tgfoid='game_character_quest_track_guard()'::regprocedure
  AND tgtype=21 AND tgdeferrable AND tginitdeferred)<>1
 OR (SELECT count(*) FROM pg_trigger WHERE tgrelid='game_character_quest_states'::regclass
  AND tgname='game_character_quest_state_consistent' AND NOT tgisinternal
  AND tgfoid='game_character_quest_state_guard()'::regprocedure
  AND tgtype=21 AND tgdeferrable AND tginitdeferred)<>1 THEN
 RAISE EXCEPTION 'SRV7 Quest consistency trigger baseline changed';END IF;
END $routing_baseline$;
DROP TRIGGER game_character_quest_track_consistent ON game_character_quest_tracks;
CREATE CONSTRAINT TRIGGER game_character_quest_track_consistent AFTER INSERT OR UPDATE ON game_character_quest_tracks
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
 WHEN (game_srv7_npc_track_event(NEW.character_id,NEW.track_key,NEW.quest_key,NEW.committed_character_revision) IS NOT TRUE)
 EXECUTE FUNCTION game_character_quest_track_guard();
CREATE CONSTRAINT TRIGGER game_srv7_npc_track_consistent AFTER INSERT OR UPDATE ON game_character_quest_tracks
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
 WHEN (game_srv7_npc_track_event(NEW.character_id,NEW.track_key,NEW.quest_key,NEW.committed_character_revision) IS TRUE)
 EXECUTE FUNCTION game_srv7_npc_track_guard();
DROP TRIGGER game_character_quest_state_consistent ON game_character_quest_states;
CREATE CONSTRAINT TRIGGER game_character_quest_state_consistent AFTER INSERT OR UPDATE ON game_character_quest_states
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
 WHEN (game_srv7_npc_state_event(NEW.character_id,NEW.quest_key,NEW.committed_character_revision) IS NOT TRUE)
 EXECUTE FUNCTION game_character_quest_state_guard();
CREATE CONSTRAINT TRIGGER game_srv7_npc_state_consistent AFTER INSERT OR UPDATE ON game_character_quest_states
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
 WHEN (game_srv7_npc_state_event(NEW.character_id,NEW.quest_key,NEW.committed_character_revision) IS TRUE)
 EXECUTE FUNCTION game_srv7_npc_state_guard();
