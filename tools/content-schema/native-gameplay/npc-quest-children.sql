-- DRAFT ONLY. Requires source arm to seal/qualify exact quest_children against
-- the bounded current native owner catalogue and full selected source policy.
-- Ordinary Quest receipt uniqueness, definition hashes and Root chain are unchanged.
CREATE TABLE game_npc_quest_children(
 parent_transaction_id UUID NOT NULL REFERENCES game_npc_cosmetic_receipts(transaction_id),
 quest_key TEXT NOT NULL,
 character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 pinned_content_revision TEXT NOT NULL,
 base_definition_hash BYTEA NOT NULL CHECK(octet_length(base_definition_hash)=32),
 evaluation_definition_hash BYTEA NOT NULL CHECK(octet_length(evaluation_definition_hash)=32),
 program_sha256 BYTEA NOT NULL CHECK(octet_length(program_sha256)=32),
 plan_sha256 BYTEA NOT NULL CHECK(octet_length(plan_sha256)=32),
 committed_revision NUMERIC(20,0) NOT NULL,
 transition_keys TEXT[] NOT NULL CHECK(cardinality(transition_keys) BETWEEN 0 AND 8 AND array_position(transition_keys,NULL) IS NULL),
 native_operations TEXT[] NOT NULL CHECK(cardinality(native_operations) BETWEEN 0 AND 8 AND array_position(native_operations,NULL) IS NULL),
 track_changes JSONB NOT NULL CHECK(jsonb_typeof(track_changes)='array' AND jsonb_array_length(track_changes) BETWEEN 1 AND 8),
 completes BOOLEAN NOT NULL,
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 PRIMARY KEY(parent_transaction_id,quest_key),
 CHECK(cardinality(transition_keys)+cardinality(native_operations)>0)
);
CREATE INDEX game_npc_quest_children_character_xact ON game_npc_quest_children(character_id,created_xact_id);
CREATE INDEX game_npc_quest_children_character_revision ON game_npc_quest_children(character_id,committed_revision);
REVOKE ALL ON game_npc_quest_children FROM PUBLIC,oteryn_game_runtime;
GRANT SELECT,INSERT ON game_npc_quest_children TO oteryn_game_runtime;
CREATE TRIGGER game_npc_quest_child_stamp BEFORE INSERT ON game_npc_quest_children
 FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();
CREATE TRIGGER game_npc_quest_child_immutable BEFORE UPDATE OR DELETE ON game_npc_quest_children
 FOR EACH ROW EXECUTE FUNCTION game_character_immutable();
CREATE TRIGGER game_npc_quest_child_no_truncate BEFORE TRUNCATE ON game_npc_quest_children
 FOR EACH STATEMENT EXECUTE FUNCTION game_character_reject_truncate();

-- Closed operation versions supplement old Quest declarations. They neither
-- change base definition hashes nor widen any ordinary evaluator bounds.
CREATE FUNCTION game_npc_quest_native_versions_valid(p_source TEXT,p_quest TEXT,p_versions TEXT[],p_changes JSONB)
 RETURNS BOOLEAN LANGUAGE plpgsql IMMUTABLE AS $$
DECLARE operation_version TEXT; expected_track TEXT; expected_before BIGINT; expected_after BIGINT;
BEGIN
 IF p_source IS NULL OR p_quest IS NULL OR p_versions IS NULL OR p_changes IS NULL OR jsonb_typeof(p_changes)<>'array'
 OR cardinality(p_versions)>8 OR array_position(p_versions,NULL) IS NOT NULL
 OR (SELECT count(DISTINCT version) FROM unnest(p_versions) version)<>cardinality(p_versions) THEN RETURN FALSE; END IF;
 FOREACH operation_version IN ARRAY p_versions LOOP
  expected_track:=NULL;expected_before:=NULL;expected_after:=NULL;
  CASE operation_version
   WHEN 'annihilator_outfit_claim_ack_v1' THEN
    IF p_source<>'oteryn:npc-cosmetic/avar_tar/demon-base'
     OR p_quest<>'oteryn:quest.npc_cosmetic_final_acquisition'
     OR NOT EXISTS(SELECT 1 FROM jsonb_array_elements(p_changes) delta
       WHERE delta->>'key'='oteryn:quest-progress/quest/u7_24/the_annihilator/reward'
       AND delta->>'quest'=p_quest AND delta->>'source_before'='1'
       AND delta->>'before'='-1' AND delta->>'after'='2') THEN RETURN FALSE;END IF;
    CONTINUE;
   WHEN 'demon_helmet_outfit_claim_ack_v1' THEN
    IF p_source<>'oteryn:npc-cosmetic/avar_tar/demon-shield'
     OR p_quest<>'oteryn:quest.npc_cosmetic_final_acquisition'
     OR NOT EXISTS(SELECT 1 FROM jsonb_array_elements(p_changes) delta
       WHERE delta->>'key'='oteryn:quest-progress/quest/u6_4/demon_helmet/rewards/demon_helmet'
       AND delta->>'quest'=p_quest AND delta->>'source_before'='1'
       AND delta->>'before'='-1' AND delta->>'after'='2') THEN RETURN FALSE;END IF;
    CONTINUE;
   WHEN 'library_permission_outfit_ack_v1' THEN
    IF p_source<>'oteryn:npc-cosmetic/dedoras/battle-mage-base'
     OR p_quest<>'oteryn:quest.npc_cosmetic_final_acquisition' THEN RETURN FALSE;END IF;
    expected_track:='oteryn:quest-progress/quest/u11_80/the_secret_library/library_permission';expected_before:=6;expected_after:=7;
   WHEN 'navigator_outfit_ack_v1' THEN
    IF p_source<>'oteryn:npc-cosmetic/navigator/deepling-helmet'
     OR p_quest<>'oteryn:quest.npc_cosmetic_final_acquisition'
     OR NOT EXISTS(SELECT 1 FROM jsonb_array_elements(p_changes) delta
       WHERE delta->>'key'='oteryn:quest-progress/navigator' AND delta->>'quest'=p_quest
       AND (delta->>'source_before')::bigint BETWEEN -1 AND 4
       AND delta->>'before'=delta->>'source_before' AND delta->>'after'='4') THEN RETURN FALSE;END IF;
    CONTINUE;
   WHEN 'grave_danger_outfit_ack_v1' THEN
    IF p_source<>'oteryn:npc-cosmetic/jack_springer/inquisition-hand' OR p_quest<>'oteryn:quest.grave_danger_quest' THEN RETURN FALSE;END IF;
    expected_track:='oteryn:quest-progress/quest/u12_20/grave_danger/questline';expected_before:=4;expected_after:=5;
   WHEN 'foolish_first_addon_ack_v1' THEN
    IF p_source<>'oteryn:npc-cosmetic/bozo/jester-addon-1' OR p_quest<>'oteryn:quest.what_a_foolish_quest' THEN RETURN FALSE;END IF;
    expected_track:='oteryn:quest-progress/quest/u8_1/what_afoolish_quest/questline';expected_before:=24;expected_after:=25;
   WHEN 'foolish_second_addon_ack_v1' THEN
    IF p_source<>'oteryn:npc-cosmetic/bozo/jester-addon-2' OR p_quest<>'oteryn:quest.what_a_foolish_quest' THEN RETURN FALSE;END IF;
    expected_track:='oteryn:quest-progress/quest/u8_1/what_afoolish_quest/questline';expected_before:=35;expected_after:=36;
   WHEN 'demon_oak_outfit_ack_v1' THEN
    IF p_source<>'oteryn:npc-cosmetic/avar_tar/demon-helmet' OR p_quest<>'oteryn:quest.demon_oak' THEN RETURN FALSE;END IF;
    expected_track:='oteryn:quest-progress/quest/u8_2/the_demon_oak/done';expected_before:=3;expected_after:=4;
   WHEN 'palimuth_outfit_ack_v1' THEN
    IF p_source NOT IN('oteryn:npc-cosmetic/palimuth/yalaharian-addon-1','oteryn:npc-cosmetic/palimuth/yalaharian-addon-2')
     OR p_quest<>'oteryn:quest.in_service_of_yalahar_quest' THEN RETURN FALSE;END IF;
    expected_track:='oteryn:quest-progress/quest/u8_4/in_service_of_yalahar/questline';expected_before:=54;expected_after:=55;
   WHEN 'secret_library_questlog_source_helper_v1' THEN
    IF p_source NOT IN('oteryn:npc-cosmetic/dedoras/battle-mage-base','oteryn:npc-cosmetic/dedoras/battle-mage-addon-1','oteryn:npc-cosmetic/dedoras/battle-mage-addon-2')
     OR p_quest<>'oteryn:quest.the_secret_library_quest' THEN RETURN FALSE;END IF;
    IF NOT EXISTS(SELECT 1 FROM jsonb_array_elements(p_changes) changed_track
     WHERE changed_track->>'key'='oteryn:quest-progress/quest/u11_80/the_secret_library/questlog'
     AND changed_track->>'quest'=p_quest AND (changed_track->>'source_before')::bigint BETWEEN -1 AND 1
     AND (changed_track->>'after')::bigint=1) THEN RETURN FALSE;END IF;
    CONTINUE;
   ELSE RETURN FALSE;
  END CASE;
  IF NOT EXISTS(SELECT 1 FROM jsonb_array_elements(p_changes) changed_track
   WHERE changed_track->>'key'=expected_track AND changed_track->>'quest'=p_quest
   AND (changed_track->>'source_before')::bigint=expected_before
   AND (changed_track->>'before')::bigint=expected_before AND (changed_track->>'after')::bigint=expected_after) THEN RETURN FALSE;END IF;
 END LOOP;
 RETURN TRUE;
END $$;
REVOKE ALL ON FUNCTION game_npc_quest_native_versions_valid(TEXT,TEXT,TEXT[],JSONB) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_npc_quest_native_versions_valid(TEXT,TEXT,TEXT[],JSONB) TO oteryn_game_runtime;

CREATE FUNCTION game_npc_quest_child_admit() RETURNS TRIGGER LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE source_context game_npc_source_context%ROWTYPE;
 parent_receipt game_npc_cosmetic_receipts%ROWTYPE;
 expected_child JSONB; source_delta JSONB; child_delta JSONB;
BEGIN
 SELECT * INTO source_context FROM game_npc_source_context
 WHERE backend_pid=pg_backend_pid() AND transaction_xid=pg_current_xact_id();
 SELECT * INTO parent_receipt FROM game_npc_cosmetic_receipts
 WHERE transaction_id=NEW.parent_transaction_id;
 IF source_context.npc_transaction_id IS DISTINCT FROM NEW.parent_transaction_id
 OR game_npc_source_parent_current(NEW.parent_transaction_id,source_context.source_key) IS DISTINCT FROM TRUE
 OR NEW.character_id IS DISTINCT FROM source_context.character_id
 OR NEW.quest_key IS NULL OR NEW.program_sha256 IS DISTINCT FROM source_context.program_sha256
 OR NEW.plan_sha256 IS DISTINCT FROM source_context.plan_sha256
 OR NEW.committed_revision IS DISTINCT FROM source_context.original_revision+1
 OR parent_receipt.created_xact_id IS DISTINCT FROM pg_current_xact_id()
 OR parent_receipt.committed_character_revision IS DISTINCT FROM NEW.committed_revision THEN
 RAISE EXCEPTION 'Quest child lacks exact current source parent' USING ERRCODE='23514'; END IF;
 SELECT value INTO expected_child FROM jsonb_array_elements(source_context.expected_plan->'quest_children')
 WHERE value->>'quest'=NEW.quest_key;
 IF game_npc_quest_native_versions_valid(source_context.source_key,NEW.quest_key,NEW.native_operations,NEW.track_changes) IS DISTINCT FROM TRUE THEN
 RAISE EXCEPTION 'Quest child has invalid native operation version' USING ERRCODE='23514'; END IF;
 IF expected_child->'writes_state' IS DISTINCT FROM to_jsonb(NEW.quest_key<>'oteryn:quest.npc_cosmetic_final_acquisition') THEN
 RAISE EXCEPTION 'Quest child changes its owner storage class' USING ERRCODE='23514';END IF;
 IF NOT FOUND OR expected_child->>'pinned_content_revision' IS DISTINCT FROM NEW.pinned_content_revision
 OR decode(expected_child->>'base_definition_hash','hex') IS DISTINCT FROM NEW.base_definition_hash
 OR decode(expected_child->>'evaluation_definition_hash','hex') IS DISTINCT FROM NEW.evaluation_definition_hash
 OR expected_child->'changes' IS DISTINCT FROM NEW.track_changes
 OR expected_child->'transition_keys' IS DISTINCT FROM to_jsonb(NEW.transition_keys)
 OR expected_child->'native_operations' IS DISTINCT FROM to_jsonb(NEW.native_operations)
 OR (expected_child->>'completes')::boolean IS DISTINCT FROM NEW.completes THEN
 RAISE EXCEPTION 'Quest child differs from qualified native owner plan' USING ERRCODE='23514'; END IF;
 IF NEW.quest_key='oteryn:quest.npc_cosmetic_final_acquisition' AND (
  NEW.completes OR cardinality(NEW.transition_keys)<>0 OR cardinality(NEW.native_operations)<>1
  OR jsonb_array_length(NEW.track_changes)<>1
  OR NEW.base_definition_hash IS DISTINCT FROM NEW.evaluation_definition_hash
  OR NEW.pinned_content_revision IS DISTINCT FROM parent_receipt.content_revision
  OR EXISTS(SELECT 1 FROM game_character_quest_states invalid_state
   WHERE invalid_state.character_id=NEW.character_id AND invalid_state.quest_key=NEW.quest_key)) THEN
 RAISE EXCEPTION 'Independent final acknowledgement cannot invent a full Quest state' USING ERRCODE='23514';END IF;
 IF EXISTS(SELECT 1 FROM game_character_quest_states existing_state
 WHERE existing_state.character_id=NEW.character_id AND existing_state.quest_key=NEW.quest_key
 AND (existing_state.pinned_content_revision<>NEW.pinned_content_revision
 OR existing_state.definition_hash<>NEW.base_definition_hash)) THEN
 RAISE EXCEPTION 'Quest child cannot repin existing quest state' USING ERRCODE='23514'; END IF;
 IF (SELECT count(DISTINCT delta->>'key') FROM jsonb_array_elements(NEW.track_changes) delta)
    <>jsonb_array_length(NEW.track_changes) THEN
 RAISE EXCEPTION 'Quest child repeats a track' USING ERRCODE='23514'; END IF;
 FOR child_delta IN SELECT value FROM jsonb_array_elements(NEW.track_changes) LOOP
  IF child_delta->>'quest' IS DISTINCT FROM NEW.quest_key THEN
   RAISE EXCEPTION 'Quest child crosses owner' USING ERRCODE='23514'; END IF;
  SELECT value INTO source_delta FROM jsonb_array_elements(source_context.expected_plan->'quest_changes')
   WHERE value->>'key'=child_delta->>'key' AND value->>'quest'=NEW.quest_key;
  IF NOT FOUND OR source_delta->'after' IS DISTINCT FROM child_delta->'after'
   OR source_delta->'before' IS DISTINCT FROM child_delta->'source_before' THEN
   RAISE EXCEPTION 'Quest child does not equal source delta' USING ERRCODE='23514'; END IF;
  -- Every source delta has both the source-observed before and mature-owner before;
  -- their only permitted difference is an explicitly sealed absent initial projection.
  IF child_delta->'source_before' IS DISTINCT FROM child_delta->'before'
   AND NOT EXISTS(SELECT 1 FROM jsonb_array_elements(expected_child->'absent_initial_projections') projection
    WHERE projection->>'key'=child_delta->>'key' AND projection->'initial'=child_delta->'before'
    AND child_delta->>'source_before'='-1'
    AND NOT (source_context.before_state->'tracks' ? (child_delta->>'key'))
    AND NOT EXISTS(SELECT 1 FROM game_character_quest_tracks stored_before
      WHERE stored_before.character_id=NEW.character_id
      AND stored_before.track_key=child_delta->>'key'))
   AND NOT ((
    expected_child->'actual_claim_projections' @> jsonb_build_array(jsonb_build_object(
      'key',child_delta->>'key','claim_transaction_id',child_delta->>'claim_transaction_id'))
    AND game_npc_final_claim_projection_current(NEW.character_id,source_context.source_key,
      child_delta->>'key',(child_delta->>'claim_transaction_id')::uuid,
      (child_delta->>'source_before')::bigint,(child_delta->>'before')::bigint) IS TRUE
   ) IS TRUE) THEN
   RAISE EXCEPTION 'Unqualified absent initial projection' USING ERRCODE='23514'; END IF;
 END LOOP;
 RETURN NEW;
END $$;
REVOKE ALL ON FUNCTION game_npc_quest_child_admit() FROM PUBLIC,oteryn_game_runtime;
CREATE TRIGGER game_npc_quest_child_admit BEFORE INSERT ON game_npc_quest_children
 FOR EACH ROW EXECUTE FUNCTION game_npc_quest_child_admit();

CREATE FUNCTION game_npc_quest_track_proven(p_character UUID,p_key TEXT,p_quest TEXT,
 p_revision NUMERIC,p_before BIGINT,p_after BIGINT,p_insert BOOLEAN)
 RETURNS BOOLEAN LANGUAGE sql SECURITY DEFINER AS $$
 SELECT EXISTS(SELECT 1 FROM game_npc_quest_children quest_child
 JOIN game_npc_cosmetic_receipts source_parent ON source_parent.transaction_id=quest_child.parent_transaction_id
 CROSS JOIN LATERAL jsonb_array_elements(quest_child.track_changes) delta
 WHERE quest_child.character_id=p_character AND quest_child.quest_key=p_quest
 AND quest_child.committed_revision=p_revision AND quest_child.created_xact_id=pg_current_xact_id()
 AND source_parent.created_xact_id=pg_current_xact_id()
 AND game_npc_source_parent_current(source_parent.transaction_id,source_parent.source_key)
 AND delta->>'key'=p_key AND (delta->>'after')::bigint=p_after
 AND (p_insert OR (delta->>'before')::bigint=p_before))
$$;
REVOKE ALL ON FUNCTION game_npc_quest_track_proven(UUID,TEXT,TEXT,NUMERIC,BIGINT,BIGINT,BOOLEAN) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_npc_quest_track_proven(UUID,TEXT,TEXT,NUMERIC,BIGINT,BIGINT,BOOLEAN) TO oteryn_game_runtime;

CREATE FUNCTION game_npc_quest_child_complete() RETURNS TRIGGER LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE child_delta JSONB;
BEGIN
 IF game_npc_source_parent_current(NEW.parent_transaction_id,
   (SELECT source_key FROM game_npc_cosmetic_receipts WHERE transaction_id=NEW.parent_transaction_id)) IS DISTINCT FROM TRUE THEN
 RAISE EXCEPTION 'Quest child parent no longer current' USING ERRCODE='23514'; END IF;
 FOR child_delta IN SELECT value FROM jsonb_array_elements(NEW.track_changes) LOOP
  IF NOT EXISTS(SELECT 1 FROM game_character_quest_tracks actual_track
   WHERE actual_track.character_id=NEW.character_id AND actual_track.quest_key=NEW.quest_key
   AND actual_track.track_key=child_delta->>'key' AND actual_track.value=(child_delta->>'after')::bigint
   AND actual_track.committed_character_revision=NEW.committed_revision) THEN
   RAISE EXCEPTION 'Quest child lacks exact stored effect' USING ERRCODE='23514'; END IF;
 END LOOP;
 IF NEW.quest_key='oteryn:quest.npc_cosmetic_final_acquisition' THEN
  IF NEW.completes OR EXISTS(SELECT 1 FROM game_character_quest_states actual_state
   WHERE actual_state.character_id=NEW.character_id AND actual_state.quest_key=NEW.quest_key) THEN
  RAISE EXCEPTION 'Final acknowledgement acquired an unproved full Quest state' USING ERRCODE='23514';END IF;
 ELSE
 IF NOT EXISTS(SELECT 1 FROM game_character_quest_states actual_state
 WHERE actual_state.character_id=NEW.character_id AND actual_state.quest_key=NEW.quest_key
 AND actual_state.pinned_content_revision=NEW.pinned_content_revision
 AND actual_state.definition_hash=NEW.base_definition_hash
 AND actual_state.committed_character_revision=NEW.committed_revision
 AND (NOT NEW.completes OR actual_state.completed_character_revision IS NOT NULL)) THEN
 RAISE EXCEPTION 'Quest child lacks exact pinned state' USING ERRCODE='23514'; END IF;
 END IF;
 RETURN NULL;
END $$;
REVOKE ALL ON FUNCTION game_npc_quest_child_complete() FROM PUBLIC,oteryn_game_runtime;
CREATE CONSTRAINT TRIGGER game_npc_quest_child_complete AFTER INSERT ON game_npc_quest_children
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_npc_quest_child_complete();

-- Remaining integration is intentionally not hidden: source arm must qualify and
-- seal quest_children; parent deferred completeness must demand every expected
-- child; existing track/state guards need only an exact proved-child alternate.
-- The ordinary Quest row guard (no rekey/repin/rewrite) remains byte-for-byte.

CREATE FUNCTION game_npc_quest_state_proven(p_character UUID,p_quest TEXT,p_revision NUMERIC,
 p_pin TEXT,p_hash BYTEA,p_started NUMERIC,p_completed NUMERIC,p_insert BOOLEAN,p_old_completed NUMERIC)
 RETURNS BOOLEAN LANGUAGE sql SECURITY DEFINER AS $$
 SELECT EXISTS(SELECT 1 FROM game_npc_quest_children quest_child
 JOIN game_npc_cosmetic_receipts source_parent ON source_parent.transaction_id=quest_child.parent_transaction_id
 WHERE quest_child.character_id=p_character AND quest_child.quest_key=p_quest
 AND quest_child.committed_revision=p_revision AND quest_child.pinned_content_revision=p_pin
 AND quest_child.base_definition_hash=p_hash AND quest_child.created_xact_id=pg_current_xact_id()
 AND source_parent.created_xact_id=pg_current_xact_id()
 AND game_npc_source_parent_current(source_parent.transaction_id,source_parent.source_key)
 AND (NOT p_insert OR p_started=p_revision)
 AND ((NOT p_insert AND p_old_completed IS NOT NULL AND p_completed=p_old_completed)
   OR (quest_child.completes AND p_old_completed IS NULL AND p_completed=p_revision)
   OR (NOT quest_child.completes AND p_completed IS NOT DISTINCT FROM p_old_completed)))
$$;
REVOKE ALL ON FUNCTION game_npc_quest_state_proven(UUID,TEXT,NUMERIC,TEXT,BYTEA,NUMERIC,NUMERIC,BOOLEAN,NUMERIC) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_npc_quest_state_proven(UUID,TEXT,NUMERIC,TEXT,BYTEA,NUMERIC,NUMERIC,BOOLEAN,NUMERIC) TO oteryn_game_runtime;

CREATE FUNCTION game_npc_quest_parent_complete(p_parent UUID) RETURNS BOOLEAN LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE source_context game_npc_source_context%ROWTYPE; expected_child JSONB; child_count INTEGER;
BEGIN
 SELECT * INTO source_context FROM game_npc_source_context
 WHERE backend_pid=pg_backend_pid() AND transaction_xid=pg_current_xact_id()
 AND npc_transaction_id=p_parent;
 IF NOT FOUND OR jsonb_typeof(source_context.expected_plan->'quest_children') IS DISTINCT FROM 'array' THEN
 RETURN FALSE; END IF;
 IF (SELECT count(DISTINCT expected->>'quest') FROM jsonb_array_elements(source_context.expected_plan->'quest_children') expected)
 <> jsonb_array_length(source_context.expected_plan->'quest_children') THEN RETURN FALSE; END IF;
 SELECT count(*) INTO child_count FROM game_npc_quest_children
 WHERE parent_transaction_id=p_parent AND created_xact_id=pg_current_xact_id();
 IF child_count<>jsonb_array_length(source_context.expected_plan->'quest_children') THEN RETURN FALSE;END IF;
 FOR expected_child IN SELECT value FROM jsonb_array_elements(source_context.expected_plan->'quest_children') LOOP
  IF NOT EXISTS(SELECT 1 FROM game_npc_quest_children actual_child
   WHERE actual_child.parent_transaction_id=p_parent AND actual_child.quest_key=expected_child->>'quest'
   AND actual_child.character_id=source_context.character_id
   AND actual_child.program_sha256=source_context.program_sha256 AND actual_child.plan_sha256=source_context.plan_sha256
   AND actual_child.committed_revision=source_context.original_revision+1
   AND actual_child.track_changes=expected_child->'changes'
   AND actual_child.created_xact_id=pg_current_xact_id()) THEN RETURN FALSE;END IF;
 END LOOP;
 RETURN TRUE;
END $$;
REVOKE ALL ON FUNCTION game_npc_quest_parent_complete(UUID) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION game_npc_quest_parent_complete(UUID) TO oteryn_game_runtime;

-- Match the source-context owner's fixed search path; temporary relations may
-- never shadow the immutable parent, source context or Quest state tables.
DO $quest_child_paths$ DECLARE signature TEXT; BEGIN
 FOREACH signature IN ARRAY ARRAY[
 'game_npc_quest_native_versions_valid(text,text,text[],jsonb)',
 'game_npc_quest_child_admit()',
 'game_npc_quest_track_proven(uuid,text,text,numeric,bigint,bigint,boolean)',
 'game_npc_quest_child_complete()',
 'game_npc_quest_state_proven(uuid,text,numeric,text,bytea,numeric,numeric,boolean,numeric)',
 'game_npc_quest_parent_complete(uuid)'] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',signature,current_schema());
 END LOOP;
END $quest_child_paths$;

-- The generic NPC parent cannot commit a final source grant while omitting its
-- separately admitted Quest child. Non-Quest programmes retain their old guard.
CREATE FUNCTION game_npc_quest_parent_guard() RETURNS TRIGGER LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF EXISTS(SELECT 1 FROM game_npc_source_quest_policies policy
   WHERE policy.program_sha256=NEW.program_sha256 AND policy.source_key=NEW.source_key)
   AND game_npc_quest_parent_complete(NEW.transaction_id) IS DISTINCT FROM TRUE THEN
  RAISE EXCEPTION 'Final source NPC parent lacks complete Quest children' USING ERRCODE='23514';
 END IF;
 RETURN NEW;
END $$;
REVOKE ALL ON FUNCTION game_npc_quest_parent_guard() FROM PUBLIC,oteryn_game_runtime;
CREATE CONSTRAINT TRIGGER game_npc_quest_parent_guard AFTER INSERT ON game_npc_cosmetic_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_npc_quest_parent_guard();
DO $path$ BEGIN
 EXECUTE format('ALTER FUNCTION game_npc_quest_parent_guard() SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $path$;
