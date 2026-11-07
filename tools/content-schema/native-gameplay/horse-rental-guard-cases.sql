DO $horse$ DECLARE caught BOOLEAN;seed BYTEA:=sha256('horse-source-test'::bytea);n INTEGER;v INTEGER;BEGIN
 caught:=FALSE;BEGIN INSERT INTO game_horse_station_worlds VALUES('01900000-0000-7000-8000-000000000042',0,0,0,TRUE);EXCEPTION WHEN check_violation THEN caught:=TRUE;END;
 IF NOT caught THEN RAISE EXCEPTION 'unarmed horse World write admitted';END IF;
 caught:=FALSE;BEGIN INSERT INTO game_horse_rental_receipts VALUES('01900000-0000-7000-8000-000000000001','01900000-0000-7000-8000-000000000041','01900000-0000-7000-8000-000000000042','01900000-0000-7000-8000-000000000050',1,'oteryn:npc.appaloosa',22,1,86401,'01900000-0000-7000-8000-000000000001',seed,2,pg_current_xact_id());EXCEPTION WHEN check_violation THEN caught:=TRUE;END;
 IF NOT caught THEN RAISE EXCEPTION 'unarmed personal rental grant admitted';END IF;
 caught:=FALSE;BEGIN PERFORM game_horse_station_catch(seed);EXCEPTION WHEN check_violation THEN caught:=TRUE;END;
 IF NOT caught THEN RAISE EXCEPTION 'unarmed horse catch admitted';END IF;
 caught:=FALSE;BEGIN INSERT INTO game_horse_rental_expiry_notices VALUES('01900000-0000-7000-8000-000000000001','01900000-0000-7000-8000-000000000050',pg_current_xact_id());EXCEPTION WHEN check_violation THEN caught:=TRUE;END;
 IF NOT caught THEN RAISE EXCEPTION 'unarmed horse expiry acknowledgement admitted';END IF;
 FOR n IN 0..255 LOOP
  v:=game_horse_station_draw(217,sha256(seed||int4send(n)),3);
  IF v NOT BETWEEN 1 AND 3 THEN RAISE EXCEPTION 'horse rental draw escaped source bounds';END IF;
 END LOOP;
 caught:=FALSE;BEGIN PERFORM game_horse_station_draw(216,seed,3);EXCEPTION WHEN check_violation THEN caught:=TRUE;END;
 IF NOT caught THEN RAISE EXCEPTION 'foreign RNG source admitted';END IF;
 RAISE NOTICE 'SRV7_HORSE_SOURCE_SECURITY_PASSED cases=6 draws=256';
END $horse$;
