-- Isolated PostgreSQL guard qualification after the Item guard harness and0052.
-- Administrative fixture rows prove constraints, not playable source admission.
\set ON_ERROR_STOP on
SET search_path=spell_items_asset_audit,pg_catalog;
CREATE FUNCTION fixture_direct_companion(command BIGINT,kind INTEGER DEFAULT 1,mutation INTEGER DEFAULT 0)
 RETURNS UUID LANGUAGE plpgsql AS $$
DECLARE t UUID:=game_character_uuid_v7(); e UUID:=game_character_uuid_v7();
 w UUID:='10000000-0000-7000-8000-000000000003'; ch UUID:='10000000-0000-7000-8000-000000000005';
 c UUID:='10000000-0000-7000-8000-000000000001'; s UUID:='10000000-0000-7000-8000-000000000004';
 a UUID:='02020202-0202-0202-0202-020202020202'; m UUID:='01010101-0101-0101-0101-010101010101';
 actor JSONB; master JSONB; src JSONB; b JSONB; intent BYTEA; cost BYTEA; now_ms BIGINT;
BEGIN
 SELECT r.cost INTO STRICT cost FROM game_spell_item_receipts r ORDER BY command_id LIMIT 1;
 now_ms:=floor(extract(epoch FROM statement_timestamp())*1000)::bigint;
 actor:=jsonb_build_object('world',ARRAY(SELECT get_byte(uuid_send(w),n) FROM generate_series(0,15)n),
 'channel',ARRAY(SELECT get_byte(uuid_send(ch),n) FROM generate_series(0,15)n),
 'scope_generation',1,'local_id',300,'local_generation',1,
 'placement',ARRAY(SELECT get_byte(uuid_send(a),n) FROM generate_series(0,15)n));
 master:=actor||jsonb_build_object('local_id',200,'placement',ARRAY(SELECT get_byte(uuid_send(m),n) FROM generate_series(0,15)n));
 src:=CASE WHEN kind=1 THEN jsonb_build_object('kind','named','requested_name','Rat')
 ELSE jsonb_build_object('kind','target','before_snapshot_digest',array_fill(CASE WHEN mutation=4 THEN 0 ELSE 3 END,ARRAY[32]),
 'prior_master',NULL,'prior_master_session',NULL) END;
 b:=jsonb_build_object('transaction',ARRAY(SELECT get_byte(uuid_send(t),n) FROM generate_series(0,15)n),
 'actor',actor,'master',master,'character',ARRAY(SELECT get_byte(uuid_send(c),n) FROM generate_series(0,15)n),
 'session',ARRAY(SELECT get_byte(uuid_send(s),n) FROM generate_series(0,15)n),
 'cell',jsonb_build_array(10,11,7),'creature_key','canary:creature/rat','creature_revision','canary-47dfd51f','source',src);
 intent:=convert_to(jsonb_build_object('cost',ARRAY(SELECT get_byte(cost,n) FROM generate_series(0,octet_length(cost)-1)n),
 'operations','[]'::jsonb,'companion',CASE WHEN mutation=7 THEN '{}'::jsonb ELSE 'null'::jsonb END,
 'direct_companion',b,'caster_origin',jsonb_build_object('lease',1,'placement',
 CASE WHEN mutation=6 THEN array_fill(9,ARRAY[16]) ELSE array_fill(1,ARRAY[16]) END))::text,'UTF8');
 IF mutation=9 THEN
  intent:=convert_to((convert_from(intent,'UTF8')::jsonb||jsonb_build_object('direct_companion',NULL))::text,'UTF8');
 ELSIF mutation=10 THEN
  intent:=convert_to((convert_from(intent,'UTF8')::jsonb-'direct_companion')::text,'UTF8');
 END IF;
 INSERT INTO game_spell_item_receipts(transaction_id,event_id,game_session_id,command_id,character_id,world_id,channel_id,ownership_generation,
 spell_family,spell_production_key,spell_revision,catalog_digest,binding,intent,cost,cost_binding,operation_count,occurred_at_unix_ms,caster_lease_generation,caster_placement_digest)
 VALUES(t,e,s,command,c,w,ch,1,'Spell',CASE WHEN kind=1 THEN 'fixture:summon' ELSE 'fixture:convince' END,'r1',
 decode(repeat('01',32),'hex'),sha256(intent),intent,cost,sha256(cost),0,now_ms,1,
 decode(repeat(CASE WHEN mutation=6 THEN '09' ELSE '01' END,16),'hex'));
 INSERT INTO game_spell_item_audit_outbox(event_id,transaction_id,occurred_at_unix_ms,expires_at_unix_ms,envelope,envelope_sha256)
 VALUES(e,t,now_ms,now_ms+7776000000,intent,sha256(intent));
 IF mutation NOT IN (1,9,10) THEN
  INSERT INTO game_spell_direct_companion_acquisition_receipts(transaction_id,world_id,channel_id,ownership_generation,
  actor_local_id,actor_local_generation,actor_placement_identity,master_character_id,master_game_session_id,
  master_actor_local_id,master_actor_local_generation,master_placement_identity,creature_key,creature_revision,
  cell_x,cell_y,cell_z,source_kind,source_json,binding_json,created_xact_id)
  VALUES(t,w,ch,CASE WHEN mutation=3 THEN 2 ELSE 1 END,300,1,a,c,s,200,1,m,
   CASE WHEN mutation=2 THEN 'canary:creature/skeleton' ELSE 'canary:creature/rat' END,'canary-47dfd51f',
   10,11,7,CASE WHEN mutation=5 THEN 3-kind ELSE kind END,src,b,
   CASE WHEN mutation=8 THEN (SELECT created_xact_id FROM game_spell_item_receipts WHERE command_id=701) ELSE pg_current_xact_id() END);
 END IF;
 RETURN t;
END $$;
BEGIN;
SELECT fixture_direct_companion(701,1,0);
SELECT fixture_direct_companion(702,2,0);
SELECT fixture_direct_companion(703,1,9);
SELECT fixture_direct_companion(704,1,10);
SET CONSTRAINTS ALL IMMEDIATE;
COMMIT;
DO $$ DECLARE n INTEGER; BEGIN
 FOR n IN 1..8 LOOP
  BEGIN
   SET CONSTRAINTS ALL DEFERRED;
   PERFORM fixture_direct_companion(710+n,CASE WHEN n=4 THEN 2 ELSE 1 END,n);
   SET CONSTRAINTS ALL IMMEDIATE;
   RAISE EXCEPTION 'direct acquisition single-invariant mutation accepted: %',n;
  EXCEPTION WHEN check_violation THEN NULL;
  END;
  IF EXISTS(SELECT 1 FROM game_spell_item_receipts WHERE command_id=710+n) THEN
   RAISE EXCEPTION 'failed direct acquisition leaked its cost/source receipt';
  END IF;
 END LOOP;
 BEGIN UPDATE game_spell_direct_companion_acquisition_receipts SET creature_key='changed';
 RAISE EXCEPTION 'direct acquisition history changed'; EXCEPTION WHEN check_violation THEN NULL; END;
 BEGIN DELETE FROM game_spell_direct_companion_acquisition_receipts;
 RAISE EXCEPTION 'direct acquisition history deleted'; EXCEPTION WHEN check_violation THEN NULL; END;
 BEGIN TRUNCATE game_spell_direct_companion_acquisition_receipts;
 RAISE EXCEPTION 'direct acquisition history truncated'; EXCEPTION WHEN check_violation THEN NULL; END;
END $$;
SELECT 'DIRECT_COMPANION_REAL_SQL_GUARDS_PASS';
