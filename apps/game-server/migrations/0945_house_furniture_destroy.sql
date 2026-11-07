-- WDESTR: bounded Crystal furniture destruction, immutable custody and timer evidence.
CREATE TABLE game_house_furniture_destroy_receipts (
 transaction_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(transaction_id)),
 game_session_id UUID NOT NULL,
 command_id NUMERIC(20,0) NOT NULL CHECK(command_id BETWEEN 1 AND 18446744073709551615),
 character_id UUID NOT NULL REFERENCES game_character_roots,
 world_id UUID NOT NULL,
 house_key TEXT NOT NULL,
 source UUID NOT NULL REFERENCES game_item_instances,
 target UUID NOT NULL REFERENCES game_item_instances,
 input JSONB NOT NULL,
 outcome TEXT NOT NULL CHECK(outcome IN('COMMITTED','FAILED_ROLL')),
 before_item JSONB NOT NULL,
 after_item JSONB NOT NULL,
 old_location JSONB NOT NULL,
 remains_item JSONB,
 placements JSONB NOT NULL,
 children JSONB NOT NULL,
 timed_write JSONB,
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 UNIQUE(game_session_id,command_id)
);
CREATE INDEX game_house_destroy_character ON game_house_furniture_destroy_receipts(character_id);
CREATE INDEX game_house_destroy_source ON game_house_furniture_destroy_receipts(source);
CREATE INDEX game_house_destroy_target ON game_house_furniture_destroy_receipts(target);
CREATE UNIQUE INDEX game_house_destroy_remains ON game_house_furniture_destroy_receipts((remains_item->>'item_instance_id'));
CREATE TRIGGER game_house_destroy_immutable BEFORE UPDATE OR DELETE ON game_house_furniture_destroy_receipts
 FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_house_destroy_no_truncate BEFORE TRUNCATE ON game_house_furniture_destroy_receipts
 EXECUTE FUNCTION game_item_reject_truncate();

CREATE FUNCTION game_house_destroy_complete(p_tx UUID) RETURNS VOID
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE r game_house_furniture_destroy_receipts%ROWTYPE; proof_row JSONB; t game_item_timed_states%ROWTYPE;
BEGIN
 SELECT * INTO r FROM game_house_furniture_destroy_receipts WHERE transaction_id=p_tx;
 IF NOT FOUND OR r.created_xact_id<>pg_current_xact_id()
  OR NOT EXISTS(SELECT 1 FROM game_item_instances i WHERE to_jsonb(i)=r.after_item)
  THEN RAISE EXCEPTION 'destroy item evidence mismatch' USING ERRCODE='23514'; END IF;
 IF r.outcome='FAILED_ROLL' THEN
  IF r.before_item<>r.after_item OR r.remains_item IS NOT NULL OR r.timed_write IS NOT NULL
   OR NOT EXISTS(SELECT 1 FROM game_item_house_interior_locations h WHERE to_jsonb(h)=r.old_location)
   THEN RAISE EXCEPTION 'failed roll changed furniture' USING ERRCODE='23514'; END IF;
 ELSE
  IF r.after_item->>'lifecycle'<>'2' OR r.after_item->>'quantity'<>'0'
   OR r.after_item->>'last_transaction_id'<>r.transaction_id::text
   OR (r.after_item->>'state_revision')::numeric<>(r.before_item->>'state_revision')::numeric+1
   OR r.before_item-ARRAY['quantity','lifecycle','last_transaction_id','state_revision']
     <>r.after_item-ARRAY['quantity','lifecycle','last_transaction_id','state_revision']
   OR EXISTS(SELECT 1 FROM game_item_house_interior_locations WHERE item_instance_id=r.target)
   OR EXISTS(SELECT 1 FROM game_item_house_reclaim_provenance WHERE item_instance_id=r.target)
   OR NOT EXISTS(SELECT 1 FROM game_item_instances i WHERE to_jsonb(i)=r.remains_item)
   OR jsonb_array_length(r.placements)<>jsonb_array_length(r.children)+1
   OR (SELECT count(DISTINCT placement_json->>'placed_transaction_id') FROM jsonb_array_elements(r.placements) placement_json)
     <>jsonb_array_length(r.placements)
   THEN RAISE EXCEPTION 'incomplete destroy retirement or remains' USING ERRCODE='23514'; END IF;
  FOR proof_row IN SELECT value FROM jsonb_array_elements(r.placements) LOOP
   IF NOT EXISTS(SELECT 1 FROM game_item_house_interior_locations h
     JOIN game_item_house_reclaim_provenance p USING(item_instance_id,world_id,house_key)
     WHERE to_jsonb(h)=proof_row AND p.placement_transaction_id=h.placed_transaction_id
      AND p.reclaim_subject_character_id=r.character_id AND p.provenance_revision=1
      AND p.written_xact_id=pg_current_xact_id())
    OR proof_row->>'spatial_position'<>r.old_location->>'spatial_position'
    OR proof_row->>'world_id'<>r.world_id::text OR proof_row->>'house_key'<>r.house_key
    THEN RAISE EXCEPTION 'destroy placement lacks provenance' USING ERRCODE='23514'; END IF;
  END LOOP;
  SELECT * INTO t FROM game_item_timed_states WHERE item_instance_id=(r.remains_item->>'item_instance_id')::uuid;
  IF t.state_revision IS DISTINCT FROM 1 OR t.charges IS NOT NULL
   OR t.remaining_ms IS DISTINCT FROM (r.input->>'duration')::bigint OR t.deadline_at IS NULL
   OR NOT EXISTS(SELECT 1 FROM game_item_timed_state_writes w WHERE to_jsonb(w)-'committed_at'=r.timed_write-'committed_at')
   OR NOT EXISTS(SELECT 1 FROM game_item_timed_definitions d WHERE
    (d.definition_family,d.definition_production_key,d.definition_revision_ref)=
    (r.input->'remains'->>'family',r.input->'remains'->>'key',r.input->'remains'->>'revision')
    AND d.full_remaining_ms=(r.input->>'duration')::bigint AND d.full_charges IS NULL AND d.lit_continuous)
   THEN RAISE EXCEPTION 'destroy timer evidence mismatch' USING ERRCODE='23514'; END IF;
 END IF;
 FOR proof_row IN SELECT value FROM jsonb_array_elements(r.children) LOOP
  IF NOT EXISTS(SELECT 1 FROM game_item_instances i WHERE to_jsonb(i)=proof_row->'item')
   OR (r.outcome='FAILED_ROLL' AND NOT (
      EXISTS(SELECT 1 FROM game_item_container_entries e WHERE to_jsonb(e)=proof_row->'entry')
      OR EXISTS(SELECT 1 FROM game_item_nested_entries e WHERE to_jsonb(e)=proof_row->'entry')))
   OR (r.outcome='COMMITTED' AND (
      EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.item_instance_id=(proof_row->'item'->>'item_instance_id')::uuid)
      OR EXISTS(SELECT 1 FROM game_item_nested_entries e WHERE e.item_instance_id=(proof_row->'item'->>'item_instance_id')::uuid)
      OR NOT EXISTS(SELECT 1 FROM jsonb_array_elements(r.placements) p WHERE p->>'item_instance_id'=proof_row->'item'->>'item_instance_id')))
   THEN RAISE EXCEPTION 'destroy child evidence mismatch' USING ERRCODE='23514'; END IF;
 END LOOP;
END $$;
CREATE FUNCTION game_house_destroy_proven() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN PERFORM game_house_destroy_complete(NEW.transaction_id); RETURN NULL; END $$;
CREATE CONSTRAINT TRIGGER game_house_destroy_proven AFTER INSERT ON game_house_furniture_destroy_receipts
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_house_destroy_proven();

CREATE FUNCTION game_house_furniture_destroy(p_session UUID,p_character UUID,p_connection NUMERIC,
 p_lease NUMERIC,p_scope NUMERIC,p_node UUID,p_node_revision NUMERIC,p_world UUID,p_house TEXT,
 p_command NUMERIC,p_input JSONB) RETURNS JSONB LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE r game_house_furniture_destroy_receipts%ROWTYPE; i game_item_instances%ROWTYPE;
 s game_item_instances%ROWTYPE; remains game_item_instances%ROWTYPE;
 h game_item_house_interior_locations%ROWTYPE; location game_item_house_interior_locations%ROWTYPE;
 w game_item_timed_state_writes%ROWTYPE; a RECORD; child RECORD; guild UUID;
 tx UUID; placement UUID; ordinal NUMERIC; source_id UUID; target_id UUID;
 children JSONB:='[]'; placements JSONB:='[]'; after_item JSONB;
 revision NUMERIC; acl NUMERIC; roll INTEGER; attack INTEGER; duration BIGINT; output TEXT;
BEGIN
 IF p_input IS NULL OR p_command IS NULL OR p_command<>trunc(p_command)
  OR p_command NOT BETWEEN 1 AND 18446744073709551615 THEN RETURN jsonb_build_object('status','STALE'); END IF;
 source_id:=(p_input->>'source')::uuid; target_id:=(p_input->>'target')::uuid;
 -- Session, Character, House, then UUID-sorted items. Replay retains the current session fence.
 BEGIN
  PERFORM game_house_podium_fence(p_session,p_character,p_connection,p_lease,p_scope,p_node,p_node_revision,p_world,p_house);
 EXCEPTION WHEN raise_exception THEN
  IF SQLERRM='PODIUM_STALE_SESSION' THEN RETURN jsonb_build_object('status','STALE'); END IF;
  RAISE;
 END;
 PERFORM 1 FROM game_character_roots WHERE character_id=p_character FOR UPDATE;
 SELECT owner_guild_id INTO guild FROM game_house_properties WHERE world_id=p_world AND house_key=p_house;
 IF guild IS NOT NULL THEN PERFORM 1 FROM game_guilds WHERE guild_id=guild AND world_id=p_world FOR SHARE; END IF;
 PERFORM 1 FROM game_house_properties WHERE world_id=p_world AND house_key=p_house FOR UPDATE;
 IF NOT FOUND THEN RETURN jsonb_build_object('status','NO_ACCESS'); END IF;
 IF (SELECT owner_guild_id FROM game_house_properties WHERE world_id=p_world AND house_key=p_house) IS DISTINCT FROM guild
  THEN RETURN jsonb_build_object('status','STALE'); END IF;
 PERFORM pg_advisory_xact_lock(hashtextextended('oteryn:house-items:'||p_world::text||':'||p_house,0));
 SELECT * INTO r FROM game_house_furniture_destroy_receipts WHERE game_session_id=p_session AND command_id=p_command;
 IF FOUND THEN
  IF r.character_id<>p_character OR r.world_id<>p_world OR r.house_key<>p_house
   OR r.source IS DISTINCT FROM source_id OR r.target IS DISTINCT FROM target_id
   OR r.input->'source_definition' IS DISTINCT FROM p_input->'source_definition'
   OR r.input->'target_definition' IS DISTINCT FROM p_input->'target_definition'
   THEN RETURN jsonb_build_object('status','STALE'); END IF;
  RETURN jsonb_build_object('status','COMMITTED','outcome',r.outcome,'receipt',r.transaction_id,
   'remains',r.remains_item->>'item_instance_id');
 END IF;
 SELECT q.*,count(*) OVER() AS grant_count INTO a FROM game_house_access(p_world,p_house,p_character) q;
 IF NOT FOUND OR a.grant_count<>1 OR a.content_fenced IS DISTINCT FROM false OR a.role IS NULL
  OR a.role NOT IN('OWNER','SUBOWNER') THEN RETURN jsonb_build_object('status','NO_ACCESS'); END IF;
 revision:=(p_input->>'revision')::numeric; acl:=(p_input->>'acl')::numeric;
 roll:=(p_input->>'roll')::integer; attack:=(p_input->>'attack')::integer; duration:=(p_input->>'duration')::bigint;
 IF acl IS NULL OR a.acl_revision IS DISTINCT FROM acl OR revision IS NULL OR revision<>trunc(revision)
  OR revision NOT BETWEEN 1 AND 18446744073709551614 OR roll IS NULL OR roll NOT BETWEEN 1 AND 80
  OR attack IS NULL OR attack NOT BETWEEN 0 AND 65535 OR source_id IS NULL OR target_id IS NULL
  OR source_id=target_id THEN RETURN jsonb_build_object('status','STALE'); END IF;
 PERFORM 1 FROM game_item_instances WHERE item_instance_id IN(source_id,target_id)
  OR item_instance_id IN(SELECT item_instance_id FROM game_item_container_entries WHERE parent_item_instance_id=target_id
   UNION ALL SELECT item_instance_id FROM game_item_nested_entries WHERE parent_item_instance_id=target_id)
  ORDER BY item_instance_id FOR UPDATE;
 SELECT * INTO i FROM game_item_instances WHERE item_instance_id=target_id AND world_id=p_world;
 SELECT * INTO s FROM game_item_instances WHERE item_instance_id=source_id AND world_id=p_world;
 IF i.item_instance_id IS NULL OR s.item_instance_id IS NULL OR i.lifecycle<>1 OR s.lifecycle<>1
  OR i.quantity<>1 OR i.state_revision<>revision
  OR jsonb_build_object('family',i.definition_family,'key',i.definition_production_key,'revision',i.definition_revision_ref)
    IS DISTINCT FROM p_input->'target_definition'
  OR jsonb_build_object('family',s.definition_family,'key',s.definition_production_key,'revision',s.definition_revision_ref)
    IS DISTINCT FROM p_input->'source_definition'
  OR s.definition_family<>'Item' OR i.definition_family<>'Item'
  OR NOT (EXISTS(SELECT 1 FROM game_item_loop_owner(source_id) o WHERE o.owner_character=p_character)
   OR EXISTS(SELECT 1 FROM game_character_equipment_slots e WHERE e.item_instance_id=source_id AND e.character_id=p_character AND e.world_id=p_world))
  THEN RETURN jsonb_build_object('status','STALE'); END IF;
 SELECT * INTO h FROM game_item_house_interior_locations WHERE item_instance_id=target_id AND world_id=p_world AND house_key=p_house FOR UPDATE;
 IF NOT FOUND OR game_house_access_tile(p_world,p_house,h.spatial_position) IS DISTINCT FROM true
  OR p_input->'remains'->>'family' IS DISTINCT FROM 'Item' OR p_input->'decay'->>'family' IS DISTINCT FROM 'Item'
  OR p_input->'decay' IS DISTINCT FROM p_input->'target_definition'
  OR coalesce(length(p_input->'remains'->>'revision'),0)=0 OR duration IS NULL
  OR p_input->'remains'->>'key' IS NULL
  OR NOT ((i.definition_production_key='oteryn:item.tibia.i2295' AND p_input->'remains'->>'key'='oteryn:item.tibia.i3145' AND duration=120000)
   OR (i.definition_production_key='oteryn:item.tibia.i2296' AND p_input->'remains'->>'key'='oteryn:item.tibia.i3146' AND duration=120000)
   OR (i.definition_production_key='oteryn:item.tibia.i4995' AND p_input->'remains'->>'key'='oteryn:item.tibia.i4996' AND duration=300000))
  OR EXISTS(SELECT 1 FROM game_item_timed_states WHERE item_instance_id=target_id)
  OR EXISTS(SELECT 1 FROM game_item_corpse_container_entries WHERE parent_item_instance_id=target_id)
  THEN RETURN jsonb_build_object('status','STALE'); END IF;
 -- Weapon registration is a closed input contract, also checked independently by SQL.
 IF substring(s.definition_production_key FROM '^oteryn:item.tibia.i([0-9]+)$')::integer IS NULL
  OR NOT (substring(s.definition_production_key FROM '^oteryn:item.tibia.i([0-9]+)$')::integer = ANY(ARRAY[860, 3264, 3265, 3266, 3267, 3268, 3269, 3270, 3271, 3272, 3273, 3274, 3275, 3276, 3278, 3279, 3280, 3281, 3282, 3283, 3284, 3285, 3286, 3288, 3289, 3290, 3291, 3292, 3294, 3295, 3296, 3297, 3298, 3299, 3300, 3301, 3302, 3305, 3309, 3310, 3311, 3312, 3313, 3314, 3315, 3316, 3317, 3318, 3319, 3320, 3321, 3322, 3323, 3324, 3325, 3326, 3327, 3328, 3329, 3331, 3332, 3333, 3334, 3335, 3336, 3337, 3338, 3339, 3340, 3341, 3342, 3343, 3344, 3346, 3348, 6101, 6528, 6553, 7379, 7380, 7381, 7382, 7383, 7384, 7385, 7386, 7387, 7388, 7389, 7390, 7391, 7392, 7402, 7403, 7404, 7405, 7406, 7407, 7408, 7409, 7410, 7411, 7412, 7413, 7414, 7415, 7416, 7417, 7418, 7419, 7420, 7421, 7422, 7423, 7424, 7425, 7426, 7427, 7428, 7429, 7430, 7431, 7432, 7433, 7434, 7435, 7436, 7437, 7449, 7450, 7451, 7452, 7453, 7454, 7455, 7456, 7773, 7774, 8096, 8097, 8098, 8099, 8100, 8101, 8102, 8103, 8104, 9373, 9375, 9376, 9385, 9386, 9387, 9396, 10388, 10389, 10390, 10391, 10392, 10406, 11657, 11692, 11693, 12510, 12673, 12683, 12731, 13987, 13991, 14001, 14040, 14043, 14250, 16160, 16161, 16162, 16175, 17109, 17784, 17812, 17813, 17824, 17828, 17859, 20065, 20066, 20067, 20068, 20069, 20070, 20071, 20072, 20073, 20074, 20075, 20076, 20077, 20078, 20079, 20080, 20081, 21171, 21172, 21173, 21174, 21176, 21177, 21178, 21179, 21180, 21219, 22171, 22172, 22727, 22759, 22760, 22762, 27449, 27450, 27451, 27452, 27453, 27454, 27523, 27525, 27651, 28464, 28723, 28724, 28725, 28829, 28830, 28831, 28832, 29419, 29421, 29422, 30283, 30395, 30396, 30398, 31580, 31614, 32093, 32616, 50163, 50164, 50165, 50168]))
  THEN RETURN jsonb_build_object('status','STALE'); END IF;
 SELECT coalesce(jsonb_agg(jsonb_build_object('item',to_jsonb(c),'entry',e.entry) ORDER BY c.item_instance_id),'[]') INTO children
  FROM (SELECT item_instance_id,to_jsonb(e) entry FROM game_item_container_entries e WHERE parent_item_instance_id=target_id
   UNION ALL SELECT item_instance_id,to_jsonb(e) FROM game_item_nested_entries e WHERE parent_item_instance_id=target_id) e
  JOIN game_item_instances c USING(item_instance_id);
 IF (p_input->>'succeeds')::boolean IS DISTINCT FROM (roll<=greatest(attack,10)) THEN
  RETURN jsonb_build_object('status','STALE'); END IF;
 tx:=game_character_uuid_v7(); after_item:=to_jsonb(i); output:='FAILED_ROLL';
 IF roll<=greatest(attack,10) THEN
  output:='COMMITTED';
  after_item:=after_item||jsonb_build_object('lifecycle',2,'quantity',0,'last_transaction_id',tx,'state_revision',i.state_revision+1);
  remains:=i; remains.item_instance_id:=game_character_uuid_v7(); remains.minted_transaction_id:=game_character_uuid_v7();
  remains.last_transaction_id:=remains.minted_transaction_id; remains.state_revision:=1;
  remains.definition_family:=p_input->'remains'->>'family'; remains.definition_production_key:=p_input->'remains'->>'key';
  remains.definition_revision_ref:=p_input->'remains'->>'revision';
  SELECT coalesce(max(stack_ordinal),0) INTO ordinal FROM game_item_house_interior_locations
   WHERE world_id=p_world AND house_key=p_house AND spatial_position=h.spatial_position;
  FOR child IN SELECT (v->'item'->>'item_instance_id')::uuid id FROM jsonb_array_elements(children) v
    UNION ALL SELECT remains.item_instance_id ORDER BY id LOOP
   placement:=game_character_uuid_v7(); ordinal:=ordinal+1;
   location:=h; location.item_instance_id:=child.id; location.stack_ordinal:=ordinal; location.placed_transaction_id:=placement;
   placements:=placements||jsonb_build_array(to_jsonb(location));
  END LOOP;
  w.item_instance_id:=remains.item_instance_id; w.expected_revision:=0; w.cause:=5;
  w.transaction_id:=game_character_uuid_v7(); w.holder_character_id:=NULL;
  w.definition_before_family:=remains.definition_family; w.definition_after_family:=remains.definition_family;
  w.definition_before_production_key:=remains.definition_production_key; w.definition_after_production_key:=remains.definition_production_key;
  w.definition_before_revision_ref:=remains.definition_revision_ref; w.definition_after_revision_ref:=remains.definition_revision_ref;
  w.full_remaining_ms:=duration; w.lit_continuous:=true; w.remaining_ms_before:=duration; w.remaining_ms_after:=duration;
  w.deadline_after:=floor(extract(epoch FROM clock_timestamp())*1000)::bigint+duration;
  w.committed_at:=floor(extract(epoch FROM clock_timestamp())*1000)::bigint; w.created_xact_id:=pg_current_xact_id();
  w.training_created_xact_id:=pg_current_xact_id();
 END IF;
 INSERT INTO game_house_furniture_destroy_receipts(transaction_id,game_session_id,command_id,character_id,world_id,house_key,
  source,target,input,outcome,before_item,after_item,old_location,remains_item,placements,children,timed_write)
 VALUES(tx,p_session,p_command,p_character,p_world,p_house,source_id,target_id,p_input,output,to_jsonb(i),after_item,to_jsonb(h),
  CASE WHEN output='COMMITTED' THEN to_jsonb(remains) END,placements,children,CASE WHEN output='COMMITTED' THEN to_jsonb(w) END);
 IF output='COMMITTED' THEN
  DELETE FROM game_item_house_reclaim_provenance WHERE item_instance_id=target_id;
  DELETE FROM game_item_house_interior_locations WHERE item_instance_id=target_id;
  DELETE FROM game_item_container_entries WHERE parent_item_instance_id=target_id;
  DELETE FROM game_item_nested_entries WHERE parent_item_instance_id=target_id;
  INSERT INTO game_item_instances SELECT(remains).*;
  FOR location IN SELECT * FROM jsonb_populate_recordset(NULL::game_item_house_interior_locations,placements) LOOP
   INSERT INTO game_item_house_interior_locations SELECT(location).*;
   INSERT INTO game_item_house_reclaim_provenance(item_instance_id,world_id,house_key,reclaim_subject_character_id,placement_transaction_id,provenance_revision)
    VALUES(location.item_instance_id,p_world,p_house,p_character,location.placed_transaction_id,1);
  END LOOP;
  UPDATE game_item_instances SET lifecycle=2,quantity=0,last_transaction_id=tx,state_revision=state_revision+1 WHERE item_instance_id=target_id;
  INSERT INTO game_item_timed_definitions VALUES(remains.definition_family,remains.definition_production_key,remains.definition_revision_ref,NULL,duration,true)
   ON CONFLICT DO NOTHING;
  INSERT INTO game_item_timed_state_writes SELECT(w).* RETURNING * INTO w;
  INSERT INTO game_item_timed_states VALUES(remains.item_instance_id,NULL,duration,w.deadline_after,1);

 END IF;
 RETURN jsonb_build_object('status',output,'outcome',output,'receipt',tx,'remains',remains.item_instance_id);
EXCEPTION WHEN invalid_text_representation OR numeric_value_out_of_range THEN
 RETURN jsonb_build_object('status','STALE');
END $$;

-- Additive, receipt-bound arms; all prior mechanics retain their original proof bodies.
DO $migration$
DECLARE n TEXT; body TEXT; arm TEXT; begin_at INTEGER;
BEGIN
 FOREACH n IN ARRAY ARRAY['game_item_instance_guard','game_item_instance_change_proven',
  'game_item_mint_consistency_guard','game_house_location_proven','game_item_container_entry_removal_proven',
  'game_item_nested_entry_proven','game_item_timed_state_write_consistency_guard'] LOOP
  body:=pg_get_functiondef((n||'()')::regprocedure);
  begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
  IF begin_at=0 THEN RAISE EXCEPTION 'unexpected destroy proof body: %',n; END IF;
  IF n='game_item_instance_guard' THEN arm:=$arm$
   IF TG_OP='UPDATE' AND EXISTS(SELECT 1 FROM game_house_furniture_destroy_receipts r
    WHERE r.target=OLD.item_instance_id AND r.before_item=to_jsonb(OLD) AND r.after_item=to_jsonb(NEW)
     AND r.outcome='COMMITTED' AND r.created_xact_id=pg_current_xact_id()) THEN RETURN NEW; END IF;
$arm$;
  ELSIF n='game_item_instance_change_proven' THEN arm:=$arm$
   IF EXISTS(SELECT 1 FROM game_house_furniture_destroy_receipts r WHERE r.target=NEW.item_instance_id
    AND r.after_item=to_jsonb(NEW) AND r.created_xact_id=pg_current_xact_id()) THEN
    PERFORM game_house_destroy_complete(NEW.last_transaction_id); RETURN NULL; END IF;
$arm$;
  ELSIF n='game_item_mint_consistency_guard' THEN arm:=$arm$
   IF EXISTS(SELECT 1 FROM game_house_furniture_destroy_receipts r WHERE r.remains_item=to_jsonb(NEW)
    AND r.outcome='COMMITTED' AND r.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;
$arm$;
  ELSIF n='game_house_location_proven' THEN arm:=$arm$
   IF TG_OP='INSERT' THEN
    IF EXISTS(SELECT 1 FROM game_house_furniture_destroy_receipts r, jsonb_array_elements(r.placements) p
     WHERE p=to_jsonb(NEW) AND r.outcome='COMMITTED' AND r.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;
   ELSIF TG_OP='DELETE' THEN
    IF EXISTS(SELECT 1 FROM game_house_furniture_destroy_receipts r WHERE r.old_location=to_jsonb(OLD)
     AND r.outcome='COMMITTED' AND r.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;
   END IF;
$arm$;
  ELSIF n IN('game_item_container_entry_removal_proven','game_item_nested_entry_proven') THEN arm:=$arm$
   IF TG_OP='DELETE' AND EXISTS(SELECT 1 FROM game_house_furniture_destroy_receipts r,
    jsonb_array_elements(r.children) c WHERE c->'entry'=to_jsonb(OLD)
     AND r.outcome='COMMITTED' AND r.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;
$arm$;
  ELSE arm:=$arm$
   IF NEW.cause=5 AND EXISTS(SELECT 1 FROM game_house_furniture_destroy_receipts r
    WHERE r.timed_write-'committed_at'=to_jsonb(NEW)-'committed_at'
     AND r.outcome='COMMITTED' AND r.created_xact_id=pg_current_xact_id()) THEN RETURN NULL; END IF;
$arm$;
  END IF;
  EXECUTE overlay(body placing E'BEGIN\n'||arm from begin_at for 5);
 END LOOP;
 -- Spilled containers keep their subtree. Retain the exclusivity count and every other guard.
 body:=pg_get_functiondef('game_item_location_exclusive(UUID)'::regprocedure);
 IF position('RAISE EXCEPTION ''HouseInterior item cannot have contents''' IN body)=0 THEN
  RAISE EXCEPTION 'missing House contents guard'; END IF;
 body:=replace(body,'RAISE EXCEPTION ''HouseInterior item cannot have contents''',
  'IF EXISTS(SELECT 1 FROM game_house_furniture_destroy_receipts r,jsonb_array_elements(r.children) c
    WHERE c->''item''->>''item_instance_id''=p_item::text AND r.outcome=''COMMITTED'') THEN RETURN; END IF;
   RAISE EXCEPTION ''HouseInterior item cannot have contents''');
 EXECUTE body;
END $migration$;
DO $$
DECLARE f REGPROCEDURE;
BEGIN
 FOREACH f IN ARRAY ARRAY['game_house_destroy_complete(UUID)'::regprocedure,
  'game_house_destroy_proven()'::regprocedure,
  'game_house_furniture_destroy(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC,UUID,TEXT,NUMERIC,JSONB)'::regprocedure] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',f,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC,oteryn_game_runtime,oteryn_game_control',f);
 END LOOP;
END $$;
REVOKE ALL ON game_house_furniture_destroy_receipts FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
GRANT SELECT ON game_house_furniture_destroy_receipts TO oteryn_game_runtime,oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_house_destroy_complete(UUID),
 game_house_furniture_destroy(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC,UUID,TEXT,NUMERIC,JSONB)
 TO oteryn_game_runtime;

-- Deadline expiry uses the same timed rows and immutable writes, with the exact target
-- retained by the destruction receipt. There is no additional scheduling table.
CREATE FUNCTION game_house_destroy_expiry_complete(w game_item_timed_state_writes) RETURNS BOOLEAN
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE r game_house_furniture_destroy_receipts%ROWTYPE; expected JSONB;
BEGIN
 SELECT * INTO r FROM game_house_furniture_destroy_receipts
  WHERE remains_item->>'item_instance_id'=w.item_instance_id::text AND outcome='COMMITTED';
 IF NOT FOUND OR w.cause<>4 OR w.expected_revision<>1 OR w.created_xact_id<>pg_current_xact_id()
  OR w.holder_character_id IS NOT NULL OR w.deadline_before IS DISTINCT FROM (r.timed_write->>'deadline_after')::bigint
  OR w.committed_at<w.deadline_before OR w.deadline_after IS NOT NULL
  OR w.charges_before IS NOT NULL OR w.charges_after IS NOT NULL OR w.remaining_ms_after IS NOT NULL
  OR w.remaining_ms_before IS DISTINCT FROM (r.input->>'duration')::bigint
  OR w.full_remaining_ms IS NOT NULL OR w.full_charges IS NOT NULL OR w.lit_continuous
  OR jsonb_build_object('family',w.definition_before_family,'key',w.definition_before_production_key,'revision',w.definition_before_revision_ref)
   IS DISTINCT FROM r.input->'remains'
  OR jsonb_build_object('family',w.definition_after_family,'key',w.definition_after_production_key,'revision',w.definition_after_revision_ref)
   IS DISTINCT FROM r.input->'decay' THEN RETURN false; END IF;
 expected:=r.remains_item||jsonb_build_object('definition_family',w.definition_after_family,
  'definition_production_key',w.definition_after_production_key,'definition_revision_ref',w.definition_after_revision_ref,
  'last_transaction_id',w.transaction_id,'state_revision',2);
 RETURN EXISTS(SELECT 1 FROM game_item_instances i JOIN game_item_timed_states t USING(item_instance_id)
  WHERE i.item_instance_id=w.item_instance_id AND to_jsonb(i)=expected AND t.state_revision=2
   AND t.charges IS NULL AND t.remaining_ms IS NULL AND t.deadline_at IS NULL)
  AND EXISTS(SELECT 1 FROM game_item_house_interior_locations h
   WHERE h.item_instance_id=w.item_instance_id AND h.world_id=r.world_id AND h.house_key=r.house_key);
END $$;
CREATE FUNCTION game_house_furniture_destroy_expire(p_session UUID,p_character UUID,p_connection NUMERIC,
 p_lease NUMERIC,p_scope NUMERIC,p_node UUID,p_node_revision NUMERIC,p_world UUID,p_house TEXT)
 RETURNS INTEGER LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE c RECORD; w game_item_timed_state_writes%ROWTYPE; total INTEGER:=0;
BEGIN
 PERFORM game_house_podium_fence(p_session,p_character,p_connection,p_lease,p_scope,p_node,p_node_revision,p_world,p_house);
 PERFORM 1 FROM game_character_roots WHERE character_id=p_character FOR UPDATE;
 PERFORM 1 FROM game_house_properties WHERE world_id=p_world AND house_key=p_house FOR UPDATE;
 PERFORM pg_advisory_xact_lock(hashtextextended('oteryn:house-items:'||p_world::text||':'||p_house,0));
 FOR c IN SELECT r.input,t.*,i.definition_family,i.definition_production_key,i.definition_revision_ref
  FROM game_item_timed_states t JOIN game_item_instances i USING(item_instance_id)
  JOIN game_item_house_interior_locations h USING(item_instance_id)
  JOIN game_house_furniture_destroy_receipts r ON r.remains_item->>'item_instance_id'=i.item_instance_id::text
  WHERE h.world_id=p_world AND h.house_key=p_house AND i.lifecycle=1 AND to_jsonb(i)=r.remains_item
   AND t.state_revision=1 AND t.deadline_at<=floor(extract(epoch FROM clock_timestamp())*1000)::bigint
  ORDER BY i.item_instance_id LIMIT 16 FOR UPDATE OF i,t LOOP
  w:=NULL; w.item_instance_id:=c.item_instance_id; w.expected_revision:=1; w.cause:=4;
  w.transaction_id:=game_character_uuid_v7(); w.definition_before_family:=c.definition_family;
  w.definition_before_production_key:=c.definition_production_key; w.definition_before_revision_ref:=c.definition_revision_ref;
  w.definition_after_family:=c.input->'decay'->>'family'; w.definition_after_production_key:=c.input->'decay'->>'key';
  w.definition_after_revision_ref:=c.input->'decay'->>'revision'; w.lit_continuous:=false;
  w.remaining_ms_before:=c.remaining_ms; w.deadline_before:=c.deadline_at;
  INSERT INTO game_item_timed_state_writes SELECT(w).* RETURNING * INTO w;
  UPDATE game_item_instances SET definition_family=w.definition_after_family,
   definition_production_key=w.definition_after_production_key,definition_revision_ref=w.definition_after_revision_ref,
   last_transaction_id=w.transaction_id,state_revision=state_revision+1 WHERE item_instance_id=c.item_instance_id;
  UPDATE game_item_timed_states SET remaining_ms=NULL,deadline_at=NULL,state_revision=state_revision+1
   WHERE item_instance_id=c.item_instance_id;
  total:=total+1;
 END LOOP;
 RETURN total;
END $$;
DO $migration$
DECLARE n TEXT; body TEXT; arm TEXT; begin_at INTEGER;
BEGIN
 FOREACH n IN ARRAY ARRAY['game_item_instance_guard','game_item_instance_change_proven',
  'game_item_timed_state_write_consistency_guard'] LOOP
  body:=pg_get_functiondef((n||'()')::regprocedure);
  begin_at:=regexp_instr(body,E'\\mBEGIN\\M',1,1,0,'i');
  IF begin_at=0 THEN RAISE EXCEPTION 'unexpected destroy expiry proof body'; END IF;
  IF n='game_item_instance_guard' THEN arm:=$arm$
   IF TG_OP='UPDATE' AND EXISTS(SELECT 1 FROM game_house_furniture_destroy_receipts r
    JOIN game_item_timed_state_writes w ON w.item_instance_id=(r.remains_item->>'item_instance_id')::uuid
    WHERE r.remains_item=to_jsonb(OLD) AND w.cause=4 AND w.expected_revision=1
     AND w.created_xact_id=pg_current_xact_id() AND w.transaction_id=NEW.last_transaction_id
     AND to_jsonb(NEW)=r.remains_item||jsonb_build_object('definition_family',r.input->'decay'->>'family',
      'definition_production_key',r.input->'decay'->>'key','definition_revision_ref',r.input->'decay'->>'revision',
      'last_transaction_id',w.transaction_id,'state_revision',2)) THEN RETURN NEW; END IF;
$arm$;
  ELSIF n='game_item_instance_change_proven' THEN arm:=$arm$
   IF EXISTS(SELECT 1 FROM game_item_timed_state_writes w WHERE w.item_instance_id=NEW.item_instance_id
    AND w.transaction_id=NEW.last_transaction_id AND game_house_destroy_expiry_complete(w)) THEN RETURN NULL; END IF;
$arm$;
  ELSE arm:=$arm$
   IF game_house_destroy_expiry_complete(NEW) THEN RETURN NULL; END IF;
$arm$;
  END IF;
  EXECUTE overlay(body placing E'BEGIN\n'||arm from begin_at for 5);
 END LOOP;
END $migration$;
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_house_destroy_expiry_complete(game_item_timed_state_writes) SET search_path=pg_catalog,%I,pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_house_furniture_destroy_expire(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC,UUID,TEXT) SET search_path=pg_catalog,%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_house_destroy_expiry_complete(game_item_timed_state_writes),
 game_house_furniture_destroy_expire(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC,UUID,TEXT)
 FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
GRANT EXECUTE ON FUNCTION game_house_destroy_expiry_complete(game_item_timed_state_writes),
 game_house_furniture_destroy_expire(UUID,UUID,NUMERIC,NUMERIC,NUMERIC,UUID,NUMERIC,UUID,TEXT) TO oteryn_game_runtime;
