DO $distance$
DECLARE r game_item_consume_transform_records%ROWTYPE;
BEGIN
 r.source_kind:=5;
 r.source_location:=jsonb_build_object('custody_kind','house','actor_position',jsonb_build_array(32310,32804,7),'house',jsonb_build_object('spatial_position',to_jsonb(int4send(32311)||int4send(32805)||int4send(7))));
 r.effects:=jsonb_build_object('ground',jsonb_build_object('x',32311,'y',32805,'floor',7));
 IF game_santa_gift_adjacent(r) IS DISTINCT FROM TRUE THEN RAISE EXCEPTION 'diagonal adjacent rejected'; END IF;
 r.source_location:=jsonb_set(r.source_location,'{actor_position}',jsonb_build_array(32309,32805,7));
 IF game_santa_gift_adjacent(r) IS DISTINCT FROM FALSE THEN RAISE EXCEPTION 'far source accepted'; END IF;
 r.source_location:=jsonb_set(r.source_location,'{actor_position}',jsonb_build_array(32311,32805,8));
 IF game_santa_gift_adjacent(r) IS DISTINCT FROM FALSE THEN RAISE EXCEPTION 'cross floor source accepted'; END IF;
 r.source_location:=jsonb_set(r.source_location,'{actor_position}',jsonb_build_array(32311,32805,7));
 r.effects:=jsonb_set(r.effects,'{ground,x}','32310');
 IF game_santa_gift_adjacent(r) IS DISTINCT FROM FALSE THEN RAISE EXCEPTION 'forged source position accepted'; END IF;
 r.source_location:=jsonb_set(r.source_location,'{custody_kind}','"container"');
 IF game_santa_gift_adjacent(r) IS DISTINCT FROM TRUE THEN RAISE EXCEPTION 'carried source must follow current actor'; END IF;
 RAISE NOTICE 'SRV7_SANTA_GIFT_DISTANCE_CASES_PASSED cases=5';
END $distance$;
SAVEPOINT santa_private_guard_cases;
SET LOCAL ROLE oteryn_game_runtime;
DO $$ BEGIN
 BEGIN
  INSERT INTO game_santa_house_source_context(backend_pid) VALUES(pg_backend_pid());
  RAISE EXCEPTION 'runtime forged private House source observation';
 EXCEPTION WHEN insufficient_privilege THEN RAISE NOTICE 'SANTA_HOUSE_GUARD runtime cannot forge private actor source'; END;
 BEGIN
  PERFORM game_santa_arm_house_source('00000000-0000-7000-8000-000000000001',1,decode(repeat('99',32),'hex'),
   '00000000-0000-7000-8000-000000000041','00000000-0000-7000-8000-000000000040',1,1,1,1,
   '00000000-0000-7000-8000-000000000042','oteryn:house.1','00000000-0000-7000-8000-000000000071','[1,1,7]','[1,1,7]');
  RAISE EXCEPTION 'wrong Node secret forged House source observation';
 EXCEPTION WHEN check_violation THEN RAISE NOTICE 'SANTA_HOUSE_GUARD actual Node secret required'; END;
END $$;
RESET ROLE;
RELEASE SAVEPOINT santa_private_guard_cases;
