-- PODIUM-1. Item-owned mutable presentation; custody and house identity stay unchanged.
CREATE FUNCTION game_podium_display_valid(b BYTEA) RETURNS BOOLEAN
LANGUAGE sql IMMUTABLE STRICT AS $$
 SELECT CASE WHEN octet_length(b)<>15 THEN false ELSE
  get_byte(b,2)<=3 AND get_byte(b,13)<=3 AND get_byte(b,14)<=7
  AND NOT EXISTS(SELECT 1 FROM unnest(ARRAY[3,4,5,6,9,10,11,12]) n WHERE get_byte(b,n)>132)
  AND ((get_byte(b,14)&1)=0 OR get_byte(b,0)>0 OR get_byte(b,1)>0)
  AND ((get_byte(b,14)&2)=0 OR get_byte(b,7)>0 OR get_byte(b,8)>0)
  AND (get_byte(b,0)>0 OR get_byte(b,1)>0 OR get_byte(b,2)=0) END
$$;
CREATE TABLE game_item_podium_states (
 item_instance_id UUID PRIMARY KEY REFERENCES game_item_instances(item_instance_id),
 revision NUMERIC(20,0) NOT NULL CHECK(revision BETWEEN 1 AND 18446744073709551615),
 display BYTEA NOT NULL CHECK(game_podium_display_valid(display))
);
-- Permanent command receipt and audit, including before/after. No player names/text.
CREATE TABLE game_item_podium_records (
 game_session_id UUID NOT NULL, command_id NUMERIC(20,0) NOT NULL CHECK(command_id BETWEEN 1 AND 18446744073709551615),
 character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 item_instance_id UUID NOT NULL REFERENCES game_item_instances(item_instance_id),
 request_binding BYTEA NOT NULL CHECK(octet_length(request_binding)=32),
 revision NUMERIC(20,0) NOT NULL CHECK(revision BETWEEN 1 AND 18446744073709551615),
 before_display BYTEA NOT NULL CHECK(game_podium_display_valid(before_display)),
 after_display BYTEA NOT NULL CHECK(game_podium_display_valid(after_display)),
 written_at TIMESTAMPTZ NOT NULL DEFAULT statement_timestamp(),
 PRIMARY KEY(game_session_id,command_id), UNIQUE(item_instance_id,revision)
);
CREATE FUNCTION game_podium_record_immutable() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN RAISE EXCEPTION 'podium audit is immutable' USING ERRCODE='23514'; END $$;
CREATE TRIGGER game_podium_record_immutable BEFORE UPDATE OR DELETE ON game_item_podium_records
 FOR EACH ROW EXECUTE FUNCTION game_podium_record_immutable();

-- Retains the same session, incarnation, assignment, house-items and ACL locks as the
-- House owner. A Channel session cannot author a World-global house Item.
CREATE FUNCTION game_podium_access(p JSONB,n UUID,nr NUMERIC,secret BYTEA) RETURNS JSONB
LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE s game_durability_reconnect_sessions%ROWTYPE; r game_character_roots%ROWTYPE;
 i game_item_instances%ROWTYPE; a RECORD; h game_item_house_interior_locations%ROWTYPE;
 v game_item_podium_states%ROWTYPE;
BEGIN
 IF p IS NULL OR NOT p ?& ARRAY['session','command','character','world','house','connection','lease','generation','content','digest','item']
  OR EXISTS(SELECT 1 FROM jsonb_each(p) x WHERE x.value='null'::jsonb)
  OR NOT game_house_scope_key_valid(p->>'house') THEN RAISE EXCEPTION 'PODIUM_1630'; END IF;
 IF game_node_prove_current_incarnation(n,nr,secret) IS DISTINCT FROM true THEN RAISE EXCEPTION 'PODIUM_1631'; END IF;
 SELECT * INTO s FROM game_durability_reconnect_sessions
 WHERE game_session_id=(p->>'session')::uuid AND character_id=(p->>'character')::uuid
  AND world_id=(p->>'world')::uuid AND runtime_scope_kind=2 AND runtime_scope_world_id=world_id
  AND runtime_scope_channel_id IS NULL AND runtime_scope_house_key=p->>'house'
  AND runtime_scope_instance_id=game_house_scope_instance_id(world_id,p->>'house')
  AND current_generation=(p->>'connection')::numeric AND character_lease_generation=(p->>'lease')::numeric
  AND scope_ownership_generation=(p->>'generation')::numeric AND session_state IN(1,2) FOR SHARE;
 IF NOT FOUND THEN RAISE EXCEPTION 'PODIUM_1631'; END IF;
 PERFORM 1 FROM game_runtime_scope_assignments
 WHERE scope_key=game_house_scope_key(s.world_id,p->>'house') AND scope_kind=2 AND world_id=s.world_id
  AND house_key=p->>'house' AND state=1 AND ownership_generation=s.scope_ownership_generation
  AND holder_node_id=n AND holder_registration_revision=nr FOR SHARE;
 IF NOT FOUND THEN RAISE EXCEPTION 'PODIUM_1631'; END IF;
 SELECT * INTO r FROM game_character_roots WHERE character_id=s.character_id
  AND account_id=s.account_id AND world_id=s.world_id AND lifecycle=1 FOR UPDATE;
 IF NOT FOUND OR r.content_revision IS DISTINCT FROM p->>'content' THEN RAISE EXCEPTION 'PODIUM_1631'; END IF;
 PERFORM 1 FROM game_house_properties WHERE world_id=s.world_id AND house_key=p->>'house'
  AND content_digest=decode(p->>'digest','hex') FOR SHARE;
 IF NOT FOUND THEN RAISE EXCEPTION 'PODIUM_1631'; END IF;
 PERFORM pg_advisory_xact_lock(hashtextextended('oteryn:house-items:'||s.world_id::text||':'||(p->>'house'),0));
 SELECT q.*,count(*) OVER() AS grant_count INTO a FROM game_house_access(s.world_id,p->>'house',s.character_id) q;
 IF NOT FOUND OR a.grant_count<>1 OR a.role IS NULL OR a.role NOT IN('OWNER','SUBOWNER','GUEST') THEN RAISE EXCEPTION 'PODIUM_1632'; END IF;
 IF a.content_fenced IS DISTINCT FROM false THEN RAISE EXCEPTION 'PODIUM_1633'; END IF;
 SELECT * INTO i FROM game_item_instances WHERE item_instance_id=(p->>'item')::uuid FOR UPDATE;
 IF NOT FOUND OR i.world_id<>s.world_id OR i.lifecycle<>1 OR i.quantity<>1
  OR i.definition_family<>'Item' OR i.definition_production_key NOT IN('oteryn:item.tibia.i35973','oteryn:item.tibia.i35974')
 THEN RAISE EXCEPTION 'PODIUM_1634'; END IF;
 SELECT * INTO h FROM game_item_house_interior_locations WHERE item_instance_id=i.item_instance_id;
 IF NOT FOUND OR h.world_id<>s.world_id OR h.house_key<>p->>'house'
  OR game_house_access_tile(s.world_id,h.house_key,h.spatial_position) IS DISTINCT FROM true THEN RAISE EXCEPTION 'PODIUM_1634'; END IF;
 SELECT * INTO v FROM game_item_podium_states WHERE item_instance_id=i.item_instance_id;
 RETURN jsonb_build_object('revision',coalesce(v.revision,0)::text,
  'display',encode(coalesce(v.display,decode('000000000000000000000000000204','hex')),'hex'),
  'sex',r.sex,'position',encode(h.spatial_position,'hex'));
END $$;
CREATE FUNCTION game_podium_apply(p JSONB,n UUID,nr NUMERIC,secret BYTEA,action SMALLINT,expected NUMERIC,display BYTEA)
RETURNS JSONB LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE before JSONB; old BYTEA; next BYTEA; rev NUMERIC; binding BYTEA; receipt game_item_podium_records%ROWTYPE;
BEGIN
 IF action IS NULL OR action NOT IN(1,2) OR expected IS NULL OR expected NOT BETWEEN 0 AND 18446744073709551615
  OR expected<>trunc(expected) OR display IS NULL OR NOT game_podium_display_valid(display) THEN RAISE EXCEPTION 'PODIUM_1630'; END IF;
 PERFORM pg_advisory_xact_lock(hashtextextended('oteryn:podium-command:'||(p->>'session')||':'||(p->>'command'),0));
 binding:=sha256(convert_to(jsonb_build_object('character',p->>'character','item',p->>'item','house',p->>'house',
  'action',action,'expected',expected,'display',encode(display,'hex'))::text,'UTF8'));
 SELECT * INTO receipt FROM game_item_podium_records
  WHERE game_session_id=(p->>'session')::uuid AND command_id=(p->>'command')::numeric;
 IF FOUND THEN
  IF receipt.request_binding<>binding THEN RAISE EXCEPTION 'PODIUM_1637'; END IF;
  RETURN jsonb_build_object('revision',receipt.revision::text,'display',encode(receipt.after_display,'hex'));
 END IF;
 before:=game_podium_access(p,n,nr,secret); rev:=(before->>'revision')::numeric;
 IF rev<>expected THEN RAISE EXCEPTION 'PODIUM_1637'; END IF;
 IF rev=18446744073709551615 THEN RAISE EXCEPTION 'PODIUM_1638'; END IF;
 old:=decode(before->>'display','hex'); next:=display;
 IF action=2 THEN
  IF get_byte(old,0)=0 AND get_byte(old,1)=0 AND get_byte(old,7)=0 AND get_byte(old,8)=0 THEN RAISE EXCEPTION 'PODIUM_1634'; END IF;
  next:=set_byte(old,13,(get_byte(old,13)+1)%4);
 END IF;
 INSERT INTO game_item_podium_records(game_session_id,command_id,character_id,item_instance_id,request_binding,revision,before_display,after_display)
 VALUES((p->>'session')::uuid,(p->>'command')::numeric,(p->>'character')::uuid,(p->>'item')::uuid,binding,rev+1,old,next);
 INSERT INTO game_item_podium_states(item_instance_id,revision,display) VALUES((p->>'item')::uuid,rev+1,next)
 ON CONFLICT(item_instance_id) DO UPDATE SET revision=excluded.revision,display=excluded.display;
 RETURN jsonb_build_object('revision',(rev+1)::text,'display',encode(next,'hex'));
END $$;
REVOKE ALL ON game_item_podium_states,game_item_podium_records FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
DO $$ DECLARE f TEXT; BEGIN
 FOREACH f IN ARRAY ARRAY['game_podium_display_valid(BYTEA)','game_podium_record_immutable()',
 'game_podium_access(JSONB,UUID,NUMERIC,BYTEA)','game_podium_apply(JSONB,UUID,NUMERIC,BYTEA,SMALLINT,NUMERIC,BYTEA)'] LOOP
  EXECUTE format('ALTER FUNCTION %s SET search_path=pg_catalog,%I,pg_temp',f,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC',f);
 END LOOP;
END $$;
GRANT EXECUTE ON FUNCTION game_podium_access(JSONB,UUID,NUMERIC,BYTEA),
 game_podium_apply(JSONB,UUID,NUMERIC,BYTEA,SMALLINT,NUMERIC,BYTEA) TO oteryn_game_runtime;
